use actix_multipart::Multipart;
use actix_web::http::header::{self, ContentDisposition, DispositionParam, DispositionType};
use actix_web::{HttpRequest, HttpResponse, get, post, web};
use common::api::{ApiError, ApiResponse};
use common::dto::UploadDTO;
use common::po::ApiResult;
use futures_util::StreamExt;
use serde::Deserialize;
use std::path::{Component, Path, PathBuf};
use tokio::io::AsyncWriteExt;

const PROFILE_DIR: &str = "uploads/profile";
const UPLOAD_SUBDIR: &str = "upload";
const DOWNLOAD_SUBDIR: &str = "download";
const RESOURCE_PREFIX: &str = "/profile";
const MAX_FILE_SIZE: usize = 50 * 1024 * 1024;
const MAX_FILE_NAME_LENGTH: usize = 127;
const ALLOWED_EXTENSIONS: &[&str] = &[
    "bmp", "gif", "jpg", "jpeg", "png", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "html", "htm",
    "txt", "rar", "zip", "gz", "bz2", "mp4", "avi", "rmvb", "pdf",
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadQuery {
    pub file_name: String,
}

fn is_allowed_extension(extension: &str) -> bool {
    ALLOWED_EXTENSIONS
        .iter()
        .any(|allowed| extension.eq_ignore_ascii_case(allowed))
}

fn file_extension(filename: &str) -> Option<String> {
    Path::new(filename)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_string)
}

fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => ch,
            _ => '_',
        })
        .collect::<String>()
        .trim_matches(['.', '_'])
        .to_string()
}

fn original_filename(field: &actix_multipart::Field) -> Result<String, ApiError> {
    field
        .content_disposition()
        .and_then(|disposition| disposition.get_filename())
        .map(str::trim)
        .filter(|filename| !filename.is_empty())
        .map(str::to_string)
        .ok_or_else(|| ApiError::BadRequest("上传文件名不能为空".into()))
}

fn validate_upload_filename(filename: &str) -> Result<String, ApiError> {
    if filename.chars().count() > MAX_FILE_NAME_LENGTH {
        return Err(ApiError::BadRequest(format!(
            "文件名长度不能超过 {MAX_FILE_NAME_LENGTH}"
        )));
    }
    let extension = file_extension(filename)
        .filter(|extension| is_allowed_extension(extension))
        .ok_or_else(|| ApiError::BadRequest("文件类型不允许上传".into()))?;
    Ok(extension.to_ascii_lowercase())
}

fn validate_download_filename(filename: &str) -> Result<(), ApiError> {
    let path = Path::new(filename);
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(ApiError::BadRequest("文件名不允许下载".into()));
    }

    if file_extension(filename)
        .filter(|extension| is_allowed_extension(extension))
        .is_none()
    {
        return Err(ApiError::BadRequest("文件类型不允许下载".into()));
    }
    Ok(())
}

fn generated_filename(original_filename: &str, extension: &str) -> String {
    let base_name = Path::new(original_filename)
        .file_stem()
        .and_then(|value| value.to_str())
        .map(sanitize_filename)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "file".to_string());
    format!(
        "{}_{}_{}.{extension}",
        chrono::Local::now().format("%Y%m%d%H%M%S"),
        base_name,
        format!("{:032x}", rand::random::<u128>())
    )
}

fn profile_url(subdir: &str, filename: &str) -> String {
    format!("{RESOURCE_PREFIX}/{subdir}/{filename}")
}

async fn save_upload_field(
    mut field: actix_multipart::Field,
    base_url: &str,
) -> Result<UploadDTO, ApiError> {
    let original_filename = original_filename(&field)?;
    let extension = validate_upload_filename(&original_filename)?;
    let new_filename = generated_filename(&original_filename, &extension);
    let upload_dir = PathBuf::from(PROFILE_DIR).join(UPLOAD_SUBDIR);
    tokio::fs::create_dir_all(&upload_dir).await.map_err(|e| {
        tracing::error!("创建上传目录失败 path={upload_dir:?}: {e}");
        ApiError::Internal("上传文件失败".into())
    })?;

    let file_path = upload_dir.join(&new_filename);
    let mut file = tokio::fs::File::create(&file_path).await.map_err(|e| {
        tracing::error!("创建上传文件失败 path={file_path:?}: {e}");
        ApiError::Internal("上传文件失败".into())
    })?;

    let mut total_bytes = 0usize;
    while let Some(chunk) = field.next().await {
        let chunk = chunk.map_err(|e| {
            tracing::warn!("读取上传文件失败: {e}");
            ApiError::BadRequest("上传文件失败".into())
        })?;
        total_bytes += chunk.len();
        if total_bytes > MAX_FILE_SIZE {
            let _ = tokio::fs::remove_file(&file_path).await;
            return Err(ApiError::BadRequest("上传文件大小不能超过 50MB".into()));
        }
        file.write_all(&chunk).await.map_err(|e| {
            tracing::error!("写入上传文件失败 path={file_path:?}: {e}");
            ApiError::Internal("上传文件失败".into())
        })?;
    }

    let file_name = profile_url(UPLOAD_SUBDIR, &new_filename);
    let url = format!("{base_url}{file_name}");
    Ok(UploadDTO {
        url,
        file_name,
        new_file_name: new_filename,
        original_filename,
    })
}

