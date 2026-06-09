use crate::common::AppState;
use actix_web::{HttpRequest, HttpResponse, delete, get, post, put, web};
use common::api::{ApiError, ApiResponse};
use common::dto::{CreateDictDataDTO, CreateDictTypeDTO, UpdateDictDataDTO, UpdateDictTypeDTO};
use common::po::ApiResult;
use common::{DictDataQuery, DictTypeQuery};
use infra::{
    SystemDictBusinessError, create_dict_data, create_dict_type, delete_dict_data,
    delete_dict_type, get_config, get_dict_data, get_dict_type, list_dict_data,
    list_dict_data_by_type, list_dict_types, update_dict_data, update_dict_type,
};

const KEYSTONE_OBJECT_NOT_FOUND_CODE: i32 = 10001;
const KEYSTONE_DICT_TYPE_HAS_DATA_CODE: i32 = 11101;

fn business_error_response(code: i32, msg: String) -> ApiResponse<()> {
    ApiResponse {
        code,
        msg: msg.clone(),
        status: "error".into(),
        message: Some(msg),
        data: None,
    }
}

fn dict_business_error_response(error: &SystemDictBusinessError) -> ApiResponse<()> {
    let code = match error {
        SystemDictBusinessError::ObjectNotFound { .. } => KEYSTONE_OBJECT_NOT_FOUND_CODE,
        SystemDictBusinessError::TypeHasData => KEYSTONE_DICT_TYPE_HAS_DATA_CODE,
    };
    business_error_response(code, error.to_string())
}

fn validate_status(status: i16) -> Result<(), ApiError> {
    if matches!(status, 0 | 1) {
        Ok(())
    } else {
        Err(ApiError::BadRequest("status 必须是 0 或 1".into()))
    }
}

fn validate_default_flag(is_default: i16) -> Result<(), ApiError> {
    if matches!(is_default, 0 | 1) {
        Ok(())
    } else {
        Err(ApiError::BadRequest("isDefault 必须是 0 或 1".into()))
    }
}

fn validate_not_blank(value: &str, field_name: &str) -> Result<(), ApiError> {
    if value.trim().is_empty() {
        Err(ApiError::BadRequest(format!("{field_name} 不能为空")))
    } else {
        Ok(())
    }
}

fn validate_dict_type(data: &CreateDictTypeDTO) -> Result<(), ApiError> {
    validate_not_blank(&data.dict_name, "dictName")?;
    validate_not_blank(&data.dict_type, "dictType")?;
    validate_status(data.status)
}

fn validate_dict_type_update(data: &UpdateDictTypeDTO) -> Result<(), ApiError> {
    validate_not_blank(&data.dict_name, "dictName")?;
    validate_not_blank(&data.dict_type, "dictType")?;
    validate_status(data.status)
}

fn validate_dict_data(data: &CreateDictDataDTO) -> Result<(), ApiError> {
    validate_not_blank(&data.dict_type, "dictType")?;
    validate_not_blank(&data.dict_label, "dictLabel")?;
    validate_not_blank(&data.dict_value, "dictValue")?;
    validate_status(data.status)?;
    validate_default_flag(data.is_default)
}

fn validate_dict_data_update(data: &UpdateDictDataDTO) -> Result<(), ApiError> {
    validate_not_blank(&data.dict_type, "dictType")?;
    validate_not_blank(&data.dict_label, "dictLabel")?;
    validate_not_blank(&data.dict_value, "dictValue")?;
    validate_status(data.status)?;
    validate_default_flag(data.is_default)
}

#[get("/getConfig")]
async fn get_config_public(app_state: web::Data<AppState>) -> ApiResult {
    match get_config(&app_state.db_pool).await {
        Ok(config) => Ok(HttpResponse::Ok().json(ApiResponse::ok(config))),
        Err(e) => {
            tracing::error!("获取系统配置失败: {e:?}");
            Err(ApiError::Internal("获取系统配置失败".into()))
        }
    }
}

#[get("/system/dict/types")]
async fn dict_types_list(
    req: HttpRequest,
    query: web::Query<DictTypeQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_dict_types(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询字典类型列表失败: {e:?}");
            Err(ApiError::Database("查询字典类型列表失败".into()))
        }
    }
}

#[get("/system/dict/type/{dict_id}")]
async fn dict_type_get(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let dict_id = path.into_inner();
    match get_dict_type(dict_id, &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            if let Some(error) = e.downcast_ref::<SystemDictBusinessError>() {
                return Ok(HttpResponse::Ok().json(dict_business_error_response(error)));
            }
            tracing::error!("查询字典类型 {dict_id} 失败: {e:?}");
            Err(ApiError::NotFound(format!("字典类型 {dict_id} 不存在")))
        }
    }
}

#[post("/system/dict/type")]
async fn dict_type_create(
    req: HttpRequest,
    body: web::Json<CreateDictTypeDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let data = body.into_inner();
    validate_dict_type(&data)?;

    match create_dict_type(&data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("新增字典类型失败: {e:?}");
            Err(ApiError::BadRequest(
                "新增字典类型失败，dictType 可能已存在".into(),
            ))
        }
    }
}

#[put("/system/dict/type/{dict_id}")]
async fn dict_type_update(
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateDictTypeDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let dict_id = path.into_inner();
    let data = body.into_inner();
    validate_dict_type_update(&data)?;

    match update_dict_type(dict_id, &data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            if let Some(error) = e.downcast_ref::<SystemDictBusinessError>() {
                return Ok(HttpResponse::Ok().json(dict_business_error_response(error)));
            }
            tracing::error!("更新字典类型 {dict_id} 失败: {e:?}");
            Err(ApiError::BadRequest(format!("更新字典类型 {dict_id} 失败")))
        }
    }
}

