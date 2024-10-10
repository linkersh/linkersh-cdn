use std::sync::Arc;

use crate::{
    auth::user::TokenHandler, db::PgClient, meili::MeiliClient, ocr::OcrClient,
    storage::StorageClient,
};

pub struct ApiState {
    pub storage: StorageClient,
    pub pg: PgClient,
    pub tokens: Arc<TokenHandler>,
    pub ocr: OcrClient,
    pub meili: MeiliClient
}

impl ApiState {
    pub async fn new() -> anyhow::Result<ApiState> {
        let ocr = OcrClient::new()?;
        let pg = PgClient::new().await?;
        let storage = StorageClient::new().await?;
        let tokens = Arc::new(TokenHandler::new()?);
        let meili = MeiliClient::new().await?;

        Ok(ApiState {
            storage,
            pg,
            ocr,
            tokens,
            meili
        })
    }
}
