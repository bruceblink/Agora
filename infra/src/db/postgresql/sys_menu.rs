use common::MenuQuery;
use common::dto::MenuDetailResponseDTO;
use common::dto::RouterDTO;
use common::dto::{
    CreateMenuDTO, MenuDTO, MenuDetailDTO, MenuDropdownDTO, MenuMetaDTO, UpdateMenuDTO,
};
use serde_json::Value;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Transaction};
use std::cmp::Ordering;
use std::fmt;

#[derive(Debug, Clone, FromRow)]
struct MenuRow {
    menu_id: i64,
    menu_name: String,
    menu_type: i16,
    router_name: String,
    parent_id: i64,
    path: String,
    is_button: bool,
    permission: String,
    meta_info: Value,
    status: i16,
    create_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
struct MenuTreeNode {
    id: i64,
    parent_id: i64,
    label: String,
    rank: Option<i32>,
}

#[derive(Debug)]
struct NormalizedMenu {
    parent_id: i64,
    menu_name: String,
    router_name: String,
    path: String,
    status: i16,
    menu_type: i16,
    is_button: bool,
    permission: String,
    meta: MenuMetaDTO,
}

struct MenuNormalizationInput<'a> {
    parent_id: Option<i64>,
    menu_name: &'a str,
    router_name: Option<&'a str>,
    path: Option<&'a str>,
    status: Option<i16>,
    menu_type: Option<i16>,
    is_button: Option<bool>,
    permission: Option<&'a str>,
    meta: Option<&'a MenuMetaDTO>,
    fallback_menu_type: Option<i16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemMenuBusinessError {
    ObjectNotFound { id: i64 },
    NameNotUnique,
    ExternalLinkMustBeHttp,
    ParentIdNotAllowSelf,
    HasChildMenus,
    AlreadyAssignedToRole,
    ButtonNotAllowedInIframeOrOutLink,
    SubMenuOnlyAllowedInCatalog,
    CanNotChangeMenuType,
}

impl fmt::Display for SystemMenuBusinessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ObjectNotFound { id } => write!(f, "找不到ID为 {id} 的 菜单"),
            Self::NameNotUnique => f.write_str("新增菜单:{} 失败，菜单名称已存在"),
            Self::ExternalLinkMustBeHttp => f.write_str("菜单外链必须以 http(s)://开头"),
            Self::ParentIdNotAllowSelf => f.write_str("父级菜单不能选择自身"),
            Self::HasChildMenus => f.write_str("存在子菜单不允许删除"),
            Self::AlreadyAssignedToRole => f.write_str("菜单已分配给角色，不允许"),
            Self::ButtonNotAllowedInIframeOrOutLink => {
                f.write_str("不允许在Iframe和外链跳转类型下创建按钮")
            }
            Self::SubMenuOnlyAllowedInCatalog => f.write_str("只允许在目录类型底下创建子菜单"),
            Self::CanNotChangeMenuType => f.write_str("不允许更改菜单的类型"),
        }
    }
}

impl std::error::Error for SystemMenuBusinessError {}

fn status_str(status: i16) -> String {
    match status {
        1 => "正常",
        0 => "停用",
        _ => "未知",
    }
    .to_string()
}

fn menu_type_str(menu_type: i16, is_button: bool) -> Option<String> {
    if is_button {
        return None;
    }

    let label = match menu_type {
        1 => "页面",
        2 => "目录",
        3 => "内嵌Iframe",
        4 => "外链跳转",
        _ => return None,
    };
    Some(label.to_string())
}

fn meta_from_value(value: Value) -> MenuMetaDTO {
    serde_json::from_value(value).unwrap_or_default()
}

