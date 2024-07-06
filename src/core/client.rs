use serde::Deserialize;
use quick_xml::de::from_str;
use tokio::sync::{Mutex, RwLock};
use std::sync::Arc;
use regex::Regex;
use tracing::instrument;

use crate::core::error::Error;
use crate::types::ns::Dispatch;
use crate::utils::ratelimiter::Ratelimiter;

#[derive(Clone, Debug)]
pub(crate) struct Client {
    ratelimiter: Ratelimiter,
    client: Arc<Mutex<reqwest::Client>>,
    user: String,
    url: String,
    pub(crate) nation: String,
    password: String,
    pin: Arc<RwLock<Option<String>>>,
    dispatch_id_re: Regex,
}

impl Client {
    pub(crate) fn new(user: &str, nation: String, password: String) -> Result<Self, Error> {
        let client = reqwest::ClientBuilder::new()
            .user_agent(user)
            .build()
            .map_err(Error::ReqwestClientBuild)?;

        let user = user.to_string();
        let url = "https://www.nationstates.net/cgi-bin/api.cgi".to_string();
        let dispatch_id_re = Regex::new(r#"(\d+)"#)
            .map_err(Error::Regex)?;

        Ok(
            Self {
                ratelimiter: Ratelimiter::new(),
                client: Arc::new(Mutex::new(client)),
                user,
                url,
                nation,
                password,
                pin: Arc::new(RwLock::new(None)),
                dispatch_id_re,
            }
        )
    }

    async fn get_pin(&self) -> String {
        let pin = self.pin.read().await;

        String::from((*pin).as_deref().unwrap_or_default())
    }

    #[instrument(skip_all)]
    async fn request(&mut self, query: String) -> Result<String, Error> {
        tracing::debug!("Acquiring ratelimiter");
        self.ratelimiter.acquire().await;
        tracing::debug!("Ratelimiter acquired");

        let client = self.client.lock().await;

        tracing::debug!("Executing request: {}", &query);
        let resp = (*client).post(&self.url)
            .header("X-Password", &self.password)
            .header("X-Pin", self.get_pin().await)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(query)
            .send()
            .await
            .map_err(Error::HTTPClient)?
            .error_for_status();

        drop(client);

        let resp = match resp {
            Ok(resp) => resp,
            Err(e) => {
                return Err(Error::ExternalServer(e));
            }
        };

        if let Some(val) = resp.headers().get("X-Pin") {
            tracing::debug!("Updating pin: {:?}", &val);
            let mut pin = self.pin.write().await;

            *pin = Some(val.to_str()
                .map_err(Error::HeaderDecode)?.to_string());
        }

        let body = resp.text().await.map_err(Error::ExternalServer)?;

        Ok(body)
    }

    #[instrument(skip_all)]
    pub(crate) async fn add_dispatch(&mut self, dispatch: Dispatch) -> Result<String, Error> {
        let message = &self.dispatch(dispatch).await?;

        return match self.dispatch_id_re.captures(message) {
            Some(captures) => Ok(captures[0].to_string()),
            None => Err(Error::Placeholder)
        };
    }

    #[instrument(skip_all)]
    pub(crate) async fn delete_dispatch( &mut self, dispatch: Dispatch) -> Result<(), Error> {
        return match self.dispatch(dispatch).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    #[instrument(skip_all)]
    async fn dispatch(&mut self, mut dispatch: Dispatch) -> Result<String, Error> {
        let query = serde_urlencoded::to_string(dispatch.clone())
            .map_err(Error::URLEncode)?;

        tracing::debug!("Executing prepare request");
        let response = from_str::<Response>(&(self.request(query).await?))
            .map_err(Error::Deserialize)?;

        if !response.is_ok() {
            return Err(Error::Placeholder);
        }

        dispatch.set_mode(crate::types::ns::Mode::Execute);
        dispatch.set_token(response.success.unwrap());

        let query = serde_urlencoded::to_string(dispatch)
            .map_err(Error::URLEncode)?;

        tracing::debug!("Executing execute request");
        let response = from_str::<Response>(&(self.request(query).await?))
            .map_err(Error::Deserialize)?;

        return match response.is_ok() {
            true => Ok(response.success.unwrap()),
            false => Err(Error::Placeholder),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
struct Response {
    success: Option<String>,
    error: Option<String>
}

impl Response {
    fn is_ok(&self) -> bool {
        self.success.is_some()
    }

    fn is_err(&self) -> bool {
        self.error.is_some()
    }
}