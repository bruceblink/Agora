use crate::common::AppState;
use actix_web::{HttpRequest, HttpResponse, delete, get, post, put, web};
use common::MenuQuery;
use common::api::{ApiError, ApiResponse};
use common::dto::{CreateMenuDTO, MenuDetailResponseDTO, UpdateMenuDTO};
use common::po::ApiResult;
use infra::{
    SystemMenuBusinessError, create_menu, delete_menu, get_menu, list_menu_dropdown, list_menus,
    update_menu,
};

const KEYSTONE_OBJECT_NOT_FOUND_CODE: i32 = 10001;
const KEYSTONE_MENU_NAME_NOT_UNIQUE_CODE: i32 = 10901;
const KEYSTONE_MENU_EXTERNAL_LINK_CODE: i32 = 10902;
const KEYSTONE_MENU_PARENT_SELF_CODE: i32 = 10903;
const KEYSTONE_MENU_HAS_CHILDREN_CODE: i32 = 10904;
const KEYSTONE_MENU_ASSIGNED_TO_ROLE_CODE: i32 = 10905;
const KEYSTONE_MENU_BUTTON_IN_LINK_CODE: i32 = 10906;
const KEYSTONE_MENU_SUBMENU_IN_CATALOG_CODE: i32 = 10907;
const KEYSTONE_MENU_TYPE_CHANGE_CODE: i32 = 10908;

fn business_error_response(code: i32, msg: String) -> ApiResponse<()> {
    ApiResponse {
        code,
        msg: msg.clone(),
        status: "error".into(),
        message: Some(msg),
        data: None,
    }
}

