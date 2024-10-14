use std::sync::Arc;

use axum::{extract::State, routing::post, Json, Router};
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    CookieJar,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{self, DiscordTokenInfo},
    state::ApiState,
};

use super::error::ApiError;

pub fn router() -> Router<Arc<ApiState>> {
    Router::new().route("/discord", post(discord_signin))
}

#[derive(Deserialize, Debug)]
pub struct DiscordSignInInfo {
    code: String,
}

#[derive(Serialize)]
pub struct SignInResp {
    username: String,
}

#[axum::debug_handler]
pub async fn discord_signin(
    State(state): State<Arc<ApiState>>,
    jar: CookieJar,
    Json(info): Json<DiscordSignInInfo>,
) -> Result<(CookieJar, Json<SignInResp>), ApiError> {
    let v = match auth::get_discord_token(info.code).await? {
        DiscordTokenInfo::Ok(token) => token,
        DiscordTokenInfo::Error {
            error,
            error_description,
            ..
        } => {
            tracing::error!(error = ?error, error_description = ?error_description, "discord auth error");
            return Err(ApiError::Unauthorized);
        }
    };

    let prof = auth::get_user_profile(&v.access_token).await?;
    auth::join_server(&v.access_token, "1280182480997060628", &prof.id).await?;

    let user_exists = sqlx::query!(
        "SELECT id, username FROM users where discord_id = $1",
        prof.id
    )
    .fetch_optional(&state.pg.inner)
    .await?;

    if let Some(user) = user_exists {
        let code = state.tokens.create_code(&state.pg, user.id).await?;
        let token = state.tokens.sign_token(user.id, code)?;
        let mut cookie = Cookie::new("token", token);
        cookie.set_http_only(true);
        cookie.set_same_site(SameSite::Strict);
        cookie.set_secure(true);
        cookie.set_path("/");

        return Ok((
            jar.add(cookie),
            Json(SignInResp {
                username: user.username,
            }),
        ));
    }

    let returned = sqlx::query!(
        "INSERT INTO users(discord_id, username, refresh_token) values ($1, $2, $3) RETURNING *",
        prof.id,
        prof.username,
        v.refresh_token
    )
    .fetch_one(&state.pg.inner)
    .await?;

    let code = state.tokens.create_code(&state.pg, returned.id).await?;
    let token = state.tokens.sign_token(returned.id, code)?;

    let mut cookie = Cookie::new("token", token);
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Strict);
    cookie.set_secure(true);
    cookie.set_path("/");

    return Ok((
        jar.add(cookie),
        Json(SignInResp {
            username: prof.username,
        }),
    ));
}
