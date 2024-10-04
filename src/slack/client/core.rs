use std::fmt::{Debug, Display};

use serde::Serialize;

pub struct Client {
    access_token: Option<String>,
}

impl Client {
    pub fn new() -> Self {
        Self {
            access_token: None,
        }
    }

    pub fn with_access_token(&mut self, access_token: String) -> &mut Self {
        self.access_token = Some(access_token);
        self
    }

    pub async fn post_form(
        &self,
        url: &str,
        body: & (impl Serialize + Debug),
    ) -> Result<Response, Error> {
        let client = reqwest::Client::new();

        log::trace!("sending post request to {}: {:?}", url, body);

        let res = client.post(url)
            .form(&body)
            .send()
            .await?
            .into();


        log::trace!(
            "received response from post request to {}: {:?}",
            url,
            res
        );

        Ok(res)
    }

    pub async fn get(
        &self,
        url: &str,
        query: Option<& (impl Serialize + Debug)>
    ) -> Result<Response, Error> {
        let client = reqwest::Client::new();

        log::trace!("sending get request to {}: {:?}", url, query);

        let mut req = client.get(url);

        if let Some(query) = query {
            req = req.query(query);
        }

        if let Some(access_token) = &self.access_token {
            req = req.bearer_auth(access_token);
        }

        let res = req.send().await?.into();

        log::trace!(
            "received response from get request to {}: {:?}",
            url,
            res
        );

        Ok(res)
    }

}

#[derive(Debug)]
pub struct Response {
    response: reqwest::Response,
}

impl Response {
    pub async fn text(self) -> Result<String, Error> {
        self.response.text().await.map_err(|err| err.into())
    }
}

impl From<reqwest::Response> for Response {
    fn from(response: reqwest::Response) -> Self {
        Self {
            response,
        }
    }
}

#[derive(Debug)]
pub struct Error {
    pub message: String,
}

impl <T: Display> From<T> for Error {
    fn from(err: T) -> Self {
        Self {
            message: err.to_string(),
        }
    }
}
