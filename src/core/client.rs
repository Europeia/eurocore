use quick_xml::de::from_str;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::instrument;

use crate::core::error::{ConfigError, Error};
use crate::core::telegram::Telegrammer;
use crate::ns::dispatch::Dispatch;
use crate::utils::ratelimiter::Ratelimiter;

#[derive(Clone, Debug)]
pub(crate) struct Client {
    ratelimiter: Ratelimiter,
    client: Arc<Mutex<reqwest::Client>>,
    url: String,
    pub(crate) nations: HashMap<String, String>,
    pin: Arc<RwLock<Option<String>>>,
    dispatch_id_re: Regex,
    pub(crate) telegram_queue: Telegrammer,
}

impl Client {
    pub(crate) fn new(
        user: &str,
        nations: HashMap<String, String>,
        telegram_client_key: String,
    ) -> Result<Self, ConfigError> {
        let client = reqwest::ClientBuilder::new().user_agent(user).build()?;

        let ratelimiter = Ratelimiter::new(
            50,
            std::time::Duration::from_millis(30_050),
            std::time::Duration::from_millis(30_050),
            std::time::Duration::from_millis(180_050),
        );

        let telegram_queue = Telegrammer::new(user, telegram_client_key, ratelimiter.clone())?;

        let url = "https://www.nationstates.net/cgi-bin/api.cgi".to_string();
        let dispatch_id_re = Regex::new(r#"(\d+)"#)?;

        Ok(Self {
            ratelimiter,
            client: Arc::new(Mutex::new(client)),
            url,
            nations,
            pin: Arc::new(RwLock::new(None)),
            dispatch_id_re,
            telegram_queue,
        })
    }

    async fn get_pin(&self) -> String {
        let pin = self.pin.read().await;

        String::from((*pin).as_deref().unwrap_or_default())
    }

    #[instrument(skip_all)]
    pub(crate) async fn authenticated_request(
        &mut self,
        query: String,
        password: &str,
    ) -> Result<String, Error> {
        tracing::debug!("Acquiring ratelimiter");
        self.ratelimiter.acquire().await;
        tracing::debug!("Ratelimiter acquired");

        let client = self.client.lock().await;

        tracing::debug!("Executing request: {}", query);
        let resp = (*client)
            .post(&self.url)
            .header("X-Password", password)
            .header("X-Pin", self.get_pin().await)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(query)
            .send()
            .await?
            .error_for_status()?;

        drop(client);

        if let Some(val) = resp.headers().get("X-Pin") {
            tracing::debug!("Updating pin: {:?}", &val);
            let mut pin = self.pin.write().await;

            *pin = Some(val.to_str().map_err(Error::HeaderDecode)?.to_string());
        }

        let body = resp.text().await?;

        Ok(body)
    }

    #[instrument(skip_all)]
    async fn dispatch(&mut self, mut dispatch: Dispatch) -> Result<String, Error> {
        let password = self
            .nations
            .get(&dispatch.nation)
            .ok_or(Error::InvalidNation)?
            .to_string();

        if let Some(text) = dispatch.text.as_mut() {
            *text = convert_to_latin_charset(text);
        }

        let query = serde_urlencoded::to_string(dispatch.clone())?;

        tracing::debug!("Executing prepare request");
        let response =
            from_str::<Response>(&(self.authenticated_request(query, &password).await?))?;

        if !response.is_ok() {
            tracing::error!("Error: {:?}", response.error);
            return Err(Error::Placeholder);
        }

        dispatch.set_mode(crate::ns::dispatch::Mode::Execute);
        dispatch.set_token(response.success.unwrap());

        let query = serde_urlencoded::to_string(dispatch).map_err(Error::URLEncode)?;

        tracing::debug!("Executing execute request");
        let response = from_str::<Response>(&(self.authenticated_request(query, &password).await?))
            .map_err(Error::Deserialize)?;

        match response.is_ok() {
            true => Ok(response.success.unwrap()),
            false => {
                tracing::error!("Error: {:?}", response.error);
                Err(Error::Placeholder)
            }
        }
    }

    #[instrument(skip_all)]
    pub(crate) async fn new_dispatch(&mut self, dispatch: Dispatch) -> Result<i32, Error> {
        let message = &self.dispatch(dispatch).await?;

        match self.dispatch_id_re.captures(message) {
            Some(captures) => Ok(captures[0].to_string().parse::<i32>()?),
            None => Err(Error::Placeholder),
        }
    }

    #[instrument(skip_all)]
    pub(crate) async fn delete_dispatch(&mut self, dispatch: Dispatch) -> Result<(), Error> {
        match self.dispatch(dispatch).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
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
