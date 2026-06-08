use crate::common::AppState;
use actix_web::{HttpRequest, HttpResponse, delete, get, post, put, web};
use common::RoleQuery;
use common::api::{ApiError, ApiResponse};
use common::dto::{CreateRoleDTO, UpdateRoleDTO, UpdateRoleDataScopeDTO, UpdateRoleStatusDTO};
use common::po::ApiResult;
use infra::{
    create_role, delete_roles, get_role, list_roles, update_role, update_role_data_scope,
    update_role_status,
};

fn validate_role_id(role_id: i64) -> Result<(), ApiError> {
    if role_id > 0 {
        Ok(())
    } else {
        Err(ApiError::BadRequest("roleId 必须为正整数".into()))
    }
}

fn parse_role_ids(value: &str) -> Result<Vec<i64>, ApiError> {
    let ids: Result<Vec<_>, _> = value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::parse::<i64>)
        .collect();

    let ids = ids.map_err(|_| ApiError::BadRequest("roleId 必须为数字".into()))?;
    if ids.is_empty() {
        return Err(ApiError::BadRequest("roleIds 不能为空".into()));
    }
    if ids.iter().any(|id| *id <= 0) {
        return Err(ApiError::BadRequest("roleIds 必须为正整数".into()));
    }
    Ok(ids)
}

#[get("/system/role/list")]
async fn roles_list(
    req: HttpRequest,
    query: web::Query<RoleQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_roles(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询角色列表失败: {e:?}");
            Err(ApiError::Database("查询角色列表失败".into()))
        }
    }
}

#[post("/system/role/export")]
async fn roles_export(
    req: HttpRequest,
    query: web::Query<RoleQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_roles(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("导出角色列表失败: {e:?}");
            Err(ApiError::Database("导出角色列表失败".into()))
        }
    }
}

#[get("/system/role/{role_id}")]
async fn role_get(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let role_id = path.into_inner();
    validate_role_id(role_id)?;

    match get_role(role_id, &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询角色 {role_id} 失败: {e:?}");
            Err(ApiError::NotFound(format!("角色 {role_id} 不存在")))
        }
    }
}

#[post("/system/role")]
async fn role_create(
    req: HttpRequest,
    body: web::Json<CreateRoleDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let data = body.into_inner();

    match create_role(&data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("新增角色失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[put("/system/role")]
async fn role_update(
    req: HttpRequest,
    body: web::Json<UpdateRoleDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let data = body.into_inner();
    validate_role_id(data.role_id)?;

    match update_role(&data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("更新角色失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[put("/system/role/{role_id}/status")]
async fn role_status_update(
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateRoleStatusDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let role_id = path.into_inner();
    validate_role_id(role_id)?;
    let data = body.into_inner();

    match update_role_status(role_id, &data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("更新角色 {role_id} 状态失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[put("/system/role/{role_id}/dataScope")]
async fn role_data_scope_update(
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateRoleDataScopeDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let role_id = path.into_inner();
    validate_role_id(role_id)?;
    let data = body.into_inner();

    match update_role_data_scope(role_id, &data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("更新角色 {role_id} 数据范围失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[delete("/system/role/{role_ids}")]
async fn role_delete(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let role_ids = parse_role_ids(&path.into_inner())?;

    match delete_roles(&role_ids, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("删除角色失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_role_ids, validate_role_id};

    #[test]
    fn validate_role_id_rejects_non_positive_ids() {
        assert!(validate_role_id(0).is_err());
        assert!(validate_role_id(-1).is_err());
    }

    #[test]
    fn parse_role_ids_accepts_single_or_comma_separated_ids() {
        assert_eq!(parse_role_ids("1").unwrap(), vec![1]);
        assert_eq!(parse_role_ids("1,2").unwrap(), vec![1, 2]);
    }

    #[test]
    fn parse_role_ids_rejects_invalid_ids() {
        assert!(parse_role_ids("").is_err());
        assert!(parse_role_ids("1,x").is_err());
        assert!(parse_role_ids("0").is_err());
    }
}
