use std::{str::FromStr, sync::Arc};

use state::ApiState;
use uuid::Uuid;

mod auth;
mod db;
mod meili;
mod ocr;
mod server;
mod state;
mod storage;
mod tasks;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    tracing_subscriber::fmt::init();

    let state = Arc::new(ApiState::new().await?);

    state
        .meili
        .index_object(
            Uuid::new_v4(),
            Uuid::from_str("ed899d51-0c98-4400-8d1d-14c988d4692d")?,
            "ticket".to_owned(),
        )
        .await?;

    state
        .meili
        .index_object(
            Uuid::new_v4(),
            Uuid::from_str("ed899d51-0c98-4400-8d1d-14c988d4692d")?,
            "ticket".to_owned(),
        )
        .await?;
    state
        .meili
        .index_object(
            Uuid::new_v4(),
            Uuid::from_str("ed899d51-0c98-4400-8d1d-14c988d4692d")?,
            "ticket".to_owned(),
        )
        .await?;
    state
        .meili
        .index_object(
            Uuid::new_v4(),
            Uuid::from_str("ed899d51-0c98-4400-8d1d-14c988d4692d")?,
            "ticket".to_owned(),
        )
        .await?;
    state
        .meili
        .index_object(
            Uuid::new_v4(),
            Uuid::from_str("ed899d51-0c98-4400-8d1d-14c988d4692d")?,
            "ticket".to_owned(),
        )
        .await?;
    state
        .meili
        .index_object(
            Uuid::new_v4(),
            Uuid::from_str("ed899d51-0c98-4400-8d1d-14c988d4692d")?,
            "ticket".to_owned(),
        )
        .await?;
    state
        .meili
        .index_object(
            Uuid::new_v4(),
            Uuid::from_str("ed899d51-0c98-4400-8d1d-14c988d4692d")?,
            "ticket".to_owned(),
        )
        .await?;
    state
        .meili
        .index_object(
            Uuid::new_v4(),
            Uuid::from_str("ed899d51-0c98-4400-8d1d-14c988d4692d")?,
            "ticket".to_owned(),
        )
        .await?;
    state
        .meili
        .index_object(
            Uuid::new_v4(),
            Uuid::from_str("ed899d51-0c98-4400-8d1d-14c988d4692d")?,
            "ticket".to_owned(),
        )
        .await?;
    state
        .meili
        .index_object(
            Uuid::new_v4(),
            Uuid::from_str("ed899d51-0c98-4400-8d1d-14c988d4692d")?,
            "ticket".to_owned(),
        )
        .await?;
    state
        .meili
        .index_object(
            Uuid::new_v4(),
            Uuid::from_str("ed899d51-0c98-4400-8d1d-14c988d4692d")?,
            "ticket".to_owned(),
        )
        .await?;
    state
        .meili
        .index_object(
            Uuid::new_v4(),
            Uuid::from_str("ed899d51-0c98-4400-8d1d-14c988d4692d")?,
            "ticket".to_owned(),
        )
        .await?;
    state
        .meili
        .index_object(
            Uuid::new_v4(),
            Uuid::from_str("ed899d51-0c98-4400-8d1d-14c988d4692d")?,
            "ticket".to_owned(),
        )
        .await?;
    state
        .meili
        .index_object(
            Uuid::new_v4(),
            Uuid::from_str("ed899d51-0c98-4400-8d1d-14c988d4692d")?,
            "ticket".to_owned(),
        )
        .await?;
    state
        .meili
        .index_object(
            Uuid::new_v4(),
            Uuid::from_str("ed899d51-0c98-4400-8d1d-14c988d4692d")?,
            "ticket".to_owned(),
        )
        .await?;
    state
        .meili
        .index_object(
            Uuid::new_v4(),
            Uuid::from_str("ed899d51-0c98-4400-8d1d-14c988d4692d")?,
            "ticket".to_owned(),
        )
        .await?;
    state
        .meili
        .index_object(
            Uuid::new_v4(),
            Uuid::from_str("ed899d51-0c98-4400-8d1d-14c988d4692d")?,
            "ticket".to_owned(),
        )
        .await?;
    state
        .meili
        .index_object(
            Uuid::new_v4(),
            Uuid::from_str("ed899d51-0c98-4400-8d1d-14c988d4692d")?,
            "ticket".to_owned(),
        )
        .await?;

    tasks::start_service(Arc::clone(&state))?;
    server::create_server(state).await?;
    Ok(())
}
