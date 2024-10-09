use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use axum_typed_multipart::{FieldData, TryFromMultipart, TypedMultipart};
use fast_image_resize::{images::Image, IntoImageView, ResizeOptions, Resizer};
use futures::{stream::FuturesUnordered, StreamExt};
use image::{
    codecs::{png::PngEncoder, webp::WebPEncoder},
    DynamicImage, EncodableLayout, ImageEncoder, ImageFormat,
};
use scopeguard::guard_on_success;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    io::{BufReader, BufWriter, Read},
    path::PathBuf,
    result,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};
use tempfile::NamedTempFile;
use tokio::{fs::File, sync::Mutex};
use uuid::Uuid;
use webp::Encoder;

use super::{error::ApiError, state::ApiState};
use crate::{
    auth::user::TokenClaims,
    db::{CdnObject, CreateCdnObject},
};

pub fn router() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/objects/:id", get(fetch_object))
        .route("/objects/:id/thumbnail", get(fetch_object_thumb))
        .route("/objects/list", get(list_objects))
        .route("/objects/upload", post(upload))
        .layer(DefaultBodyLimit::max(5000000000))
        .route("/objects/delete", post(delete_objects))
        .route("/objects/publish", post(publish_object))
        .route("/*slug", get(fetch_obj_by_slug))
}

pub async fn fetch_obj_by_slug(
    State(state): State<Arc<ApiState>>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let obj_pg = match state.pg.fetch_cdn_object_slug(&slug).await {
        Ok(v) => v,
        Err(error) => {
            tracing::error!(error = ?error, "error when fetching a cdn object from postgres");
            return Err(ApiError::CdnObjectNotFound.into());
        }
    };

    let obj_s3 = match state
        .storage
        .get_user_object(obj_pg.user_id, obj_pg.id)
        .await
    {
        Ok(v) => v,
        Err(error) => {
            tracing::error!(error = ?error, "error when fetching a cdn object from s3");
            return Err(ApiError::CdnObjectNotFound.into());
        }
    };

    let response = axum::http::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, obj_pg.content_type) // Set appropriate MIME type
        .header(header::CONTENT_LENGTH, obj_s3.len())
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", obj_pg.file_name),
        )
        .body(axum::body::Body::from(obj_s3))?;

    Ok(response)
}

#[derive(Deserialize)]
pub struct PublishObjectReq {
    id: Uuid,
}

pub async fn publish_object(
    State(state): State<Arc<ApiState>>,
    Extension(claims): Extension<TokenClaims>,
    Json(body): Json<PublishObjectReq>,
) -> Result<Json<CdnObject>, ApiError> {
    let mut object = match state.pg.fetch_cdn_object(claims.sub, body.id).await {
        Ok(v) => v,
        Err(error) => {
            tracing::error!(error = ?error, "error when fetching a cdn object from postgres");
            return Err(ApiError::CdnObjectNotFound.into());
        }
    };

    if object.is_public || object.slug.is_some() {
        return Err(ApiError::ObjectIsAlreadyPublic.into());
    }

    let slug = state.pg.create_slug_and_publish(object.id).await?;

    object.slug = Some(slug);
    object.is_public = true;

    Ok(Json(object))
}

