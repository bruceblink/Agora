use crate::common::AppState;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, delete, get, post, put, web};
use common::api::{ApiError, ApiResponse};
use common::dto::{
    CreateDeptDTO, CreatePostDTO, CreateSystemUserDTO, ResetUserPasswordDTO, UpdateDeptDTO,
    UpdatePostDTO, UpdateSystemUserDTO, UpdateUserStatusDTO,
};
use common::po::ApiResult;
use common::utils::JwtClaims;
use common::{DeptQuery, PostQuery, SystemUserQuery};
use infra::{
    create_dept, create_post, create_system_user, delete_dept, delete_posts, delete_system_users,
    get_dept, get_post, get_user_detail, list_depts, list_posts, list_system_users, parse_id_list,
    update_dept, update_post, update_system_user, update_system_user_password,
    update_system_user_status,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeletePostQuery {
    ids: String,
}

fn current_user_id(req: &HttpRequest) -> Option<i64> {
    req.extensions().get::<JwtClaims>().map(|claims| claims.uid)
}

fn validate_positive_id(id: i64, field_name: &str) -> Result<(), ApiError> {
    if id > 0 {
        Ok(())
    } else {
        Err(ApiError::BadRequest(format!("{field_name} 必须为正整数")))
    }
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

#[get("/system/depts")]
async fn depts_list(
    req: HttpRequest,
    query: web::Query<DeptQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_depts(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询部门列表失败: {e:?}");
            Err(ApiError::Database("查询部门列表失败".into()))
        }
    }
}

#[get("/system/depts/dropdown")]
async fn depts_dropdown(req: HttpRequest, app_state: web::Data<AppState>) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_depts(
        &DeptQuery {
            dept_id: None,
            parent_id: None,
            status: None,
            dept_name: None,
        },
        &app_state.db_pool,
    )
    .await
    {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询部门下拉树失败: {e:?}");
            Err(ApiError::Database("查询部门下拉树失败".into()))
        }
    }
}

#[get("/system/dept/{dept_id}")]
async fn dept_get(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let dept_id = path.into_inner();
    validate_positive_id(dept_id, "deptId")?;

    match get_dept(dept_id, &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询部门 {dept_id} 失败: {e:?}");
            Err(ApiError::NotFound(format!("部门 {dept_id} 不存在")))
        }
    }
}

