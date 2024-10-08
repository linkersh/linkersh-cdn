use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

// #[non_exhaustive]
pub enum ApiError {
    Unauthorized,
    CdnObjectNotFound,
    ObjectIsAlreadyPublic,
    Internal(anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::Internal(error) => {
                tracing::error!(error = ?error, "app error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
            }
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized").into_response(),
            Self::CdnObjectNotFound => (StatusCode::NOT_FOUND, "Not Found").into_response(),
            Self::ObjectIsAlreadyPublic => (StatusCode::CONFLICT, "Conflict").into_response(),
        }
    }
}

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::Internal(err.into())
    }
}
