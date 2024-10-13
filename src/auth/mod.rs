use std::{collections::HashMap, env};

use anyhow::Ok;
use reqwest::Client;
use serde::Deserialize;

pub mod user;

#[derive(Deserialize, Debug)]
pub struct DiscordToken {
    pub access_token: String,
    // pub token_type: String,
    // pub expires_in: u64,
    pub refresh_token: String,
    // pub scope: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum DiscordTokenInfo {
    Ok(DiscordToken),
    Error {
        error: Option<String>,
        error_description: Option<String>,
        error_uri: Option<String>,
    },
}

pub async fn get_discord_token(code: String) -> anyhow::Result<DiscordTokenInfo> {
    let discord_client_id = env::var("DISCORD_CLIENT_ID")?;
    let discord_client_secret = env::var("DISCORD_CLIENT_SECRET")?;
    let discord_redirect_uri = env::var("DISCORD_REDIRECT_URI")?;

    let client = Client::new();
    let mut data = HashMap::new();

    data.insert("grant_type".to_owned(), "authorization_code".to_owned());
    data.insert("code".to_owned(), code);
    data.insert("redirect_uri".to_owned(), discord_redirect_uri);

    let response = client
        .post("https://discord.com/api/oauth2/token")
        .basic_auth(discord_client_id, Some(discord_client_secret))
        .form(&data)
        .send()
        .await?;

    let resp: DiscordTokenInfo = response.json().await?;
    Ok(resp)
}

#[derive(Deserialize, Debug)]
pub struct DiscordUserProfile {
    pub username: String,
    pub id: String,
}

pub async fn get_user_profile(access: &str) -> anyhow::Result<DiscordUserProfile> {
    let client = Client::new();

    let request = client
        .get("https://discord.com/api/v10/users/@me")
        .header("Authorization", format!("Bearer {access}"))
        .send()
        .await?;

    let resp: DiscordUserProfile = request.json().await?;
    Ok(resp)
}

// #[derive(Deserialize)]
// pub struct DiscordUserServer {
//     pub id: String,
//     pub name: String,
// }

// pub async fn get_user_servers(access: &str) -> anyhow::Result<Vec<DiscordUserServer>> {
//     let client = Client::new();

//     let request = client
//         .get("https://discord.com/api/v10/users/@me/guilds")
//         .header("Authorization", format!("Bearer {access}"))
//         .send()
//         .await?;

//     let resp: Vec<DiscordUserServer> = request.json().await?;
//     Ok(resp)
// }

pub async fn join_server(access: &str, guild_id: &str, user_id: &str) -> anyhow::Result<()> {
    let discord_bot_token = env::var("DISCORD_TOKEN")?;

    let client = Client::new();
    let request = client
        .put(format!(
            "https://discord.com/api/v10/guilds/{guild_id}/members/{user_id}"
        ))
        .header("Authorization", format!("Bot {discord_bot_token}"))
        .json(&serde_json::json!({ "access_token": access }))
        .send()
        .await?;

    if !request.status().is_success() {
        let resp = request.text().await?;
        tracing::info!(error = ?resp, "failed to join {user_id} to guild {guild_id}");
    }

    Ok(())
}
