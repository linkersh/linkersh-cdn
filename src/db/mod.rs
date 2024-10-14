use std::env;

use anyhow::Result;
use chrono::NaiveDateTime;
use serde::Serialize;
use sqlx::{prelude::FromRow, PgConnection, Pool, Postgres};
use uuid::Uuid;

// Searchable objects:
// - Text files
// - Image files: png, jpeg, webp (with OCR)

pub struct CreateCdnObject {
    pub id: Uuid,
    pub user_id: Uuid,
    pub content_type: String,
    pub content_size: i64,
    pub hash: String,
    pub file_name: String,
}

/// Indicates this object has been OCR'd and indexed into meilisearch
pub const COF_INDEXED: i64 = 1;

/// Indicates this object is searchable, i.e image or text
pub const COF_SEARCHABLE: i64 = 2;

#[derive(FromRow, Serialize, Clone, Debug)]
pub struct CdnObject {
    pub id: Uuid,
    pub user_id: Uuid,
    pub uploaded_at: NaiveDateTime,
    pub content_type: String,
    pub content_size: i64,
    pub file_name: String,
    pub slug: Option<String>,
    pub is_public: bool,
    pub sha256_hash: String,
    pub flags: i64,
}

#[derive(Clone)]
pub struct PgClient {
    pub inner: Pool<Postgres>,
}

impl PgClient {
    pub async fn new() -> Result<PgClient> {
        let pg_uri = env::var("DATABASE_URL").expect("environment variable 'DATABASE_URL' not set");
        let pool = sqlx::PgPool::connect(&pg_uri).await?;

        tracing::info!("connected to postgres");
        Ok(PgClient { inner: pool })
    }

    pub async fn list_cdn_object(
        &self,
        user_id: Uuid,
        limit: i32,
        skip: i32,
    ) -> anyhow::Result<Vec<CdnObject>> {
        let objects: Vec<CdnObject> =
            sqlx::query_as("SELECT * FROM cdn_objects WHERE user_id = $1 ORDER BY uploaded_at DESC LIMIT $2 OFFSET $3 ")
                .bind(user_id)
                .bind(limit)
                .bind(skip)
                .fetch_all(&self.inner)
                .await?;
        Ok(objects)
    }

    pub async fn fetch_cdn_object(
        &self,
        user_id: Uuid,
        object_id: Uuid,
    ) -> anyhow::Result<CdnObject> {
        let object: CdnObject = sqlx::query_as(
            "SELECT * FROM cdn_objects WHERE user_id = $1 AND id = $2",
        )
        .bind(user_id)
        .bind(object_id)
        .fetch_one(&self.inner)
        .await?;
        Ok(object)
    }

    pub async fn fetch_cdn_object_slug(&self, slug: &str) -> anyhow::Result<CdnObject> {
        let object: CdnObject = sqlx::query_as("SELECT * FROM cdn_objects WHERE slug = $1")
            .bind(slug)
            .fetch_one(&self.inner)
            .await?;
        Ok(object)
    }

    pub async fn find_existing_hash(&self, user_id: Uuid, hash: &str) -> anyhow::Result<bool> {
        let obj = sqlx::query!(
            "SELECT sha256_hash FROM cdn_objects WHERE user_id = $1 AND sha256_hash = $2",
            user_id,
            hash
        )
        .fetch_optional(&self.inner)
        .await?;
        Ok(obj.map(|x| x.sha256_hash == hash).unwrap_or(false))
    }

    pub async fn create_cdn_object(
        &self,
        obj: CreateCdnObject,
        conn: Option<&mut PgConnection>,
        flags: i64,
    ) -> anyhow::Result<CdnObject> {
        let query = sqlx::query_as!(
            CdnObject,
            r#"
            INSERT INTO cdn_objects (id, user_id, content_type, content_size, file_name, is_public, sha256_hash, flags)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
        "#,
            obj.id,
            obj.user_id,
            obj.content_type,
            obj.content_size,
            obj.file_name,
            false,
            obj.hash,
            flags
        );

        if let Some(conn) = conn {
            let cdn_obj = query.fetch_one(conn).await?;
            Ok(cdn_obj)
        } else {
            let cdn_obj = query.fetch_one(&self.inner).await?;
            Ok(cdn_obj)
        }
    }

    pub async fn delete_cdn_objects(
        &self,
        user_id: Uuid,
        objects: &Vec<Uuid>,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            "DELETE FROM cdn_objects WHERE user_id = $1 AND id = ANY($2)",
            user_id,
            objects
        )
        .execute(&self.inner)
        .await?;

        Ok(())
    }

    pub async fn create_slug_and_publish(&self, object_id: Uuid) -> anyhow::Result<String> {
        let slug_num = object_id.as_fields().0;
        let slug = format!("{:x}", slug_num);

        sqlx::query!(
            "UPDATE cdn_objects SET slug = $1, is_public = true WHERE id = $2",
            slug.to_owned(),
            object_id
        )
        .execute(&self.inner)
        .await?;

        Ok(slug)
    }
}
