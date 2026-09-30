//! Mojang session join support for online-mode login.

use serde::Serialize;

pub const MOJANG_JOIN_SERVER_URL: &str = "https://sessionserver.mojang.com/session/minecraft/join";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OnlineAccount {
    pub username: String,
    pub profile_id: String,
    pub access_token: String,
}

impl OnlineAccount {
    pub fn normalized_profile_id(&self) -> Result<String, AuthError> {
        let normalized = self.profile_id.replace('-', "");

        if normalized.is_empty() {
            Err(AuthError::InvalidProfileId)
        } else {
            Ok(normalized)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthError {
    InvalidProfileId,
    HttpStatus { status: u16, body: String },
    Transport(String),
}

pub trait SessionJoiner {
    fn join_server(&self, account: &OnlineAccount, server_hash: &str) -> Result<(), AuthError>;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MojangSessionJoiner;

#[derive(Serialize)]
struct JoinServerRequest<'a> {
    #[serde(rename = "accessToken")]
    access_token: &'a str,
    #[serde(rename = "selectedProfile")]
    selected_profile: &'a str,
    #[serde(rename = "serverId")]
    server_id: &'a str,
}

impl SessionJoiner for MojangSessionJoiner {
    fn join_server(&self, account: &OnlineAccount, server_hash: &str) -> Result<(), AuthError> {
        let normalized_profile = account.normalized_profile_id()?;
        let request = JoinServerRequest {
            access_token: &account.access_token,
            selected_profile: &normalized_profile,
            server_id: server_hash,
        };

        match ureq::post(MOJANG_JOIN_SERVER_URL).send_json(request) {
            Ok(_) => Ok(()),
            Err(ureq::Error::Status(status, response)) => Err(AuthError::HttpStatus {
                status: status as u16,
                body: response.into_string().unwrap_or_default(),
            }),
            Err(ureq::Error::Transport(error)) => Err(AuthError::Transport(error.to_string())),
        }
    }
}
