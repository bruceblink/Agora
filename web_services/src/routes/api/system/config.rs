use crate::common::AppState;
use actix_web::{HttpRequest, HttpResponse, delete, get, put, web};
use common::SystemConfigQuery;
use common::api::{ApiError, ApiResponse};
use common::dto::UpdateSystemConfigDTO;
use common::po::ApiResult;
use infra::{get_system_config, list_system_configs, update_system_config};

fn validate_config_value(config_value: &str) -> Result<(), ApiError> {
    if config_value.trim().is_empty() {
        Err(ApiError::BadRequest("configValue 不能为空".into()))
    } else {
        Ok(())
    }
}

#[get("/system/configs")]
async fn system_configs_list(
    req: HttpRequest,
    query: web::Query<SystemConfigQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_system_configs(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询系统配置列表失败: {e:?}");
            Err(ApiError::Database("查询系统配置列表失败".into()))
        }
    }
}

#[get("/system/config/{config_id}")]
async fn system_config_get(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let config_id = path.into_inner();
    match get_system_config(config_id, &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询系统配置 {config_id} 失败: {e:?}");
            Err(ApiError::NotFound(format!("系统配置 {config_id} 不存在")))
        }
    }
}

#[put("/system/config/{config_id}")]
async fn system_config_update(
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateSystemConfigDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let config_id = path.into_inner();
    let data = body.into_inner();
    validate_config_value(&data.config_value)?;

    match update_system_config(config_id, &data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("更新系统配置 {config_id} 失败: {e:?}");
            Err(ApiError::BadRequest(format!(
                "更新系统配置 {config_id} 失败"
            )))
        }
    }
}

#[delete("/system/configs/cache")]
async fn system_config_cache_refresh(
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(())))
}

#[cfg(test)]
mod tests {
    use super::validate_config_value;

    #[test]
    fn validate_config_value_rejects_blank_value() {
        assert!(validate_config_value("").is_err());
        assert!(validate_config_value("  ").is_err());
    }

    #[test]
    fn validate_config_value_accepts_non_blank_value() {
        assert!(validate_config_value("false").is_ok());
    }
}
