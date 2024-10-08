use std::sync::Arc;

use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

use crate::auth::{self, GithubTokenInfo};

use super::{error::ApiError, state::ApiState};

pub fn router() -> Router<Arc<ApiState>> {
    Router::new().route("/gh_singin", post(github_signin))
}

#[derive(Deserialize, Debug)]
pub struct GithubSignInInfo {
    code: String,
}

#[derive(Serialize)]
pub struct GithubSignInResp {
    token: String,
    username: String,
}

pub async fn github_signin(
    State(state): State<Arc<ApiState>>,
    Json(info): Json<GithubSignInInfo>,
) -> Result<Json<GithubSignInResp>, ApiError> {
    let token = auth::get_github_token(info.code).await?;
    let GithubTokenInfo::Ok { access_token, .. } = token else {
        return Err(ApiError::Unauthorized);
    };

    let user = auth::get_github_user_info(&access_token).await?;
    let user_exists = sqlx::query!(
        "SELECT id, username FROM users where github_id = $1",
        user.id
    )
    .fetch_optional(&state.pg.inner)
    .await?;

    if let Some(user) = user_exists {
        let code = state.tokens.create_code(&state.pg, user.id).await?;
        let token = state.tokens.sign_token(user.id, code)?;
        return Ok(Json(GithubSignInResp {
            token,
            username: user.username,
        }));
    }

    let returned = sqlx::query!(
        "INSERT INTO users(github_id, username) values ($1, $2) RETURNING *",
        user.id,
        user.login
    )
    .fetch_one(&state.pg.inner)
    .await?;

    let code = state.tokens.create_code(&state.pg, returned.id).await?;
    let token = state.tokens.sign_token(returned.id, code)?;

    Ok(Json(GithubSignInResp {
        token,
        username: returned.username,
    }))
}
