use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures_util::future::BoxFuture;
use std::{
    sync::Arc,
    task::{Context, Poll},
};
use tower::{Layer, Service};

use crate::state::ApiState;

struct InnerAuthLayer {
    ignored_routes: Vec<String>,
}

#[derive(Clone)]
pub struct AuthLayer {
    inner: Arc<InnerAuthLayer>,
    state: Arc<ApiState>,
}

impl AuthLayer {
    pub fn new(ignored_routes: Vec<String>, state: Arc<ApiState>) -> Self {
        AuthLayer {
            inner: Arc::new(InnerAuthLayer { ignored_routes }),
            state,
        }
    }
}

impl<S: Clone> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware {
            inner,
            layer: self.clone(),
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
pub struct AuthMiddleware<S>
where
    S: Clone,
{
    inner: S,
    layer: AuthLayer,
    state: Arc<ApiState>,
}

impl<S: Clone> Service<Request> for AuthMiddleware<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: Request) -> Self::Future {
        tracing::debug!(
            "AUTH: received request {} {}",
            request.method(),
            request.uri()
        );

        let mut force_auth = true;
        for r in &self.layer.inner.ignored_routes {
            if request.uri().path().starts_with("/cdn")
                && !request.uri().path().starts_with("/cdn/objects")
            {
                force_auth = false;
                break;
            }

            if request.uri().path().starts_with(r) {
                force_auth = false;
                break;
            }
        }

        let mut auth_token: Option<String> = None;
        if force_auth {
            let headers = request.headers();
            let auth_header = headers.get("Authorization");
            auth_token = auth_header
                .map(|x| x.to_str().ok())
                .flatten()
                .map(|x| x.to_owned());
        }

        let mut inner = self.inner.clone();
        let state = self.state.clone();
        let response = async move {
            if force_auth {
                let Some(auth_token) = auth_token else {
                    tracing::warn!(
                        "the authentication header is None, even though authentication is forced"
                    );
                    return Ok((StatusCode::UNAUTHORIZED, "Unauthorized").into_response());
                };

                let claims = match state.tokens.verify_token(&auth_token) {
                    Ok(claims) => claims,
                    Err(error) => {
                        tracing::warn!(error = ?error, "failed to verify user's token claims");
                        return Ok((StatusCode::UNAUTHORIZED, "Unauthorized").into_response());
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
                    return Ok((StatusCode::UNAUTHORIZED, "Unauthorized").into_response());
                };
                assert_eq!(secret_code.user_id, claims.sub);

                let ext = request.extensions_mut();
                ext.insert(claims);
            }

            let future = inner.call(request);
            let response: Response = future.await?;
            Ok(response)
        };

        Box::pin(response)
    }
}
