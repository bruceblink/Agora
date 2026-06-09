use crate::common::AppState;
use crate::routes::api::export::{
    EXPORT_PAGE_SIZE, datetime, optional, xlsx_from_rows, xlsx_response,
};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, delete, get, post, web};
use common::api::{ApiError, ApiResponse};
use common::dto::{AddOperationLogDTO, LoginLogDTO, OperationLogDTO};
use common::po::ApiResult;
use common::utils::JwtClaims;
use common::{LoginLogQuery, OperationLogQuery};
use infra::{
    OperationLogContext, add_operation_log, delete_login_logs, delete_operation_logs,
    list_login_logs, list_operation_logs, parse_id_list,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteLoginLogQuery {
    #[serde(default)]
    ids: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteOperationLogQuery {
    #[serde(default)]
    operation_ids: String,
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

fn force_login_export_page(query: &mut LoginLogQuery) {
    query.page = Some(1);
    query.page_num = Some(1);
    query.page_size = Some(EXPORT_PAGE_SIZE);
}

fn force_operation_export_page(query: &mut OperationLogQuery) {
    query.page = Some(1);
    query.page_num = Some(1);
    query.page_size = Some(EXPORT_PAGE_SIZE);
}

fn login_log_export_row(item: &LoginLogDTO) -> Vec<String> {
    vec![
        item.log_id.clone(),
        item.username.clone(),
        item.ip_address.clone(),
        item.login_location.clone(),
        item.operation_system.clone(),
        item.browser.clone(),
        item.status_str.clone(),
        item.msg.clone(),
        datetime(&item.login_time),
    ]
}

fn operation_log_export_row(item: &OperationLogDTO) -> Vec<String> {
    vec![
        item.operation_id.to_string(),
        item.business_type_str.clone(),
        item.request_method.clone(),
        item.request_module.clone(),
        item.request_url.clone(),
        item.called_method.clone(),
        item.operator_type_str.clone(),
        optional(&item.user_id),
        optional(&item.username),
        optional(&item.operator_ip),
        optional(&item.operator_location),
        optional(&item.dept_id),
        optional(&item.dept_name),
        optional(&item.operation_param),
        optional(&item.operation_result),
        item.status_str.clone(),
        optional(&item.error_stack),
        datetime(&item.operation_time),
    ]
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
    let mut export_query = query.into_inner();
    force_login_export_page(&mut export_query);

    match list_login_logs(&export_query, &app_state.db_pool).await {
        Ok(data) => {
            let headers = [
                "ID",
                "用户名",
                "ip地址",
                "登录地点",
                "操作系统",
                "浏览器",
                "状态",
                "描述",
                "登录时间",
            ];
            let rows = data
                .items
                .iter()
                .map(login_log_export_row)
                .collect::<Vec<_>>();
            let bytes = xlsx_from_rows("登录日志", &headers, &rows)?;
            Ok(xlsx_response("login-logs.xlsx", bytes))
        }
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
    let ids = parse_id_list(&query.into_inner().ids, "ids")
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
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
    let mut export_query = query.into_inner();
    force_operation_export_page(&mut export_query);

    match list_operation_logs(&export_query, &app_state.db_pool).await {
        Ok(data) => {
            let headers = [
                "ID",
                "操作类型",
                "操作类型",
                "操作类型",
                "操作类型",
                "操作类型",
                "操作人类型",
                "用户ID",
                "用户名",
                "ip地址",
                "ip地点",
                "部门ID",
                "部门",
                "操作参数",
                "操作结果",
                "状态",
                "错误堆栈",
                "操作时间",
            ];
            let rows = data
                .items
                .iter()
                .map(operation_log_export_row)
                .collect::<Vec<_>>();
            let bytes = xlsx_from_rows("操作日志", &headers, &rows)?;
            Ok(xlsx_response("operation-logs.xlsx", bytes))
        }
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
    let operation_ids = parse_id_list(&query.into_inner().operation_ids, "operationIds")
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
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
    use super::{
        DeleteLoginLogQuery, DeleteOperationLogQuery, login_log_export_row,
        operation_log_export_row, validate_positive_ids,
    };
    use chrono::Utc;
    use common::dto::{LoginLogDTO, OperationLogDTO};

    #[test]
    fn delete_log_queries_accept_comma_separated_ids() {
        let login_query = actix_web::web::Query::<DeleteLoginLogQuery>::from_query("ids=1,2")
            .expect("login log query parses");
        let operation_query =
            actix_web::web::Query::<DeleteOperationLogQuery>::from_query("operationIds=3,4")
                .expect("operation log query parses");

        assert_eq!(login_query.into_inner().ids, "1,2");
        assert_eq!(operation_query.into_inner().operation_ids, "3,4");
    }

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

    #[test]
    fn login_log_export_row_uses_keystone_column_order() {
        let row = login_log_export_row(&LoginLogDTO {
            log_id: "7".into(),
            username: "admin".into(),
            ip_address: "127.0.0.1".into(),
            login_location: "内网IP".into(),
            operation_system: "Windows".into(),
            browser: "Chrome".into(),
            status: 1,
            status_str: "登录成功".into(),
            msg: "ok".into(),
            login_time: Utc::now(),
        });

        let prefix = row.iter().take(8).map(String::as_str).collect::<Vec<_>>();
        assert_eq!(
            prefix,
            vec![
                "7",
                "admin",
                "127.0.0.1",
                "内网IP",
                "Windows",
                "Chrome",
                "登录成功",
                "ok"
            ]
        );
    }

    #[test]
    fn operation_log_export_row_uses_keystone_column_order() {
        let row = operation_log_export_row(&OperationLogDTO {
            operation_id: 9,
            business_type: 1,
            business_type_str: "添加".into(),
            request_method: "POST".into(),
            request_module: "用户管理".into(),
            request_url: "/system/users".into(),
            called_method: "handler".into(),
            operator_type: 2,
            operator_type_str: "Web用户".into(),
            user_id: Some(1),
            username: Some("admin".into()),
            operator_ip: Some("127.0.0.1".into()),
            operator_location: Some("内网IP".into()),
            dept_id: Some(1),
            dept_name: Some("总部".into()),
            operation_param: Some("{}".into()),
            operation_result: Some("".into()),
            status: 1,
            status_str: "成功".into(),
            error_stack: None,
            operation_time: Utc::now(),
        });

        let prefix = row.iter().take(7).map(String::as_str).collect::<Vec<_>>();
        assert_eq!(
            prefix,
            vec![
                "9",
                "添加",
                "POST",
                "用户管理",
                "/system/users",
                "handler",
                "Web用户"
            ]
        );
    }
}