fn rank_order(left: Option<i32>, right: Option<i32>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_menu_rows(left: &MenuRow, right: &MenuRow) -> Ordering {
    let left_meta = meta_from_value(left.meta_info.clone());
    let right_meta = meta_from_value(right.meta_info.clone());
    rank_order(left_meta.rank, right_meta.rank)
        .then_with(|| right.parent_id.cmp(&left.parent_id))
        .then_with(|| left.menu_id.cmp(&right.menu_id))
}

fn compare_tree_nodes(left: &MenuTreeNode, right: &MenuTreeNode) -> Ordering {
    rank_order(left.rank, right.rank).then_with(|| left.id.cmp(&right.id))
}

fn menu_dto(row: MenuRow) -> MenuDTO {
    let meta = meta_from_value(row.meta_info);
    MenuDTO {
        id: row.menu_id,
        parent_id: row.parent_id,
        menu_name: row.menu_name,
        router_name: row.router_name,
        path: row.path,
        rank: meta.rank,
        menu_type: if row.is_button { 0 } else { row.menu_type },
        menu_type_str: menu_type_str(row.menu_type, row.is_button),
        is_button: row.is_button,
        status: row.status,
        status_str: status_str(row.status),
        create_time: row.create_time,
        icon: meta.icon,
    }
}

fn menu_detail_dto(row: MenuRow) -> MenuDetailDTO {
    let permission = row.permission.clone();
    let meta = meta_from_value(row.meta_info.clone());
    MenuDetailDTO {
        menu: menu_dto(row),
        permission,
        meta,
    }
}

fn validate_status(status: i16) -> anyhow::Result<()> {
    if matches!(status, 0 | 1) {
        Ok(())
    } else {
        Err(anyhow::anyhow!("菜单状态必须是 0 或 1"))
    }
}

fn validate_menu_type(menu_type: i16, is_button: bool) -> anyhow::Result<()> {
    if is_button {
        if (0..=4).contains(&menu_type) {
            return Ok(());
        }
    } else if (1..=4).contains(&menu_type) {
        return Ok(());
    }

    Err(anyhow::anyhow!("菜单类型必须是 1、2、3 或 4"))
}

fn normalize_menu(input: MenuNormalizationInput<'_>) -> anyhow::Result<NormalizedMenu> {
    let parent_id = input.parent_id.unwrap_or(0);
    if parent_id < 0 {
        return Err(anyhow::anyhow!("父级菜单ID不能小于 0"));
    }

    let menu_name = input.menu_name.trim().to_string();
    if menu_name.is_empty() {
        return Err(anyhow::anyhow!("菜单名称不能为空"));
    }
    if menu_name.chars().count() > 50 {
        return Err(anyhow::anyhow!("菜单名称长度不能超过50个字符"));
    }

    let router_name = input.router_name.unwrap_or("").trim().to_string();
    let path = input.path.unwrap_or("").trim().to_string();
    if path.chars().count() > 200 {
        return Err(anyhow::anyhow!("路由地址不能超过200个字符"));
    }

    let permission = input.permission.unwrap_or("").trim().to_string();
    if permission.chars().count() > 100 {
        return Err(anyhow::anyhow!("权限标识长度不能超过100个字符"));
    }

    let status = input.status.unwrap_or(0);
    validate_status(status)?;

    let is_button = input.is_button.unwrap_or(false);
    let menu_type = input.menu_type.or(input.fallback_menu_type).unwrap_or(0);
    validate_menu_type(menu_type, is_button)?;

    if menu_type == 4 && !(path.starts_with("http://") || path.starts_with("https://")) {
        return Err(SystemMenuBusinessError::ExternalLinkMustBeHttp.into());
    }

    Ok(NormalizedMenu {
        parent_id,
        menu_name,
        router_name,
        path,
        status,
        menu_type,
        is_button,
        permission,
        meta: input.meta.cloned().unwrap_or_default(),
    })
}

async fn check_parent_rules(menu: &NormalizedMenu, db_pool: &PgPool) -> anyhow::Result<()> {
    if menu.parent_id == 0 {
        return Ok(());
    }

    let parent: Option<(i16,)> =
        sqlx::query_as("SELECT menu_type FROM sys_menu WHERE menu_id = $1 AND deleted = FALSE")
            .bind(menu.parent_id)
            .fetch_optional(db_pool)
            .await?;

    if let Some((parent_menu_type,)) = parent {
        if menu.is_button && matches!(parent_menu_type, 3 | 4) {
            return Err(SystemMenuBusinessError::ButtonNotAllowedInIframeOrOutLink.into());
        }

        if !menu.is_button && parent_menu_type != 2 {
            return Err(SystemMenuBusinessError::SubMenuOnlyAllowedInCatalog.into());
        }
    }

    Ok(())
}

async fn check_menu_name_unique(
    menu_id: Option<i64>,
    menu_name: &str,
    parent_id: i64,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    let duplicated: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM sys_menu
            WHERE menu_name = $1
              AND parent_id = $2
              AND ($3::BIGINT IS NULL OR menu_id <> $3)
              AND deleted = FALSE
        )
        "#,
    )
    .bind(menu_name)
    .bind(parent_id)
    .bind(menu_id)
    .fetch_one(db_pool)
    .await?;

    if duplicated {
        Err(SystemMenuBusinessError::NameNotUnique.into())
    } else {
        Ok(())
    }
}

