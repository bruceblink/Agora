use crate::common::AppState;
use actix_web::{HttpRequest, HttpResponse, delete, get, post, put, web};
use common::MenuQuery;
use common::api::{ApiError, ApiResponse};
use common::dto::{CreateMenuDTO, UpdateMenuDTO};
use common::po::ApiResult;
use infra::{create_menu, delete_menu, get_menu, list_menu_dropdown, list_menus, update_menu};

fn validate_menu_id(menu_id: i64) -> Result<(), ApiError> {
    if menu_id >= 0 {
        Ok(())
    } else {
        Err(ApiError::BadRequest("menuId 不能小于 0".into()))
    }
}

#[get("/system/menus")]
async fn menus_list(
    req: HttpRequest,
    query: web::Query<MenuQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_menus(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询菜单列表失败: {e:?}");
            Err(ApiError::Database("查询菜单列表失败".into()))
        }
    }
}

#[get("/system/menus/dropdown")]
async fn menu_dropdown(req: HttpRequest, app_state: web::Data<AppState>) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_menu_dropdown(&app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询菜单下拉树失败: {e:?}");
            Err(ApiError::Database("查询菜单下拉树失败".into()))
        }
    }
}

#[get("/system/menus/{menu_id}")]
async fn menu_get(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let menu_id = path.into_inner();
    validate_menu_id(menu_id)?;

    match get_menu(menu_id, &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询菜单 {menu_id} 失败: {e:?}");
            Err(ApiError::NotFound(format!("菜单 {menu_id} 不存在")))
        }
    }
}

#[post("/system/menus")]
async fn menu_create(
    req: HttpRequest,
    body: web::Json<CreateMenuDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let data = body.into_inner();

    match create_menu(&data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("新增菜单失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[put("/system/menus/{menu_id}")]
async fn menu_update(
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateMenuDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let menu_id = path.into_inner();
    validate_menu_id(menu_id)?;
    let data = body.into_inner();

    match update_menu(menu_id, &data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("更新菜单 {menu_id} 失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[delete("/system/menus/{menu_id}")]
async fn menu_delete(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let menu_id = path.into_inner();
    validate_menu_id(menu_id)?;

    match delete_menu(menu_id, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("删除菜单 {menu_id} 失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate_menu_id;

    #[test]
    fn validate_menu_id_allows_keystone_zero_or_positive_ids() {
        assert!(validate_menu_id(0).is_ok());
        assert!(validate_menu_id(1).is_ok());
    }

    #[test]
    fn validate_menu_id_rejects_negative_ids() {
        assert!(validate_menu_id(-1).is_err());
    }
}
