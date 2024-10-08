mod auth;
mod db;
mod server;
mod storage;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    tracing_subscriber::fmt::init();

    server::create_server().await?;
    Ok(())
}