async fn grant_permission_to_admin(
    tx: &mut Transaction<'_, Postgres>,
    permission: &str,
    menu_name: &str,
) -> anyhow::Result<()> {
    if permission.is_empty() {
        return Ok(());
    }

    sqlx::query(
        r#"
        INSERT INTO permissions (name, description)
        VALUES ($1, $2)
        ON CONFLICT (name) DO NOTHING
        "#,
    )
    .bind(permission)
    .bind(menu_name)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO role_permissions (role_id, permission_id)
        SELECT r.id, p.id
        FROM roles r
        JOIN permissions p ON p.name = $1
        WHERE r.name = 'admin'
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(permission)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

fn build_dropdown_tree(nodes: &[MenuTreeNode], parent_id: i64) -> Vec<MenuDropdownDTO> {
    let mut children: Vec<&MenuTreeNode> = nodes
        .iter()
        .filter(|node| node.parent_id == parent_id)
        .collect();
    children.sort_by(|left, right| compare_tree_nodes(left, right));

    children
        .into_iter()
        .map(|node| MenuDropdownDTO {
            id: node.id,
            parent_id: node.parent_id,
            label: node.label.clone(),
            children: build_dropdown_tree(nodes, node.id),
        })
        .collect()
}

fn router_dto(row: MenuRow, children: Vec<RouterDTO>) -> RouterDTO {
    let mut meta = meta_from_value(row.meta_info);
    if !row.permission.is_empty() {
        meta.auths = Some(vec![row.permission]);
    }

    RouterDTO {
        name: row.router_name,
        path: row.path,
        redirect: None,
        component: None,
        rank: meta.rank,
        meta,
        children,
    }
}

fn build_router_tree(rows: &[MenuRow], parent_id: i64) -> Vec<RouterDTO> {
    let mut children: Vec<&MenuRow> = rows
        .iter()
        .filter(|row| row.parent_id == parent_id && !row.is_button)
        .collect();
    children.sort_by(|left, right| compare_menu_rows(left, right));

    children
        .into_iter()
        .map(|row| {
            let child_routes = build_router_tree(rows, row.menu_id);
            router_dto(row.clone(), child_routes)
        })
        .collect()
}

pub async fn list_menus(query: &MenuQuery, db_pool: &PgPool) -> anyhow::Result<Vec<MenuDTO>> {
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
            SELECT menu_id, menu_name, menu_type, router_name, parent_id, path,
                   is_button, permission, meta_info, status, created_at AS create_time
            FROM sys_menu
            WHERE deleted = FALSE
        "#,
    );

    if let Some(is_button) = query.is_button {
        query_builder.push(" AND is_button = ");
        query_builder.push_bind(is_button);
    }

    let mut rows: Vec<MenuRow> = query_builder.build_query_as().fetch_all(db_pool).await?;
    rows.sort_by(compare_menu_rows);

    Ok(rows.into_iter().map(menu_dto).collect())
}

