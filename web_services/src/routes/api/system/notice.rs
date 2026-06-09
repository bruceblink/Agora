use crate::common::AppState;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, delete, get, post, put, web};
use common::NoticeQuery;
use common::api::{ApiError, ApiResponse};
use common::dto::{CreateNoticeDTO, UpdateNoticeDTO};
use common::po::ApiResult;
use common::utils::JwtClaims;
use infra::{
    SystemNoticeNotFoundError, create_notice, delete_notices, get_notice, list_notices,
    parse_id_list, update_notice,
};
use serde::Deserialize;

const KEYSTONE_OBJECT_NOT_FOUND_CODE: i32 = 10001;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteNoticeQuery {
    #[serde(default)]
    notice_ids: String,
}

fn notice_not_found_response(error: &SystemNoticeNotFoundError) -> ApiResponse<()> {
    let msg = error.to_string();
    ApiResponse {
        code: KEYSTONE_OBJECT_NOT_FOUND_CODE,
        msg: msg.clone(),
        status: "error".into(),
        message: Some(msg),
        data: None,
    }
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

#[get("/system/notices/database/slave")]
async fn notices_slave_list(
    req: HttpRequest,
    query: web::Query<NoticeQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_notices(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询从库通知公告列表失败: {e:?}");
            Err(ApiError::Database("查询从库通知公告列表失败".into()))
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
            if let Some(error) = e.downcast_ref::<SystemNoticeNotFoundError>() {
                return Ok(HttpResponse::Ok().json(notice_not_found_response(error)));
            }
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
            if let Some(error) = e.downcast_ref::<SystemNoticeNotFoundError>() {
                return Ok(HttpResponse::Ok().json(notice_not_found_response(error)));
            }
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
    let notice_ids = parse_id_list(&query.into_inner().notice_ids, "noticeIds")
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
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
    use super::{
        DeleteNoticeQuery, KEYSTONE_OBJECT_NOT_FOUND_CODE, notice_not_found_response,
        validate_delete_ids,
    };
    use infra::SystemNoticeNotFoundError;
    use serde_json::json;

    #[test]
    fn delete_notice_query_accepts_comma_separated_ids() {
        let query = actix_web::web::Query::<DeleteNoticeQuery>::from_query("noticeIds=1,2")
            .expect("notice query parses");

        assert_eq!(query.into_inner().notice_ids, "1,2");
    }

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

    #[test]
    fn notice_not_found_response_matches_keystone_business_error() {
        let value = serde_json::to_value(notice_not_found_response(&SystemNoticeNotFoundError {
            notice_id: 12,
        }))
        .unwrap_or_else(|_| json!(null));

        assert_eq!(value["code"], KEYSTONE_OBJECT_NOT_FOUND_CODE);
        assert_eq!(value["msg"], "找不到ID为 12 的 通知公告");
        assert_eq!(value["status"], "error");
        assert_eq!(value["message"], "找不到ID为 12 的 通知公告");
        assert!(value.get("data").is_none());
    }
}
