use std::env;

use s3::{creds::Credentials, Bucket, Region};
use tokio::fs::File;
use uuid::Uuid;

pub struct StorageClient {
    inner: Box<Bucket>,
}

pub struct UploadedObject {
    pub id: Uuid,
    pub size: usize,
}

pub struct ObjectList {}

impl StorageClient {
    pub async fn new() -> anyhow::Result<StorageClient> {
        let access_key =
            env::var("S3_ACCESS_KEY").expect("environment variable 'S3_ACCESS_KEY' is not set");
        let secret_key =
            env::var("S3_SECRET_KEY").expect("environment variable 'S3_SECRET_KEY' is not set");
        let endpoint_uri =
            env::var("S3_ENDPOINT").expect("environment variable 'S3_ENDPOINT' is not set");

        let mut client = Bucket::new(
            "linkersh",
            Region::Custom {
                region: "eu-central-1".to_owned(),
                endpoint: endpoint_uri,
            },
            Credentials {
                access_key: Some(access_key),
                secret_key: Some(secret_key),
                expiration: None,
                security_token: None,
                session_token: None,
            },
        )?;
        client.set_path_style();
        client.set_listobjects_v2();

        if let Err(error) = client.head_object("/").await {
            panic!("{}", error);
        }

        tracing::info!("connected to s3 bucket");
        Ok(StorageClient { inner: client })
    }

    pub async fn upload_user_object(
        &self,
        user_id: Uuid,
        content: &mut File,
        content_type: &str,
    ) -> anyhow::Result<UploadedObject> {
        let id = Uuid::new_v4();
        let response = self
            .inner
            .put_object_stream_with_content_type(
                content,
                format!("/vaults/{user_id}/{id}"),
                &content_type,
            )
            .await?;

        Ok(UploadedObject {
            id,
            size: response.uploaded_bytes(),
        })
    }

    pub async fn delete_user_object(&self, user_id: Uuid, object_id: Uuid) -> anyhow::Result<()> {
        self.inner
            .delete_object(format!("/vaults/{user_id}/{object_id}"))
            .await?;

        Ok(())
    }

    pub async fn get_user_object(&self, user_id: Uuid, object_id: Uuid) -> anyhow::Result<Vec<u8>> {
        let content = self
            .inner
            .get_object(format!("/vaults/{user_id}/{object_id}"))
            .await?;
        Ok(content.to_vec())
    }
}
