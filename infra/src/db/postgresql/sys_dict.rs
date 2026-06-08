use common::dto::{
    ConfigDTO, CreateDictDataDTO, CreateDictTypeDTO, DictDataDTO, DictTypeDTO, DictionaryDataDTO,
    UpdateDictDataDTO, UpdateDictTypeDTO,
};
use common::po::PageData;
use common::{DictDataQuery, DictTypeQuery};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use std::collections::{BTreeMap, HashMap};

const DEFAULT_PAGE: u32 = 1;
const DEFAULT_PAGE_SIZE: u32 = 20;

#[derive(Debug, FromRow)]
struct DictTypeWithTotal {
    dict_id: i64,
    dict_name: String,
    dict_type: String,
    status: i16,
    remark: Option<String>,
    create_time: chrono::DateTime<chrono::Utc>,
    total_count: i64,
}

#[derive(Debug, FromRow)]
struct DictDataWithTotal {
    dict_code: i64,
    dict_type: String,
    dict_label: String,
    dict_value: String,
    dict_sort: i32,
    is_default: i16,
    css_class: Option<String>,
    list_class: Option<String>,
    status: i16,
    remark: Option<String>,
    create_time: chrono::DateTime<chrono::Utc>,
    total_count: i64,
}

#[derive(Debug, FromRow)]
struct DictDataMapRow {
    dict_type: String,
    dict_label: String,
    dict_value: String,
    list_class: Option<String>,
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

fn dict_type_dto(row: DictTypeWithTotal) -> DictTypeDTO {
    DictTypeDTO {
        dict_id: row.dict_id,
        dict_name: row.dict_name,
        dict_type: row.dict_type,
        status: row.status,
        remark: row.remark,
        create_time: row.create_time,
    }
}

fn dict_data_dto(row: DictDataWithTotal) -> DictDataDTO {
    DictDataDTO {
        dict_code: row.dict_code,
        dict_type: row.dict_type,
        dict_label: row.dict_label,
        dict_value: row.dict_value,
        dict_sort: row.dict_sort,
        is_default: row.is_default,
        css_class: row.css_class,
        list_class: row.list_class,
        status: row.status,
        remark: row.remark,
        create_time: row.create_time,
    }
}

pub async fn list_dict_types(
    query: &DictTypeQuery,
    db_pool: &PgPool,
) -> anyhow::Result<PageData<DictTypeDTO>> {
    let (page, page_size, offset) = page_bounds(query.page, query.page_size);
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
            SELECT dict_id, dict_name, dict_type, status, remark, created_at AS create_time,
                   COUNT(*) OVER() AS total_count
            FROM sys_dict_type
            WHERE 1 = 1
        "#,
    );

    if let Some(dict_name) = query.dict_name.as_deref().filter(|value| !value.is_empty()) {
        query_builder.push(" AND dict_name ILIKE ");
        query_builder.push_bind(format!("%{dict_name}%"));
    }

    if let Some(dict_type) = query.dict_type.as_deref().filter(|value| !value.is_empty()) {
        query_builder.push(" AND dict_type ILIKE ");
        query_builder.push_bind(format!("%{dict_type}%"));
    }

    if let Some(status) = query.status {
        query_builder.push(" AND status = ");
        query_builder.push_bind(status);
    }

    query_builder.push(" ORDER BY dict_id ASC LIMIT ");
    query_builder.push_bind(page_size as i64);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let rows: Vec<DictTypeWithTotal> = query_builder.build_query_as().fetch_all(db_pool).await?;
    let total_count = rows.first().map(|row| row.total_count).unwrap_or(0);
    let items = rows.into_iter().map(dict_type_dto).collect();

    Ok(PageData {
        items,
        total_count: total_count as usize,
        page,
        page_size,
        total_pages: total_pages(total_count, page_size),
    })
}

pub async fn get_dict_type(dict_id: i64, db_pool: &PgPool) -> anyhow::Result<DictTypeDTO> {
    let row = sqlx::query_as::<_, DictTypeDTO>(
        r#"
        SELECT dict_id, dict_name, dict_type, status, remark, created_at AS create_time
        FROM sys_dict_type
        WHERE dict_id = $1
        "#,
    )
    .bind(dict_id)
    .fetch_optional(db_pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("字典类型不存在"))?;

    Ok(row)
}

pub async fn create_dict_type(data: &CreateDictTypeDTO, db_pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO sys_dict_type (dict_name, dict_type, status, remark)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(&data.dict_name)
    .bind(&data.dict_type)
    .bind(data.status)
    .bind(&data.remark)
    .execute(db_pool)
    .await?;

    Ok(())
}

