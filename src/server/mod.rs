use axum::{routing::get, Router};
use middleware::AuthLayer;
use std::{env, sync::Arc};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    trace::{self, TraceLayer},
};
use tracing::Level;

use crate::{db::PgClient, state::ApiState};

mod auth;
mod cdn;
mod error;
mod middleware;

/// Creates and runs the API server
pub async fn create_server(state: Arc<ApiState>) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(root))
        .nest("/cdn", cdn::router())
        .nest("/auth", auth::router())
        .layer(
            ServiceBuilder::new()
                .layer(CompressionLayer::new())
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                        .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
                )
                .layer(AuthLayer::new(vec!["/auth".to_owned()], state.clone())),
        )
        .with_state(state);

    let addr = env::var("HOST").unwrap_or(String::from("127.0.0.1:6601"));
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("serving on http://{addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn root() -> String {
    "root".to_owned()
}
