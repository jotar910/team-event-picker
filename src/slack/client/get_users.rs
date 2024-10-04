use serde::{Deserialize, Serialize};

use crate::slack::client;

#[derive(Serialize, Debug)]
pub struct ClientQuery {
    pub team_id: String,
}

#[derive(Deserialize)]
pub struct ClientResponse {
    pub ok: bool,
    pub error: Option<String>,
    #[serde(default = "Vec::new")]
    pub members: Vec<ClientUser>,
}

impl From<String> for ClientResponse {
    fn from(body: String) -> Self {
        serde_json::from_str(&body)
            .expect("failed to parse response")
    }
}

#[derive(Deserialize, Debug)]
pub struct ClientUser {
    pub id: String,
    pub name: String,
    #[serde(default = "String::new")]
    pub real_name: String,
}

#[derive(Debug, Clone)]
pub struct Client {
    pub team_id: String,
    pub access_token: String,
}

pub fn new(team_id: String, access_token: String) -> Client {
    Client {
        team_id,
        access_token,
    }
}

impl Client {
    pub async fn execute(self) -> Result<ClientResponse, client::Error> {
        Ok(client::Client::new()
            .with_access_token(self.access_token)
            .get(
                "https://slack.com/api/users.list",
                Some(&ClientQuery {
                    team_id: self.team_id,
                }),
            )
            .await?
            .text()
            .await?
            .into())
    }
}