pub async fn get_menu(
    menu_id: i64,
    db_pool: &PgPool,
) -> anyhow::Result<Option<MenuDetailResponseDTO>> {
    let row = sqlx::query_as::<_, MenuRow>(
        r#"
        SELECT menu_id, menu_name, menu_type, router_name, parent_id, path,
               is_button, permission, meta_info, status, created_at AS create_time
        FROM sys_menu
        WHERE menu_id = $1 AND deleted = FALSE
        "#,
    )
    .bind(menu_id)
    .fetch_optional(db_pool)
    .await?;

    Ok(row.map(menu_detail_dto).map(MenuDetailResponseDTO::from))
}

pub async fn list_menu_dropdown(db_pool: &PgPool) -> anyhow::Result<Vec<MenuDropdownDTO>> {
    let rows: Vec<MenuRow> = sqlx::query_as(
        r#"
        SELECT menu_id, menu_name, menu_type, router_name, parent_id, path,
               is_button, permission, meta_info, status, created_at AS create_time
        FROM sys_menu
        WHERE deleted = FALSE
        "#,
    )
    .fetch_all(db_pool)
    .await?;

    let mut nodes: Vec<MenuTreeNode> = rows
        .into_iter()
        .map(|row| {
            let meta = meta_from_value(row.meta_info);
            MenuTreeNode {
                id: row.menu_id,
                parent_id: row.parent_id,
                label: row.menu_name,
                rank: meta.rank,
            }
        })
        .collect();
    nodes.sort_by(compare_tree_nodes);

    Ok(build_dropdown_tree(&nodes, 0))
}

pub async fn list_user_router_tree(
    user_id: i64,
    db_pool: &PgPool,
) -> anyhow::Result<Vec<RouterDTO>> {
    let mut rows: Vec<MenuRow> = sqlx::query_as(
        r#"
        SELECT DISTINCT m.menu_id, m.menu_name, m.menu_type, m.router_name, m.parent_id,
               m.path, m.is_button, m.permission, m.meta_info, m.status,
               m.created_at AS create_time
        FROM sys_menu m
        JOIN sys_role_menu rm ON rm.menu_id = m.menu_id
        JOIN user_roles ur ON ur.role_id = rm.role_id
        JOIN roles r ON r.id = ur.role_id
        WHERE ur.user_id = $1
          AND m.status = 1
          AND m.deleted = FALSE
          AND r.status = 1
          AND r.deleted = FALSE
        "#,
    )
    .bind(user_id)
    .fetch_all(db_pool)
    .await?;

    rows.sort_by(compare_menu_rows);
    Ok(build_router_tree(&rows, 0))
}

pub async fn create_menu(data: &CreateMenuDTO, db_pool: &PgPool) -> anyhow::Result<()> {
    let menu = normalize_menu(MenuNormalizationInput {
        parent_id: data.parent_id,
        menu_name: &data.menu_name,
        router_name: data.router_name.as_deref(),
        path: data.path.as_deref(),
        status: data.status,
        menu_type: data.menu_type,
        is_button: data.is_button,
        permission: data.permission.as_deref(),
        meta: data.meta.as_ref(),
        fallback_menu_type: None,
    })?;

    check_parent_rules(&menu, db_pool).await?;
    check_menu_name_unique(None, &menu.menu_name, menu.parent_id, db_pool).await?;

    let meta_value = serde_json::to_value(&menu.meta)?;
    let mut tx = db_pool.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO sys_menu (
            menu_name, menu_type, router_name, parent_id, path, is_button,
            permission, meta_info, status, remark
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, '')
        "#,
    )
    .bind(&menu.menu_name)
    .bind(menu.menu_type)
    .bind(&menu.router_name)
    .bind(menu.parent_id)
    .bind(&menu.path)
    .bind(menu.is_button)
    .bind(&menu.permission)
    .bind(meta_value)
    .bind(menu.status)
    .execute(&mut *tx)
    .await?;

    grant_permission_to_admin(&mut tx, &menu.permission, &menu.menu_name).await?;
    tx.commit().await?;

    Ok(())
}

