use std::sync::Arc;

use crate::{auth::user::TokenHandler, db::PgClient, storage::StorageClient};

pub struct ApiState {
    pub storage: StorageClient,
    pub pg: PgClient,
    pub tokens: Arc<TokenHandler>,
}

impl ApiState {
    pub async fn new() -> anyhow::Result<ApiState> {
        let storage = StorageClient::new().await?;
        let pg = PgClient::new().await?;
        let tokens = Arc::new(TokenHandler::new()?);

        Ok(ApiState {
            storage,
            pg,
            tokens,
        })
    }
}