fn menu_business_error_response(error: &SystemMenuBusinessError) -> ApiResponse<()> {
    let code = match error {
        SystemMenuBusinessError::ObjectNotFound { .. } => KEYSTONE_OBJECT_NOT_FOUND_CODE,
        SystemMenuBusinessError::NameNotUnique => KEYSTONE_MENU_NAME_NOT_UNIQUE_CODE,
        SystemMenuBusinessError::ExternalLinkMustBeHttp => KEYSTONE_MENU_EXTERNAL_LINK_CODE,
        SystemMenuBusinessError::ParentIdNotAllowSelf => KEYSTONE_MENU_PARENT_SELF_CODE,
        SystemMenuBusinessError::HasChildMenus => KEYSTONE_MENU_HAS_CHILDREN_CODE,
        SystemMenuBusinessError::AlreadyAssignedToRole => KEYSTONE_MENU_ASSIGNED_TO_ROLE_CODE,
        SystemMenuBusinessError::ButtonNotAllowedInIframeOrOutLink => {
            KEYSTONE_MENU_BUTTON_IN_LINK_CODE
        }
        SystemMenuBusinessError::SubMenuOnlyAllowedInCatalog => {
            KEYSTONE_MENU_SUBMENU_IN_CATALOG_CODE
        }
        SystemMenuBusinessError::CanNotChangeMenuType => KEYSTONE_MENU_TYPE_CHANGE_CODE,
    };
    business_error_response(code, error.to_string())
}

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
        Ok(Some(data)) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Ok(None) => Ok(HttpResponse::Ok().json(ApiResponse::ok(MenuDetailResponseDTO::empty()))),
        Err(e) => {
            tracing::error!("查询菜单 {menu_id} 失败: {e:?}");
            Err(ApiError::Database("查询菜单失败".into()))
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
            if let Some(error) = e.downcast_ref::<SystemMenuBusinessError>() {
                return Ok(HttpResponse::Ok().json(menu_business_error_response(error)));
            }
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
            if let Some(error) = e.downcast_ref::<SystemMenuBusinessError>() {
                return Ok(HttpResponse::Ok().json(menu_business_error_response(error)));
            }
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
            if let Some(error) = e.downcast_ref::<SystemMenuBusinessError>() {
                return Ok(HttpResponse::Ok().json(menu_business_error_response(error)));
            }
            tracing::error!("删除菜单 {menu_id} 失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        KEYSTONE_MENU_ASSIGNED_TO_ROLE_CODE, KEYSTONE_MENU_BUTTON_IN_LINK_CODE,
        KEYSTONE_MENU_EXTERNAL_LINK_CODE, KEYSTONE_MENU_HAS_CHILDREN_CODE,
        KEYSTONE_MENU_NAME_NOT_UNIQUE_CODE, KEYSTONE_MENU_PARENT_SELF_CODE,
        KEYSTONE_MENU_SUBMENU_IN_CATALOG_CODE, KEYSTONE_MENU_TYPE_CHANGE_CODE,
        KEYSTONE_OBJECT_NOT_FOUND_CODE, menu_business_error_response, validate_menu_id,
    };
    use common::api::ApiResponse;
    use common::dto::MenuDetailResponseDTO;
    use infra::SystemMenuBusinessError;
    use serde_json::json;

    #[test]
    fn validate_menu_id_allows_keystone_zero_or_positive_ids() {
        assert!(validate_menu_id(0).is_ok());
        assert!(validate_menu_id(1).is_ok());
    }

    #[test]
    fn validate_menu_id_rejects_negative_ids() {
        assert!(validate_menu_id(-1).is_err());
    }

    #[test]
    fn missing_menu_detail_matches_keystone_empty_success() {
        let value = serde_json::to_value(ApiResponse::ok(MenuDetailResponseDTO::empty()))
            .unwrap_or_else(|_| json!(null));

        assert_eq!(value["code"], 0);
        assert_eq!(value["msg"], "操作成功");
        assert_eq!(value["status"], "ok");
        assert!(value["data"].is_object());
        assert!(value["data"]["id"].is_null());
        assert!(value["data"]["meta"].is_null());
    }

    #[test]
    fn menu_business_responses_match_keystone_errors() {
        let cases = [
            (
                SystemMenuBusinessError::ObjectNotFound { id: 5 },
                KEYSTONE_OBJECT_NOT_FOUND_CODE,
                "找不到ID为 5 的 菜单",
            ),
            (
                SystemMenuBusinessError::NameNotUnique,
                KEYSTONE_MENU_NAME_NOT_UNIQUE_CODE,
                "新增菜单:{} 失败，菜单名称已存在",
            ),
            (
                SystemMenuBusinessError::ExternalLinkMustBeHttp,
                KEYSTONE_MENU_EXTERNAL_LINK_CODE,
                "菜单外链必须以 http(s)://开头",
            ),
            (
                SystemMenuBusinessError::ParentIdNotAllowSelf,
                KEYSTONE_MENU_PARENT_SELF_CODE,
                "父级菜单不能选择自身",
            ),
            (
                SystemMenuBusinessError::HasChildMenus,
                KEYSTONE_MENU_HAS_CHILDREN_CODE,
                "存在子菜单不允许删除",
            ),
            (
                SystemMenuBusinessError::AlreadyAssignedToRole,
                KEYSTONE_MENU_ASSIGNED_TO_ROLE_CODE,
                "菜单已分配给角色，不允许",
            ),
            (
                SystemMenuBusinessError::ButtonNotAllowedInIframeOrOutLink,
                KEYSTONE_MENU_BUTTON_IN_LINK_CODE,
                "不允许在Iframe和外链跳转类型下创建按钮",
            ),
            (
                SystemMenuBusinessError::SubMenuOnlyAllowedInCatalog,
                KEYSTONE_MENU_SUBMENU_IN_CATALOG_CODE,
                "只允许在目录类型底下创建子菜单",
            ),
            (
                SystemMenuBusinessError::CanNotChangeMenuType,
                KEYSTONE_MENU_TYPE_CHANGE_CODE,
                "不允许更改菜单的类型",
            ),
        ];

        for (error, code, message) in cases {
            let value = serde_json::to_value(menu_business_error_response(&error))
                .unwrap_or_else(|_| json!(null));
            assert_eq!(value["code"], code);
            assert_eq!(value["msg"], message);
            assert_eq!(value["message"], message);
            assert_eq!(value["status"], "error");
            assert!(value.get("data").is_none());
        }
    }
}