#[delete("/system/dict/type/{dict_id}")]
async fn dict_type_delete(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let dict_id = path.into_inner();
    match delete_dict_type(dict_id, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            if let Some(error) = e.downcast_ref::<SystemDictBusinessError>() {
                return Ok(HttpResponse::Ok().json(dict_business_error_response(error)));
            }
            tracing::error!("删除字典类型 {dict_id} 失败: {e:?}");
            Err(ApiError::BadRequest(format!(
                "删除字典类型 {dict_id} 失败，可能仍存在字典数据"
            )))
        }
    }
}

#[get("/system/dict/data/list")]
async fn dict_data_list(
    req: HttpRequest,
    query: web::Query<DictDataQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_dict_data(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询字典数据列表失败: {e:?}");
            Err(ApiError::Database("查询字典数据列表失败".into()))
        }
    }
}

#[get("/system/dict/data/type/{dict_type}")]
async fn dict_data_by_type(path: web::Path<String>, app_state: web::Data<AppState>) -> ApiResult {
    let dict_type = path.into_inner();
    match list_dict_data_by_type(&dict_type, &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询字典类型 {dict_type} 的字典数据失败: {e:?}");
            Err(ApiError::Database("查询字典数据失败".into()))
        }
    }
}

#[get("/system/dict/data/{dict_code}")]
async fn dict_data_get(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let dict_code = path.into_inner();
    match get_dict_data(dict_code, &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            if let Some(error) = e.downcast_ref::<SystemDictBusinessError>() {
                return Ok(HttpResponse::Ok().json(dict_business_error_response(error)));
            }
            tracing::error!("查询字典数据 {dict_code} 失败: {e:?}");
            Err(ApiError::NotFound(format!("字典数据 {dict_code} 不存在")))
        }
    }
}

#[post("/system/dict/data")]
async fn dict_data_create(
    req: HttpRequest,
    body: web::Json<CreateDictDataDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let data = body.into_inner();
    validate_dict_data(&data)?;

    match create_dict_data(&data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("新增字典数据失败: {e:?}");
            Err(ApiError::BadRequest("新增字典数据失败".into()))
        }
    }
}

#[put("/system/dict/data/{dict_code}")]
async fn dict_data_update(
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateDictDataDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let dict_code = path.into_inner();
    let data = body.into_inner();
    validate_dict_data_update(&data)?;

    match update_dict_data(dict_code, &data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            if let Some(error) = e.downcast_ref::<SystemDictBusinessError>() {
                return Ok(HttpResponse::Ok().json(dict_business_error_response(error)));
            }
            tracing::error!("更新字典数据 {dict_code} 失败: {e:?}");
            Err(ApiError::NotFound(format!("字典数据 {dict_code} 不存在")))
        }
    }
}

#[delete("/system/dict/data/{dict_code}")]
async fn dict_data_delete(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let dict_code = path.into_inner();
    match delete_dict_data(dict_code, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            if let Some(error) = e.downcast_ref::<SystemDictBusinessError>() {
                return Ok(HttpResponse::Ok().json(dict_business_error_response(error)));
            }
            tracing::error!("删除字典数据 {dict_code} 失败: {e:?}");
            Err(ApiError::NotFound(format!("字典数据 {dict_code} 不存在")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        KEYSTONE_DICT_TYPE_HAS_DATA_CODE, KEYSTONE_OBJECT_NOT_FOUND_CODE,
        dict_business_error_response, validate_default_flag, validate_status,
    };
    use infra::SystemDictBusinessError;

    #[test]
    fn validate_status_accepts_keystone_flags() {
        assert!(validate_status(0).is_ok());
        assert!(validate_status(1).is_ok());
        assert!(validate_status(2).is_err());
    }

    #[test]
    fn validate_default_flag_accepts_boolean_ints() {
        assert!(validate_default_flag(0).is_ok());
        assert!(validate_default_flag(1).is_ok());
        assert!(validate_default_flag(-1).is_err());
    }

    #[test]
    fn dict_object_not_found_response_matches_keystone_business_error() {
        let value = serde_json::to_value(dict_business_error_response(
            &SystemDictBusinessError::ObjectNotFound {
                id: 9,
                object_name: "字典数据",
            },
        ))
        .unwrap_or(serde_json::Value::Null);

        assert_eq!(value["code"], KEYSTONE_OBJECT_NOT_FOUND_CODE);
        assert_eq!(value["msg"], "找不到ID为 9 的 字典数据");
        assert_eq!(value["status"], "error");
        assert_eq!(value["message"], "找不到ID为 9 的 字典数据");
        assert!(value.get("data").is_none());
    }

    #[test]
    fn dict_type_has_data_response_matches_keystone_business_error() {
        let value = serde_json::to_value(dict_business_error_response(
            &SystemDictBusinessError::TypeHasData,
        ))
        .unwrap_or(serde_json::Value::Null);

        assert_eq!(value["code"], KEYSTONE_DICT_TYPE_HAS_DATA_CODE);
        assert_eq!(value["msg"], "字典类型下存在字典数据，请先删除字典数据");
        assert_eq!(value["status"], "error");
        assert_eq!(value["message"], "字典类型下存在字典数据，请先删除字典数据");
        assert!(value.get("data").is_none());
    }
}
