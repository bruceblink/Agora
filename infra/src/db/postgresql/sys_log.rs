use chrono::{DateTime, Days, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use common::dto::{AddOperationLogDTO, LoginLogDTO, OperationLogDTO};
use common::po::PageData;
use common::{LoginLogQuery, OperationLogQuery};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};

const DEFAULT_PAGE: u32 = 1;
const DEFAULT_PAGE_SIZE: u32 = 10;
const MAX_PAGE_SIZE: u32 = 500;

#[derive(Debug, Clone)]
pub struct LoginLogInput {
    pub username: String,
    pub ip_address: String,
    pub login_location: String,
    pub browser: String,
    pub operation_system: String,
    pub status: i16,
    pub msg: String,
    pub login_time: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OperationLogContext {
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub operator_ip: String,
    pub request_url: String,
    pub request_method: String,
}

#[derive(Debug, FromRow)]
struct LoginLogWithTotal {
    info_id: i64,
    username: String,
    ip_address: String,
    login_location: String,
    browser: String,
    operation_system: String,
    status: i16,
    msg: String,
    login_time: DateTime<Utc>,
    total_count: i64,
}

#[derive(Debug, FromRow)]
struct OperationLogWithTotal {
    operation_id: i64,
    business_type: i16,
    request_method: i16,
    request_module: String,
    request_url: String,
    called_method: String,
    operator_type: i16,
    user_id: Option<i64>,
    username: Option<String>,
    operator_ip: Option<String>,
    operator_location: Option<String>,
    dept_id: Option<i64>,
    dept_name: Option<String>,
    operation_param: Option<String>,
    operation_result: Option<String>,
    status: i16,
    error_stack: Option<String>,
    operation_time: DateTime<Utc>,
    total_count: i64,
}

fn page_bounds(
    page: Option<u32>,
    page_num: Option<u32>,
    page_size: Option<u32>,
) -> (u32, u32, i64) {
    let page = page.or(page_num).unwrap_or(DEFAULT_PAGE).max(1);
    let page_size = page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let offset = ((page - 1) * page_size) as i64;
    (page, page_size, offset)
}

fn total_pages(total_count: i64, page_size: u32) -> u32 {
    if total_count == 0 {
        0
    } else {
        ((total_count as f64) / (page_size as f64)).ceil() as u32
    }
}

fn parse_i16_filter(value: &str, field_name: &str) -> anyhow::Result<i16> {
    value
        .parse::<i16>()
        .map_err(|_| anyhow::anyhow!("{field_name} 必须是数字"))
}

fn day_start(value: NaiveDate) -> DateTime<Utc> {
    value.and_time(NaiveTime::MIN).and_utc()
}

fn next_day_start(value: NaiveDate) -> DateTime<Utc> {
    value
        .checked_add_days(Days::new(1))
        .unwrap_or(value)
        .and_time(NaiveTime::MIN)
        .and_utc()
}

fn sort_direction(value: Option<&str>) -> Option<&'static str> {
    match value {
        Some("ascending") | Some("asc") | Some("ASC") => Some("ASC"),
        Some("descending") | Some("desc") | Some("DESC") => Some("DESC"),
        _ => None,
    }
}

fn login_sort_column(value: Option<&str>) -> &'static str {
    match value {
        Some("logId") | Some("infoId") => "info_id",
        Some("username") => "username",
        Some("ipAddress") => "ip_address",
        Some("status") => "status",
        Some("loginTime") => "login_time",
        _ => "login_time",
    }
}

fn operation_sort_column(value: Option<&str>) -> &'static str {
    match value {
        Some("operationId") => "operation_id",
        Some("businessType") => "business_type",
        Some("requestModule") => "request_module",
        Some("username") => "username",
        Some("status") => "status",
        Some("operationTime") => "operation_time",
        _ => "operation_time",
    }
}

fn login_status_label(status: i16) -> String {
    match status {
        1 => "登录成功",
        2 => "退出成功",
        3 => "注册",
        0 => "登录失败",
        _ => "未知",
    }
    .to_string()
}

