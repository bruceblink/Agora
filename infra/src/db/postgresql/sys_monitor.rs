use chrono::{DateTime, Utc};
use common::OnlineUserQuery;
use common::dto::OnlineUserDTO;
use common::po::PageData;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};

const DEFAULT_PAGE: u32 = 1;
const DEFAULT_PAGE_SIZE: u32 = 10;
const MAX_PAGE_SIZE: u32 = 500;

#[derive(Debug, Clone)]
pub struct SessionLoginInfo {
    pub ip_address: String,
    pub login_location: String,
    pub browser: String,
    pub operation_system: String,
}

#[derive(Debug, FromRow)]
struct OnlineUserRow {
    token_id: String,
    dept_name: Option<String>,
    username: String,
    ip_address: Option<String>,
    login_location: Option<String>,
    browser: Option<String>,
    operation_system: Option<String>,
    login_time: DateTime<Utc>,
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
    if total_count <= 0 {
        0
    } else {
        ((total_count as u32) + page_size - 1) / page_size
    }
}

#[cfg(test)]
fn token_id(token: &str) -> String {
    token.chars().take(16).collect()
}

fn to_epoch_millis(value: DateTime<Utc>) -> i64 {
    value.timestamp_millis()
}

fn online_user_dto(row: OnlineUserRow) -> OnlineUserDTO {
    OnlineUserDTO {
        token_id: row.token_id,
        dept_name: row.dept_name,
        username: row.username,
        ip_address: row.ip_address.unwrap_or_default(),
        login_location: row.login_location.unwrap_or_default(),
        browser: row.browser.unwrap_or_default(),
        operation_system: row.operation_system.unwrap_or_default(),
        login_time: to_epoch_millis(row.login_time),
    }
}

pub fn build_session_login_info(
    ip_address: &str,
    login_location: &str,
    browser: &str,
    operation_system: &str,
) -> SessionLoginInfo {
    SessionLoginInfo {
        ip_address: ip_address.chars().take(128).collect(),
        login_location: login_location.chars().take(255).collect(),
        browser: browser.chars().take(50).collect(),
        operation_system: operation_system.chars().take(50).collect(),
    }
}

pub async fn list_online_users(
    query: &OnlineUserQuery,
    db_pool: &PgPool,
) -> anyhow::Result<PageData<OnlineUserDTO>> {
    let (page, page_size, offset) = page_bounds(query.page, query.page_num, query.page_size);
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
        WITH latest_login AS (
            SELECT DISTINCT ON (username)
                   username, ip_address, login_location, browser, operation_system, login_time
            FROM sys_login_info
            WHERE deleted = FALSE
              AND status = 1
            ORDER BY username, login_time DESC, info_id DESC
        )
        SELECT LEFT(rt.token, 16) AS token_id,
               d.dept_name,
               u.username,
               COALESCE(rt.login_ip, latest_login.ip_address, u.login_ip, '') AS ip_address,
               COALESCE(rt.login_location, latest_login.login_location, '') AS login_location,
               COALESCE(rt.browser, latest_login.browser, '') AS browser,
               COALESCE(rt.operation_system, latest_login.operation_system, '') AS operation_system,
               COALESCE(latest_login.login_time, u.login_date, rt.created_at) AS login_time,
               COUNT(*) OVER() AS total_count
        FROM refresh_tokens rt
        JOIN user_info u ON u.id = rt.user_id
        LEFT JOIN sys_dept d ON d.dept_id = u.dept_id AND d.deleted = FALSE
        LEFT JOIN latest_login ON latest_login.username = u.username
        WHERE rt.revoked = FALSE
          AND rt.expires_at > now()
          AND rt.session_expires_at > now()
          AND COALESCE(u.deleted, FALSE) = FALSE
        "#,
    );

    if let Some(username) = query.username.as_deref().filter(|value| !value.is_empty()) {
        query_builder.push(" AND u.username ILIKE ");
        query_builder.push_bind(format!("%{username}%"));
    }
    if let Some(ip_address) = query
        .ip_address
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        query_builder
            .push(" AND COALESCE(rt.login_ip, latest_login.ip_address, u.login_ip, '') ILIKE ");
        query_builder.push_bind(format!("%{ip_address}%"));
    }

    query_builder.push(" ORDER BY rt.created_at DESC, rt.id DESC LIMIT ");
    query_builder.push_bind(page_size as i64);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let rows: Vec<OnlineUserRow> = query_builder.build_query_as().fetch_all(db_pool).await?;
    let total_count = rows.first().map(|row| row.total_count).unwrap_or(0);
    let items = rows.into_iter().map(online_user_dto).collect();

    Ok(PageData {
        items,
        total_count: total_count as usize,
        page,
        page_size,
        total_pages: total_pages(total_count, page_size),
    })
}

pub async fn revoke_online_user(token_id: &str, db_pool: &PgPool) -> anyhow::Result<u64> {
    if token_id.trim().is_empty() {
        return Err(anyhow::anyhow!("tokenId 不能为空"));
    }

    let result = sqlx::query(
        r#"
        UPDATE refresh_tokens
        SET revoked = TRUE
        WHERE token LIKE ($1 || '%')
          AND revoked = FALSE
        "#,
    )
    .bind(token_id)
    .execute(db_pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn active_refresh_token_count(db_pool: &PgPool) -> anyhow::Result<i64> {
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM refresh_tokens
        WHERE revoked = FALSE
          AND expires_at > now()
          AND session_expires_at > now()
        "#,
    )
    .fetch_one(db_pool)
    .await?;

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::{build_session_login_info, page_bounds, token_id, total_pages};

    #[test]
    fn page_bounds_accepts_page_num_alias() {
        assert_eq!(page_bounds(None, Some(2), Some(25)), (2, 25, 25));
    }

    #[test]
    fn token_id_uses_first_sixteen_chars() {
        assert_eq!(token_id("1234567890abcdefghijklmn"), "1234567890abcdef");
    }

    #[test]
    fn total_pages_rounds_up() {
        assert_eq!(total_pages(11, 10), 2);
        assert_eq!(total_pages(0, 10), 0);
    }

    #[test]
    fn build_session_login_info_truncates_values() {
        let info = build_session_login_info(&"1".repeat(130), "内网IP", "Chrome", "Windows");
        assert_eq!(info.ip_address.len(), 128);
    }
}