pub async fn update_menu(
    menu_id: i64,
    data: &UpdateMenuDTO,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    let existing: Option<(i16, bool)> = sqlx::query_as(
        "SELECT menu_type, is_button FROM sys_menu WHERE menu_id = $1 AND deleted = FALSE",
    )
    .bind(menu_id)
    .fetch_optional(db_pool)
    .await?;

    let (existing_menu_type, existing_is_button) =
        existing.ok_or(SystemMenuBusinessError::ObjectNotFound { id: menu_id })?;

    let menu = normalize_menu(MenuNormalizationInput {
        parent_id: data.parent_id,
        menu_name: &data.menu_name,
        router_name: data.router_name.as_deref(),
        path: data.path.as_deref(),
        status: data.status,
        menu_type: data.menu_type,
        is_button: data.is_button,
        permission: data.permission.as_deref(),
        meta: data.meta.as_ref(),
        fallback_menu_type: Some(existing_menu_type),
    })?;

    if menu_id == menu.parent_id {
        return Err(SystemMenuBusinessError::ParentIdNotAllowSelf.into());
    }

    if !existing_is_button && existing_menu_type != menu.menu_type {
        return Err(SystemMenuBusinessError::CanNotChangeMenuType.into());
    }

    check_parent_rules(&menu, db_pool).await?;
    check_menu_name_unique(Some(menu_id), &menu.menu_name, menu.parent_id, db_pool).await?;

    let meta_value = serde_json::to_value(&menu.meta)?;
    let mut tx = db_pool.begin().await?;

    let rows_affected = sqlx::query(
        r#"
        UPDATE sys_menu
        SET menu_name = $2,
            menu_type = $3,
            router_name = $4,
            parent_id = $5,
            path = $6,
            is_button = $7,
            permission = $8,
            meta_info = $9,
            status = $10
        WHERE menu_id = $1 AND deleted = FALSE
        "#,
    )
    .bind(menu_id)
    .bind(&menu.menu_name)
    .bind(menu.menu_type)
    .bind(&menu.router_name)
    .bind(menu.parent_id)
    .bind(&menu.path)
    .bind(menu.is_button)
    .bind(&menu.permission)
    .bind(meta_value)
    .bind(menu.status)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(SystemMenuBusinessError::ObjectNotFound { id: menu_id }.into());
    }

    grant_permission_to_admin(&mut tx, &menu.permission, &menu.menu_name).await?;
    tx.commit().await?;

    Ok(())
}

