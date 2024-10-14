use axum::{
    extract::{Request, State},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use axum_extra::extract::CookieJar;
use http::StatusCode;
use std::{env, sync::Arc};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    trace::{self, TraceLayer},
};
use tracing::Level;

use crate::state::ApiState;

mod auth;
mod cdn;
mod error;
mod profile;

async fn auth_middleware(
    State(state): State<Arc<ApiState>>,
    mut request: Request,
    next: Next,
) -> Response {
    if request.uri().path().starts_with("/api/auth") {
        return next.run(request).await;
    }

    let jar = CookieJar::from_headers(request.headers());
    let token = jar.get("token");

    let Some(token) = token else {
        tracing::debug!("no cookie specified.");
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    };

    let token = token.value();
    let claims = match state.tokens.verify_token(token) {
        Ok(c) => c,
        Err(error) => {
            tracing::error!(error = ?error, "failed to verify token");
            return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
        }
    };

    let Ok(secret_code) = sqlx::query!(
        "SELECT user_id FROM user_secret_codes WHERE user_id = $1 and code = $2",
        claims.sub,
        claims.code
    )
    .fetch_one(&state.pg.inner)
    .await
    else {
        tracing::warn!("user's secret code doesnt exist in db, rejecting request");
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    };

    assert_eq!(secret_code.user_id, claims.sub);

    let ext = request.extensions_mut();
    ext.insert(claims);

    next.run(request).await
}

/// Creates and runs the API server
pub async fn create_server(state: Arc<ApiState>) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(root))
        .nest(
            "/api",
            Router::new()
                .nest("/cdn", cdn::router())
                .nest("/auth", auth::router())
                .nest("/user", profile::router()),
        )
        .layer(
            ServiceBuilder::new().layer(CompressionLayer::new()).layer(
                TraceLayer::new_for_http()
                    .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                    .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
            ),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
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