#[post("/system/dept")]
async fn dept_create(
    req: HttpRequest,
    body: web::Json<CreateDeptDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let data = body.into_inner();

    match create_dept(&data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("新增部门失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[put("/system/dept/{dept_id}")]
async fn dept_update(
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateDeptDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let dept_id = path.into_inner();
    validate_positive_id(dept_id, "deptId")?;
    let data = body.into_inner();

    match update_dept(dept_id, &data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("更新部门 {dept_id} 失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[delete("/system/dept/{dept_id}")]
async fn dept_delete(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let dept_id = path.into_inner();
    validate_positive_id(dept_id, "deptId")?;

    match delete_dept(dept_id, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("删除部门 {dept_id} 失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[get("/system/post/list")]
async fn posts_list(
    req: HttpRequest,
    query: web::Query<PostQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_posts(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询岗位列表失败: {e:?}");
            Err(ApiError::Database("查询岗位列表失败".into()))
        }
    }
}

#[get("/system/post/excel")]
async fn posts_export(
    req: HttpRequest,
    query: web::Query<PostQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_posts(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("导出岗位列表失败: {e:?}");
            Err(ApiError::Database("导出岗位列表失败".into()))
        }
    }
}

#[get("/system/post/{post_id}")]
async fn post_get(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let post_id = path.into_inner();
    validate_positive_id(post_id, "postId")?;

    match get_post(post_id, &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询岗位 {post_id} 失败: {e:?}");
            Err(ApiError::NotFound(format!("岗位 {post_id} 不存在")))
        }
    }
}

#[post("/system/post")]
async fn post_create(
    req: HttpRequest,
    body: web::Json<CreatePostDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let data = body.into_inner();

    match create_post(&data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("新增岗位失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[put("/system/post")]
async fn post_update(
    req: HttpRequest,
    body: web::Json<UpdatePostDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let data = body.into_inner();

    match update_post(&data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("更新岗位失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[delete("/system/post")]
async fn posts_delete(
    req: HttpRequest,
    query: web::Query<DeletePostQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let ids = parse_id_list(&query.into_inner().ids, "ids")
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    match delete_posts(&ids, &app_state.db_pool).await {
        Ok(_) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("删除岗位失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[get("/system/users")]
async fn users_list(
    req: HttpRequest,
    query: web::Query<SystemUserQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_system_users(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询用户列表失败: {e:?}");
            Err(ApiError::Database("查询用户列表失败".into()))
        }
    }
}

#[get("/system/users/excel")]
async fn users_export(
    req: HttpRequest,
    query: web::Query<SystemUserQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_system_users(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("导出用户列表失败: {e:?}");
            Err(ApiError::Database("导出用户列表失败".into()))
        }
    }
}

#[get("/system/users/excelTemplate")]
async fn users_excel_template(req: HttpRequest, app_state: web::Data<AppState>) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(serde_json::json!([
        {
            "deptId": 1,
            "username": "demo",
            "nickname": "演示用户",
            "email": "demo@example.com",
            "phoneNumber": "15800000000",
            "sex": 2,
            "password": "password123",
            "status": 1,
            "roleId": 2,
            "postId": 4,
            "remark": ""
        }
    ]))))
}

#[post("/system/users/excel")]
async fn users_import(req: HttpRequest, app_state: web::Data<AppState>) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    Err(ApiError::BadRequest(
        "用户 Excel 导入暂不支持，请使用新增用户接口".into(),
    ))
}

#[get("/system/users/{user_id}")]
async fn user_get(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let user_id = path.into_inner();
    validate_positive_id(user_id, "userId")?;

    match get_user_detail(Some(user_id), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询用户 {user_id} 失败: {e:?}");
            Err(ApiError::NotFound(format!("用户 {user_id} 不存在")))
        }
    }
}

#[post("/system/users")]
async fn user_create(
    req: HttpRequest,
    body: web::Json<CreateSystemUserDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let data = body.into_inner();
    let password_hash = hash_password(&data.password)?;

    match create_system_user(
        &data,
        &password_hash,
        current_user_id(&req),
        &app_state.db_pool,
    )
    .await
    {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("新增用户失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[put("/system/users/{user_id}")]
async fn user_update(
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateSystemUserDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let user_id = path.into_inner();
    validate_positive_id(user_id, "userId")?;
    let data = body.into_inner();

    match update_system_user(user_id, &data, current_user_id(&req), &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("更新用户 {user_id} 失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[put("/system/users/{user_id}/password")]
async fn user_password_update(
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<ResetUserPasswordDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let user_id = path.into_inner();
    validate_positive_id(user_id, "userId")?;
    let data = body.into_inner();
    let password_hash = hash_password(&data.password)?;

    match update_system_user_password(user_id, &data, &password_hash, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("重置用户 {user_id} 密码失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[put("/system/users/{user_id}/status")]
async fn user_status_update(
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateUserStatusDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let user_id = path.into_inner();
    validate_positive_id(user_id, "userId")?;
    let data = body.into_inner();

    match update_system_user_status(user_id, &data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("更新用户 {user_id} 状态失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[delete("/system/users/{user_ids}")]
async fn users_delete(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let user_ids = parse_id_list(&path.into_inner(), "userIds")
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    match delete_system_users(&user_ids, current_user_id(&req), &app_state.db_pool).await {
        Ok(_) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("删除用户失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{hash_password, validate_positive_id};

    #[test]
    fn validate_positive_id_rejects_non_positive_values() {
        assert!(validate_positive_id(0, "userId").is_err());
        assert!(validate_positive_id(-1, "userId").is_err());
    }

    #[test]
    fn hash_password_rejects_short_password() {
        assert!(hash_password("short").is_err());
    }
}
