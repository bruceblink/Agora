use common::RoleQuery;
use common::dto::{
    CreateRoleDTO, RoleDTO, UpdateRoleDTO, UpdateRoleDataScopeDTO, UpdateRoleStatusDTO,
};
use common::po::PageData;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Transaction};
use std::collections::HashSet;

const DEFAULT_PAGE: u32 = 1;
const DEFAULT_PAGE_SIZE: u32 = 20;

#[derive(Debug, FromRow)]
struct RoleWithTotal {
    role_id: i64,
    role_name: String,
    role_key: String,
    role_sort: i32,
    status: i16,
    remark: Option<String>,
    create_time: chrono::DateTime<chrono::Utc>,
    data_scope: i16,
    dept_id_set: String,
    total_count: i64,
}

#[derive(Debug, FromRow)]
struct RoleRow {
    role_id: i64,
    role_name: String,
    role_key: String,
    role_sort: i32,
    status: i16,
    remark: Option<String>,
    create_time: chrono::DateTime<chrono::Utc>,
    data_scope: i16,
    dept_id_set: String,
}

#[derive(Debug)]
struct NormalizedRole {
    role_name: String,
    role_key: String,
    role_sort: i32,
    status: i16,
    remark: Option<String>,
    data_scope: i16,
    menu_ids: Vec<i64>,
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

fn parse_status(value: Option<&str>) -> anyhow::Result<i16> {
    let Some(value) = value.filter(|value| !value.trim().is_empty()) else {
        return Ok(1);
    };

    let status = value
        .trim()
        .parse::<i16>()
        .map_err(|_| anyhow::anyhow!("角色状态必须是数字"))?;
    if matches!(status, 0 | 1) {
        Ok(status)
    } else {
        Err(anyhow::anyhow!("角色状态必须是 0 或 1"))
    }
}

fn parse_data_scope(value: Option<&str>) -> anyhow::Result<i16> {
    let Some(value) = value.filter(|value| !value.trim().is_empty()) else {
        return Ok(1);
    };

    let data_scope = value
        .trim()
        .parse::<i16>()
        .map_err(|_| anyhow::anyhow!("数据范围必须是数字"))?;
    if (1..=5).contains(&data_scope) {
        Ok(data_scope)
    } else {
        Err(anyhow::anyhow!("数据范围必须是 1 到 5"))
    }
}

fn validate_data_scope_value(data_scope: i16) -> anyhow::Result<()> {
    if (1..=5).contains(&data_scope) {
        Ok(())
    } else {
        Err(anyhow::anyhow!("数据范围必须是 1 到 5"))
    }
}

fn normalize_role(
    role_name: &str,
    role_key: &str,
    role_sort: i32,
    remark: Option<&str>,
    data_scope: Option<&str>,
    status: Option<&str>,
    menu_ids: &[i64],
) -> anyhow::Result<NormalizedRole> {
    let role_name = role_name.trim().to_string();
    if role_name.is_empty() {
        return Err(anyhow::anyhow!("角色名称不能为空"));
    }
    if role_name.chars().count() > 30 {
        return Err(anyhow::anyhow!("角色名称长度不能超过30个字符"));
    }

    let role_key = role_key.trim().to_string();
    if role_key.is_empty() {
        return Err(anyhow::anyhow!("权限字符不能为空"));
    }
    if role_key.chars().count() > 100 {
        return Err(anyhow::anyhow!("权限字符长度不能超过100个字符"));
    }

    if role_sort < 0 {
        return Err(anyhow::anyhow!("显示顺序不能小于 0"));
    }

    if menu_ids.iter().any(|id| *id <= 0) {
        return Err(anyhow::anyhow!("menuIds 必须为正整数"));
    }

    Ok(NormalizedRole {
        role_name,
        role_key,
        role_sort,
        status: parse_status(status)?,
        remark: remark
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        data_scope: parse_data_scope(data_scope)?,
        menu_ids: menu_ids.to_vec(),
    })
}

fn parse_dept_ids(dept_id_set: &str) -> Vec<i64> {
    dept_id_set
        .split(',')
        .filter_map(|value| value.trim().parse::<i64>().ok())
        .collect()
}

fn role_dto(row: RoleRow, selected_menu_list: Vec<i64>) -> RoleDTO {
    let selected_dept_list = parse_dept_ids(&row.dept_id_set);
    RoleDTO {
        role_id: row.role_id,
        role_name: row.role_name,
        role_key: row.role_key,
        role_sort: row.role_sort,
        status: row.status,
        remark: row.remark,
        create_time: row.create_time,
        data_scope: row.data_scope,
        selected_menu_list,
        selected_dept_list,
    }
}

fn role_dto_with_total(row: RoleWithTotal) -> RoleDTO {
    role_dto(
        RoleRow {
            role_id: row.role_id,
            role_name: row.role_name,
            role_key: row.role_key,
            role_sort: row.role_sort,
            status: row.status,
            remark: row.remark,
            create_time: row.create_time,
            data_scope: row.data_scope,
            dept_id_set: row.dept_id_set,
        },
        Vec::new(),
    )
}

async fn check_role_name_unique(
    role_id: Option<i64>,
    role_name: &str,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    let duplicated: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM roles
            WHERE role_name = $1
              AND ($2::BIGINT IS NULL OR id <> $2)
              AND deleted = FALSE
        )
        "#,
    )
    .bind(role_name)
    .bind(role_id)
    .fetch_one(db_pool)
    .await?;

    if duplicated {
        Err(anyhow::anyhow!("角色名称：{role_name}, 已存在"))
    } else {
        Ok(())
    }
}