pub async fn update_dict_type(
    dict_id: i64,
    data: &UpdateDictTypeDTO,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    let mut tx = db_pool.begin().await?;
    let old_dict_type: String =
        sqlx::query_scalar("SELECT dict_type FROM sys_dict_type WHERE dict_id = $1")
            .bind(dict_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| anyhow::anyhow!("字典类型不存在"))?;

    let rows_affected = sqlx::query(
        r#"
        UPDATE sys_dict_type
        SET dict_name = $2,
            dict_type = $3,
            status = $4,
            remark = $5
        WHERE dict_id = $1
        "#,
    )
    .bind(dict_id)
    .bind(&data.dict_name)
    .bind(&data.dict_type)
    .bind(data.status)
    .bind(&data.remark)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(anyhow::anyhow!("字典类型不存在"));
    }

    if old_dict_type != data.dict_type {
        sqlx::query("UPDATE sys_dict_data SET dict_type = $2 WHERE dict_type = $1")
            .bind(old_dict_type)
            .bind(&data.dict_type)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn delete_dict_type(dict_id: i64, db_pool: &PgPool) -> anyhow::Result<()> {
    let mut tx = db_pool.begin().await?;
    let dict_type: String =
        sqlx::query_scalar("SELECT dict_type FROM sys_dict_type WHERE dict_id = $1")
            .bind(dict_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| anyhow::anyhow!("字典类型不存在"))?;

    let data_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sys_dict_data WHERE dict_type = $1")
            .bind(&dict_type)
            .fetch_one(&mut *tx)
            .await?;

    if data_count > 0 {
        return Err(anyhow::anyhow!("字典类型存在数据，不能删除"));
    }

    sqlx::query("DELETE FROM sys_dict_type WHERE dict_id = $1")
        .bind(dict_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn list_dict_data(
    query: &DictDataQuery,
    db_pool: &PgPool,
) -> anyhow::Result<PageData<DictDataDTO>> {
    let (page, page_size, offset) = page_bounds(query.page, query.page_size);
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
            SELECT dict_code, dict_type, dict_label, dict_value, dict_sort, is_default,
                   css_class, list_class, status, remark, created_at AS create_time,
                   COUNT(*) OVER() AS total_count
            FROM sys_dict_data
            WHERE 1 = 1
        "#,
    );

    if let Some(dict_type) = query.dict_type.as_deref().filter(|value| !value.is_empty()) {
        query_builder.push(" AND dict_type = ");
        query_builder.push_bind(dict_type);
    }

    if let Some(dict_label) = query
        .dict_label
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        query_builder.push(" AND dict_label ILIKE ");
        query_builder.push_bind(format!("%{dict_label}%"));
    }

    if let Some(status) = query.status {
        query_builder.push(" AND status = ");
        query_builder.push_bind(status);
    }

    query_builder.push(" ORDER BY dict_type ASC, dict_sort ASC, dict_code ASC LIMIT ");
    query_builder.push_bind(page_size as i64);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let rows: Vec<DictDataWithTotal> = query_builder.build_query_as().fetch_all(db_pool).await?;
    let total_count = rows.first().map(|row| row.total_count).unwrap_or(0);
    let items = rows.into_iter().map(dict_data_dto).collect();

    Ok(PageData {
        items,
        total_count: total_count as usize,
        page,
        page_size,
        total_pages: total_pages(total_count, page_size),
    })
}

pub async fn list_dict_data_by_type(
    dict_type: &str,
    db_pool: &PgPool,
) -> anyhow::Result<Vec<DictDataDTO>> {
    let rows = sqlx::query_as::<_, DictDataDTO>(
        r#"
        SELECT dict_code, dict_type, dict_label, dict_value, dict_sort, is_default,
               css_class, list_class, status, remark, created_at AS create_time
        FROM sys_dict_data
        WHERE dict_type = $1
        ORDER BY dict_sort ASC, dict_code ASC
        "#,
    )
    .bind(dict_type)
    .fetch_all(db_pool)
    .await?;

    Ok(rows)
}

pub async fn get_dict_data(dict_code: i64, db_pool: &PgPool) -> anyhow::Result<DictDataDTO> {
    let row = sqlx::query_as::<_, DictDataDTO>(
        r#"
        SELECT dict_code, dict_type, dict_label, dict_value, dict_sort, is_default,
               css_class, list_class, status, remark, created_at AS create_time
        FROM sys_dict_data
        WHERE dict_code = $1
        "#,
    )
    .bind(dict_code)
    .fetch_optional(db_pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("字典数据不存在"))?;

    Ok(row)
}

pub async fn create_dict_data(data: &CreateDictDataDTO, db_pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO sys_dict_data (
            dict_type, dict_label, dict_value, dict_sort, is_default,
            css_class, list_class, status, remark
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(&data.dict_type)
    .bind(&data.dict_label)
    .bind(&data.dict_value)
    .bind(data.dict_sort)
    .bind(data.is_default)
    .bind(&data.css_class)
    .bind(&data.list_class)
    .bind(data.status)
    .bind(&data.remark)
    .execute(db_pool)
    .await?;

    Ok(())
}

pub async fn update_dict_data(
    dict_code: i64,
    data: &UpdateDictDataDTO,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    let rows_affected = sqlx::query(
        r#"
        UPDATE sys_dict_data
        SET dict_type = $2,
            dict_label = $3,
            dict_value = $4,
            dict_sort = $5,
            is_default = $6,
            css_class = $7,
            list_class = $8,
            status = $9,
            remark = $10
        WHERE dict_code = $1
        "#,
    )
    .bind(dict_code)
    .bind(&data.dict_type)
    .bind(&data.dict_label)
    .bind(&data.dict_value)
    .bind(data.dict_sort)
    .bind(data.is_default)
    .bind(&data.css_class)
    .bind(&data.list_class)
    .bind(data.status)
    .bind(&data.remark)
    .execute(db_pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(anyhow::anyhow!("字典数据不存在"));
    }

    Ok(())
}

pub async fn delete_dict_data(dict_code: i64, db_pool: &PgPool) -> anyhow::Result<()> {
    let rows_affected = sqlx::query("DELETE FROM sys_dict_data WHERE dict_code = $1")
        .bind(dict_code)
        .execute(db_pool)
        .await?
        .rows_affected();

    if rows_affected == 0 {
        return Err(anyhow::anyhow!("字典数据不存在"));
    }

    Ok(())
}

pub async fn get_config(db_pool: &PgPool) -> anyhow::Result<ConfigDTO> {
    let is_captcha_on = crate::is_captcha_on(db_pool).await?;
    let type_rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT dict_type
        FROM sys_dict_type
        WHERE status = 1
        ORDER BY dict_id ASC
        "#,
    )
    .fetch_all(db_pool)
    .await?;

    if type_rows.is_empty() {
        return Ok(ConfigDTO {
            is_captcha_on,
            dictionary: BTreeMap::new(),
        });
    }

    let dict_types: Vec<String> = type_rows.iter().map(|row| row.0.clone()).collect();
    let rows: Vec<DictDataMapRow> = sqlx::query_as(
        r#"
        SELECT dict_type, dict_label, dict_value, list_class
        FROM sys_dict_data
        WHERE status = 1 AND dict_type = ANY($1)
        ORDER BY dict_type ASC, dict_sort ASC, dict_code ASC
        "#,
    )
    .bind(&dict_types)
    .fetch_all(db_pool)
    .await?;

    let mut data_by_type: HashMap<String, Vec<DictionaryDataDTO>> = HashMap::new();
    for row in rows {
        let value = row.dict_value.parse::<i32>().map_err(|_| {
            anyhow::anyhow!("字典值必须是整数: {}={}", row.dict_type, row.dict_value)
        })?;
        data_by_type
            .entry(row.dict_type)
            .or_default()
            .push(DictionaryDataDTO {
                label: row.dict_label,
                value,
                css_tag: row.list_class,
            });
    }

    let mut dictionary = BTreeMap::new();
    for dict_type in dict_types {
        dictionary.insert(
            dict_type.clone(),
            data_by_type.remove(&dict_type).unwrap_or_default(),
        );
    }

    Ok(ConfigDTO {
        is_captcha_on,
        dictionary,
    })
}

#[cfg(test)]
mod tests {
    use super::{page_bounds, total_pages};

    #[test]
    fn page_bounds_defaults_to_first_page() {
        assert_eq!(page_bounds(None, None), (1, 20, 0));
    }

    #[test]
    fn page_bounds_never_returns_zero_page() {
        assert_eq!(page_bounds(Some(0), Some(0)), (1, 1, 0));
    }

    #[test]
    fn total_pages_rounds_up() {
        assert_eq!(total_pages(21, 20), 2);
        assert_eq!(total_pages(0, 20), 0);
    }
}