async fn collect_uploads(
    mut payload: Multipart,
    first_only: bool,
    base_url: &str,
) -> Result<Vec<UploadDTO>, ApiError> {
    let mut uploads = Vec::new();
    while let Some(item) = payload.next().await {
        let field = item.map_err(|e| {
            tracing::warn!("读取 multipart 字段失败: {e}");
            ApiError::BadRequest("上传文件失败".into())
        })?;
        if field.name() != Some("file") && field.name() != Some("files") {
            continue;
        }

        uploads.push(save_upload_field(field, base_url).await?);
        if first_only {
            break;
        }
    }

    if uploads.is_empty() {
        return Err(ApiError::BadRequest("上传文件不能为空".into()));
    }
    Ok(uploads)
}

fn request_base_url(req: &HttpRequest) -> String {
    let connection_info = req.connection_info();
    format!(
        "{}://{}",
        connection_info.scheme(),
        connection_info.host().trim_end_matches('/')
    )
}

#[get("/file/download")]
async fn file_download(query: web::Query<DownloadQuery>) -> ApiResult {
    let file_name = query.into_inner().file_name;
    validate_download_filename(&file_name)?;
    let file_path = PathBuf::from(PROFILE_DIR)
        .join(DOWNLOAD_SUBDIR)
        .join(&file_name);
    let content = tokio::fs::read(&file_path).await.map_err(|e| {
        tracing::warn!("读取下载文件失败 path={file_path:?}: {e}");
        ApiError::NotFound("文件不存在".into())
    })?;

    let download_name = format!("{}_{}", chrono::Utc::now().timestamp_millis(), file_name);
    Ok(HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "application/octet-stream"))
        .insert_header(ContentDisposition {
            disposition: DispositionType::Attachment,
            parameters: vec![DispositionParam::Filename(download_name)],
        })
        .body(content))
}

#[post("/file/upload")]
async fn file_upload(req: HttpRequest, payload: Multipart) -> ApiResult {
    let base_url = request_base_url(&req);
    let upload = collect_uploads(payload, true, &base_url).await?.remove(0);
    Ok(HttpResponse::Ok().json(ApiResponse::ok(upload)))
}

#[post("/file/uploads")]
async fn file_uploads(req: HttpRequest, payload: Multipart) -> ApiResult {
    let base_url = request_base_url(&req);
    let uploads = collect_uploads(payload, false, &base_url).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(uploads)))
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_FILE_SIZE, file_extension, generated_filename, is_allowed_extension, sanitize_filename,
        validate_download_filename,
    };

    #[test]
    fn file_size_limit_matches_keystone_default() {
        assert_eq!(MAX_FILE_SIZE, 50 * 1024 * 1024);
    }

    #[test]
    fn allowed_extensions_match_keystone_file_contract() {
        assert!(is_allowed_extension("jpg"));
        assert!(is_allowed_extension("XLSX"));
        assert!(is_allowed_extension("pdf"));
        assert!(!is_allowed_extension("exe"));
    }

    #[test]
    fn download_validation_rejects_parent_paths_and_bad_types() {
        assert!(validate_download_filename("readme.txt").is_ok());
        assert!(validate_download_filename("../readme.txt").is_err());
        assert!(validate_download_filename("readme.exe").is_err());
    }

    #[test]
    fn generated_filename_keeps_sanitized_base_and_extension() {
        let name = generated_filename("报表 2026.xlsx", "xlsx");
        assert!(name.ends_with(".xlsx"));
        assert!(name.contains("2026"));
        assert_eq!(file_extension(&name).as_deref(), Some("xlsx"));
        assert_eq!(sanitize_filename("..bad name.."), "bad_name");
    }
}