async fn check_role_key_unique(
    role_id: Option<i64>,
    role_key: &str,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    let duplicated: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM roles
            WHERE role_key = $1
              AND ($2::BIGINT IS NULL OR id <> $2)
              AND deleted = FALSE
        )
        "#,
    )
    .bind(role_key)
    .bind(role_id)
    .fetch_one(db_pool)
    .await?;

    if duplicated {
        Err(anyhow::anyhow!("角色标识：{role_key}, 已存在"))
    } else {
        Ok(())
    }
}

async fn selected_menu_ids(role_id: i64, db_pool: &PgPool) -> anyhow::Result<Vec<i64>> {
    let menu_ids = sqlx::query_scalar(
        "SELECT menu_id FROM sys_role_menu WHERE role_id = $1 ORDER BY menu_id ASC",
    )
    .bind(role_id)
    .fetch_all(db_pool)
    .await?;

    Ok(menu_ids)
}

async fn ensure_menu_ids_exist(
    menu_ids: &[i64],
    tx: &mut Transaction<'_, Postgres>,
) -> anyhow::Result<()> {
    if menu_ids.is_empty() {
        return Ok(());
    }

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sys_menu WHERE menu_id = ANY($1) AND deleted = FALSE",
    )
    .bind(menu_ids)
    .fetch_one(&mut **tx)
    .await?;

    if count as usize == menu_ids.len() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("menuIds 包含不存在的菜单"))
    }
}

