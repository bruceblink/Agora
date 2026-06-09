use crate::common::AppState;
use crate::routes::api::export::{
    EXPORT_PAGE_SIZE, datetime, optional, xlsx_from_rows, xlsx_response,
};
use actix_web::{HttpRequest, HttpResponse, delete, get, post, put, web};
use common::api::{ApiError, ApiResponse};
use common::dto::{
    CreateRoleDTO, RoleDTO, UpdateRoleDTO, UpdateRoleDataScopeDTO, UpdateRoleStatusDTO,
};
use common::po::ApiResult;
use common::{RoleQuery, RoleUserQuery};
use infra::{
    SystemRoleBusinessError, create_role, delete_roles, get_role, grant_role_to_users,
    list_role_users, list_roles, revoke_roles_from_users, update_role, update_role_data_scope,
    update_role_status,
};

const KEYSTONE_OBJECT_NOT_FOUND_CODE: i32 = 10001;
const KEYSTONE_ROLE_NAME_NOT_UNIQUE_CODE: i32 = 11001;
const KEYSTONE_ROLE_KEY_NOT_UNIQUE_CODE: i32 = 11002;
const KEYSTONE_ROLE_DUPLICATED_DEPT_CODE: i32 = 11003;
const KEYSTONE_ROLE_ASSIGNED_TO_USER_CODE: i32 = 11004;
const KEYSTONE_ROLE_NOT_AVAILABLE_CODE: i32 = 11005;

fn business_error_response(code: i32, msg: String) -> ApiResponse<()> {
    ApiResponse {
        code,
        msg: msg.clone(),
        status: "error".into(),
        message: Some(msg),
        data: None,
    }
}

fn role_business_error_response(error: &SystemRoleBusinessError) -> ApiResponse<()> {
    let code = match error {
        SystemRoleBusinessError::ObjectNotFound { .. } => KEYSTONE_OBJECT_NOT_FOUND_CODE,
        SystemRoleBusinessError::NameNotUnique { .. } => KEYSTONE_ROLE_NAME_NOT_UNIQUE_CODE,
        SystemRoleBusinessError::KeyNotUnique { .. } => KEYSTONE_ROLE_KEY_NOT_UNIQUE_CODE,
        SystemRoleBusinessError::DuplicatedDept => KEYSTONE_ROLE_DUPLICATED_DEPT_CODE,
        SystemRoleBusinessError::AlreadyAssignedToUser => KEYSTONE_ROLE_ASSIGNED_TO_USER_CODE,
        SystemRoleBusinessError::RoleNotAvailable { .. } => KEYSTONE_ROLE_NOT_AVAILABLE_CODE,
    };
    business_error_response(code, error.to_string())
}

fn validate_role_id(role_id: i64) -> Result<(), ApiError> {
    if role_id > 0 {
        Ok(())
    } else {
        Err(ApiError::BadRequest("roleId 必须为正整数".into()))
    }
}

fn parse_positive_ids(value: &str, field_name: &str) -> Result<Vec<i64>, ApiError> {
    let ids: Result<Vec<_>, _> = value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::parse::<i64>)
        .collect();

    let ids = ids.map_err(|_| ApiError::BadRequest(format!("{field_name} 必须为数字")))?;
    if ids.is_empty() {
        return Err(ApiError::BadRequest(format!("{field_name} 不能为空")));
    }
    if ids.iter().any(|id| *id <= 0) {
        return Err(ApiError::BadRequest(format!("{field_name} 必须为正整数")));
    }
    Ok(ids)
}

fn parse_role_ids(value: &str) -> Result<Vec<i64>, ApiError> {
    parse_positive_ids(value, "roleIds")
}

fn parse_user_ids(value: &str) -> Result<Vec<i64>, ApiError> {
    parse_positive_ids(value, "userIds")
}

fn force_role_export_page(query: &mut RoleQuery) {
    query.page = Some(1);
    query.page_num = Some(1);
    query.page_size = Some(EXPORT_PAGE_SIZE);
}

