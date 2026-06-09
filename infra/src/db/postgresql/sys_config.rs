use common::SystemConfigQuery;
use common::dto::{SystemConfigDTO, SystemConfigDetailDTO, UpdateSystemConfigDTO};
use common::po::PageData;
use serde_json::Value;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use std::fmt;

const CAPTCHA_CONFIG_KEY: &str = "sys.account.captchaOnOff";
const DEFAULT_PAGE: u32 = 1;
const DEFAULT_PAGE_SIZE: u32 = 20;

#[derive(Debug, FromRow)]
struct SystemConfigWithTotal {
    config_id: i64,
    config_name: String,
    config_key: String,
    config_options: Value,
    config_value: String,
    is_allow_change: bool,
    remark: Option<String>,
    create_time: chrono::DateTime<chrono::Utc>,
    total_count: i64,
}

#[derive(Debug, FromRow)]
struct SystemConfigRow {
    config_id: i64,
    config_name: String,
    config_key: String,
    config_options: Value,
    config_value: String,
    is_allow_change: bool,
    remark: Option<String>,
    create_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemConfigValidationError {
    ValueEmpty,
    ValueNotInOptions,
}

impl fmt::Display for SystemConfigValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ValueEmpty => "参数键值不允许为空",
            Self::ValueNotInOptions => "参数键值不存在列表中",
        };
        f.write_str(message)
    }
}

impl std::error::Error for SystemConfigValidationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemConfigNotFoundError {
    pub config_id: i64,
}

impl fmt::Display for SystemConfigNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "找不到ID为 {} 的 参数配置", self.config_id)
    }
}

impl std::error::Error for SystemConfigNotFoundError {}

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

fn parse_config_options(raw_options: &Value) -> Vec<String> {
    raw_options
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn allow_change_flag(is_allow_change: bool) -> (i16, String) {
    if is_allow_change {
        (1, "是".to_string())
    } else {
        (0, "否".to_string())
    }
}

fn to_dto(row: SystemConfigRow) -> SystemConfigDTO {
    let (is_allow_change, is_allow_change_str) = allow_change_flag(row.is_allow_change);
    SystemConfigDTO {
        config_id: row.config_id.to_string(),
        config_name: row.config_name,
        config_key: row.config_key,
        config_value: row.config_value,
        config_options: parse_config_options(&row.config_options),
        is_allow_change,
        is_allow_change_str,
        remark: row.remark,
        create_time: row.create_time,
    }
}

fn to_dto_with_total(row: SystemConfigWithTotal) -> SystemConfigDTO {
    to_dto(SystemConfigRow {
        config_id: row.config_id,
        config_name: row.config_name,
        config_key: row.config_key,
        config_options: row.config_options,
        config_value: row.config_value,
        is_allow_change: row.is_allow_change,
        remark: row.remark,
        create_time: row.create_time,
    })
}

fn validate_config_value(
    config_value: &str,
    config_options: &[String],
) -> Result<(), SystemConfigValidationError> {
    if config_value.trim().is_empty() {
        return Err(SystemConfigValidationError::ValueEmpty);
    }

    if !config_options.is_empty() && !config_options.iter().any(|option| option == config_value) {
        return Err(SystemConfigValidationError::ValueNotInOptions);
    }

    Ok(())
}

pub async fn list_system_configs(
    query: &SystemConfigQuery,
    db_pool: &PgPool,
) -> anyhow::Result<PageData<SystemConfigDTO>> {
    let (page, page_size, offset) = page_bounds(query.page, query.page_size);
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
            SELECT config_id, config_name, config_key, config_options, config_value,
                   is_allow_change, remark, created_at AS create_time,
                   COUNT(*) OVER() AS total_count
            FROM sys_config
            WHERE 1 = 1
        "#,
    );

    if let Some(config_name) = query
        .config_name
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        query_builder.push(" AND config_name ILIKE ");
        query_builder.push_bind(format!("%{config_name}%"));
    }

    if let Some(config_key) = query
        .config_key
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        query_builder.push(" AND config_key = ");
        query_builder.push_bind(config_key);
    }

    if let Some(is_allow_change) = query.is_allow_change {
        query_builder.push(" AND is_allow_change = ");
        query_builder.push_bind(is_allow_change);
    }

    query_builder.push(" ORDER BY config_id ASC LIMIT ");
    query_builder.push_bind(page_size as i64);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let rows: Vec<SystemConfigWithTotal> =
        query_builder.build_query_as().fetch_all(db_pool).await?;
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

