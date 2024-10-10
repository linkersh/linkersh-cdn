use std::{sync::Arc, time::Duration};

use futures::{stream::FuturesUnordered, StreamExt};
use ocrs::ImageSource;

use crate::{
    db::{CdnObject, COF_INDEXED, COF_SEARCHABLE},
    state::ApiState,
};

fn image_to_text(state: Arc<ApiState>, buf: Vec<u8>) -> anyhow::Result<String> {
    let img = image::load_from_memory(&buf)?.into_rgb8();
    let img_source = ImageSource::from_bytes(img.as_raw(), img.dimensions())?;
    let text = state.ocr.image_to_text(img_source)?;

    Ok(text)
}

async fn process_objects(objs: Vec<CdnObject>, state: Arc<ApiState>) -> anyhow::Result<()> {
    for obj in objs {
        let s3_data = state.storage.get_user_object(obj.user_id, obj.id).await?;
        let state_clone = Arc::clone(&state);
        let lines = tokio::task::spawn_blocking(|| -> anyhow::Result<String> {
            match image_to_text(state_clone, s3_data) {
                Ok(v) => Ok(v),
                Err(error) => {
                    tracing::error!(error = ?error, "failed to OCR an image");
                    return Err(error);
                }
            }
        })
        .await??;

        state.meili.index_object(obj.id, obj.user_id, lines).await?;
        sqlx::query!(
            "UPDATE cdn_objects SET flags = flags | $1 WHERE id = $2",
            COF_INDEXED,
            obj.id
        )
        .execute(&state.pg.inner)
        .await?;
        tracing::debug!("object {} has been processed with OCR", obj.id);
    }

    Ok(())
}

async fn run_tasks(state: &Arc<ApiState>) -> anyhow::Result<()> {
    let objects = sqlx::query_as!(
        CdnObject,
        "SELECT * FROM cdn_objects WHERE (flags & $1) = $1 AND (flags & $2) = 0",
        COF_SEARCHABLE,
        COF_INDEXED
    )
    .fetch_all(&state.pg.inner)
    .await?;

    let threads: usize = 1; // std::thread::available_parallelism()?.into();

    if !objects.is_empty() {
        tracing::info!(
            "need to OCR {} objects, using {threads} worker threads",
            objects.len()
        );
    }

    let chunks = objects.chunks(threads);
    let futures = FuturesUnordered::new();

    for ch in chunks {
        let objects = ch.to_vec();
        let state_clone = Arc::clone(&state);

        futures.push(async move {
            if let Err(error) = process_objects(objects, state_clone).await {
                tracing::error!(error = ?error, "failed to process objects");
            }
        });
    }

    futures.collect::<Vec<()>>().await;
    Ok(())
}

pub fn start_service(state: Arc<ApiState>) -> anyhow::Result<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(error) = run_tasks(&state).await {
                tracing::error!(error = ?error, "failed to run background tasks");
            }
        }
    });

    Ok(())
}
