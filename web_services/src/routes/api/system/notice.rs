use crate::common::AppState;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, delete, get, post, put, web};
use common::NoticeQuery;
use common::api::{ApiError, ApiResponse};
use common::dto::{CreateNoticeDTO, UpdateNoticeDTO};
use common::po::ApiResult;
use common::utils::JwtClaims;
use infra::{create_notice, delete_notices, get_notice, list_notices, update_notice};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteNoticeQuery {
    #[serde(default)]
    notice_ids: Vec<i64>,
}

fn current_user_id(req: &HttpRequest) -> Result<i64, ApiError> {
    req.extensions()
        .get::<JwtClaims>()
        .map(|claims| claims.uid)
        .ok_or_else(|| ApiError::Unauthorized("未授权".into()))
}

fn validate_delete_ids(notice_ids: &[i64]) -> Result<(), ApiError> {
    if notice_ids.is_empty() {
        return Err(ApiError::BadRequest("noticeIds 不能为空".into()));
    }
    if notice_ids.iter().any(|id| *id <= 0) {
        return Err(ApiError::BadRequest("noticeIds 必须为正整数".into()));
    }
    Ok(())
}

#[get("/system/notices")]
async fn notices_list(
    req: HttpRequest,
    query: web::Query<NoticeQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_notices(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询通知公告列表失败: {e:?}");
            Err(ApiError::Database("查询通知公告列表失败".into()))
        }
    }
}

#[get("/system/notices/{notice_id}")]
async fn notice_get(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let notice_id = path.into_inner();
    match get_notice(notice_id, &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询通知公告 {notice_id} 失败: {e:?}");
            Err(ApiError::NotFound(format!("通知公告 {notice_id} 不存在")))
        }
    }
}

#[post("/system/notices")]
async fn notice_create(
    req: HttpRequest,
    body: web::Json<CreateNoticeDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let creator_id = current_user_id(&req)?;
    let data = body.into_inner();

    match create_notice(&data, creator_id, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("新增通知公告失败: {e:?}");
            Err(ApiError::BadRequest("新增通知公告失败".into()))
        }
    }
}

#[put("/system/notices/{notice_id}")]
async fn notice_update(
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateNoticeDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let notice_id = path.into_inner();
    let data = body.into_inner();

    match update_notice(notice_id, &data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("更新通知公告 {notice_id} 失败: {e:?}");
            Err(ApiError::BadRequest(format!(
                "更新通知公告 {notice_id} 失败"
            )))
        }
    }
}

#[delete("/system/notices")]
async fn notice_delete(
    req: HttpRequest,
    query: web::Query<DeleteNoticeQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let notice_ids = query.into_inner().notice_ids;
    validate_delete_ids(&notice_ids)?;

    match delete_notices(&notice_ids, &app_state.db_pool).await {
        Ok(_) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("删除通知公告失败: {e:?}");
            Err(ApiError::BadRequest("删除通知公告失败".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate_delete_ids;

    #[test]
    fn validate_delete_ids_rejects_empty_ids() {
        assert!(validate_delete_ids(&[]).is_err());
    }

    #[test]
    fn validate_delete_ids_rejects_non_positive_ids() {
        assert!(validate_delete_ids(&[1, 0]).is_err());
        assert!(validate_delete_ids(&[-1]).is_err());
    }

    #[test]
    fn validate_delete_ids_accepts_positive_ids() {
        assert!(validate_delete_ids(&[1, 2]).is_ok());
    }
}
