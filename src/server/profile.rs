use std::sync::Arc;

use axum::{extract::State, routing::get, Extension, Json, Router};
use serde::Serialize;
use uuid::Uuid;

use crate::{auth::user::TokenClaims, state::ApiState};

use super::error::ApiError;

pub fn router() -> Router<Arc<ApiState>> {
    Router::new().route("/profile", get(user_profile))
}

#[derive(Serialize)]
pub struct UserProfile {
    id: Uuid,
    discord_id: String,
    username: String,
}

pub async fn user_profile(
    State(state): State<Arc<ApiState>>,
    Extension(claims): Extension<TokenClaims>,
) -> anyhow::Result<Json<UserProfile>, ApiError> {
    let profile = sqlx::query!(
        "SELECT id, username, discord_id FROM users where id = $1",
        claims.sub
    )
    .fetch_one(&state.pg.inner)
    .await?;

    Ok(Json(UserProfile {
        discord_id: profile.discord_id,
        username: profile.username,
        id: profile.id,
    }))
}