pub async fn delete_menu(menu_id: i64, db_pool: &PgPool) -> anyhow::Result<()> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM sys_menu WHERE menu_id = $1 AND deleted = FALSE)",
    )
    .bind(menu_id)
    .fetch_one(db_pool)
    .await?;

    if !exists {
        return Err(SystemMenuBusinessError::ObjectNotFound { id: menu_id }.into());
    }

    let has_child: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM sys_menu WHERE parent_id = $1 AND deleted = FALSE)",
    )
    .bind(menu_id)
    .fetch_one(db_pool)
    .await?;
    if has_child {
        return Err(SystemMenuBusinessError::HasChildMenus.into());
    }

    let assigned_to_role: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM sys_role_menu WHERE menu_id = $1)")
            .bind(menu_id)
            .fetch_one(db_pool)
            .await?;
    if assigned_to_role {
        return Err(SystemMenuBusinessError::AlreadyAssignedToRole.into());
    }

    sqlx::query("UPDATE sys_menu SET deleted = TRUE WHERE menu_id = $1")
        .bind(menu_id)
        .execute(db_pool)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        MenuNormalizationInput, MenuRow, SystemMenuBusinessError, build_router_tree,
        normalize_menu, validate_menu_type, validate_status,
    };
    use chrono::Utc;

    #[test]
    fn validate_status_accepts_keystone_flags() {
        assert!(validate_status(0).is_ok());
        assert!(validate_status(1).is_ok());
        assert!(validate_status(2).is_err());
    }

    #[test]
    fn validate_menu_type_requires_real_type_for_non_buttons() {
        assert!(validate_menu_type(1, false).is_ok());
        assert!(validate_menu_type(4, false).is_ok());
        assert!(validate_menu_type(0, false).is_err());
        assert!(validate_menu_type(0, true).is_ok());
    }

    #[test]
    fn normalize_menu_rejects_external_link_without_http() {
        let result = normalize_menu(MenuNormalizationInput {
            parent_id: Some(0),
            menu_name: "外链",
            router_name: Some("External"),
            path: Some("/external"),
            status: Some(1),
            menu_type: Some(4),
            is_button: Some(false),
            permission: None,
            meta: None,
            fallback_menu_type: None,
        });
        assert!(result.is_err());
    }

    #[test]
    fn business_errors_match_keystone_messages() {
        let cases = [
            (
                SystemMenuBusinessError::ObjectNotFound { id: 6 },
                "找不到ID为 6 的 菜单",
            ),
            (
                SystemMenuBusinessError::NameNotUnique,
                "新增菜单:{} 失败，菜单名称已存在",
            ),
            (
                SystemMenuBusinessError::ExternalLinkMustBeHttp,
                "菜单外链必须以 http(s)://开头",
            ),
            (
                SystemMenuBusinessError::ParentIdNotAllowSelf,
                "父级菜单不能选择自身",
            ),
            (
                SystemMenuBusinessError::HasChildMenus,
                "存在子菜单不允许删除",
            ),
            (
                SystemMenuBusinessError::AlreadyAssignedToRole,
                "菜单已分配给角色，不允许",
            ),
            (
                SystemMenuBusinessError::ButtonNotAllowedInIframeOrOutLink,
                "不允许在Iframe和外链跳转类型下创建按钮",
            ),
            (
                SystemMenuBusinessError::SubMenuOnlyAllowedInCatalog,
                "只允许在目录类型底下创建子菜单",
            ),
            (
                SystemMenuBusinessError::CanNotChangeMenuType,
                "不允许更改菜单的类型",
            ),
        ];

        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn build_router_tree_ignores_buttons_and_maps_permission_to_auths() {
        let rows = vec![
            MenuRow {
                menu_id: 1,
                menu_name: "系统管理".to_string(),
                menu_type: 2,
                router_name: "System".to_string(),
                parent_id: 0,
                path: "/system".to_string(),
                is_button: false,
                permission: String::new(),
                meta_info: serde_json::json!({"title": "系统管理", "rank": 1}),
                status: 1,
                create_time: Utc::now(),
            },
            MenuRow {
                menu_id: 2,
                menu_name: "用户管理".to_string(),
                menu_type: 1,
                router_name: "SystemUser".to_string(),
                parent_id: 1,
                path: "/system/user/index".to_string(),
                is_button: false,
                permission: "system:user:list".to_string(),
                meta_info: serde_json::json!({"title": "用户管理"}),
                status: 1,
                create_time: Utc::now(),
            },
            MenuRow {
                menu_id: 3,
                menu_name: "用户新增".to_string(),
                menu_type: 0,
                router_name: String::new(),
                parent_id: 2,
                path: String::new(),
                is_button: true,
                permission: "system:user:add".to_string(),
                meta_info: serde_json::json!({"title": "用户新增"}),
                status: 1,
                create_time: Utc::now(),
            },
        ];

        let routers = build_router_tree(&rows, 0);

        assert_eq!(routers.len(), 1);
        assert_eq!(routers[0].children.len(), 1);
        assert_eq!(routers[0].children[0].name, "SystemUser");
        assert_eq!(
            routers[0].children[0].meta.auths,
            Some(vec!["system:user:list".to_string()])
        );
        assert!(routers[0].children[0].children.is_empty());
    }

    #[test]
    fn normalize_menu_rejects_blank_or_long_name() {
        assert!(
            normalize_menu(MenuNormalizationInput {
                parent_id: None,
                menu_name: "",
                router_name: None,
                path: None,
                status: Some(1),
                menu_type: Some(1),
                is_button: Some(false),
                permission: None,
                meta: None,
                fallback_menu_type: None,
            })
            .is_err()
        );
        let long_name = "a".repeat(51);
        assert!(
            normalize_menu(MenuNormalizationInput {
                parent_id: None,
                menu_name: &long_name,
                router_name: None,
                path: None,
                status: Some(1),
                menu_type: Some(1),
                is_button: Some(false),
                permission: None,
                meta: None,
                fallback_menu_type: None,
            })
            .is_err()
        );
    }
}