#[axum::debug_handler]
pub async fn fetch_object_thumb(
    State(state): State<Arc<ApiState>>,
    Extension(claims): Extension<TokenClaims>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let obj_pg = match state.pg.fetch_cdn_object(claims.sub, id).await {
        Ok(v) => v,
        Err(error) => {
            tracing::error!(error = ?error, "error when fetching a cdn object from postgres");
            return Err(ApiError::CdnObjectNotFound.into());
        }
    };

    if !obj_pg.content_type.starts_with("image/") {
        return Err(ApiError::ObjectHasNoThumbnail.into());
    }

    if let Ok(object) = state.storage.get_object_thumb(claims.sub, id).await {
        tracing::debug!("loading thubmnail for object {} from cache", obj_pg.id);
        let response = axum::http::Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "image/webp") // Set appropriate MIME type
            .header(header::CONTENT_LENGTH, object.len())
            .header(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", obj_pg.file_name),
            )
            .body(axum::body::Body::from(object))?;
        return Ok(response);
    }

    let obj_s3 = match state.storage.get_user_object(claims.sub, id).await {
        Ok(v) => v,
        Err(error) => {
            tracing::error!(error = ?error, "error when fetching a cdn object from s3");
            return Err(ApiError::CdnObjectNotFound.into());
        }
    };

    let buffer = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
        let src_image = image::load_from_memory(&obj_s3)?;

        let dst_width = 256;
        let dst_height = 256;
        let mut dst_image = Image::new(dst_width, dst_height, src_image.pixel_type().unwrap());

        let mut resizer = Resizer::new();
        resizer
            .resize(
                &src_image,
                &mut dst_image,
                &Some(ResizeOptions::new().fit_into_destination(None)),
            )
            .unwrap();

        let mut result_buf = BufWriter::new(Vec::new());
        WebPEncoder::new_lossless(&mut result_buf)
            .write_image(
                dst_image.buffer(),
                dst_width,
                dst_height,
                src_image.color().into(),
            )
            .unwrap();

        let result_buf = result_buf.into_inner()?;
        let dynimg = image::load_from_memory(&result_buf)?;
        let webp_encoder = Encoder::from_image(&dynimg).unwrap();
        let webp_image = webp_encoder.encode_simple(false, 70.0).unwrap();

        Ok(webp_image.to_vec()) 
    })
    .await??;

    let out = state
        .storage
        .upload_object_thumb(claims.sub, obj_pg.id, buffer, "image/webp")
        .await?;

    tracing::debug!("create a thumbnail for object {}", obj_pg.id);

    let response = axum::http::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/webp") // Set appropriate MIME type
        .header(header::CONTENT_LENGTH, out.buffer.len())
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", obj_pg.file_name),
        )
        .body(axum::body::Body::from(out.buffer))?;

    Ok(response)
}

pub async fn fetch_object(
    State(state): State<Arc<ApiState>>,
    Extension(claims): Extension<TokenClaims>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let obj_pg = match state.pg.fetch_cdn_object(claims.sub, id).await {
        Ok(v) => v,
        Err(error) => {
            tracing::error!(error = ?error, "error when fetching a cdn object from postgres");
            return Err(ApiError::CdnObjectNotFound.into());
        }
    };

    let obj_s3 = match state.storage.get_user_object(claims.sub, id).await {
        Ok(v) => v,
        Err(error) => {
            tracing::error!(error = ?error, "error when fetching a cdn object from s3");
            return Err(ApiError::CdnObjectNotFound.into());
        }
    };

    let response = axum::http::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, obj_pg.content_type) // Set appropriate MIME type
        .header(header::CONTENT_LENGTH, obj_s3.len())
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", obj_pg.file_name),
        )
        .body(axum::body::Body::from(obj_s3))?;

    Ok(response)
}

pub async fn list_objects(
    Extension(claims): Extension<TokenClaims>,
    State(state): State<Arc<ApiState>>,
) -> Result<Json<Vec<CdnObject>>, ApiError> {
    let objects = state.pg.list_cdn_object(claims.sub).await?;
    Ok(Json(objects))
}

#[derive(Deserialize)]
pub struct DeleteObjectsRequest {
    files: Vec<Uuid>,
}

pub async fn delete_objects(
    State(state): State<Arc<ApiState>>,
    Extension(claims): Extension<TokenClaims>,
    Json(body): Json<DeleteObjectsRequest>,
) -> Result<(), ApiError> {
    state.pg.delete_cdn_objects(claims.sub, &body.files).await?;
    for file in body.files {
        state.storage.delete_user_object(claims.sub, file).await?;
    }

    Ok(())
}

#[derive(TryFromMultipart, Debug)]
pub struct UploadRequest {
    #[form_data(limit = "5GiB")]
    files: Vec<FieldData<NamedTempFile>>,
    // options: String,
}