fn business_type_label(value: i16) -> String {
    match value {
        0 => "其他操作",
        1 => "添加",
        2 => "修改",
        3 => "删除",
        4 => "授权",
        5 => "导出",
        6 => "导入",
        7 => "强退",
        8 => "清空",
        _ => "其他操作",
    }
    .to_string()
}

fn request_method_label(value: i16) -> String {
    match value {
        1 => "GET",
        2 => "POST",
        3 => "PUT",
        4 => "DELETE",
        -1 => "UNKNOWN",
        _ => "UNKNOWN",
    }
    .to_string()
}

fn operator_type_label(value: i16) -> String {
    match value {
        1 => "其他",
        2 => "Web用户",
        3 => "手机端用户",
        _ => "其他",
    }
    .to_string()
}

fn operation_status_label(status: i16) -> String {
    match status {
        1 => "成功",
        0 => "失败",
        _ => "未知",
    }
    .to_string()
}

fn request_method_value(method: &str) -> i16 {
    match method.to_ascii_uppercase().as_str() {
        "GET" => 1,
        "POST" => 2,
        "PUT" => 3,
        "DELETE" => 4,
        _ => -1,
    }
}

pub fn private_ip_location(ip_address: &str) -> String {
    let ip = ip_address.trim();
    if ip.is_empty() {
        return String::new();
    }
    let mut segments = ip.split('.');
    let is_private_172 = matches!(
        (segments.next(), segments.next()),
        (Some("172"), Some(second))
            if second
                .parse::<u8>()
                .is_ok_and(|value| (16..=31).contains(&value))
    );

    if ip == "127.0.0.1"
        || ip == "::1"
        || ip.eq_ignore_ascii_case("localhost")
        || ip.starts_with("10.")
        || ip.starts_with("192.168.")
        || is_private_172
    {
        "内网IP".to_string()
    } else {
        "未知".to_string()
    }
}

pub fn browser_from_user_agent(user_agent: &str) -> String {
    if user_agent.contains("Edg/") {
        "Edge".to_string()
    } else if user_agent.contains("Chrome/") {
        "Chrome".to_string()
    } else if user_agent.contains("Firefox/") {
        "Firefox".to_string()
    } else if user_agent.contains("Safari/") {
        "Safari".to_string()
    } else if user_agent.is_empty() {
        String::new()
    } else {
        "Unknown".to_string()
    }
}

