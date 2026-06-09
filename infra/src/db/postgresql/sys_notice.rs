use common::NoticeQuery;
use common::dto::{CreateNoticeDTO, NoticeDTO, UpdateNoticeDTO};
use common::po::PageData;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use std::fmt;

const DEFAULT_PAGE: u32 = 1;
const DEFAULT_PAGE_SIZE: u32 = 20;

#[derive(Debug, FromRow)]
struct NoticeWithTotal {
    notice_id: i64,
    notice_title: String,
    notice_type: i16,
    notice_content: String,
    status: i16,
    create_time: chrono::DateTime<chrono::Utc>,
    creator_name: Option<String>,
    total_count: i64,
}

#[derive(Debug, FromRow)]
struct NoticeRow {
    notice_id: i64,
    notice_title: String,
    notice_type: i16,
    notice_content: String,
    status: i16,
    create_time: chrono::DateTime<chrono::Utc>,
    creator_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemNoticeNotFoundError {
    pub notice_id: i64,
}

impl fmt::Display for SystemNoticeNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "找不到ID为 {} 的 通知公告", self.notice_id)
    }
}

impl std::error::Error for SystemNoticeNotFoundError {}

fn page_bounds(page: Option<u32>, page_size: Option<u32>) -> (u32, u32, i64) {
    let page = page.unwrap_or(DEFAULT_PAGE).max(1);
    let page_size = page_size.unwrap_or(DEFAULT_PAGE_SIZE).max(1);
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

fn to_dto(row: NoticeRow) -> NoticeDTO {
    NoticeDTO {
        notice_id: row.notice_id.to_string(),
        notice_title: row.notice_title,
        notice_type: row.notice_type,
        notice_content: row.notice_content,
        status: row.status,
        create_time: row.create_time,
        creator_name: row.creator_name,
    }
}

fn to_dto_with_total(row: NoticeWithTotal) -> NoticeDTO {
    to_dto(NoticeRow {
        notice_id: row.notice_id,
        notice_title: row.notice_title,
        notice_type: row.notice_type,
        notice_content: row.notice_content,
        status: row.status,
        create_time: row.create_time,
        creator_name: row.creator_name,
    })
}

fn parse_notice_type(value: &str) -> anyhow::Result<i16> {
    let notice_type = value
        .parse::<i16>()
        .map_err(|_| anyhow::anyhow!("公告类型必须是数字"))?;
    if matches!(notice_type, 1 | 2) {
        Ok(notice_type)
    } else {
        Err(anyhow::anyhow!("公告类型必须是 1 或 2"))
    }
}

fn parse_notice_status(value: &str) -> anyhow::Result<i16> {
    let status = value
        .parse::<i16>()
        .map_err(|_| anyhow::anyhow!("公告状态必须是数字"))?;
    if matches!(status, 0 | 1) {
        Ok(status)
    } else {
        Err(anyhow::anyhow!("公告状态必须是 0 或 1"))
    }
}

fn validate_notice_fields(
    notice_title: &str,
    notice_type: &str,
    notice_content: &str,
    status: &str,
) -> anyhow::Result<(i16, i16)> {
    if notice_title.trim().is_empty() {
        return Err(anyhow::anyhow!("公告标题不能为空"));
    }
    if notice_title.chars().count() > 50 {
        return Err(anyhow::anyhow!("公告标题不能超过 50 个字符"));
    }
    if notice_content.trim().is_empty() {
        return Err(anyhow::anyhow!("公告内容不能为空"));
    }

    Ok((
        parse_notice_type(notice_type)?,
        parse_notice_status(status)?,
    ))
}

pub async fn list_notices(
    query: &NoticeQuery,
    db_pool: &PgPool,
) -> anyhow::Result<PageData<NoticeDTO>> {
    let (page, page_size, offset) = page_bounds(query.page, query.page_size);
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
            SELECT n.notice_id, n.notice_title, n.notice_type, n.notice_content,
                   n.status, n.created_at AS create_time, u.username AS creator_name,
                   COUNT(*) OVER() AS total_count
            FROM sys_notice n
            LEFT JOIN user_info u ON u.id = n.creator_id
            WHERE 1 = 1
        "#,
    );

    if let Some(notice_title) = query
        .notice_title
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        query_builder.push(" AND n.notice_title ILIKE ");
        query_builder.push_bind(format!("%{notice_title}%"));
    }

    if let Some(notice_type) = query
        .notice_type
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        let notice_type = parse_notice_type(notice_type)?;
        query_builder.push(" AND n.notice_type = ");
        query_builder.push_bind(notice_type);
    }

    if let Some(creator_name) = query
        .creator_name
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        query_builder.push(" AND u.username ILIKE ");
        query_builder.push_bind(format!("%{creator_name}%"));
    }

    query_builder.push(" ORDER BY n.created_at DESC, n.notice_id DESC LIMIT ");
    query_builder.push_bind(page_size as i64);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let rows: Vec<NoticeWithTotal> = query_builder.build_query_as().fetch_all(db_pool).await?;
    let total_count = rows.first().map(|row| row.total_count).unwrap_or(0);
    let items = rows.into_iter().map(to_dto_with_total).collect();

    Ok(PageData {
        items,
        total_count: total_count as usize,
        page,
        page_size,
        total_pages: total_pages(total_count, page_size),
    })
}