pub async fn get_system_config(
    config_id: i64,
    db_pool: &PgPool,
) -> anyhow::Result<Option<SystemConfigDetailDTO>> {
    let row = sqlx::query_as::<_, SystemConfigRow>(
        r#"
        SELECT config_id, config_name, config_key, config_options, config_value,
               is_allow_change, remark, created_at AS create_time
        FROM sys_config
        WHERE config_id = $1
        "#,
    )
    .bind(config_id)
    .fetch_optional(db_pool)
    .await?;

    Ok(row.map(to_dto).map(SystemConfigDetailDTO::from))
}

pub async fn update_system_config(
    config_id: i64,
    data: &UpdateSystemConfigDTO,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    let row = sqlx::query_as::<_, SystemConfigRow>(
        r#"
        SELECT config_id, config_name, config_key, config_options, config_value,
               is_allow_change, remark, created_at AS create_time
        FROM sys_config
        WHERE config_id = $1
        "#,
    )
    .bind(config_id)
    .fetch_optional(db_pool)
    .await?
    .ok_or(SystemConfigNotFoundError { config_id })?;

    let config_options = parse_config_options(&row.config_options);
    validate_config_value(&data.config_value, &config_options)?;

    sqlx::query(
        r#"
        UPDATE sys_config
        SET config_value = $2
        WHERE config_id = $1
        "#,
    )
    .bind(config_id)
    .bind(&data.config_value)
    .execute(db_pool)
    .await?;

    Ok(())
}

pub async fn get_config_value_by_key(key: &str, db_pool: &PgPool) -> anyhow::Result<String> {
    if key.trim().is_empty() {
        return Ok(String::new());
    }

    let value = sqlx::query_scalar::<_, String>(
        r#"
        SELECT config_value
        FROM sys_config
        WHERE config_key = $1
        "#,
    )
    .bind(key)
    .fetch_optional(db_pool)
    .await?;

    Ok(value.unwrap_or_default())
}

pub async fn is_captcha_on(db_pool: &PgPool) -> anyhow::Result<bool> {
    let value = get_config_value_by_key(CAPTCHA_CONFIG_KEY, db_pool).await?;
    Ok(value.eq_ignore_ascii_case("true"))
}

#[cfg(test)]
mod tests {
    use super::{
        SystemConfigValidationError, allow_change_flag, parse_config_options, validate_config_value,
    };
    use serde_json::json;

    #[test]
    fn parse_config_options_ignores_non_string_values() {
        assert_eq!(
            parse_config_options(&json!(["true", 1, "false"])),
            vec!["true".to_string(), "false".to_string()]
        );
    }

    #[test]
    fn validate_config_value_rejects_empty_value() {
        assert_eq!(
            validate_config_value(" ", &[]).unwrap_err(),
            SystemConfigValidationError::ValueEmpty
        );
    }

    #[test]
    fn validate_config_value_rejects_value_outside_options() {
        assert_eq!(
            validate_config_value("maybe", &["true".to_string(), "false".to_string()]).unwrap_err(),
            SystemConfigValidationError::ValueNotInOptions
        );
    }

    #[test]
    fn allow_change_flag_matches_keystone_yes_no() {
        assert_eq!(allow_change_flag(true), (1, "是".to_string()));
        assert_eq!(allow_change_flag(false), (0, "否".to_string()));
    }
}
