//! Microsoft account auth flow and lightweight account-source parsing.

use crate::auth::OnlineAccount;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

pub const MICROSOFT_CLIENT_ID: &str = "00000000402b5328";
pub const MICROSOFT_REDIRECT_URI: &str = "https://login.live.com/oauth20_desktop.srf";
pub const MICROSOFT_SCOPE: &str = "XboxLive.signin offline_access";

const MICROSOFT_TOKEN_URL: &str = "https://login.live.com/oauth20_token.srf";
const XBOX_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MINECRAFT_LOGIN_URL: &str =
    "https://api.minecraftservices.com/authentication/login_with_xbox";
const MINECRAFT_ENTITLEMENTS_URL: &str = "https://api.minecraftservices.com/entitlements/mcstore";
const MINECRAFT_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MicrosoftSession {
    pub account: OnlineAccount,
    pub refresh_token: String,
    pub expires_at: SystemTime,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AccountSourceKind {
    LauncherAccessToken,
    RefreshToken,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountSource {
    pub source_label: String,
    pub username: String,
    pub profile_id: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub kind: AccountSourceKind,
}

impl AccountSource {
    pub fn online_account(&self) -> Option<OnlineAccount> {
        Some(OnlineAccount {
            username: self.username.clone(),
            profile_id: self.profile_id.clone()?,
            access_token: self.access_token.clone()?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MicrosoftAuthError {
    MissingCode,
    MissingRefreshToken,
    MissingResponseField(&'static str),
    MissingMinecraftOwnership,
    HttpStatus { status: u16, body: String },
    Parse(String),
    Transport(String),
    Xbox(String),
}

pub fn microsoft_login_url() -> String {
    format!(
        "https://login.live.com/oauth20_authorize.srf?client_id={MICROSOFT_CLIENT_ID}&response_type=code&redirect_uri={MICROSOFT_REDIRECT_URI}&scope=XboxLive.signin%20offline_access&prompt=login"
    )
}

pub fn authenticate_with_refresh_token(
    refresh_token: &str,
) -> Result<MicrosoftSession, MicrosoftAuthError> {
    if refresh_token.trim().is_empty() {
        return Err(MicrosoftAuthError::MissingRefreshToken);
    }

    let response = post_form_json(
        MICROSOFT_TOKEN_URL,
        &[
            ("client_id", MICROSOFT_CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("redirect_uri", MICROSOFT_REDIRECT_URI),
            ("refresh_token", refresh_token),
            ("scope", MICROSOFT_SCOPE),
        ],
    )?;

    let access_token = require_string(&response, "access_token")?;
    let refresh_token = response
        .get("refresh_token")
        .and_then(Value::as_str)
        .unwrap_or(refresh_token)
        .to_owned();
    let expires_in = response
        .get("expires_in")
        .and_then(Value::as_u64)
        .unwrap_or(3600);
    let expires_at = SystemTime::now()
        .checked_add(Duration::from_secs(expires_in.saturating_sub(30)))
        .unwrap_or(SystemTime::now());

    authenticate_with_microsoft_access_token(&access_token, refresh_token, expires_at)
}

pub fn authenticate_with_authorization_input(
    input: &str,
) -> Result<MicrosoftSession, MicrosoftAuthError> {
    let code = extract_authorization_code(input)?;
    let response = post_form_json(
        MICROSOFT_TOKEN_URL,
        &[
            ("client_id", MICROSOFT_CLIENT_ID),
            ("grant_type", "authorization_code"),
            ("redirect_uri", MICROSOFT_REDIRECT_URI),
            ("code", &code),
        ],
    )?;

    let access_token = require_string(&response, "access_token")?;
    let refresh_token = require_string(&response, "refresh_token")?;
    let expires_in = response
        .get("expires_in")
        .and_then(Value::as_u64)
        .unwrap_or(3600);
    let expires_at = SystemTime::now()
        .checked_add(Duration::from_secs(expires_in.saturating_sub(30)))
        .unwrap_or(SystemTime::now());

    authenticate_with_microsoft_access_token(&access_token, refresh_token, expires_at)
}

pub fn load_account_sources_from_path(
    path: &Path,
) -> Result<Vec<AccountSource>, MicrosoftAuthError> {
    let contents = fs::read_to_string(path)
        .map_err(|error| MicrosoftAuthError::Transport(error.to_string()))?;
    load_account_sources_from_json(&contents)
}

pub fn load_account_sources_from_json(
    contents: &str,
) -> Result<Vec<AccountSource>, MicrosoftAuthError> {
    let root: Value = serde_json::from_str(contents)
        .map_err(|error| MicrosoftAuthError::Parse(error.to_string()))?;
    let mut sources = Vec::new();

    if let Some(accounts) = root.get("Accounts").and_then(Value::as_array) {
        for account in accounts {
            let username = optional_string(account, "Name")
                .or_else(|| optional_string(account, "name"))
                .unwrap_or_else(|| "Unknown".to_owned());
            let profile_id = optional_string(account, "UUID")
                .or_else(|| optional_string(account, "uuid"))
                .map(|value| normalize_profile_id(&value));
            let refresh_token = optional_string(account, "Refresh Token")
                .or_else(|| optional_string(account, "refresh_token"));

            if let Some(refresh_token) = refresh_token {
                sources.push(AccountSource {
                    source_label: "custom".to_owned(),
                    username,
                    profile_id,
                    access_token: None,
                    refresh_token: Some(refresh_token),
                    kind: AccountSourceKind::RefreshToken,
                });
            }
        }
    }

    if let Some(accounts) = root.get("accounts").and_then(Value::as_object) {
        for account in accounts.values() {
            let profile = account.get("minecraftProfile").unwrap_or(&Value::Null);
            let username = optional_string(profile, "name")
                .or_else(|| optional_string(account, "username"))
                .unwrap_or_else(|| "Unknown".to_owned());
            let profile_id =
                optional_string(profile, "id").map(|value| normalize_profile_id(&value));
            let access_token = optional_string(account, "accessToken");

            if access_token.is_some() && profile_id.is_some() {
                sources.push(AccountSource {
                    source_label: "launcher".to_owned(),
                    username,
                    profile_id,
                    access_token,
                    refresh_token: None,
                    kind: AccountSourceKind::LauncherAccessToken,
                });
            }
        }
    }

    Ok(sources)
}

fn authenticate_with_microsoft_access_token(
    microsoft_access_token: &str,
    refresh_token: String,
    expires_at: SystemTime,
) -> Result<MicrosoftSession, MicrosoftAuthError> {
    let (xbox_token, user_hash) = xbox_live_authenticate(microsoft_access_token)?;
    let xsts_token = xbox_xsts_authorize(&xbox_token)?;
    let minecraft_token = minecraft_login_with_xbox(&user_hash, &xsts_token)?;
    ensure_minecraft_ownership(&minecraft_token)?;
    let (username, profile_id) = minecraft_profile(&minecraft_token)?;

    Ok(MicrosoftSession {
        account: OnlineAccount {
            username,
            profile_id,
            access_token: minecraft_token,
        },
        refresh_token,
        expires_at,
    })
}

fn xbox_live_authenticate(access_token: &str) -> Result<(String, String), MicrosoftAuthError> {
    let request = serde_json::json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={access_token}"),
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT",
    });
    let response = post_json(XBOX_AUTH_URL, &request)?;
    let token = require_string(&response, "Token")?;
    let uhs = response
        .get("DisplayClaims")
        .and_then(|claims| claims.get("xui"))
        .and_then(Value::as_array)
        .and_then(|entries| entries.first())
        .and_then(|entry| entry.get("uhs"))
        .and_then(Value::as_str)
        .ok_or(MicrosoftAuthError::MissingResponseField(
            "DisplayClaims.xui[0].uhs",
        ))?;
    Ok((token, uhs.to_owned()))
}

fn xbox_xsts_authorize(xbox_token: &str) -> Result<String, MicrosoftAuthError> {
    let request = serde_json::json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbox_token],
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT",
    });
    let response = post_json(XSTS_AUTH_URL, &request)?;

    if let Some(code) = response.get("XErr").and_then(Value::as_i64) {
        return Err(MicrosoftAuthError::Xbox(code.to_string()));
    }

    require_string(&response, "Token")
}

fn minecraft_login_with_xbox(uhs: &str, xsts_token: &str) -> Result<String, MicrosoftAuthError> {
    let request = serde_json::json!({
        "identityToken": format!("XBL3.0 x={uhs};{xsts_token}"),
    });
    let response = post_json(MINECRAFT_LOGIN_URL, &request)?;
    require_string(&response, "access_token")
}

fn ensure_minecraft_ownership(minecraft_token: &str) -> Result<(), MicrosoftAuthError> {
    let response = get_json(
        MINECRAFT_ENTITLEMENTS_URL,
        &[("Authorization", &format!("Bearer {minecraft_token}"))],
    )?;
    let owns_minecraft = response
        .get("items")
        .and_then(Value::as_array)
        .map(|items| {
            items.iter().any(|item| {
                item.get("name")
                    .and_then(Value::as_str)
                    .map(|name| matches!(name, "product_minecraft" | "game_minecraft"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    if owns_minecraft {
        Ok(())
    } else {
        Err(MicrosoftAuthError::MissingMinecraftOwnership)
    }
}

fn minecraft_profile(minecraft_token: &str) -> Result<(String, String), MicrosoftAuthError> {
    let response = get_json(
        MINECRAFT_PROFILE_URL,
        &[("Authorization", &format!("Bearer {minecraft_token}"))],
    )?;
    Ok((
        require_string(&response, "name")?,
        normalize_profile_id(&require_string(&response, "id")?),
    ))
}

fn extract_authorization_code(input: &str) -> Result<String, MicrosoftAuthError> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Err(MicrosoftAuthError::MissingCode);
    }

    if let Some((_, rest)) = trimmed.split_once("code=") {
        let code = rest.split('&').next().unwrap_or_default();
        if !code.is_empty() {
            return Ok(code.to_owned());
        }
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Err(MicrosoftAuthError::MissingCode);
    }

    Ok(trimmed.to_owned())
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn normalize_profile_id(value: &str) -> String {
    value.replace('-', "")
}

fn require_string(value: &Value, key: &'static str) -> Result<String, MicrosoftAuthError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(MicrosoftAuthError::MissingResponseField(key))
}

fn post_form_json(url: &str, fields: &[(&str, &str)]) -> Result<Value, MicrosoftAuthError> {
    let body = fields
        .iter()
        .map(|(key, value)| format!("{key}={}", urlencoding::encode(value)))
        .collect::<Vec<_>>()
        .join("&");
    parse_response(
        ureq::post(url)
            .set("Content-Type", "application/x-www-form-urlencoded")
            .send_string(&body),
    )
}

fn post_json(url: &str, body: &Value) -> Result<Value, MicrosoftAuthError> {
    parse_response(ureq::post(url).send_json(body.clone()))
}

fn get_json(url: &str, headers: &[(&str, &str)]) -> Result<Value, MicrosoftAuthError> {
    let mut request = ureq::get(url);
    for (name, value) in headers {
        request = request.set(name, value);
    }
    parse_response(request.call())
}

fn parse_response(
    response: Result<ureq::Response, ureq::Error>,
) -> Result<Value, MicrosoftAuthError> {
    match response {
        Ok(response) => response
            .into_json()
            .map_err(|error| MicrosoftAuthError::Parse(error.to_string())),
        Err(ureq::Error::Status(status, response)) => Err(MicrosoftAuthError::HttpStatus {
            status: status as u16,
            body: response.into_string().unwrap_or_default(),
        }),
        Err(ureq::Error::Transport(error)) => Err(MicrosoftAuthError::Transport(error.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        extract_authorization_code, load_account_sources_from_json, microsoft_login_url,
        AccountSourceKind,
    };

    #[test]
    fn extracts_code_from_callback_url() {
        assert_eq!(
            extract_authorization_code(
                "https://login.live.com/oauth20_desktop.srf?code=abc123&lc=1041"
            )
            .expect("code should extract"),
            "abc123"
        );
    }

    #[test]
    fn accepts_raw_authorization_code() {
        assert_eq!(
            extract_authorization_code("M.CODE").expect("raw code should be accepted"),
            "M.CODE"
        );
    }

    #[test]
    fn parses_custom_refresh_token_accounts() {
        let sources = load_account_sources_from_json(
            r#"{"Accounts":[{"Name":"b4UmU","UUID":"b2ac6244ab984ecbbe23fc59ee4b079c","Refresh Token":"refresh"}]}"#,
        )
        .expect("custom account file should parse");

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].username, "b4UmU");
        assert_eq!(sources[0].kind, AccountSourceKind::RefreshToken);
        assert_eq!(sources[0].refresh_token.as_deref(), Some("refresh"));
    }

    #[test]
    fn parses_launcher_access_token_accounts() {
        let sources = load_account_sources_from_json(
            r#"{"accounts":{"abc":{"accessToken":"token","username":"user@example.com","minecraftProfile":{"id":"b2ac6244-ab98-4ecb-be23-fc59ee4b079c","name":"b4UmU"}}}}"#,
        )
        .expect("launcher account file should parse");

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].username, "b4UmU");
        assert_eq!(sources[0].kind, AccountSourceKind::LauncherAccessToken);
        assert_eq!(
            sources[0].profile_id.as_deref(),
            Some("b2ac6244ab984ecbbe23fc59ee4b079c")
        );
        assert_eq!(sources[0].access_token.as_deref(), Some("token"));
    }

    #[test]
    fn login_url_targets_desktop_callback() {
        let url = microsoft_login_url();
        assert!(url.contains("response_type=code"));
        assert!(url.contains("oauth20_desktop.srf"));
    }
}
