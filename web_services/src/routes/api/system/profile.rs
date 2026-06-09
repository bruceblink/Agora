use crate::common::AppState;
use actix_multipart::Multipart;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, get, post, put, web};
use common::api::{ApiError, ApiResponse};
use common::dto::{UpdateOwnPasswordDTO, UpdateProfileDTO, UploadFileDTO};
use common::po::ApiResult;
use common::utils::JwtClaims;
use futures_util::StreamExt;
use infra::{get_user_profile, update_own_password, update_user_avatar, update_user_profile};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

const AVATAR_FORM_FIELD: &str = "avatarfile";
const AVATAR_DIR: &str = "uploads/avatar";
const AVATAR_URL_PREFIX: &str = "/uploads/avatar";
const MAX_AVATAR_BYTES: usize = 5 * 1024 * 1024;

fn current_user_id(req: &HttpRequest) -> Result<i64, ApiError> {
    req.extensions()
        .get::<JwtClaims>()
        .map(|claims| claims.uid)
        .ok_or_else(|| ApiError::Unauthorized("未携带或非法的 JWT".into()))
}

fn hash_password(password: &str) -> Result<String, ApiError> {
    if password.len() < 8 {
        return Err(ApiError::BadRequest("密码长度不能少于 8 位".into()));
    }
    bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|e| {
        tracing::error!("密码哈希失败: {e}");
        ApiError::Internal("服务器内部错误".into())
    })
}

fn extension_from_content_type(
    content_type: Option<&mime::Mime>,
) -> Result<&'static str, ApiError> {
    match content_type {
        Some(value) if *value == mime::IMAGE_JPEG => Ok("jpg"),
        Some(value) if *value == mime::IMAGE_PNG => Ok("png"),
        Some(value) if *value == mime::IMAGE_GIF => Ok("gif"),
        Some(value) if value.type_() == mime::IMAGE && value.subtype().as_str() == "webp" => {
            Ok("webp")
        }
        _ => Err(ApiError::BadRequest(
            "头像文件类型仅支持 jpeg、png、gif 或 webp".into(),
        )),
    }
}

async fn save_avatar_file(mut payload: Multipart, user_id: i64) -> Result<String, ApiError> {
    while let Some(item) = payload.next().await {
        let mut field = item.map_err(|e| {
            tracing::warn!("读取头像 multipart 字段失败: {e}");
            ApiError::BadRequest("头像上传失败".into())
        })?;

        if field.name() != Some(AVATAR_FORM_FIELD) {
            continue;
        }

        let extension = extension_from_content_type(field.content_type())?;
        let filename = format!(
            "user-{user_id}-{}-{random}.{extension}",
            chrono::Utc::now().timestamp_millis(),
            random = rand::random::<u32>()
        );
        let upload_dir = PathBuf::from(AVATAR_DIR);
        tokio::fs::create_dir_all(&upload_dir).await.map_err(|e| {
            tracing::error!("创建头像目录失败: {e}");
            ApiError::Internal("头像上传失败".into())
        })?;

        let file_path = upload_dir.join(&filename);
        let mut file = tokio::fs::File::create(&file_path).await.map_err(|e| {
            tracing::error!("创建头像文件失败 path={file_path:?}: {e}");
            ApiError::Internal("头像上传失败".into())
        })?;

        let mut total_bytes = 0usize;
        while let Some(chunk) = field.next().await {
            let chunk = chunk.map_err(|e| {
                tracing::warn!("读取头像上传内容失败: {e}");
                ApiError::BadRequest("头像上传失败".into())
            })?;
            total_bytes += chunk.len();
            if total_bytes > MAX_AVATAR_BYTES {
                let _ = tokio::fs::remove_file(&file_path).await;
                return Err(ApiError::BadRequest("头像文件不能超过 5MB".into()));
            }
            file.write_all(&chunk).await.map_err(|e| {
                tracing::error!("写入头像文件失败 path={file_path:?}: {e}");
                ApiError::Internal("头像上传失败".into())
            })?;
        }

        if total_bytes == 0 {
            let _ = tokio::fs::remove_file(&file_path).await;
            return Err(ApiError::BadRequest("头像文件不能为空".into()));
        }

        return Ok(format!("{AVATAR_URL_PREFIX}/{filename}"));
    }

    Err(ApiError::BadRequest(format!(
        "缺少 {AVATAR_FORM_FIELD} 文件字段"
    )))
}

#[get("/system/user/profile")]
async fn profile_get(req: HttpRequest, app_state: web::Data<AppState>) -> ApiResult {
    let user_id = current_user_id(&req)?;
    match get_user_profile(user_id, &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询个人资料失败 user_id={user_id}: {e:?}");
            Err(ApiError::NotFound("用户不存在".into()))
        }
    }
}

#[put("/system/user/profile")]
async fn profile_update(
    req: HttpRequest,
    body: web::Json<UpdateProfileDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    let user_id = current_user_id(&req)?;
    match update_user_profile(user_id, &body.into_inner(), &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("更新个人资料失败 user_id={user_id}: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[put("/system/user/profile/password")]
async fn profile_password_update(
    req: HttpRequest,
    body: web::Json<UpdateOwnPasswordDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    let user_id = current_user_id(&req)?;
    let data = body.into_inner();
    let password_hash = hash_password(&data.new_password)?;
    match update_own_password(user_id, &data, &password_hash, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("更新个人密码失败 user_id={user_id}: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[post("/system/user/profile/avatar")]
async fn profile_avatar_update(
    req: HttpRequest,
    payload: Multipart,
    app_state: web::Data<AppState>,
) -> ApiResult {
    let user_id = current_user_id(&req)?;
    let avatar_url = save_avatar_file(payload, user_id).await?;
    match update_user_avatar(user_id, &avatar_url, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::ok(UploadFileDTO {
            img_url: avatar_url,
        }))),
        Err(e) => {
            tracing::error!("更新个人头像失败 user_id={user_id}: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_AVATAR_BYTES, extension_from_content_type};

    #[test]
    fn avatar_size_limit_matches_product_guardrail() {
        assert_eq!(MAX_AVATAR_BYTES, 5 * 1024 * 1024);
    }

    #[test]
    fn extension_from_content_type_accepts_supported_images() {
        assert_eq!(
            extension_from_content_type(Some(&mime::IMAGE_PNG)).unwrap(),
            "png"
        );
        assert!(extension_from_content_type(Some(&mime::APPLICATION_JSON)).is_err());
    }
}
