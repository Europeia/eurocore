use quick_xml::de;
use regex::Regex;
use serde::Deserialize;
use tracing::instrument;

use crate::core::error::{ConfigError, Error};
use crate::ns::dispatch;
use crate::ns::dispatch::Action;
use crate::ns::nation::NationList;
use crate::ns::telegram::{Telegram, TgType};
use crate::utils::ratelimiter::{Ratelimiter, Target};

#[derive(Clone, Debug)]
pub(crate) struct Client {
    pub(crate) ratelimiter: Ratelimiter,
    client: reqwest::Client,
    url: String,
    nations: NationList,
    dispatch_id_regex: Regex,
}

impl Client {
    pub(crate) fn new(
        user: &str,
        nations: NationList,
        ratelimiter: Ratelimiter,
    ) -> Result<Self, ConfigError> {
        let client = reqwest::ClientBuilder::new().user_agent(user).build()?;

        let url = String::from("https://www.nationstates.net/cgi-bin/api.cgi");

        Ok(Self {
            ratelimiter,
            client,
            url,
            nations,
            dispatch_id_regex: Regex::new(r#"(\d+)"#)?,
        })
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
        let _resp = self
            .client
            .get(&self.url)
            .query(&query)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    #[instrument(skip_all)]
    pub(crate) async fn contains_nation(&self, nation: &str) -> bool {
        self.nations.contains_nation(nation).await
    }

    #[instrument(skip_all)]
    async fn dispatch(
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
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?
            .error_for_status()?)
    }

    #[instrument(skip_all)]
    pub(crate) async fn post_dispatch(
        &mut self,
        mut dispatch: dispatch::IntermediateDispatch,
    ) -> Result<i32, Error> {
        let password = self.nations.get_password(&dispatch.nation).await?;

        let dispatch_id = match dispatch.action {
            Action::Add { .. } => None,
            Action::Edit { id, .. } => Some(id),
            Action::Remove { id, .. } => Some(id),
        };

        match &mut dispatch.action {
            Action::Add { ref mut text, .. } => {
                *text = convert_to_latin_charset(text);

                self.ratelimiter
                    .acquire_for(Target::Restricted(&dispatch.nation))
                    .await
            }
            Action::Edit { ref mut text, .. } => {
                *text = convert_to_latin_charset(text);

                self.ratelimiter.acquire_for(Target::Standard).await
            }
            Action::Remove { .. } => self.ratelimiter.acquire_for(Target::Standard).await,
        }

        let mut dispatch = dispatch::Dispatch::from(dispatch);

        let resp = self
            .dispatch(
                &password,
                &self.nations.get_pin(&dispatch.nation).await?,
                serde_urlencoded::to_string(dispatch.clone())?,
            )
            .await?;

        if let Some(val) = resp.headers().get("X-Pin") {
            self.nations
                .set_pin(&dispatch.nation, val.to_str().map_err(Error::HeaderDecode)?)
                .await?;
        }

        let response = de::from_str::<Response>(&resp.text().await?)?;

        if !response.is_ok() {
            tracing::error!("Error: {:?}", response.error);
            return Err(Error::Placeholder);
        }

        dispatch.set_mode(dispatch::Mode::Execute);
        dispatch.set_token(response.success.unwrap());

        self.ratelimiter.acquire_for(Target::Standard).await;

        let resp = self
            .dispatch(
                &password,
                &self.nations.get_pin(&dispatch.nation).await?,
                serde_urlencoded::to_string(dispatch)?,
            )
            .await?;

        let response = de::from_str::<Response>(&resp.text().await?)?;

        if response.is_ok() {
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
            tracing::error!("Error: {:?}", response.error);
            Err(Error::Placeholder)
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

fn convert_to_latin_charset(input: &str) -> String {
    input
        .replace("’", "'")
        .replace("“", "\"")
        .replace("”", "\"")
        .replace("—", "-")
        .replace("–", "-")
        .replace("…", "...")
        .replace("‘", "'")
}
