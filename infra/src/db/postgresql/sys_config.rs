use common::SystemConfigQuery;
use common::dto::{SystemConfigDTO, UpdateSystemConfigDTO};
use common::po::PageData;
use serde_json::Value;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};

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

fn validate_config_value(config_value: &str, config_options: &[String]) -> anyhow::Result<()> {
    if config_value.trim().is_empty() {
        return Err(anyhow::anyhow!("配置值不能为空"));
    }

    if !config_options.is_empty() && !config_options.iter().any(|option| option == config_value) {
        return Err(anyhow::anyhow!("配置值不在可选项中"));
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
) -> anyhow::Result<SystemConfigDTO> {
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
    .ok_or_else(|| anyhow::anyhow!("参数配置不存在"))?;

    Ok(to_dto(row))
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
    .ok_or_else(|| anyhow::anyhow!("参数配置不存在"))?;

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
    use super::{allow_change_flag, parse_config_options, validate_config_value};
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
        assert!(validate_config_value(" ", &[]).is_err());
    }

    #[test]
    fn validate_config_value_rejects_value_outside_options() {
        assert!(
            validate_config_value("maybe", &["true".to_string(), "false".to_string()]).is_err()
        );
    }

    #[test]
    fn allow_change_flag_matches_keystone_yes_no() {
        assert_eq!(allow_change_flag(true), (1, "是".to_string()));
        assert_eq!(allow_change_flag(false), (0, "否".to_string()));
    }
}
