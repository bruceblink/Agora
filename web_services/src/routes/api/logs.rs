use crate::common::AppState;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, delete, get, post, web};
use common::api::{ApiError, ApiResponse};
use common::dto::AddOperationLogDTO;
use common::po::ApiResult;
use common::utils::JwtClaims;
use common::{LoginLogQuery, OperationLogQuery};
use infra::{
    OperationLogContext, add_operation_log, delete_login_logs, delete_operation_logs,
    list_login_logs, list_operation_logs,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteLoginLogQuery {
    #[serde(default)]
    ids: Vec<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteOperationLogQuery {
    #[serde(default)]
    operation_ids: Vec<i64>,
}

fn request_ip(req: &HttpRequest) -> String {
    req.connection_info()
        .realip_remote_addr()
        .unwrap_or_default()
        .to_string()
}

fn operation_context(req: &HttpRequest) -> OperationLogContext {
    let claims = req.extensions().get::<JwtClaims>().cloned();
    OperationLogContext {
        user_id: claims.as_ref().map(|claims| claims.uid),
        username: claims.map(|claims| claims.sub),
        operator_ip: request_ip(req),
        request_url: req.uri().path().to_string(),
        request_method: req.method().as_str().to_string(),
    }
}

fn validate_positive_ids(ids: &[i64], field_name: &str) -> Result<(), ApiError> {
    if ids.is_empty() {
        return Err(ApiError::BadRequest(format!("{field_name} 不能为空")));
    }
    if ids.iter().any(|id| *id <= 0) {
        return Err(ApiError::BadRequest(format!("{field_name} 必须为正整数")));
    }
    Ok(())
}

#[get("/logs/loginLogs")]
async fn login_logs_list(
    req: HttpRequest,
    query: web::Query<LoginLogQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_login_logs(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询登录日志列表失败: {e:?}");
            Err(ApiError::Database("查询登录日志列表失败".into()))
        }
    }
}

#[get("/logs/loginLogs/excel")]
async fn login_logs_export(
    req: HttpRequest,
    query: web::Query<LoginLogQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_login_logs(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("导出登录日志失败: {e:?}");
            Err(ApiError::Database("导出登录日志失败".into()))
        }
    }
}

#[delete("/logs/loginLogs")]
async fn login_logs_delete(
    req: HttpRequest,
    query: web::Query<DeleteLoginLogQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let ids = query.into_inner().ids;
    validate_positive_ids(&ids, "ids")?;

    match delete_login_logs(&ids, &app_state.db_pool).await {
        Ok(_) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("删除登录日志失败: {e:?}");
            Err(ApiError::BadRequest("删除登录日志失败".into()))
        }
    }
}

#[get("/logs/operationLogs")]
async fn operation_logs_list(
    req: HttpRequest,
    query: web::Query<OperationLogQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_operation_logs(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询操作日志列表失败: {e:?}");
            Err(ApiError::Database("查询操作日志列表失败".into()))
        }
    }
}

#[get("/logs/operationLogs/excel")]
async fn operation_logs_export(
    req: HttpRequest,
    query: web::Query<OperationLogQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_operation_logs(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("导出操作日志失败: {e:?}");
            Err(ApiError::Database("导出操作日志失败".into()))
        }
    }
}

#[post("/logs/operationLogs")]
async fn operation_log_create(
    req: HttpRequest,
    body: web::Json<AddOperationLogDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let context = operation_context(&req);
    let data = body.into_inner();

    match add_operation_log(&data, &context, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("新增操作日志失败: {e:?}");
            Err(ApiError::BadRequest("新增操作日志失败".into()))
        }
    }
}

#[delete("/logs/operationLogs")]
async fn operation_logs_delete(
    req: HttpRequest,
    query: web::Query<DeleteOperationLogQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let operation_ids = query.into_inner().operation_ids;
    validate_positive_ids(&operation_ids, "operationIds")?;

    match delete_operation_logs(&operation_ids, &app_state.db_pool).await {
        Ok(_) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("删除操作日志失败: {e:?}");
            Err(ApiError::BadRequest("删除操作日志失败".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate_positive_ids;

    #[test]
    fn validate_positive_ids_rejects_empty_ids() {
        assert!(validate_positive_ids(&[], "ids").is_err());
    }

    #[test]
    fn validate_positive_ids_rejects_non_positive_ids() {
        assert!(validate_positive_ids(&[1, 0], "ids").is_err());
        assert!(validate_positive_ids(&[-1], "ids").is_err());
    }

    #[test]
    fn validate_positive_ids_accepts_positive_ids() {
        assert!(validate_positive_ids(&[1, 2], "ids").is_ok());
    }
}