pub async fn get_notice(notice_id: i64, db_pool: &PgPool) -> anyhow::Result<NoticeDTO> {
    let row = sqlx::query_as::<_, NoticeRow>(
        r#"
        SELECT n.notice_id, n.notice_title, n.notice_type, n.notice_content,
               n.status, n.created_at AS create_time, u.username AS creator_name
        FROM sys_notice n
        LEFT JOIN user_info u ON u.id = n.creator_id
        WHERE n.notice_id = $1
        "#,
    )
    .bind(notice_id)
    .fetch_optional(db_pool)
    .await?
    .ok_or(SystemNoticeNotFoundError { notice_id })?;

    Ok(to_dto(row))
}

pub async fn create_notice(
    data: &CreateNoticeDTO,
    creator_id: i64,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    let (notice_type, status) = validate_notice_fields(
        &data.notice_title,
        &data.notice_type,
        &data.notice_content,
        &data.status,
    )?;

    sqlx::query(
        r#"
        INSERT INTO sys_notice (
            notice_title, notice_type, notice_content, status, creator_id, remark
        )
        VALUES ($1, $2, $3, $4, $5, '')
        "#,
    )
    .bind(&data.notice_title)
    .bind(notice_type)
    .bind(&data.notice_content)
    .bind(status)
    .bind(creator_id)
    .execute(db_pool)
    .await?;

    Ok(())
}

pub async fn update_notice(
    notice_id: i64,
    data: &UpdateNoticeDTO,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    let (notice_type, status) = validate_notice_fields(
        &data.notice_title,
        &data.notice_type,
        &data.notice_content,
        &data.status,
    )?;

    let rows_affected = sqlx::query(
        r#"
        UPDATE sys_notice
        SET notice_title = $2,
            notice_type = $3,
            notice_content = $4,
            status = $5
        WHERE notice_id = $1
        "#,
    )
    .bind(notice_id)
    .bind(&data.notice_title)
    .bind(notice_type)
    .bind(&data.notice_content)
    .bind(status)
    .execute(db_pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(SystemNoticeNotFoundError { notice_id }.into());
    }

    Ok(())
}

pub async fn delete_notices(notice_ids: &[i64], db_pool: &PgPool) -> anyhow::Result<u64> {
    if notice_ids.is_empty() {
        return Err(anyhow::anyhow!("noticeIds 不能为空"));
    }

    let result = sqlx::query("DELETE FROM sys_notice WHERE notice_id = ANY($1)")
        .bind(notice_ids)
        .execute(db_pool)
        .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::{
        SystemNoticeNotFoundError, parse_notice_status, parse_notice_type, validate_notice_fields,
    };

    #[test]
    fn parse_notice_type_accepts_keystone_values() {
        assert_eq!(parse_notice_type("1").unwrap(), 1);
        assert_eq!(parse_notice_type("2").unwrap(), 2);
        assert!(parse_notice_type("3").is_err());
    }

    #[test]
    fn parse_notice_status_accepts_keystone_values() {
        assert_eq!(parse_notice_status("0").unwrap(), 0);
        assert_eq!(parse_notice_status("1").unwrap(), 1);
        assert!(parse_notice_status("2").is_err());
    }

    #[test]
    fn validate_notice_fields_rejects_blank_title_or_content() {
        assert!(validate_notice_fields("", "1", "内容", "1").is_err());
        assert!(validate_notice_fields("标题", "1", "", "1").is_err());
    }

    #[test]
    fn validate_notice_fields_rejects_long_title() {
        let title = "a".repeat(51);
        assert!(validate_notice_fields(&title, "1", "内容", "1").is_err());
    }

    #[test]
    fn not_found_error_matches_keystone_message() {
        assert_eq!(
            SystemNoticeNotFoundError { notice_id: 12 }.to_string(),
            "找不到ID为 12 的 通知公告"
        );
    }
}