async fn sync_role_menus_and_permissions(
    role_id: i64,
    menu_ids: &[i64],
    tx: &mut Transaction<'_, Postgres>,
) -> anyhow::Result<()> {
    ensure_menu_ids_exist(menu_ids, tx).await?;

    sqlx::query("DELETE FROM sys_role_menu WHERE role_id = $1")
        .bind(role_id)
        .execute(&mut **tx)
        .await?;

    for menu_id in menu_ids {
        sqlx::query(
            r#"
            INSERT INTO sys_role_menu (role_id, menu_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(role_id)
        .bind(menu_id)
        .execute(&mut **tx)
        .await?;
    }

    sqlx::query(
        r#"
        DELETE FROM role_permissions
        WHERE role_id = $1
          AND permission_id IN (
              SELECT p.id
              FROM permissions p
              JOIN sys_menu m ON m.permission = p.name
              WHERE m.permission <> ''
          )
        "#,
    )
    .bind(role_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO permissions (name, description)
        SELECT DISTINCT permission, menu_name
        FROM sys_menu
        WHERE menu_id = ANY($1)
          AND permission <> ''
          AND deleted = FALSE
        ON CONFLICT (name) DO NOTHING
        "#,
    )
    .bind(menu_ids)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO role_permissions (role_id, permission_id)
        SELECT DISTINCT $1, p.id
        FROM permissions p
        JOIN sys_menu m ON m.permission = p.name
        WHERE m.menu_id = ANY($2)
          AND m.permission <> ''
          AND m.deleted = FALSE
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(role_id)
    .bind(menu_ids)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn list_roles(query: &RoleQuery, db_pool: &PgPool) -> anyhow::Result<PageData<RoleDTO>> {
    let (page, page_size, offset) = page_bounds(query.page, query.page_size);
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
            SELECT id AS role_id, role_name, role_key, role_sort, status, remark,
                   created_at AS create_time, data_scope, dept_id_set,
                   COUNT(*) OVER() AS total_count
            FROM roles
            WHERE deleted = FALSE
        "#,
    );

    if let Some(role_name) = query.role_name.as_deref().filter(|value| !value.is_empty()) {
        query_builder.push(" AND role_name ILIKE ");
        query_builder.push_bind(format!("%{role_name}%"));
    }

    if let Some(role_key) = query.role_key.as_deref().filter(|value| !value.is_empty()) {
        query_builder.push(" AND role_key = ");
        query_builder.push_bind(role_key);
    }

    if let Some(status) = query.status.as_deref().filter(|value| !value.is_empty()) {
        let status = parse_status(Some(status))?;
        query_builder.push(" AND status = ");
        query_builder.push_bind(status);
    }

    query_builder.push(" ORDER BY role_sort ASC, id ASC LIMIT ");
    query_builder.push_bind(page_size as i64);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let rows: Vec<RoleWithTotal> = query_builder.build_query_as().fetch_all(db_pool).await?;
    let total_count = rows.first().map(|row| row.total_count).unwrap_or(0);
    let items = rows.into_iter().map(role_dto_with_total).collect();

    Ok(PageData {
        items,
        total_count: total_count as usize,
        page,
        page_size,
        total_pages: total_pages(total_count, page_size),
    })
}

pub async fn get_role(role_id: i64, db_pool: &PgPool) -> anyhow::Result<RoleDTO> {
    let row = sqlx::query_as::<_, RoleRow>(
        r#"
        SELECT id AS role_id, role_name, role_key, role_sort, status, remark,
               created_at AS create_time, data_scope, dept_id_set
        FROM roles
        WHERE id = $1 AND deleted = FALSE
        "#,
    )
    .bind(role_id)
    .fetch_optional(db_pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("角色不存在"))?;

    let menu_ids = selected_menu_ids(role_id, db_pool).await?;
    Ok(role_dto(row, menu_ids))
}

pub async fn create_role(data: &CreateRoleDTO, db_pool: &PgPool) -> anyhow::Result<()> {
    let role = normalize_role(
        &data.role_name,
        &data.role_key,
        data.role_sort,
        data.remark.as_deref(),
        data.data_scope.as_deref(),
        data.status.as_deref(),
        &data.menu_ids,
    )?;

    check_role_name_unique(None, &role.role_name, db_pool).await?;
    check_role_key_unique(None, &role.role_key, db_pool).await?;

    let mut tx = db_pool.begin().await?;
    let role_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO roles (
            name, role_name, role_key, role_sort, data_scope, dept_id_set,
            status, remark, description, deleted
        )
        VALUES ($1, $2, $3, $4, $5, '', $6, $7, $7, FALSE)
        RETURNING id
        "#,
    )
    .bind(&role.role_key)
    .bind(&role.role_name)
    .bind(&role.role_key)
    .bind(role.role_sort)
    .bind(role.data_scope)
    .bind(role.status)
    .bind(&role.remark)
    .fetch_one(&mut *tx)
    .await?;

    sync_role_menus_and_permissions(role_id, &role.menu_ids, &mut tx).await?;
    tx.commit().await?;

    Ok(())
}

pub async fn update_role(data: &UpdateRoleDTO, db_pool: &PgPool) -> anyhow::Result<()> {
    let role = normalize_role(
        &data.role_name,
        &data.role_key,
        data.role_sort,
        data.remark.as_deref(),
        data.data_scope.as_deref(),
        data.status.as_deref(),
        &data.menu_ids,
    )?;

    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM roles WHERE id = $1 AND deleted = FALSE)")
            .bind(data.role_id)
            .fetch_one(db_pool)
            .await?;
    if !exists {
        return Err(anyhow::anyhow!("角色不存在"));
    }

    check_role_name_unique(Some(data.role_id), &role.role_name, db_pool).await?;
    check_role_key_unique(Some(data.role_id), &role.role_key, db_pool).await?;

    let mut tx = db_pool.begin().await?;
    sqlx::query(
        r#"
        UPDATE roles
        SET name = $2,
            role_name = $3,
            role_key = $4,
            role_sort = $5,
            data_scope = $6,
            status = $7,
            remark = $8,
            description = $8
        WHERE id = $1 AND deleted = FALSE
        "#,
    )
    .bind(data.role_id)
    .bind(&role.role_key)
    .bind(&role.role_name)
    .bind(&role.role_key)
    .bind(role.role_sort)
    .bind(role.data_scope)
    .bind(role.status)
    .bind(&role.remark)
    .execute(&mut *tx)
    .await?;

    sync_role_menus_and_permissions(data.role_id, &role.menu_ids, &mut tx).await?;
    tx.commit().await?;

    Ok(())
}

