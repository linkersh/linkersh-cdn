use std::env;

use chrono::{DateTime, Utc};
use meilisearch_sdk::{
    client::Client,
    search::{SearchResult, SearchResults},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct ObjectOcrDoc {
    pub id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

pub struct MeiliClient {
    client: Client,
}

impl MeiliClient {
    pub async fn new() -> anyhow::Result<MeiliClient> {
        let meilisearch_api_key = env::var("MEILI_MASTER_KEY")?;
        let client = Client::new("http://127.0.0.1:6606", Some(meilisearch_api_key)).unwrap();

        let idx = client.index("objects_ocr");

        // idx.delete_all_documents().await?;
        idx.set_filterable_attributes(&["user_id"]).await?;
        idx.set_sortable_attributes(&["created_at"]).await?;

        Ok(MeiliClient { client })
    }

    pub async fn index_object(&self, id: Uuid, user_id: Uuid, text: String) -> anyhow::Result<()> {
        let idx = self.client.index("objects_ocr");
        idx.add_documents(
            &[ObjectOcrDoc {
                id,
                user_id,
                content: text,
                created_at: Utc::now(),
            }],
            Some("id"),
        )
        .await?;
        Ok(())
    }

    pub async fn search_objects(
        &self,
        user_id: Uuid,
        text: &str,
        offset: usize,
    ) -> anyhow::Result<SearchResults<ObjectOcrDoc>> {
        let idx = self.client.index("objects_ocr");
        let result: SearchResults<ObjectOcrDoc> = idx
            .search()
            .with_query(text)
            .with_filter(&format!("user_id = \"{}\"", user_id))
            .with_limit(16)
            .with_page(1)
            .with_offset(offset)
            .with_sort(&["created_at:desc"])
            .execute()
            .await?;

        Ok(result)
    }
}
