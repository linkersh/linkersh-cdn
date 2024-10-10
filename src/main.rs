use std::sync::Arc;

use state::ApiState;

mod auth;
mod db;
mod ocr;
mod server;
mod state;
mod storage;
mod tasks;
mod meili;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    tracing_subscriber::fmt::init();

    let state = Arc::new(ApiState::new().await?);

    tasks::start_service(Arc::clone(&state))?;
    server::create_server(state).await?;
    Ok(())
}
