use std::env;

use anyhow::Ok;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

pub mod user;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum GithubTokenInfo {
    Ok {
        access_token: String,
    },
    Error {
        error: Option<String>,
        error_description: Option<String>,
        error_uri: Option<String>,
    },
}

pub async fn get_github_token(code: String) -> anyhow::Result<GithubTokenInfo> {
    let github_client_id = env::var("GITHUB_CLIENT_ID")?;
    let github_client_secret = env::var("GITHUB_CLIENT_SECRET")?;
    let github_redirect_uri = env::var("GITHUB_REDIRECT_URI")?;

    let client = Client::new();
    let request = client
        .post("https://github.com/login/oauth/access_token")
        .json(&json!({
            "client_id": github_client_id,
            "client_secret": github_client_secret,
            "code": code,
            "redirect_uri": github_redirect_uri
        }))
        .send()
        .await?;

    let resp = request.text().await?;
    let token_info: GithubTokenInfo = serde_urlencoded::from_str(&resp)?;
    Ok(token_info)
}

#[derive(Deserialize, Debug)]
pub struct GithubUser {
    pub login: String,
    pub id: i64,
}

pub async fn get_github_user_info(token: &str) -> anyhow::Result<GithubUser> {
    let client = Client::new();
    let request = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "Linker.sh User Agent")
        .send()
        .await?;
    let resp: GithubUser = request.json().await?;
    Ok(resp)
}