fn compute_sha256(filename: &PathBuf) -> anyhow::Result<String> {
    let file = std::fs::File::open(filename)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();

    let mut buffer = [0; 8196];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

#[tracing::instrument(skip_all)]
async fn process_upload(
    user_id: Uuid,
    state: Arc<ApiState>,
    file: FieldData<NamedTempFile>,
    objects: Arc<Mutex<Vec<CreateCdnObject>>>,
) -> anyhow::Result<()> {
    tracing::debug!("processing file {:?}", file.contents.path());

    let path = file.contents.path().to_owned();
    let hash = compute_sha256(&path)?;

    tracing::debug!("uploading file hash is {hash}");
    let existing_hash = state.pg.find_existing_hash(&hash).await?;
    if existing_hash {
        tracing::debug!("skipping hash {hash}, it already exists");
        return Ok(());
    }

    let mut content = File::open(path).await?;
    let content_type = file
        .metadata
        .content_type
        .unwrap_or("application/octet-stream".to_owned());

    let obj = state
        .storage
        .upload_user_object(user_id, &mut content, &content_type)
        .await?;

    let prefix = obj.id.to_string().chars().take(12).collect::<String>();
    let file_name = file
        .metadata
        .file_name
        .unwrap_or(format!("{prefix}_no_file_name"));
    let cdn_obj = CreateCdnObject {
        content_type,
        file_name,
        content_size: obj.size.try_into()?,
        user_id,
        hash,
        id: obj.id,
    };

    let mut objects = objects.lock().await;
    objects.push(cdn_obj);

    drop(objects);
    Ok(())
}

pub async fn upload(
    Extension(claims): Extension<TokenClaims>,
    State(state): State<Arc<ApiState>>,
    TypedMultipart(body): TypedMultipart<UploadRequest>,
) -> Result<Json<Vec<CdnObject>>, ApiError> {
    let uploaded_objects: Arc<Mutex<Vec<CreateCdnObject>>> =
        Arc::new(Mutex::new(Vec::with_capacity(body.files.len())));

    let uo_copy = Arc::clone(&uploaded_objects);
    let state_copy = Arc::clone(&state);
    let user_id = claims.sub;

    let success_cond = Arc::new(AtomicBool::new(false));
    let success_guard = guard_on_success(Arc::clone(&success_cond), move |success_cond| {
        tokio::spawn(async move {
            let is_success = success_cond.load(Ordering::SeqCst);
            if is_success {
                return;
            }

            let lock = uo_copy.lock().await;
            for obj in lock.iter() {
                if let Err(error) = state_copy.storage.delete_user_object(user_id, obj.id).await {
                    tracing::error!(error = ?error, "failed to delete object in defer");
                }
            }
        });
    });

    let start = Instant::now();
    let mut trans = state.pg.inner.begin().await?;
    let mut objects = Vec::with_capacity(body.files.len());
    let future_set = FuturesUnordered::new();

    for file in body.files {
        let statec = Arc::clone(&state);
        let uploaded_objects = uploaded_objects.clone();
        future_set.push(async move {
            if let Err(error) = process_upload(claims.sub, statec, file, uploaded_objects).await {
                tracing::error!(error = ?error, "process upload error");
            }
        });
    }

    future_set.collect::<Vec<()>>().await;

    let mut up_objects_lock = uploaded_objects.lock().await;
    let mut up_objects = Vec::with_capacity(up_objects_lock.len());

    std::mem::swap(&mut *up_objects_lock, &mut up_objects);
    drop(up_objects_lock);

    let cdn_objects_len = up_objects.len();
    for o in up_objects.into_iter() {
        let created_object = state.pg.create_cdn_object(o, Some(&mut *trans)).await?;
        objects.push(created_object);
    }

    trans.commit().await?;
    tracing::info!(
        "created {} cdn objects, elapsed: {:.2?}",
        cdn_objects_len,
        start.elapsed()
    );

    success_guard.store(false, Ordering::SeqCst);

    Ok(Json(objects))
}
