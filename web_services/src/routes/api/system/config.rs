use crate::common::AppState;
use actix_web::{HttpRequest, HttpResponse, delete, get, put, web};
use common::SystemConfigQuery;
use common::api::{ApiError, ApiResponse};
use common::dto::UpdateSystemConfigDTO;
use common::po::ApiResult;
use infra::{
    SystemConfigValidationError, get_system_config, list_system_configs, update_system_config,
};

const KEYSTONE_CONFIG_VALUE_EMPTY_CODE: i32 = 10601;
const KEYSTONE_CONFIG_VALUE_OPTIONS_CODE: i32 = 10602;

fn config_validation_response(error: SystemConfigValidationError) -> ApiResponse<()> {
    let (code, msg) = match error {
        SystemConfigValidationError::ValueEmpty => (
            KEYSTONE_CONFIG_VALUE_EMPTY_CODE,
            SystemConfigValidationError::ValueEmpty.to_string(),
        ),
        SystemConfigValidationError::ValueNotInOptions => (
            KEYSTONE_CONFIG_VALUE_OPTIONS_CODE,
            SystemConfigValidationError::ValueNotInOptions.to_string(),
        ),
    };

    ApiResponse {
        code,
        msg: msg.clone(),
        status: "error".into(),
        message: Some(msg),
        data: None,
    }
}

fn validate_config_value(config_value: &str) -> Result<(), SystemConfigValidationError> {
    if config_value.trim().is_empty() {
        Err(SystemConfigValidationError::ValueEmpty)
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
    if let Err(error) = validate_config_value(&data.config_value) {
        return Ok(HttpResponse::Ok().json(config_validation_response(error)));
    }

    match update_system_config(config_id, &data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            if let Some(error) = e.downcast_ref::<SystemConfigValidationError>() {
                return Ok(HttpResponse::Ok().json(config_validation_response(*error)));
            }
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
    use super::{
        KEYSTONE_CONFIG_VALUE_EMPTY_CODE, KEYSTONE_CONFIG_VALUE_OPTIONS_CODE,
        config_validation_response, validate_config_value,
    };
    use infra::SystemConfigValidationError;
    use serde_json::json;

    #[test]
    fn validate_config_value_rejects_blank_value() {
        assert!(validate_config_value("").is_err());
        assert!(validate_config_value("  ").is_err());
    }

    #[test]
    fn validate_config_value_accepts_non_blank_value() {
        assert!(validate_config_value("false").is_ok());
    }

    #[test]
    fn config_validation_response_matches_keystone_business_errors() {
        let empty = serde_json::to_value(config_validation_response(
            SystemConfigValidationError::ValueEmpty,
        ))
        .unwrap_or_else(|_| json!(null));
        assert_eq!(empty["code"], KEYSTONE_CONFIG_VALUE_EMPTY_CODE);
        assert_eq!(empty["msg"], "参数键值不允许为空");
        assert_eq!(empty["status"], "error");
        assert_eq!(empty["message"], "参数键值不允许为空");
        assert!(empty.get("data").is_none());

        let options = serde_json::to_value(config_validation_response(
            SystemConfigValidationError::ValueNotInOptions,
        ))
        .unwrap_or_else(|_| json!(null));
        assert_eq!(options["code"], KEYSTONE_CONFIG_VALUE_OPTIONS_CODE);
        assert_eq!(options["msg"], "参数键值不存在列表中");
        assert_eq!(options["status"], "error");
        assert_eq!(options["message"], "参数键值不存在列表中");
        assert!(options.get("data").is_none());
    }
}
