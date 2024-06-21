use crate::core::error::Error;
use crate::utils::ratelimiter::Ratelimiter;

pub(crate) struct Client {
    ratelimiter: Ratelimiter,
    client: reqwest::Client,
    user: String,
}

impl Client {
    pub(crate) fn new(user: &str) -> Result<Self, Error> {
        let client = reqwest::ClientBuilder::new()
            .user_agent(&user)
            .build()
            .map_err(|e| Error::ReqwestClientBuildError(e))?;

        let user = user.to_string();

        Ok(
            Self {
                ratelimiter: Ratelimiter::new(),
                client,
                user
            }
        )
    }

    pub(crate) async fn request(&self, url: &str) -> Result<(), Error> {
        self.ratelimiter.acquire().await;

        Ok(())
    }
}