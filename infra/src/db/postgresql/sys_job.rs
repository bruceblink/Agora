use chrono::{DateTime, Days, NaiveDate, NaiveTime, Utc};
use common::JobQuery;
use common::dto::{CreateJobDTO, JobDTO, UpdateJobDTO, UpdateJobStatusDTO};
use common::po::PageData;
use serde_json::json;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};

const DEFAULT_PAGE: u32 = 1;
const DEFAULT_PAGE_SIZE: u32 = 10;
const MAX_PAGE_SIZE: u32 = 500;

#[derive(Debug, Clone)]
pub struct ScheduledTaskRunConfig {
    pub name: String,
    pub cmd: String,
    pub url: String,
    pub arg: String,
    pub cron_expr: String,
    pub retry_times: u8,
}

#[derive(Debug, FromRow)]
struct JobWithTotal {
    id: i64,
    name: String,
    cron: String,
    job_group: String,
    invoke_target: String,
    concurrent: i16,
    status: i16,
    remark: Option<String>,
    created_at: DateTime<Utc>,
    total_count: i64,
}

#[derive(Debug, FromRow)]
struct JobRow {
    id: i64,
    name: String,
    cron: String,
    job_group: String,
    invoke_target: String,
    concurrent: i16,
    status: i16,
    remark: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct RunConfigRow {
    name: String,
    cron: String,
    params: serde_json::Value,
    retry_times: i16,
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

fn job_sort_column(value: Option<&str>) -> &'static str {
    match value {
        Some("jobId") => "id",
        Some("jobName") => "name",
        Some("jobGroup") => "job_group",
        Some("status") => "status",
        Some("createTime") => "created_at",
        _ => "created_at",
    }
}

fn yes_or_no_label(value: i16) -> String {
    match value {
        1 => "是",
        0 => "否",
        _ => "未知",
    }
    .to_string()
}

fn job_status_label(value: i16) -> String {
    match value {
        1 => "正常",
        0 => "暂停",
        _ => "未知",
    }
    .to_string()
}

fn to_dto(row: JobRow) -> JobDTO {
    JobDTO {
        job_id: row.id,
        job_name: row.name,
        job_group: row.job_group,
        invoke_target: row.invoke_target,
        cron_expression: row.cron,
        concurrent: row.concurrent,
        concurrent_str: yes_or_no_label(row.concurrent),
        status: row.status,
        status_str: job_status_label(row.status),
        remark: row.remark,
        create_time: row.created_at,
    }
}

fn to_dto_with_total(row: JobWithTotal) -> JobDTO {
    to_dto(JobRow {
        id: row.id,
        name: row.name,
        cron: row.cron,
        job_group: row.job_group,
        invoke_target: row.invoke_target,
        concurrent: row.concurrent,
        status: row.status,
        remark: row.remark,
        created_at: row.created_at,
    })
}

fn validate_job_flags(status: i16, concurrent: i16) -> anyhow::Result<()> {
    if !matches!(status, 0 | 1) {
        return Err(anyhow::anyhow!("任务状态必须是 0 或 1"));
    }
    if !matches!(concurrent, 0 | 1) {
        return Err(anyhow::anyhow!("并发标志必须是 0 或 1"));
    }
    Ok(())
}

fn validate_job_fields(
    job_name: &str,
    job_group: &str,
    invoke_target: &str,
    cron_expression: &str,
    status: i16,
    concurrent: i16,
) -> anyhow::Result<String> {
    if job_name.trim().is_empty() {
        return Err(anyhow::anyhow!("任务名称不能为空"));
    }
    if job_group.trim().is_empty() {
        return Err(anyhow::anyhow!("任务组名不能为空"));
    }
    if cron_expression.trim().is_empty() {
        return Err(anyhow::anyhow!("Cron 表达式不能为空"));
    }
    validate_job_flags(status, concurrent)?;
    extract_invoke_target_method(invoke_target)
}

pub fn extract_invoke_target_method(invoke_target: &str) -> anyhow::Result<String> {
    let trimmed = invoke_target.trim();
    if !trimmed.ends_with("()") {
        return Err(anyhow::anyhow!("调用目标格式必须为 bean.method()"));
    }

    let target = &trimmed[..trimmed.len() - 2];
    let Some((bean, method)) = target.split_once('.') else {
        return Err(anyhow::anyhow!("调用目标格式必须为 bean.method()"));
    };

    if !is_identifier(bean) || !is_identifier(method) {
        return Err(anyhow::anyhow!("调用目标格式必须为 bean.method()"));
    }

    Ok(method.to_string())
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn params_for_invoke_target(
    invoke_target: &str,
    existing: Option<&serde_json::Value>,
) -> serde_json::Value {
    let cmd = extract_invoke_target_method(invoke_target).unwrap_or_default();
    let arg = existing
        .and_then(|value| value.get("arg"))
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let url = existing
        .and_then(|value| value.get("url"))
        .and_then(|value| value.as_str())
        .unwrap_or("");

    json!({
        "cmd": cmd,
        "arg": arg,
        "url": url,
        "invokeTarget": invoke_target,
    })
}

fn normalize_retry_times(retry_times: i16) -> anyhow::Result<u8> {
    u8::try_from(retry_times)
        .map_err(|_| anyhow::anyhow!("定时任务重试次数超出范围: {retry_times}"))
}

pub async fn list_jobs(query: &JobQuery, db_pool: &PgPool) -> anyhow::Result<PageData<JobDTO>> {
    let (page, page_size, offset) = page_bounds(query.page, query.page_num, query.page_size);
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
            SELECT id, name, cron, job_group, invoke_target, concurrent,
                   status, remark, created_at, COUNT(*) OVER() AS total_count
            FROM scheduled_tasks
            WHERE deleted = FALSE
        "#,
    );

    if let Some(job_name) = query.job_name.as_deref().filter(|value| !value.is_empty()) {
        query_builder.push(" AND name ILIKE ");
        query_builder.push_bind(format!("%{job_name}%"));
    }

    if let Some(job_group) = query.job_group.as_deref().filter(|value| !value.is_empty()) {
        query_builder.push(" AND job_group ILIKE ");
        query_builder.push_bind(format!("%{job_group}%"));
    }

    if let Some(status) = query.status {
        validate_job_flags(status, 0)?;
        query_builder.push(" AND status = ");
        query_builder.push_bind(status);
    }

    if let Some(begin_time) = query.begin_time {
        query_builder.push(" AND created_at >= ");
        query_builder.push_bind(day_start(begin_time));
    }

    if let Some(end_time) = query.end_time {
        query_builder.push(" AND created_at < ");
        query_builder.push_bind(next_day_start(end_time));
    }

    query_builder.push(" ORDER BY ");
    query_builder.push(job_sort_column(query.order_column.as_deref()));
    query_builder.push(" ");
    query_builder.push(sort_direction(query.order_direction.as_deref()).unwrap_or("DESC"));
    query_builder.push(" NULLS LAST, id DESC LIMIT ");
    query_builder.push_bind(page_size as i64);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let rows: Vec<JobWithTotal> = query_builder.build_query_as().fetch_all(db_pool).await?;
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

pub async fn get_job(job_id: i64, db_pool: &PgPool) -> anyhow::Result<JobDTO> {
    let row = sqlx::query_as::<_, JobRow>(
        r#"
        SELECT id, name, cron, job_group, invoke_target, concurrent,
               status, remark, created_at
        FROM scheduled_tasks
        WHERE id = $1
          AND deleted = FALSE
        "#,
    )
    .bind(job_id)
    .fetch_optional(db_pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("定时任务不存在"))?;

    Ok(to_dto(row))
}

pub async fn create_job(data: &CreateJobDTO, db_pool: &PgPool) -> anyhow::Result<()> {
    validate_job_fields(
        &data.job_name,
        &data.job_group,
        &data.invoke_target,
        &data.cron_expression,
        data.status,
        data.concurrent,
    )?;
    let params = params_for_invoke_target(&data.invoke_target, None);

    sqlx::query(
        r#"
        INSERT INTO scheduled_tasks (
            name, cron, params, is_enabled, retry_times, last_status,
            job_group, invoke_target, concurrent, status, remark, deleted
        )
        VALUES ($1, $2, $3, $4, 0, 'pending', $5, $6, $7, $8, $9, FALSE)
        "#,
    )
    .bind(data.job_name.trim())
    .bind(data.cron_expression.trim())
    .bind(params)
    .bind(data.status == 1)
    .bind(data.job_group.trim())
    .bind(data.invoke_target.trim())
    .bind(data.concurrent)
    .bind(data.status)
    .bind(data.remark.as_deref().unwrap_or(""))
    .execute(db_pool)
    .await?;

    Ok(())
}

pub async fn update_job(job_id: i64, data: &UpdateJobDTO, db_pool: &PgPool) -> anyhow::Result<()> {
    validate_job_fields(
        &data.job_name,
        &data.job_group,
        &data.invoke_target,
        &data.cron_expression,
        data.status,
        data.concurrent,
    )?;

    let existing_params = sqlx::query_scalar::<_, serde_json::Value>(
        r#"
        SELECT params
        FROM scheduled_tasks
        WHERE id = $1
          AND deleted = FALSE
        "#,
    )
    .bind(job_id)
    .fetch_optional(db_pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("定时任务不存在"))?;
    let params = params_for_invoke_target(&data.invoke_target, Some(&existing_params));

    let rows_affected = sqlx::query(
        r#"
        UPDATE scheduled_tasks
        SET name = $2,
            cron = $3,
            params = $4,
            is_enabled = $5,
            job_group = $6,
            invoke_target = $7,
            concurrent = $8,
            status = $9,
            remark = $10,
            updated_at = NOW()
        WHERE id = $1
          AND deleted = FALSE
        "#,
    )
    .bind(job_id)
    .bind(data.job_name.trim())
    .bind(data.cron_expression.trim())
    .bind(params)
    .bind(data.status == 1)
    .bind(data.job_group.trim())
    .bind(data.invoke_target.trim())
    .bind(data.concurrent)
    .bind(data.status)
    .bind(data.remark.as_deref().unwrap_or(""))
    .execute(db_pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(anyhow::anyhow!("定时任务不存在"));
    }

    Ok(())
}

pub async fn update_job_status(
    job_id: i64,
    data: &UpdateJobStatusDTO,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    validate_job_flags(data.status, 0)?;
    let rows_affected = sqlx::query(
        r#"
        UPDATE scheduled_tasks
        SET status = $2,
            is_enabled = $3,
            updated_at = NOW()
        WHERE id = $1
          AND deleted = FALSE
        "#,
    )
    .bind(job_id)
    .bind(data.status)
    .bind(data.status == 1)
    .execute(db_pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(anyhow::anyhow!("定时任务不存在"));
    }

    Ok(())
}

pub async fn delete_jobs(job_ids: &[i64], db_pool: &PgPool) -> anyhow::Result<u64> {
    if job_ids.is_empty() {
        return Err(anyhow::anyhow!("jobIds 不能为空"));
    }
    if job_ids.iter().any(|id| *id <= 0) {
        return Err(anyhow::anyhow!("jobIds 必须为正整数"));
    }

    let result = sqlx::query("DELETE FROM scheduled_tasks WHERE id = ANY($1)")
        .bind(job_ids)
        .execute(db_pool)
        .await?;

    Ok(result.rows_affected())
}

pub async fn get_scheduled_task_run_config(
    job_id: i64,
    db_pool: &PgPool,
) -> anyhow::Result<ScheduledTaskRunConfig> {
    let row = sqlx::query_as::<_, RunConfigRow>(
        r#"
        SELECT name, cron, params, retry_times
        FROM scheduled_tasks
        WHERE id = $1
          AND deleted = FALSE
        "#,
    )
    .bind(job_id)
    .fetch_optional(db_pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("定时任务不存在"))?;

    let cmd = row
        .params
        .get("cmd")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    if cmd.trim().is_empty() {
        return Err(anyhow::anyhow!("定时任务缺少 params.cmd"));
    }

    Ok(ScheduledTaskRunConfig {
        name: row.name,
        cmd,
        url: row
            .params
            .get("url")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string(),
        arg: row
            .params
            .get("arg")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string(),
        cron_expr: row.cron,
        retry_times: normalize_retry_times(row.retry_times)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        extract_invoke_target_method, job_status_label, params_for_invoke_target,
        validate_job_flags, yes_or_no_label,
    };

    #[test]
    fn extract_invoke_target_method_accepts_keystone_format() {
        assert_eq!(
            extract_invoke_target_method("timerTask.fetch_all_news()").unwrap(),
            "fetch_all_news"
        );
        assert!(extract_invoke_target_method("fetch_all_news").is_err());
        assert!(extract_invoke_target_method("timerTask.fetch-all-news()").is_err());
    }

    #[test]
    fn labels_match_keystone_enums() {
        assert_eq!(job_status_label(1), "正常");
        assert_eq!(job_status_label(0), "暂停");
        assert_eq!(yes_or_no_label(1), "是");
        assert_eq!(yes_or_no_label(0), "否");
    }

    #[test]
    fn validate_job_flags_accepts_zero_or_one() {
        assert!(validate_job_flags(1, 0).is_ok());
        assert!(validate_job_flags(2, 0).is_err());
        assert!(validate_job_flags(1, 2).is_err());
    }

    #[test]
    fn params_for_invoke_target_preserves_existing_arg_and_url() {
        let existing = serde_json::json!({"arg": "https://example.com", "url": "u"});
        let params = params_for_invoke_target("timerTask.fetch_all_news()", Some(&existing));

        assert_eq!(params["cmd"], "fetch_all_news");
        assert_eq!(params["arg"], "https://example.com");
        assert_eq!(params["url"], "u");
    }
}