pub async fn update_role_status(
    role_id: i64,
    data: &UpdateRoleStatusDTO,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    if !matches!(data.status, 0 | 1) {
        return Err(anyhow::anyhow!("角色状态必须是 0 或 1"));
    }

    let rows_affected =
        sqlx::query("UPDATE roles SET status = $2 WHERE id = $1 AND deleted = FALSE")
            .bind(role_id)
            .bind(data.status)
            .execute(db_pool)
            .await?
            .rows_affected();

    if rows_affected == 0 {
        return Err(anyhow::anyhow!("角色不存在"));
    }

    Ok(())
}

pub async fn update_role_data_scope(
    role_id: i64,
    data: &UpdateRoleDataScopeDTO,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    let data_scope = data.data_scope.unwrap_or(1);
    validate_data_scope_value(data_scope)?;

    let mut seen = HashSet::new();
    for dept_id in &data.dept_ids {
        if *dept_id <= 0 {
            return Err(anyhow::anyhow!("deptIds 必须为正整数"));
        }
        if !seen.insert(*dept_id) {
            return Err(anyhow::anyhow!("重复的部门id"));
        }
    }
    let dept_id_set = data
        .dept_ids
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(",");

    let rows_affected = sqlx::query(
        "UPDATE roles SET data_scope = $2, dept_id_set = $3 WHERE id = $1 AND deleted = FALSE",
    )
    .bind(role_id)
    .bind(data_scope)
    .bind(dept_id_set)
    .execute(db_pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(anyhow::anyhow!("角色不存在"));
    }

    Ok(())
}

pub async fn delete_roles(role_ids: &[i64], db_pool: &PgPool) -> anyhow::Result<()> {
    if role_ids.is_empty() {
        return Err(anyhow::anyhow!("roleIds 不能为空"));
    }
    if role_ids.iter().any(|id| *id <= 0) {
        return Err(anyhow::anyhow!("roleIds 必须为正整数"));
    }

    let assigned_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_roles WHERE role_id = ANY($1)")
            .bind(role_ids)
            .fetch_one(db_pool)
            .await?;
    if assigned_count > 0 {
        return Err(anyhow::anyhow!(
            "角色已分配给用户，请先取消分配，再删除角色"
        ));
    }

    let mut tx = db_pool.begin().await?;
    sqlx::query("DELETE FROM sys_role_menu WHERE role_id = ANY($1)")
        .bind(role_ids)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM role_permissions WHERE role_id = ANY($1)")
        .bind(role_ids)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM roles WHERE id = ANY($1)")
        .bind(role_ids)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{normalize_role, parse_data_scope, parse_dept_ids, parse_status};

    #[test]
    fn parse_status_accepts_keystone_values() {
        assert_eq!(parse_status(Some("0")).unwrap(), 0);
        assert_eq!(parse_status(Some("1")).unwrap(), 1);
        assert!(parse_status(Some("2")).is_err());
    }

    #[test]
    fn parse_data_scope_accepts_keystone_values() {
        assert_eq!(parse_data_scope(Some("1")).unwrap(), 1);
        assert_eq!(parse_data_scope(Some("5")).unwrap(), 5);
        assert!(parse_data_scope(Some("6")).is_err());
    }

    #[test]
    fn normalize_role_rejects_blank_name_or_key() {
        assert!(normalize_role("", "admin", 1, None, Some("1"), Some("1"), &[]).is_err());
        assert!(normalize_role("管理员", "", 1, None, Some("1"), Some("1"), &[]).is_err());
    }

    #[test]
    fn normalize_role_rejects_invalid_sort_or_menu_ids() {
        assert!(normalize_role("管理员", "admin", -1, None, Some("1"), Some("1"), &[]).is_err());
        assert!(normalize_role("管理员", "admin", 1, None, Some("1"), Some("1"), &[0]).is_err());
    }

    #[test]
    fn parse_dept_ids_ignores_empty_parts() {
        assert_eq!(parse_dept_ids("1, 2,,x"), vec![1, 2]);
    }
}