pub fn operation_system_from_user_agent(user_agent: &str) -> String {
    if user_agent.contains("Windows") {
        "Windows".to_string()
    } else if user_agent.contains("Mac OS X") {
        "Mac OS X".to_string()
    } else if user_agent.contains("Android") {
        "Android".to_string()
    } else if user_agent.contains("iPhone") || user_agent.contains("iPad") {
        "iOS".to_string()
    } else if user_agent.contains("Linux") {
        "Linux".to_string()
    } else if user_agent.is_empty() {
        String::new()
    } else {
        "Unknown".to_string()
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn ensure_max_len(value: Option<&str>, max_chars: usize, field_name: &str) -> anyhow::Result<()> {
    if value.is_some_and(|value| value.chars().count() > max_chars) {
        Err(anyhow::anyhow!("{field_name} 不能超过 {max_chars} 个字符"))
    } else {
        Ok(())
    }
}

fn parse_operation_time(value: Option<&str>) -> DateTime<Utc> {
    value
        .and_then(|value| NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").ok())
        .map(|value| value.and_utc())
        .unwrap_or_else(Utc::now)
}

fn to_login_dto(row: LoginLogWithTotal) -> LoginLogDTO {
    LoginLogDTO {
        log_id: row.info_id.to_string(),
        username: row.username,
        ip_address: row.ip_address,
        login_location: row.login_location,
        operation_system: row.operation_system,
        browser: row.browser,
        status: row.status,
        status_str: login_status_label(row.status),
        msg: row.msg,
        login_time: row.login_time,
    }
}

fn to_operation_dto(row: OperationLogWithTotal) -> OperationLogDTO {
    OperationLogDTO {
        operation_id: row.operation_id,
        business_type: row.business_type,
        business_type_str: business_type_label(row.business_type),
        request_method: request_method_label(row.request_method),
        request_module: row.request_module,
        request_url: row.request_url,
        called_method: row.called_method,
        operator_type: row.operator_type,
        operator_type_str: operator_type_label(row.operator_type),
        user_id: row.user_id,
        username: row.username,
        operator_ip: row.operator_ip,
        operator_location: row.operator_location,
        dept_id: row.dept_id,
        dept_name: row.dept_name,
        operation_param: row.operation_param,
        operation_result: row.operation_result,
        status: row.status,
        status_str: operation_status_label(row.status),
        error_stack: row.error_stack,
        operation_time: row.operation_time,
    }
}

pub fn build_login_log(
    username: &str,
    ip_address: &str,
    user_agent: &str,
    status: i16,
    msg: &str,
) -> LoginLogInput {
    LoginLogInput {
        username: truncate_chars(username, 50),
        ip_address: truncate_chars(ip_address, 128),
        login_location: private_ip_location(ip_address),
        browser: truncate_chars(&browser_from_user_agent(user_agent), 50),
        operation_system: truncate_chars(&operation_system_from_user_agent(user_agent), 50),
        status,
        msg: truncate_chars(msg, 255),
        login_time: Utc::now(),
    }
}

pub async fn record_login_info(input: &LoginLogInput, db_pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO sys_login_info (
            username, ip_address, login_location, browser, operation_system,
            status, msg, login_time
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(&input.username)
    .bind(&input.ip_address)
    .bind(&input.login_location)
    .bind(&input.browser)
    .bind(&input.operation_system)
    .bind(input.status)
    .bind(&input.msg)
    .bind(input.login_time)
    .execute(db_pool)
    .await?;

    Ok(())
}

pub async fn list_login_logs(
    query: &LoginLogQuery,
    db_pool: &PgPool,
) -> anyhow::Result<PageData<LoginLogDTO>> {
    let (page, page_size, offset) = page_bounds(query.page, query.page_num, query.page_size);
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
            SELECT info_id, username, ip_address, login_location, browser,
                   operation_system, status, msg, login_time,
                   COUNT(*) OVER() AS total_count
            FROM sys_login_info
            WHERE deleted = FALSE
        "#,
    );

    if let Some(ip_address) = query
        .ip_address
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        query_builder.push(" AND ip_address ILIKE ");
        query_builder.push_bind(format!("%{ip_address}%"));
    }

    if let Some(status) = query.status.as_deref().filter(|value| !value.is_empty()) {
        let status = parse_i16_filter(status, "登录状态")?;
        query_builder.push(" AND status = ");
        query_builder.push_bind(status);
    }

    if let Some(username) = query.username.as_deref().filter(|value| !value.is_empty()) {
        query_builder.push(" AND username ILIKE ");
        query_builder.push_bind(format!("%{username}%"));
    }

    if let Some(begin_time) = query.begin_time {
        query_builder.push(" AND login_time >= ");
        query_builder.push_bind(day_start(begin_time));
    }

    if let Some(end_time) = query.end_time {
        query_builder.push(" AND login_time < ");
        query_builder.push_bind(next_day_start(end_time));
    }

    query_builder.push(" ORDER BY ");
    query_builder.push(login_sort_column(query.order_column.as_deref()));
    query_builder.push(" ");
    query_builder.push(sort_direction(query.order_direction.as_deref()).unwrap_or("DESC"));
    query_builder.push(" NULLS LAST, info_id DESC LIMIT ");
    query_builder.push_bind(page_size as i64);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let rows: Vec<LoginLogWithTotal> = query_builder.build_query_as().fetch_all(db_pool).await?;
    let total_count = rows.first().map(|row| row.total_count).unwrap_or(0);
    let items = rows.into_iter().map(to_login_dto).collect();

    Ok(PageData {
        items,
        total_count: total_count as usize,
        page,
        page_size,
        total_pages: total_pages(total_count, page_size),
    })
}

pub async fn delete_login_logs(ids: &[i64], db_pool: &PgPool) -> anyhow::Result<u64> {
    if ids.is_empty() {
        return Err(anyhow::anyhow!("ids 不能为空"));
    }
    if ids.iter().any(|id| *id <= 0) {
        return Err(anyhow::anyhow!("ids 必须为正整数"));
    }

    let result = sqlx::query(
        r#"
        UPDATE sys_login_info
        SET deleted = TRUE
        WHERE info_id = ANY($1)
          AND deleted = FALSE
        "#,
    )
    .bind(ids)
    .execute(db_pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn list_operation_logs(
    query: &OperationLogQuery,
    db_pool: &PgPool,
) -> anyhow::Result<PageData<OperationLogDTO>> {
    let (page, page_size, offset) = page_bounds(query.page, query.page_num, query.page_size);
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
            SELECT operation_id, business_type, request_method, request_module,
                   request_url, called_method, operator_type, user_id, username,
                   operator_ip, operator_location, dept_id, dept_name,
                   operation_param, operation_result, status, error_stack,
                   operation_time, COUNT(*) OVER() AS total_count
            FROM sys_operation_log
            WHERE deleted = FALSE
        "#,
    );

    if let Some(business_type) = query
        .business_type
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        let business_type = parse_i16_filter(business_type, "业务类型")?;
        query_builder.push(" AND business_type = ");
        query_builder.push_bind(business_type);
    }

    if let Some(status) = query.status.as_deref().filter(|value| !value.is_empty()) {
        let status = parse_i16_filter(status, "操作状态")?;
        query_builder.push(" AND status = ");
        query_builder.push_bind(status);
    }

    if let Some(username) = query.username.as_deref().filter(|value| !value.is_empty()) {
        query_builder.push(" AND username ILIKE ");
        query_builder.push_bind(format!("%{username}%"));
    }

    if let Some(request_module) = query
        .request_module
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        query_builder.push(" AND request_module ILIKE ");
        query_builder.push_bind(format!("%{request_module}%"));
    }

    if let Some(begin_time) = query.begin_time {
        query_builder.push(" AND operation_time >= ");
        query_builder.push_bind(day_start(begin_time));
    }

    if let Some(end_time) = query.end_time {
        query_builder.push(" AND operation_time < ");
        query_builder.push_bind(next_day_start(end_time));
    }

    query_builder.push(" ORDER BY ");
    query_builder.push(operation_sort_column(query.order_column.as_deref()));
    query_builder.push(" ");
    query_builder.push(sort_direction(query.order_direction.as_deref()).unwrap_or("DESC"));
    query_builder.push(" NULLS LAST, operation_id DESC LIMIT ");
    query_builder.push_bind(page_size as i64);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let rows: Vec<OperationLogWithTotal> =
        query_builder.build_query_as().fetch_all(db_pool).await?;
    let total_count = rows.first().map(|row| row.total_count).unwrap_or(0);
    let items = rows.into_iter().map(to_operation_dto).collect();

    Ok(PageData {
        items,
        total_count: total_count as usize,
        page,
        page_size,
        total_pages: total_pages(total_count, page_size),
    })
}

pub async fn add_operation_log(
    data: &AddOperationLogDTO,
    context: &OperationLogContext,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    ensure_max_len(data.request_module.as_deref(), 64, "请求模块")?;
    ensure_max_len(data.request_url.as_deref(), 256, "请求URL")?;
    ensure_max_len(data.called_method.as_deref(), 128, "调用方法")?;
    ensure_max_len(data.operation_param.as_deref(), 2048, "请求参数")?;
    ensure_max_len(data.operation_result.as_deref(), 2048, "返回参数")?;
    ensure_max_len(data.error_stack.as_deref(), 2048, "错误消息")?;

    let business_type = data.business_type.unwrap_or(0);
    let request_method = data
        .request_method
        .unwrap_or_else(|| request_method_value(&context.request_method));
    let request_url = data
        .request_url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(&context.request_url);
    let operator_type = data.operator_type.unwrap_or(2);
    let status = data.status.unwrap_or(1);
    let operation_time = parse_operation_time(data.operation_time.as_deref());
    let operator_location = private_ip_location(&context.operator_ip);

    sqlx::query(
        r#"
        INSERT INTO sys_operation_log (
            business_type, request_method, request_module, request_url, called_method,
            operator_type, user_id, username, operator_ip, operator_location,
            dept_id, dept_name, operation_param, operation_result, status,
            error_stack, operation_time
        )
        VALUES (
            $1, $2, $3, $4, $5,
            $6, $7, $8, $9, $10,
            0, '', $11, $12, $13,
            $14, $15
        )
        "#,
    )
    .bind(business_type)
    .bind(request_method)
    .bind(data.request_module.as_deref().unwrap_or(""))
    .bind(request_url)
    .bind(data.called_method.as_deref().unwrap_or(""))
    .bind(operator_type)
    .bind(context.user_id)
    .bind(context.username.as_deref().unwrap_or(""))
    .bind(&context.operator_ip)
    .bind(operator_location)
    .bind(data.operation_param.as_deref().unwrap_or(""))
    .bind(data.operation_result.as_deref().unwrap_or(""))
    .bind(status)
    .bind(data.error_stack.as_deref().unwrap_or(""))
    .bind(operation_time)
    .execute(db_pool)
    .await?;

    Ok(())
}

pub async fn delete_operation_logs(ids: &[i64], db_pool: &PgPool) -> anyhow::Result<u64> {
    if ids.is_empty() {
        return Err(anyhow::anyhow!("operationIds 不能为空"));
    }
    if ids.iter().any(|id| *id <= 0) {
        return Err(anyhow::anyhow!("operationIds 必须为正整数"));
    }

    let result = sqlx::query(
        r#"
        UPDATE sys_operation_log
        SET deleted = TRUE
        WHERE operation_id = ANY($1)
          AND deleted = FALSE
        "#,
    )
    .bind(ids)
    .execute(db_pool)
    .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::{
        browser_from_user_agent, build_login_log, business_type_label, operation_status_label,
        parse_operation_time, request_method_label, request_method_value, total_pages,
    };

    #[test]
    fn enum_labels_match_keystone_values() {
        assert_eq!(business_type_label(1), "添加");
        assert_eq!(business_type_label(8), "清空");
        assert_eq!(request_method_label(2), "POST");
        assert_eq!(operation_status_label(0), "失败");
    }

    #[test]
    fn request_method_value_accepts_known_methods() {
        assert_eq!(request_method_value("GET"), 1);
        assert_eq!(request_method_value("post"), 2);
        assert_eq!(request_method_value("PATCH"), -1);
    }

    #[test]
    fn parse_operation_time_falls_back_for_invalid_value() {
        let parsed = parse_operation_time(Some("2026-06-09 10:11:12"));
        assert_eq!(
            parsed.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2026-06-09 10:11:12"
        );

        let fallback = parse_operation_time(Some("bad"));
        assert!(fallback.timestamp() > 0);
    }

    #[test]
    fn build_login_log_derives_location_and_user_agent() {
        let log = build_login_log(
            "admin",
            "127.0.0.1",
            "Mozilla/5.0 (Windows NT 10.0) Chrome/120.0",
            1,
            "登录成功",
        );
        assert_eq!(log.login_location, "内网IP");
        assert_eq!(log.browser, "Chrome");
        assert_eq!(log.operation_system, "Windows");
    }

    #[test]
    fn browser_parser_handles_empty_user_agent() {
        assert_eq!(browser_from_user_agent(""), "");
    }

    #[test]
    fn total_pages_rounds_up() {
        assert_eq!(total_pages(0, 10), 0);
        assert_eq!(total_pages(11, 10), 2);
    }
}
