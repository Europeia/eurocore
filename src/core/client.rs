use htmlentity::entity::ICodedDataTrait;
use htmlentity::{
    self,
    entity::{CharacterSet, EncodeType},
};
use quick_xml::de;
use regex::Regex;
use serde::Deserialize;
use tracing::instrument;

use crate::core::error::{ConfigError, Error};
use crate::ns::dispatch;
use crate::ns::dispatch::Action;
use crate::ns::nation::NationList;
use crate::ns::rmbpost::IntermediateRmbPost;
use crate::ns::telegram::{Telegram, TgType};
use crate::ns::types::Mode;
use crate::utils::ratelimiter::{Ratelimiter, Target};

#[derive(Clone)]
pub(crate) struct Client {
    pub(crate) ratelimiter: Ratelimiter,
    client: reqwest::Client,
    url: String,
    dispatch_nations: NationList,
    rmbpost_nations: NationList,
    dispatch_id_regex: Regex,
    rmbpost_id_regex: Regex,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("ratelimiter", &self.ratelimiter)
            .field("dispatch_nations", &self.dispatch_nations)
            .field("rmbpost_nations", &self.rmbpost_nations)
            .finish()
    }
}

impl Client {
    pub(crate) fn new(
        user: &str,
        dispatch_nations: NationList,
        rmbpost_nations: NationList,
        ratelimiter: Ratelimiter,
    ) -> Result<Self, ConfigError> {
        let client = reqwest::ClientBuilder::new().user_agent(user).build()?;

        let url = String::from("https://www.nationstates.net/cgi-bin/api.cgi");

        Ok(Self {
            ratelimiter,
            client,
            url,
            dispatch_nations,
            rmbpost_nations,
            dispatch_id_regex: Regex::new(r#"(\d+)"#)?,
            rmbpost_id_regex: Regex::new(r#"=(\d+)#"#)?,
        })
    }

    #[instrument(skip_all)]
    pub(crate) async fn get_dispatch_nation_names(&self) -> Vec<String> {
        self.dispatch_nations.get_nation_names().await
    }

    #[instrument(skip_all)]
    pub(crate) async fn contains_dispatch_nation(&self, nation: &str) -> bool {
        self.dispatch_nations.contains_nation(nation).await
    }

    #[instrument(skip_all)]
    pub(crate) async fn get_rmbpost_nation_names(&self) -> Vec<String> {
        self.rmbpost_nations.get_nation_names().await
    }

    #[instrument(skip_all)]
    pub(crate) async fn send_telegram(&self, telegram: Telegram) -> Result<(), Error> {
        match telegram.tg_type {
            TgType::Recruitment => {
                self.ratelimiter
                    .acquire_for(Target::Restricted(&telegram.sender))
                    .await;
            }
            TgType::Standard => {
                self.ratelimiter
                    .acquire_for(Target::Telegram(&telegram.sender))
                    .await;
            }
        }

        // do i need this? return to this at some point
        let query = serde_urlencoded::to_string(telegram)?;

        tracing::debug!("Sending telegram");
        self.client
            .get(&self.url)
            .query(&query)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    #[instrument(skip_all)]
    async fn execute(
        &mut self,
        password: &str,
        pin: &str,
        body: String,
    ) -> Result<reqwest::Response, Error> {
        Ok(self
            .client
            .post(&self.url)
            .header("X-Password", password)
            .header("X-Pin", pin)
            .header(
                "Content-Type",
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .body(body)
            .send()
            .await?
            .error_for_status()?)
    }

    fn encode(&self, input: &str) -> String {
        input
            .chars()
            .map(|char| {
                if char.is_ascii() {
                    char.to_string()
                } else {
                    htmlentity::entity::encode(
                        char.encode_utf8(&mut [0; 4]).as_bytes(),
                        &EncodeType::Decimal,
                        &CharacterSet::All,
                    )
                    .to_string()
                    .unwrap()
                }
            })
            .collect()
    }

    #[instrument(skip_all)]
    pub(crate) async fn post_dispatch(
        &mut self,
        mut dispatch: dispatch::IntermediateDispatch,
    ) -> Result<i32, Error> {
        let password = self.dispatch_nations.get_password(&dispatch.nation).await?;

        let dispatch_id = match dispatch.action {
            Action::Add { .. } => None,
            Action::Edit { id, .. } => Some(id),
            Action::Remove { id, .. } => Some(id),
        };

        match &mut dispatch.action {
            Action::Add { ref mut text, .. } => {
                *text = self.encode(text.as_str());

                self.ratelimiter
                    .acquire_for(Target::Restricted(&dispatch.nation))
                    .await
            }
            Action::Edit { ref mut text, .. } => {
                *text = self.encode(text.as_str());

                self.ratelimiter.acquire_for(Target::Standard).await
            }
            Action::Remove { .. } => self.ratelimiter.acquire_for(Target::Standard).await,
        }

        let mut dispatch = dispatch::Dispatch::from(dispatch);

        let resp = self
            .execute(
                &password,
                &self.dispatch_nations.get_pin(&dispatch.nation).await?,
                serde_urlencoded::to_string(dispatch.clone())?,
            )
            .await?;

        if let Some(val) = resp.headers().get("X-Pin") {
            self.dispatch_nations
                .set_pin(&dispatch.nation, val.to_str().map_err(Error::HeaderDecode)?)
                .await?;
        }

        let response = de::from_str::<Response>(&resp.text().await?)?;

        if !response.is_ok() {
            return Err(Error::NationStates(response.error.unwrap()));
        }

        dispatch.set_mode(Mode::Execute);
        dispatch.set_token(response.success.unwrap());

        self.ratelimiter.acquire_for(Target::Standard).await;

        let resp = self
            .execute(
                &password,
                &self.dispatch_nations.get_pin(&dispatch.nation).await?,
                serde_urlencoded::to_string(dispatch)?,
            )
            .await?;

        let response = de::from_str::<Response>(&resp.text().await?)?;

        if response.is_ok() {
            // is this a stupid way to do this? idk, maybe
            // but also, the only instance where dispatch_id will be None is for a new dispatch
            // in which case, the response returned from NS 100% contains the id for the new dispatch
            // it would be so much cooler if we could always reply on the response containing the id
            // but alas
            match dispatch_id {
                Some(id) => Ok(id),
                None => Ok(self
                    .dispatch_id_regex
                    .find(&response.success.unwrap())
                    .unwrap()
                    .as_str()
                    .parse()?),
            }
        } else {
            Err(Error::NationStates(response.error.unwrap()))
        }
    }

    #[instrument(skip_all)]
    pub(crate) async fn post_rmbpost(
        &mut self,
        mut rmbpost: IntermediateRmbPost,
    ) -> Result<i32, Error> {
        let password = self.rmbpost_nations.get_password(&rmbpost.nation).await?;

        rmbpost.text = self.encode(&rmbpost.text);

        self.ratelimiter
            .acquire_for(Target::Restricted(&rmbpost.nation))
            .await;

        let rmbpost = crate::ns::rmbpost::RmbPost::from(rmbpost);

        let resp = self
            .execute(
                &password,
                &self.rmbpost_nations.get_pin(rmbpost.nation()).await?,
                serde_urlencoded::to_string(rmbpost.clone())?,
            )
            .await?;

        if let Some(val) = resp.headers().get("X-Pin") {
            self.rmbpost_nations
                .set_pin(rmbpost.nation(), val.to_str().map_err(Error::HeaderDecode)?)
                .await?;
        }

        let response = de::from_str::<Response>(&resp.text().await?)?;

        if !response.is_ok() {
            return Err(Error::NationStates(response.error.unwrap()));
        }

        let rmbpost = rmbpost.prepare(response.success.unwrap());

        self.ratelimiter.acquire_for(Target::Standard).await;

        let resp = self
            .execute(
                &password,
                &self.rmbpost_nations.get_pin(rmbpost.nation()).await?,
                serde_urlencoded::to_string(rmbpost)?,
            )
            .await?;

        let response = de::from_str::<Response>(&resp.text().await?)?;

        if response.is_ok() {
            Ok(self
                .rmbpost_id_regex
                .captures(&response.success.unwrap())
                .unwrap()[1]
                .parse()?)
        } else {
            Err(Error::NationStates(response.error.unwrap()))
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
struct Response {
    success: Option<String>,
    error: Option<String>,
}

impl Response {
    fn is_ok(&self) -> bool {
        self.success.is_some()
    }
}