fn role_export_row(item: &RoleDTO) -> Vec<String> {
    vec![
        item.role_id.to_string(),
        item.role_name.clone(),
        item.role_key.clone(),
        item.role_sort.to_string(),
        item.status.to_string(),
        optional(&item.remark),
        datetime(&item.create_time),
        item.data_scope.to_string(),
    ]
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

#[get("/system/role/{role_id}/allocated/list")]
pub async fn role_allocated_users_list(
    req: HttpRequest,
    path: web::Path<i64>,
    query: web::Query<RoleUserQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let role_id = path.into_inner();
    validate_role_id(role_id)?;

    match list_role_users(role_id, &query.into_inner(), true, &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询角色 {role_id} 已分配用户失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[get("/system/role/{role_id}/unallocated/list")]
pub async fn role_unallocated_users_list(
    req: HttpRequest,
    path: web::Path<i64>,
    query: web::Query<RoleUserQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let role_id = path.into_inner();
    validate_role_id(role_id)?;

    match list_role_users(role_id, &query.into_inner(), false, &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询角色 {role_id} 未分配用户失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
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
    let mut export_query = query.into_inner();
    force_role_export_page(&mut export_query);

    match list_roles(&export_query, &app_state.db_pool).await {
        Ok(data) => {
            let headers = [
                "角色ID",
                "角色名称",
                "角色标识",
                "角色排序",
                "角色状态",
                "备注",
                "创建时间",
                "数据范围",
            ];
            let rows = data.items.iter().map(role_export_row).collect::<Vec<_>>();
            let bytes = xlsx_from_rows("角色列表", &headers, &rows)?;
            Ok(xlsx_response("roles.xlsx", bytes))
        }
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
            if let Some(error) = e.downcast_ref::<SystemRoleBusinessError>() {
                return Ok(HttpResponse::Ok().json(role_business_error_response(error)));
            }
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
            if let Some(error) = e.downcast_ref::<SystemRoleBusinessError>() {
                return Ok(HttpResponse::Ok().json(role_business_error_response(error)));
            }
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
            if let Some(error) = e.downcast_ref::<SystemRoleBusinessError>() {
                return Ok(HttpResponse::Ok().json(role_business_error_response(error)));
            }
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
            if let Some(error) = e.downcast_ref::<SystemRoleBusinessError>() {
                return Ok(HttpResponse::Ok().json(role_business_error_response(error)));
            }
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
            if let Some(error) = e.downcast_ref::<SystemRoleBusinessError>() {
                return Ok(HttpResponse::Ok().json(role_business_error_response(error)));
            }
            tracing::error!("删除角色失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[delete("/system/role/users/{user_ids}/grant/bulk")]
pub async fn role_users_grant_delete(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let user_ids = parse_user_ids(&path.into_inner())?;

    match revoke_roles_from_users(&user_ids, &app_state.db_pool).await {
        Ok(_) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("批量解除角色用户关联失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[post("/system/role/{role_id}/users/{user_ids}/grant/bulk")]
pub async fn role_users_grant_create(
    req: HttpRequest,
    path: web::Path<(i64, String)>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let (role_id, user_ids) = path.into_inner();
    validate_role_id(role_id)?;
    let user_ids = parse_user_ids(&user_ids)?;

    match grant_role_to_users(role_id, &user_ids, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            if let Some(error) = e.downcast_ref::<SystemRoleBusinessError>() {
                return Ok(HttpResponse::Ok().json(role_business_error_response(error)));
            }
            tracing::error!("批量添加角色 {role_id} 用户关联失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use common::api::ApiError;
    use common::dto::RoleDTO;
    use infra::SystemRoleBusinessError;
    use serde_json::json;

    use super::{
        KEYSTONE_OBJECT_NOT_FOUND_CODE, KEYSTONE_ROLE_ASSIGNED_TO_USER_CODE,
        KEYSTONE_ROLE_DUPLICATED_DEPT_CODE, KEYSTONE_ROLE_KEY_NOT_UNIQUE_CODE,
        KEYSTONE_ROLE_NAME_NOT_UNIQUE_CODE, KEYSTONE_ROLE_NOT_AVAILABLE_CODE, parse_role_ids,
        parse_user_ids, role_business_error_response, role_export_row, validate_role_id,
    };
    use chrono::Utc;

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

    #[test]
    fn parse_user_ids_uses_user_field_name() {
        let err = parse_user_ids("x").unwrap_err();
        match err {
            ApiError::BadRequest(message) => assert!(message.contains("userIds")),
            _ => panic!("expected bad request"),
        }
    }

    #[test]
    fn role_business_responses_match_keystone_errors() {
        let cases = [
            (
                SystemRoleBusinessError::ObjectNotFound { id: 3 },
                KEYSTONE_OBJECT_NOT_FOUND_CODE,
                "找不到ID为 3 的 角色",
            ),
            (
                SystemRoleBusinessError::NameNotUnique {
                    role_name: "管理员".to_string(),
                },
                KEYSTONE_ROLE_NAME_NOT_UNIQUE_CODE,
                "角色名称：管理员, 已存在",
            ),
            (
                SystemRoleBusinessError::KeyNotUnique {
                    role_key: "admin".to_string(),
                },
                KEYSTONE_ROLE_KEY_NOT_UNIQUE_CODE,
                "角色标识：admin, 已存在",
            ),
            (
                SystemRoleBusinessError::DuplicatedDept,
                KEYSTONE_ROLE_DUPLICATED_DEPT_CODE,
                "重复的部门id",
            ),
            (
                SystemRoleBusinessError::AlreadyAssignedToUser,
                KEYSTONE_ROLE_ASSIGNED_TO_USER_CODE,
                "角色已分配给用户，请先取消分配，再删除角色",
            ),
            (
                SystemRoleBusinessError::RoleNotAvailable {
                    role_name: "审计员".to_string(),
                },
                KEYSTONE_ROLE_NOT_AVAILABLE_CODE,
                "角色：审计员 已禁用，无法分配给用户",
            ),
        ];

        for (error, code, message) in cases {
            let value = serde_json::to_value(role_business_error_response(&error))
                .unwrap_or_else(|_| json!(null));
            assert_eq!(value["code"], code);
            assert_eq!(value["msg"], message);
            assert_eq!(value["message"], message);
            assert_eq!(value["status"], "error");
            assert!(value.get("data").is_none());
        }
    }

    #[test]
    fn role_export_row_uses_keystone_column_order() {
        let row = role_export_row(&RoleDTO {
            role_id: 1,
            role_name: "超级管理员".into(),
            role_key: "admin".into(),
            role_sort: 1,
            status: 1,
            remark: Some("系统内置".into()),
            create_time: Utc::now(),
            data_scope: 1,
            selected_menu_list: vec![],
            selected_dept_list: vec![],
        });

        let prefix = row.iter().take(6).map(String::as_str).collect::<Vec<_>>();
        assert_eq!(
            prefix,
            vec!["1", "超级管理员", "admin", "1", "1", "系统内置"]
        );
    }
}
