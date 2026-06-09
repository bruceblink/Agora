use chrono::{DateTime, Days, NaiveDate, NaiveTime, Utc};
use common::dto::{
    CreateDeptDTO, CreatePostDTO, CreateSystemUserDTO, DeptDTO, PostDTO, ResetUserPasswordDTO,
    RoleDTO, SystemUserDTO, UpdateDeptDTO, UpdateOwnPasswordDTO, UpdatePostDTO, UpdateProfileDTO,
    UpdateSystemUserDTO, UpdateUserStatusDTO, UserDetailDTO, UserProfileDTO,
};
use common::po::PageData;
use common::{DeptQuery, PostQuery, RoleQuery, SystemUserQuery};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Transaction};

const DEFAULT_PAGE: u32 = 1;
const DEFAULT_PAGE_SIZE: u32 = 10;
const MAX_PAGE_SIZE: u32 = 500;

#[derive(Debug, FromRow)]
struct DeptRow {
    id: i64,
    parent_id: i64,
    dept_name: String,
    order_num: i32,
    leader_name: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    status: i16,
    create_time: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct PostRow {
    post_id: i64,
    post_code: String,
    post_name: String,
    post_sort: i32,
    remark: Option<String>,
    status: i16,
    create_time: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct PostWithTotal {
    post_id: i64,
    post_code: String,
    post_name: String,
    post_sort: i32,
    remark: Option<String>,
    status: i16,
    create_time: DateTime<Utc>,
    total_count: i64,
}

#[derive(Debug, FromRow)]
struct SystemUserRow {
    user_id: i64,
    post_id: Option<i64>,
    post_name: Option<String>,
    role_id: Option<i64>,
    role_name: Option<String>,
    dept_id: Option<i64>,
    dept_name: Option<String>,
    username: String,
    nickname: Option<String>,
    user_type: Option<i16>,
    email: Option<String>,
    phone_number: Option<String>,
    sex: Option<i16>,
    avatar: Option<String>,
    status: i16,
    login_ip: Option<String>,
    login_date: Option<DateTime<Utc>>,
    creator_id: Option<i64>,
    creator_name: Option<String>,
    create_time: DateTime<Utc>,
    updater_id: Option<i64>,
    updater_name: Option<String>,
    update_time: Option<DateTime<Utc>>,
    remark: Option<String>,
}

#[derive(Debug, FromRow)]
struct SystemUserWithTotal {
    user_id: i64,
    post_id: Option<i64>,
    post_name: Option<String>,
    role_id: Option<i64>,
    role_name: Option<String>,
    dept_id: Option<i64>,
    dept_name: Option<String>,
    username: String,
    nickname: Option<String>,
    user_type: Option<i16>,
    email: Option<String>,
    phone_number: Option<String>,
    sex: Option<i16>,
    avatar: Option<String>,
    status: i16,
    login_ip: Option<String>,
    login_date: Option<DateTime<Utc>>,
    creator_id: Option<i64>,
    creator_name: Option<String>,
    create_time: DateTime<Utc>,
    updater_id: Option<i64>,
    updater_name: Option<String>,
    update_time: Option<DateTime<Utc>>,
    remark: Option<String>,
    total_count: i64,
}

#[derive(Debug)]
struct NormalizedDept {
    parent_id: i64,
    dept_name: String,
    order_num: i32,
    leader_name: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    status: i16,
}

#[derive(Debug)]
struct NormalizedPost {
    post_code: String,
    post_name: String,
    post_sort: i32,
    remark: Option<String>,
    status: i16,
}

#[derive(Debug)]
struct NormalizedUser {
    dept_id: Option<i64>,
    username: String,
    nickname: Option<String>,
    email: Option<String>,
    phone_number: Option<String>,
    sex: i16,
    avatar: Option<String>,
    status: i16,
    role_id: Option<i64>,
    post_id: Option<i64>,
    remark: Option<String>,
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

fn status_label(status: i16) -> String {
    match status {
        1 => "正常",
        0 => "停用",
        _ => "未知",
    }
    .to_string()
}

fn user_status_to_login_status(status: i16) -> &'static str {
    match status {
        1 => "active",
        0 | 2 | 3 => "inactive",
        _ => "inactive",
    }
}

fn sort_direction(value: Option<&str>) -> Option<&'static str> {
    match value {
        Some("ascending") | Some("asc") | Some("ASC") => Some("ASC"),
        Some("descending") | Some("desc") | Some("DESC") => Some("DESC"),
        _ => None,
    }
}

fn post_sort_column(value: Option<&str>) -> &'static str {
    match value {
        Some("postId") => "post_id",
        Some("postCode") => "post_code",
        Some("postName") => "post_name",
        Some("postSort") => "post_sort",
        Some("status") => "status",
        Some("createTime") => "created_at",
        _ => "post_sort",
    }
}

fn user_sort_column(value: Option<&str>) -> &'static str {
    match value {
        Some("userId") => "u.id",
        Some("username") => "u.username",
        Some("status") => "u.user_status",
        Some("createTime") => "u.created_at",
        _ => "u.created_at",
    }
}

fn trim_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn parse_status_value(value: Option<&serde_json::Value>, default: i16) -> anyhow::Result<i16> {
    let Some(value) = value else {
        return Ok(default);
    };

    let status = if let Some(number) = value.as_i64() {
        i16::try_from(number).map_err(|_| anyhow::anyhow!("状态值超出范围"))?
    } else if let Some(value) = value.as_str() {
        if value.trim().is_empty() {
            default
        } else {
            value
                .trim()
                .parse::<i16>()
                .map_err(|_| anyhow::anyhow!("状态必须是数字"))?
        }
    } else {
        return Err(anyhow::anyhow!("状态必须是数字"));
    };

    validate_common_status(status)?;
    Ok(status)
}

fn validate_common_status(status: i16) -> anyhow::Result<()> {
    if matches!(status, 0 | 1) {
        Ok(())
    } else {
        Err(anyhow::anyhow!("状态必须是 0 或 1"))
    }
}

fn validate_user_status(status: i16) -> anyhow::Result<()> {
    if matches!(status, 0 | 1 | 2 | 3) {
        Ok(())
    } else {
        Err(anyhow::anyhow!("用户状态必须是 0、1、2 或 3"))
    }
}

fn validate_sex(sex: i16) -> anyhow::Result<()> {
    if matches!(sex, 0 | 1 | 2) {
        Ok(())
    } else {
        Err(anyhow::anyhow!("用户性别必须是 0、1 或 2"))
    }
}

fn normalize_dept(
    parent_id: i64,
    dept_name: &str,
    order_num: i32,
    leader_name: Option<&str>,
    phone: Option<&str>,
    email: Option<&str>,
    status: Option<i16>,
) -> anyhow::Result<NormalizedDept> {
    if parent_id < 0 {
        return Err(anyhow::anyhow!("上级部门ID不能小于 0"));
    }

    let dept_name = dept_name.trim().to_string();
    if dept_name.is_empty() {
        return Err(anyhow::anyhow!("部门名称不能为空"));
    }
    if dept_name.chars().count() > 30 {
        return Err(anyhow::anyhow!("部门名称长度不能超过30个字符"));
    }
    if order_num < 0 {
        return Err(anyhow::anyhow!("显示顺序不能小于 0"));
    }

    let status = status.unwrap_or(1);
    validate_common_status(status)?;

    Ok(NormalizedDept {
        parent_id,
        dept_name,
        order_num,
        leader_name: trim_optional(leader_name),
        phone: trim_optional(phone),
        email: trim_optional(email),
        status,
    })
}

fn normalize_post(
    post_code: &str,
    post_name: &str,
    post_sort: i32,
    remark: Option<&str>,
    status: Option<&serde_json::Value>,
) -> anyhow::Result<NormalizedPost> {
    let post_code = post_code.trim().to_string();
    if post_code.is_empty() {
        return Err(anyhow::anyhow!("岗位编码不能为空"));
    }
    if post_code.chars().count() > 64 {
        return Err(anyhow::anyhow!("岗位编码长度不能超过64个字符"));
    }

    let post_name = post_name.trim().to_string();
    if post_name.is_empty() {
        return Err(anyhow::anyhow!("岗位名称不能为空"));
    }
    if post_name.chars().count() > 64 {
        return Err(anyhow::anyhow!("岗位名称长度不能超过64个字符"));
    }
    if post_sort < 0 {
        return Err(anyhow::anyhow!("显示顺序不能小于 0"));
    }

    Ok(NormalizedPost {
        post_code,
        post_name,
        post_sort,
        remark: trim_optional(remark),
        status: parse_status_value(status, 1)?,
    })
}

fn normalize_user(
    username: Option<&str>,
    nickname: Option<&str>,
    email: Option<&str>,
    phone_number: Option<&str>,
    sex: Option<i16>,
    avatar: Option<&str>,
    status: Option<i16>,
    role_id: Option<i64>,
    dept_id: Option<i64>,
    post_id: Option<i64>,
    remark: Option<&str>,
) -> anyhow::Result<NormalizedUser> {
    let username = username
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("用户名不能为空"))?
        .to_string();
    if username.chars().count() > 100 {
        return Err(anyhow::anyhow!("用户名长度不能超过100个字符"));
    }

    let status = status.unwrap_or(1);
    validate_user_status(status)?;
    let sex = sex.unwrap_or(2);
    validate_sex(sex)?;

    for (field, field_name) in [
        (role_id, "roleId"),
        (dept_id, "deptId"),
        (post_id, "postId"),
    ] {
        if field.is_some_and(|id| id <= 0) {
            return Err(anyhow::anyhow!("{field_name} 必须为正整数"));
        }
    }

    Ok(NormalizedUser {
        dept_id,
        username,
        nickname: trim_optional(nickname),
        email: trim_optional(email),
        phone_number: trim_optional(phone_number),
        sex,
        avatar: trim_optional(avatar),
        status,
        role_id,
        post_id,
        remark: trim_optional(remark),
    })
}

fn to_dept_dto(row: DeptRow) -> DeptDTO {
    DeptDTO {
        id: row.id,
        parent_id: row.parent_id,
        dept_name: row.dept_name,
        order_num: row.order_num,
        leader_name: row.leader_name,
        phone: row.phone,
        email: row.email,
        status: row.status,
        status_str: status_label(row.status),
        create_time: row.create_time,
    }
}

fn to_post_dto(row: PostRow) -> PostDTO {
    PostDTO {
        post_id: row.post_id,
        post_code: row.post_code,
        post_name: row.post_name,
        post_sort: row.post_sort,
        remark: row.remark,
        status: row.status,
        status_str: status_label(row.status),
        create_time: row.create_time,
    }
}

fn to_post_dto_with_total(row: PostWithTotal) -> PostDTO {
    to_post_dto(PostRow {
        post_id: row.post_id,
        post_code: row.post_code,
        post_name: row.post_name,
        post_sort: row.post_sort,
        remark: row.remark,
        status: row.status,
        create_time: row.create_time,
    })
}

fn to_system_user_dto(row: SystemUserRow) -> SystemUserDTO {
    SystemUserDTO {
        user_id: row.user_id,
        post_id: row.post_id,
        post_name: row.post_name,
        role_id: row.role_id,
        role_name: row.role_name,
        dept_id: row.dept_id,
        dept_name: row.dept_name,
        username: row.username,
        nickname: row.nickname,
        user_type: row.user_type,
        email: row.email,
        phone_number: row.phone_number,
        sex: row.sex,
        avatar: row.avatar,
        status: row.status,
        login_ip: row.login_ip,
        login_date: row.login_date,
        creator_id: row.creator_id,
        creator_name: row.creator_name,
        create_time: row.create_time,
        updater_id: row.updater_id,
        updater_name: row.updater_name,
        update_time: row.update_time,
        remark: row.remark,
    }
}

fn to_system_user_dto_with_total(row: SystemUserWithTotal) -> SystemUserDTO {
    to_system_user_dto(SystemUserRow {
        user_id: row.user_id,
        post_id: row.post_id,
        post_name: row.post_name,
        role_id: row.role_id,
        role_name: row.role_name,
        dept_id: row.dept_id,
        dept_name: row.dept_name,
        username: row.username,
        nickname: row.nickname,
        user_type: row.user_type,
        email: row.email,
        phone_number: row.phone_number,
        sex: row.sex,
        avatar: row.avatar,
        status: row.status,
        login_ip: row.login_ip,
        login_date: row.login_date,
        creator_id: row.creator_id,
        creator_name: row.creator_name,
        create_time: row.create_time,
        updater_id: row.updater_id,
        updater_name: row.updater_name,
        update_time: row.update_time,
        remark: row.remark,
    })
}

async fn check_dept_name_unique(
    dept_id: Option<i64>,
    dept_name: &str,
    parent_id: i64,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    let duplicated: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM sys_dept
            WHERE dept_name = $1
              AND parent_id = $2
              AND ($3::BIGINT IS NULL OR dept_id <> $3)
              AND deleted = FALSE
        )
        "#,
    )
    .bind(dept_name)
    .bind(parent_id)
    .bind(dept_id)
    .fetch_one(db_pool)
    .await?;

    if duplicated {
        Err(anyhow::anyhow!("同级部门名称已存在"))
    } else {
        Ok(())
    }
}

async fn dept_exists(dept_id: i64, db_pool: &PgPool) -> anyhow::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM sys_dept WHERE dept_id = $1 AND deleted = FALSE)",
    )
    .bind(dept_id)
    .fetch_one(db_pool)
    .await
    .map_err(Into::into)
}

async fn post_exists(post_id: i64, db_pool: &PgPool) -> anyhow::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM sys_post WHERE post_id = $1 AND deleted = FALSE)",
    )
    .bind(post_id)
    .fetch_one(db_pool)
    .await
    .map_err(Into::into)
}

async fn role_exists(role_id: i64, db_pool: &PgPool) -> anyhow::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM roles WHERE id = $1 AND deleted = FALSE)")
        .bind(role_id)
        .fetch_one(db_pool)
        .await
        .map_err(Into::into)
}

async fn ensure_user_relations(user: &NormalizedUser, db_pool: &PgPool) -> anyhow::Result<()> {
    if let Some(dept_id) = user.dept_id
        && !dept_exists(dept_id, db_pool).await?
    {
        return Err(anyhow::anyhow!("部门不存在"));
    }
    if let Some(post_id) = user.post_id
        && !post_exists(post_id, db_pool).await?
    {
        return Err(anyhow::anyhow!("岗位不存在"));
    }
    if let Some(role_id) = user.role_id
        && !role_exists(role_id, db_pool).await?
    {
        return Err(anyhow::anyhow!("角色不存在"));
    }
    Ok(())
}

async fn check_post_unique(
    post_id: Option<i64>,
    post_code: &str,
    post_name: &str,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    let duplicated_code: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM sys_post
            WHERE post_code = $1
              AND ($2::BIGINT IS NULL OR post_id <> $2)
              AND deleted = FALSE
        )
        "#,
    )
    .bind(post_code)
    .bind(post_id)
    .fetch_one(db_pool)
    .await?;
    if duplicated_code {
        return Err(anyhow::anyhow!("岗位编码已存在"));
    }

    let duplicated_name: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM sys_post
            WHERE post_name = $1
              AND ($2::BIGINT IS NULL OR post_id <> $2)
              AND deleted = FALSE
        )
        "#,
    )
    .bind(post_name)
    .bind(post_id)
    .fetch_one(db_pool)
    .await?;
    if duplicated_name {
        return Err(anyhow::anyhow!("岗位名称已存在"));
    }

    Ok(())
}

async fn check_user_unique(
    user_id: Option<i64>,
    username: &str,
    email: Option<&str>,
    phone_number: Option<&str>,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    let duplicated_username: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM user_info
            WHERE username = $1
              AND ($2::BIGINT IS NULL OR id <> $2)
              AND deleted = FALSE
        )
        "#,
    )
    .bind(username)
    .bind(user_id)
    .fetch_one(db_pool)
    .await?;
    if duplicated_username {
        return Err(anyhow::anyhow!("用户名已存在"));
    }

    if let Some(email) = email.filter(|value| !value.is_empty()) {
        let duplicated_email: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM user_info
                WHERE email = $1
                  AND ($2::BIGINT IS NULL OR id <> $2)
                  AND deleted = FALSE
            )
            "#,
        )
        .bind(email)
        .bind(user_id)
        .fetch_one(db_pool)
        .await?;
        if duplicated_email {
            return Err(anyhow::anyhow!("用户邮箱已存在"));
        }
    }

    if let Some(phone_number) = phone_number.filter(|value| !value.is_empty()) {
        let duplicated_phone: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM user_info
                WHERE phone_number = $1
                  AND ($2::BIGINT IS NULL OR id <> $2)
                  AND deleted = FALSE
            )
            "#,
        )
        .bind(phone_number)
        .bind(user_id)
        .fetch_one(db_pool)
        .await?;
        if duplicated_phone {
            return Err(anyhow::anyhow!("手机号码已存在"));
        }
    }

    Ok(())
}

async fn ancestors_for_parent(parent_id: i64, db_pool: &PgPool) -> anyhow::Result<String> {
    if parent_id == 0 {
        return Ok("0".to_string());
    }

    let parent: Option<(String,)> =
        sqlx::query_as("SELECT ancestors FROM sys_dept WHERE dept_id = $1 AND deleted = FALSE")
            .bind(parent_id)
            .fetch_optional(db_pool)
            .await?;

    let (parent_ancestors,) = parent.ok_or_else(|| anyhow::anyhow!("上级部门不存在"))?;
    Ok(format!("{parent_ancestors},{parent_id}"))
}

async fn selected_user_role_id(user_id: i64, db_pool: &PgPool) -> anyhow::Result<Option<i64>> {
    let role_id = sqlx::query_scalar(
        r#"
        SELECT ur.role_id
        FROM user_roles ur
        JOIN roles r ON r.id = ur.role_id
        WHERE ur.user_id = $1
          AND r.deleted = FALSE
        ORDER BY r.role_sort ASC NULLS LAST, r.id ASC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(db_pool)
    .await?;

    Ok(role_id)
}

async fn sync_single_user_role(
    user_id: i64,
    role_id: Option<i64>,
    tx: &mut Transaction<'_, Postgres>,
) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM user_roles WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut **tx)
        .await?;

    if let Some(role_id) = role_id {
        sqlx::query(
            r#"
            INSERT INTO user_roles (user_id, role_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(user_id)
        .bind(role_id)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

pub async fn list_depts(query: &DeptQuery, db_pool: &PgPool) -> anyhow::Result<Vec<DeptDTO>> {
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
            SELECT dept_id AS id, parent_id, dept_name, order_num, leader_name,
                   phone, email, status, created_at AS create_time
            FROM sys_dept
            WHERE deleted = FALSE
        "#,
    );

    if let Some(dept_id) = query.dept_id {
        query_builder.push(" AND dept_id = ");
        query_builder.push_bind(dept_id);
    }
    if let Some(parent_id) = query.parent_id {
        query_builder.push(" AND parent_id = ");
        query_builder.push_bind(parent_id);
    }
    if let Some(status) = query.status {
        validate_common_status(status)?;
        query_builder.push(" AND status = ");
        query_builder.push_bind(status);
    }
    if let Some(dept_name) = query.dept_name.as_deref().filter(|value| !value.is_empty()) {
        query_builder.push(" AND dept_name ILIKE ");
        query_builder.push_bind(format!("%{dept_name}%"));
    }

    query_builder.push(" ORDER BY parent_id ASC, order_num ASC, dept_id ASC");

    let rows: Vec<DeptRow> = query_builder.build_query_as().fetch_all(db_pool).await?;
    Ok(rows.into_iter().map(to_dept_dto).collect())
}

pub async fn get_dept(dept_id: i64, db_pool: &PgPool) -> anyhow::Result<DeptDTO> {
    let row = sqlx::query_as::<_, DeptRow>(
        r#"
        SELECT dept_id AS id, parent_id, dept_name, order_num, leader_name,
               phone, email, status, created_at AS create_time
        FROM sys_dept
        WHERE dept_id = $1
          AND deleted = FALSE
        "#,
    )
    .bind(dept_id)
    .fetch_optional(db_pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("部门不存在"))?;

    Ok(to_dept_dto(row))
}

pub async fn create_dept(data: &CreateDeptDTO, db_pool: &PgPool) -> anyhow::Result<()> {
    let dept = normalize_dept(
        data.parent_id,
        &data.dept_name,
        data.order_num,
        data.leader_name.as_deref(),
        data.phone.as_deref(),
        data.email.as_deref(),
        data.status,
    )?;
    check_dept_name_unique(None, &dept.dept_name, dept.parent_id, db_pool).await?;
    let ancestors = ancestors_for_parent(dept.parent_id, db_pool).await?;

    sqlx::query(
        r#"
        INSERT INTO sys_dept (
            parent_id, ancestors, dept_name, order_num, leader_name,
            phone, email, status, deleted
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, FALSE)
        "#,
    )
    .bind(dept.parent_id)
    .bind(ancestors)
    .bind(&dept.dept_name)
    .bind(dept.order_num)
    .bind(&dept.leader_name)
    .bind(&dept.phone)
    .bind(&dept.email)
    .bind(dept.status)
    .execute(db_pool)
    .await?;

    Ok(())
}

pub async fn update_dept(
    dept_id: i64,
    data: &UpdateDeptDTO,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    if data.dept_id.or(data.id).is_some_and(|id| id != dept_id) {
        return Err(anyhow::anyhow!("路径 deptId 与请求体不一致"));
    }

    let dept = normalize_dept(
        data.parent_id,
        &data.dept_name,
        data.order_num,
        data.leader_name.as_deref(),
        data.phone.as_deref(),
        data.email.as_deref(),
        data.status,
    )?;
    if dept_id == dept.parent_id {
        return Err(anyhow::anyhow!("上级部门不能选择自身"));
    }

    let exists = dept_exists(dept_id, db_pool).await?;
    if !exists {
        return Err(anyhow::anyhow!("部门不存在"));
    }
    check_dept_name_unique(Some(dept_id), &dept.dept_name, dept.parent_id, db_pool).await?;
    let ancestors = ancestors_for_parent(dept.parent_id, db_pool).await?;

    sqlx::query(
        r#"
        UPDATE sys_dept
        SET parent_id = $2,
            ancestors = $3,
            dept_name = $4,
            order_num = $5,
            leader_name = $6,
            phone = $7,
            email = $8,
            status = $9
        WHERE dept_id = $1
          AND deleted = FALSE
        "#,
    )
    .bind(dept_id)
    .bind(dept.parent_id)
    .bind(ancestors)
    .bind(&dept.dept_name)
    .bind(dept.order_num)
    .bind(&dept.leader_name)
    .bind(&dept.phone)
    .bind(&dept.email)
    .bind(dept.status)
    .execute(db_pool)
    .await?;

    Ok(())
}

pub async fn delete_dept(dept_id: i64, db_pool: &PgPool) -> anyhow::Result<()> {
    if dept_id <= 0 {
        return Err(anyhow::anyhow!("deptId 必须为正整数"));
    }
    if !dept_exists(dept_id, db_pool).await? {
        return Err(anyhow::anyhow!("部门不存在"));
    }

    let has_child: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM sys_dept WHERE parent_id = $1 AND deleted = FALSE)",
    )
    .bind(dept_id)
    .fetch_one(db_pool)
    .await?;
    if has_child {
        return Err(anyhow::anyhow!("存在子部门不允许删除"));
    }

    let has_user: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM user_info WHERE dept_id = $1 AND deleted = FALSE)",
    )
    .bind(dept_id)
    .fetch_one(db_pool)
    .await?;
    if has_user {
        return Err(anyhow::anyhow!("部门已分配给用户，不允许删除"));
    }

    sqlx::query("UPDATE sys_dept SET deleted = TRUE WHERE dept_id = $1")
        .bind(dept_id)
        .execute(db_pool)
        .await?;

    Ok(())
}

pub async fn list_posts(query: &PostQuery, db_pool: &PgPool) -> anyhow::Result<PageData<PostDTO>> {
    let (page, page_size, offset) = page_bounds(query.page, query.page_num, query.page_size);
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
            SELECT post_id, post_code, post_name, post_sort, remark, status,
                   created_at AS create_time, COUNT(*) OVER() AS total_count
            FROM sys_post
            WHERE deleted = FALSE
        "#,
    );

    if let Some(post_code) = query.post_code.as_deref().filter(|value| !value.is_empty()) {
        query_builder.push(" AND post_code = ");
        query_builder.push_bind(post_code);
    }
    if let Some(post_name) = query.post_name.as_deref().filter(|value| !value.is_empty()) {
        query_builder.push(" AND post_name ILIKE ");
        query_builder.push_bind(format!("%{post_name}%"));
    }
    if let Some(status) = query.status {
        validate_common_status(status)?;
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
    query_builder.push(post_sort_column(query.order_column.as_deref()));
    query_builder.push(" ");
    query_builder.push(sort_direction(query.order_direction.as_deref()).unwrap_or("ASC"));
    query_builder.push(" NULLS LAST, post_id ASC LIMIT ");
    query_builder.push_bind(page_size as i64);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let rows: Vec<PostWithTotal> = query_builder.build_query_as().fetch_all(db_pool).await?;
    let total_count = rows.first().map(|row| row.total_count).unwrap_or(0);
    let items = rows.into_iter().map(to_post_dto_with_total).collect();

    Ok(PageData {
        items,
        total_count: total_count as usize,
        page,
        page_size,
        total_pages: total_pages(total_count, page_size),
    })
}

pub async fn list_all_posts(db_pool: &PgPool) -> anyhow::Result<Vec<PostDTO>> {
    let rows: Vec<PostRow> = sqlx::query_as(
        r#"
        SELECT post_id, post_code, post_name, post_sort, remark, status,
               created_at AS create_time
        FROM sys_post
        WHERE deleted = FALSE
        ORDER BY post_sort ASC, post_id ASC
        "#,
    )
    .fetch_all(db_pool)
    .await?;

    Ok(rows.into_iter().map(to_post_dto).collect())
}

pub async fn get_post(post_id: i64, db_pool: &PgPool) -> anyhow::Result<PostDTO> {
    let row = sqlx::query_as::<_, PostRow>(
        r#"
        SELECT post_id, post_code, post_name, post_sort, remark, status,
               created_at AS create_time
        FROM sys_post
        WHERE post_id = $1
          AND deleted = FALSE
        "#,
    )
    .bind(post_id)
    .fetch_optional(db_pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("岗位不存在"))?;

    Ok(to_post_dto(row))
}

pub async fn create_post(data: &CreatePostDTO, db_pool: &PgPool) -> anyhow::Result<()> {
    let post = normalize_post(
        &data.post_code,
        &data.post_name,
        data.post_sort,
        data.remark.as_deref(),
        data.status.as_ref(),
    )?;
    check_post_unique(None, &post.post_code, &post.post_name, db_pool).await?;

    sqlx::query(
        r#"
        INSERT INTO sys_post (
            post_code, post_name, post_sort, status, remark, deleted
        )
        VALUES ($1, $2, $3, $4, $5, FALSE)
        "#,
    )
    .bind(&post.post_code)
    .bind(&post.post_name)
    .bind(post.post_sort)
    .bind(post.status)
    .bind(&post.remark)
    .execute(db_pool)
    .await?;

    Ok(())
}

pub async fn update_post(data: &UpdatePostDTO, db_pool: &PgPool) -> anyhow::Result<()> {
    if data.post_id <= 0 {
        return Err(anyhow::anyhow!("postId 必须为正整数"));
    }
    if !post_exists(data.post_id, db_pool).await? {
        return Err(anyhow::anyhow!("岗位不存在"));
    }

    let post = normalize_post(
        &data.post_code,
        &data.post_name,
        data.post_sort,
        data.remark.as_deref(),
        data.status.as_ref(),
    )?;
    check_post_unique(
        Some(data.post_id),
        &post.post_code,
        &post.post_name,
        db_pool,
    )
    .await?;

    sqlx::query(
        r#"
        UPDATE sys_post
        SET post_code = $2,
            post_name = $3,
            post_sort = $4,
            status = $5,
            remark = $6
        WHERE post_id = $1
          AND deleted = FALSE
        "#,
    )
    .bind(data.post_id)
    .bind(&post.post_code)
    .bind(&post.post_name)
    .bind(post.post_sort)
    .bind(post.status)
    .bind(&post.remark)
    .execute(db_pool)
    .await?;

    Ok(())
}

pub async fn delete_posts(post_ids: &[i64], db_pool: &PgPool) -> anyhow::Result<u64> {
    if post_ids.is_empty() {
        return Err(anyhow::anyhow!("ids 不能为空"));
    }
    if post_ids.iter().any(|id| *id <= 0) {
        return Err(anyhow::anyhow!("ids 必须为正整数"));
    }

    let assigned_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_info WHERE post_id = ANY($1) AND deleted = FALSE",
    )
    .bind(post_ids)
    .fetch_one(db_pool)
    .await?;
    if assigned_count > 0 {
        return Err(anyhow::anyhow!("岗位已分配给用户，不允许删除"));
    }

    let result = sqlx::query(
        r#"
        UPDATE sys_post
        SET deleted = TRUE
        WHERE post_id = ANY($1)
          AND deleted = FALSE
        "#,
    )
    .bind(post_ids)
    .execute(db_pool)
    .await?;

    Ok(result.rows_affected())
}

fn user_select_sql(with_total: bool) -> &'static str {
    if with_total {
        r#"
            SELECT u.id AS user_id, u.post_id, p.post_name,
                   ur.role_id, r.role_name, u.dept_id, d.dept_name,
                   u.username, u.nickname, u.user_type, u.email,
                   u.phone_number, u.sex, COALESCE(u.avatar, u.avatar_url) AS avatar,
                   COALESCE(u.user_status, CASE WHEN u.status = 'active' THEN 1 ELSE 0 END) AS status,
                   u.login_ip, u.login_date, u.creator_id, creator.username AS creator_name,
                   u.created_at AS create_time, u.updater_id, updater.username AS updater_name,
                   u.updated_at AS update_time, u.remark, COUNT(*) OVER() AS total_count
            FROM user_info u
            LEFT JOIN sys_dept d ON d.dept_id = u.dept_id AND d.deleted = FALSE
            LEFT JOIN sys_post p ON p.post_id = u.post_id AND p.deleted = FALSE
            LEFT JOIN LATERAL (
                SELECT ur.role_id
                FROM user_roles ur
                JOIN roles role_order ON role_order.id = ur.role_id
                WHERE ur.user_id = u.id
                  AND role_order.deleted = FALSE
                ORDER BY role_order.role_sort ASC NULLS LAST, role_order.id ASC
                LIMIT 1
            ) ur ON TRUE
            LEFT JOIN roles r ON r.id = ur.role_id AND r.deleted = FALSE
            LEFT JOIN user_info creator ON creator.id = u.creator_id
            LEFT JOIN user_info updater ON updater.id = u.updater_id
            WHERE u.deleted = FALSE
        "#
    } else {
        r#"
            SELECT u.id AS user_id, u.post_id, p.post_name,
                   ur.role_id, r.role_name, u.dept_id, d.dept_name,
                   u.username, u.nickname, u.user_type, u.email,
                   u.phone_number, u.sex, COALESCE(u.avatar, u.avatar_url) AS avatar,
                   COALESCE(u.user_status, CASE WHEN u.status = 'active' THEN 1 ELSE 0 END) AS status,
                   u.login_ip, u.login_date, u.creator_id, creator.username AS creator_name,
                   u.created_at AS create_time, u.updater_id, updater.username AS updater_name,
                   u.updated_at AS update_time, u.remark
            FROM user_info u
            LEFT JOIN sys_dept d ON d.dept_id = u.dept_id AND d.deleted = FALSE
            LEFT JOIN sys_post p ON p.post_id = u.post_id AND p.deleted = FALSE
            LEFT JOIN LATERAL (
                SELECT ur.role_id
                FROM user_roles ur
                JOIN roles role_order ON role_order.id = ur.role_id
                WHERE ur.user_id = u.id
                  AND role_order.deleted = FALSE
                ORDER BY role_order.role_sort ASC NULLS LAST, role_order.id ASC
                LIMIT 1
            ) ur ON TRUE
            LEFT JOIN roles r ON r.id = ur.role_id AND r.deleted = FALSE
            LEFT JOIN user_info creator ON creator.id = u.creator_id
            LEFT JOIN user_info updater ON updater.id = u.updater_id
            WHERE u.deleted = FALSE
        "#
    }
}

pub async fn list_system_users(
    query: &SystemUserQuery,
    db_pool: &PgPool,
) -> anyhow::Result<PageData<SystemUserDTO>> {
    let (page, page_size, offset) = page_bounds(query.page, query.page_num, query.page_size);
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(user_select_sql(true));

    if let Some(user_id) = query.user_id {
        query_builder.push(" AND u.id = ");
        query_builder.push_bind(user_id);
    }
    if let Some(username) = query.username.as_deref().filter(|value| !value.is_empty()) {
        query_builder.push(" AND u.username ILIKE ");
        query_builder.push_bind(format!("%{username}%"));
    }
    if let Some(phone_number) = query
        .phone_number
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        query_builder.push(" AND u.phone_number ILIKE ");
        query_builder.push_bind(format!("%{phone_number}%"));
    }
    if let Some(status) = query.status {
        validate_user_status(status)?;
        query_builder.push(
            " AND COALESCE(u.user_status, CASE WHEN u.status = 'active' THEN 1 ELSE 0 END) = ",
        );
        query_builder.push_bind(status);
    }
    if let Some(dept_id) = query.dept_id {
        query_builder.push(
            r#"
            AND (
                u.dept_id = 
            "#,
        );
        query_builder.push_bind(dept_id);
        query_builder.push(
            r#"
                OR u.dept_id IN (
                    SELECT child.dept_id
                    FROM sys_dept child
                    WHERE child.deleted = FALSE
                      AND (
                          child.ancestors = 
            "#,
        );
        query_builder.push_bind(dept_id.to_string());
        query_builder.push(" OR child.ancestors LIKE ");
        query_builder.push_bind(format!("{dept_id},%"));
        query_builder.push(" OR child.ancestors LIKE ");
        query_builder.push_bind(format!("%,{dept_id},%"));
        query_builder.push(" OR child.ancestors LIKE ");
        query_builder.push_bind(format!("%,{dept_id}"));
        query_builder.push(")))");
    }
    if let Some(begin_time) = query.begin_time {
        query_builder.push(" AND u.created_at >= ");
        query_builder.push_bind(day_start(begin_time));
    }
    if let Some(end_time) = query.end_time {
        query_builder.push(" AND u.created_at < ");
        query_builder.push_bind(next_day_start(end_time));
    }

    query_builder.push(" ORDER BY ");
    query_builder.push(user_sort_column(query.order_column.as_deref()));
    query_builder.push(" ");
    query_builder.push(sort_direction(query.order_direction.as_deref()).unwrap_or("DESC"));
    query_builder.push(" NULLS LAST, u.id DESC LIMIT ");
    query_builder.push_bind(page_size as i64);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let rows: Vec<SystemUserWithTotal> = query_builder.build_query_as().fetch_all(db_pool).await?;
    let total_count = rows.first().map(|row| row.total_count).unwrap_or(0);
    let items = rows
        .into_iter()
        .map(to_system_user_dto_with_total)
        .collect();

    Ok(PageData {
        items,
        total_count: total_count as usize,
        page,
        page_size,
        total_pages: total_pages(total_count, page_size),
    })
}

pub async fn get_system_user(user_id: i64, db_pool: &PgPool) -> anyhow::Result<SystemUserDTO> {
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(user_select_sql(false));
    query_builder.push(" AND u.id = ");
    query_builder.push_bind(user_id);

    let row: SystemUserRow = query_builder
        .build_query_as()
        .fetch_optional(db_pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("用户不存在"))?;

    Ok(to_system_user_dto(row))
}

pub async fn get_user_profile(user_id: i64, db_pool: &PgPool) -> anyhow::Result<UserProfileDTO> {
    let user = get_system_user(user_id, db_pool).await?;
    let role_name = user.role_name.clone();
    let post_name = user.post_name.clone();

    Ok(UserProfileDTO {
        user,
        role_name,
        post_name,
    })
}

pub async fn get_user_detail(
    user_id: Option<i64>,
    db_pool: &PgPool,
) -> anyhow::Result<UserDetailDTO> {
    let user = if let Some(user_id) = user_id {
        Some(get_system_user(user_id, db_pool).await?)
    } else {
        None
    };

    let role_options_page = super::sys_role::list_roles(
        &RoleQuery {
            role_name: None,
            role_key: None,
            status: None,
            page: Some(1),
            page_size: Some(MAX_PAGE_SIZE),
        },
        db_pool,
    )
    .await?;
    let role_options: Vec<RoleDTO> = role_options_page.items;
    let post_options = list_all_posts(db_pool).await?;
    let permissions = if let Some(user) = &user {
        list_user_permissions(user.user_id, db_pool).await?
    } else {
        Vec::new()
    };
    let post_id = user.as_ref().and_then(|user| user.post_id);
    let role_id = user.as_ref().and_then(|user| user.role_id);

    Ok(UserDetailDTO {
        user,
        role_options,
        post_options,
        post_id,
        role_id,
        permissions,
    })
}

async fn list_user_permissions(user_id: i64, db_pool: &PgPool) -> anyhow::Result<Vec<String>> {
    let permissions: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT DISTINCT m.permission
        FROM sys_menu m
        JOIN sys_role_menu rm ON rm.menu_id = m.menu_id
        JOIN user_roles ur ON ur.role_id = rm.role_id
        JOIN roles r ON r.id = ur.role_id
        WHERE ur.user_id = $1
          AND m.permission <> ''
          AND m.status = 1
          AND m.deleted = FALSE
          AND r.status = 1
          AND r.deleted = FALSE
        ORDER BY m.permission ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(db_pool)
    .await?;

    Ok(permissions)
}

pub async fn update_user_profile(
    user_id: i64,
    data: &UpdateProfileDTO,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    if user_id <= 0 {
        return Err(anyhow::anyhow!("userId 必须为正整数"));
    }

    let existing = get_system_user(user_id, db_pool).await?;
    let sex = data.sex.or(existing.sex).unwrap_or(2);
    validate_sex(sex)?;

    let nickname = match data.nickname.as_deref() {
        Some(value) => trim_optional(Some(value)),
        None => existing.nickname.clone(),
    };
    let phone_number = match data.phone_number.as_deref() {
        Some(value) => trim_optional(Some(value)),
        None => existing.phone_number.clone(),
    };
    let email = match data.email.as_deref() {
        Some(value) => trim_optional(Some(value)),
        None => existing.email.clone(),
    };

    check_user_unique(
        Some(user_id),
        &existing.username,
        email.as_deref(),
        phone_number.as_deref(),
        db_pool,
    )
    .await?;

    let rows_affected = sqlx::query(
        r#"
        UPDATE user_info
        SET nickname = $2,
            display_name = $3,
            phone_number = $4,
            email = $5,
            sex = $6
        WHERE id = $1
          AND deleted = FALSE
        "#,
    )
    .bind(user_id)
    .bind(&nickname)
    .bind(nickname.as_deref().unwrap_or(&existing.username))
    .bind(&phone_number)
    .bind(&email)
    .bind(sex)
    .execute(db_pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(anyhow::anyhow!("用户不存在"));
    }

    Ok(())
}

pub async fn create_system_user(
    data: &CreateSystemUserDTO,
    password_hash: &str,
    creator_id: Option<i64>,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    let user = normalize_user(
        Some(&data.username),
        data.nickname.as_deref(),
        data.email.as_deref(),
        data.phone_number.as_deref(),
        data.sex,
        data.avatar.as_deref(),
        data.status,
        data.role_id,
        data.dept_id,
        data.post_id,
        data.remark.as_deref(),
    )?;
    ensure_user_relations(&user, db_pool).await?;
    check_user_unique(
        None,
        &user.username,
        user.email.as_deref(),
        user.phone_number.as_deref(),
        db_pool,
    )
    .await?;

    let mut tx = db_pool.begin().await?;
    let user_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO user_info (
            username, password, email, display_name, avatar_url, tenant_id,
            status, nickname, phone_number, sex, avatar, user_status, dept_id,
            post_id, remark, creator_id, deleted
        )
        VALUES (
            $1, $2, $3, $4, $5, 'default',
            $6, $7, $8, $9, $10, $11, $12,
            $13, $14, $15, FALSE
        )
        RETURNING id
        "#,
    )
    .bind(&user.username)
    .bind(password_hash)
    .bind(&user.email)
    .bind(user.nickname.as_deref().unwrap_or(&user.username))
    .bind(&user.avatar)
    .bind(user_status_to_login_status(user.status))
    .bind(&user.nickname)
    .bind(&user.phone_number)
    .bind(user.sex)
    .bind(&user.avatar)
    .bind(user.status)
    .bind(user.dept_id)
    .bind(user.post_id)
    .bind(&user.remark)
    .bind(creator_id)
    .fetch_one(&mut *tx)
    .await?;

    sync_single_user_role(user_id, user.role_id, &mut tx).await?;
    tx.commit().await?;

    Ok(())
}

pub async fn update_system_user(
    path_user_id: i64,
    data: &UpdateSystemUserDTO,
    updater_id: Option<i64>,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    if data.user_id.is_some_and(|user_id| user_id != path_user_id) {
        return Err(anyhow::anyhow!("路径 userId 与请求体不一致"));
    }

    let existing = get_system_user(path_user_id, db_pool).await?;
    let user = normalize_user(
        data.username.as_deref().or(Some(&existing.username)),
        data.nickname.as_deref().or(existing.nickname.as_deref()),
        data.email.as_deref().or(existing.email.as_deref()),
        data.phone_number
            .as_deref()
            .or(existing.phone_number.as_deref()),
        data.sex.or(existing.sex),
        data.avatar.as_deref().or(existing.avatar.as_deref()),
        data.status.or(Some(existing.status)),
        data.role_id.or(existing.role_id),
        data.dept_id.or(existing.dept_id),
        data.post_id.or(existing.post_id),
        data.remark.as_deref().or(existing.remark.as_deref()),
    )?;
    ensure_user_relations(&user, db_pool).await?;
    check_user_unique(
        Some(path_user_id),
        &user.username,
        user.email.as_deref(),
        user.phone_number.as_deref(),
        db_pool,
    )
    .await?;

    let mut tx = db_pool.begin().await?;
    sqlx::query(
        r#"
        UPDATE user_info
        SET username = $2,
            email = $3,
            display_name = $4,
            avatar_url = $5,
            status = $6,
            nickname = $7,
            phone_number = $8,
            sex = $9,
            avatar = $10,
            user_status = $11,
            dept_id = $12,
            post_id = $13,
            remark = $14,
            updater_id = $15
        WHERE id = $1
          AND deleted = FALSE
        "#,
    )
    .bind(path_user_id)
    .bind(&user.username)
    .bind(&user.email)
    .bind(user.nickname.as_deref().unwrap_or(&user.username))
    .bind(&user.avatar)
    .bind(user_status_to_login_status(user.status))
    .bind(&user.nickname)
    .bind(&user.phone_number)
    .bind(user.sex)
    .bind(&user.avatar)
    .bind(user.status)
    .bind(user.dept_id)
    .bind(user.post_id)
    .bind(&user.remark)
    .bind(updater_id)
    .execute(&mut *tx)
    .await?;

    sync_single_user_role(path_user_id, user.role_id, &mut tx).await?;
    tx.commit().await?;

    Ok(())
}

pub async fn update_system_user_password(
    user_id: i64,
    data: &ResetUserPasswordDTO,
    password_hash: &str,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    if data
        .user_id
        .is_some_and(|body_user_id| body_user_id != user_id)
    {
        return Err(anyhow::anyhow!("路径 userId 与请求体不一致"));
    }
    if data.password.len() < 8 {
        return Err(anyhow::anyhow!("密码长度不能少于 8 位"));
    }

    let rows_affected = sqlx::query(
        r#"
        UPDATE user_info
        SET password = $2,
            token_version = token_version + 1
        WHERE id = $1
          AND deleted = FALSE
        "#,
    )
    .bind(user_id)
    .bind(password_hash)
    .execute(db_pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(anyhow::anyhow!("用户不存在"));
    }

    Ok(())
}

pub async fn update_own_password(
    user_id: i64,
    data: &UpdateOwnPasswordDTO,
    password_hash: &str,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    if data
        .user_id
        .is_some_and(|body_user_id| body_user_id != user_id)
    {
        return Err(anyhow::anyhow!("路径 userId 与请求体不一致"));
    }
    if data.old_password.len() < 8 || data.new_password.len() < 8 {
        return Err(anyhow::anyhow!("密码长度不能少于 8 位"));
    }
    if data.old_password == data.new_password {
        return Err(anyhow::anyhow!("新密码不能与旧密码相同"));
    }

    let current_password: Option<Option<String>> = sqlx::query_scalar(
        r#"
        SELECT password
        FROM user_info
        WHERE id = $1
          AND deleted = FALSE
        "#,
    )
    .bind(user_id)
    .fetch_optional(db_pool)
    .await?;

    let current_password = current_password
        .ok_or_else(|| anyhow::anyhow!("用户不存在"))?
        .ok_or_else(|| anyhow::anyhow!("当前用户未设置本地密码"))?;
    let verified = bcrypt::verify(&data.old_password, &current_password)
        .map_err(|_| anyhow::anyhow!("旧密码校验失败"))?;
    if !verified {
        return Err(anyhow::anyhow!("旧密码不正确"));
    }

    let rows_affected = sqlx::query(
        r#"
        UPDATE user_info
        SET password = $2,
            token_version = token_version + 1
        WHERE id = $1
          AND deleted = FALSE
        "#,
    )
    .bind(user_id)
    .bind(password_hash)
    .execute(db_pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(anyhow::anyhow!("用户不存在"));
    }

    Ok(())
}

pub async fn update_system_user_status(
    user_id: i64,
    data: &UpdateUserStatusDTO,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    if data
        .user_id
        .is_some_and(|body_user_id| body_user_id != user_id)
    {
        return Err(anyhow::anyhow!("路径 userId 与请求体不一致"));
    }
    validate_user_status(data.status)?;

    let rows_affected = sqlx::query(
        r#"
        UPDATE user_info
        SET user_status = $2,
            status = $3,
            token_version = token_version + 1
        WHERE id = $1
          AND deleted = FALSE
        "#,
    )
    .bind(user_id)
    .bind(data.status)
    .bind(user_status_to_login_status(data.status))
    .execute(db_pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(anyhow::anyhow!("用户不存在"));
    }

    Ok(())
}

pub async fn update_user_avatar(
    user_id: i64,
    avatar_url: &str,
    db_pool: &PgPool,
) -> anyhow::Result<()> {
    if user_id <= 0 {
        return Err(anyhow::anyhow!("userId 必须为正整数"));
    }

    let avatar_url = avatar_url.trim();
    if avatar_url.is_empty() {
        return Err(anyhow::anyhow!("头像地址不能为空"));
    }

    let rows_affected = sqlx::query(
        r#"
        UPDATE user_info
        SET avatar = $2,
            avatar_url = $2
        WHERE id = $1
          AND deleted = FALSE
        "#,
    )
    .bind(user_id)
    .bind(avatar_url)
    .execute(db_pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(anyhow::anyhow!("用户不存在"));
    }

    Ok(())
}

pub async fn delete_system_users(
    user_ids: &[i64],
    current_user_id: Option<i64>,
    db_pool: &PgPool,
) -> anyhow::Result<u64> {
    if user_ids.is_empty() {
        return Err(anyhow::anyhow!("userIds 不能为空"));
    }
    if user_ids.iter().any(|id| *id <= 0) {
        return Err(anyhow::anyhow!("userIds 必须为正整数"));
    }
    if let Some(current_user_id) = current_user_id
        && user_ids.contains(&current_user_id)
    {
        return Err(anyhow::anyhow!("当前登录用户不能删除"));
    }

    let admin_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_info WHERE id = ANY($1) AND is_admin = TRUE AND deleted = FALSE",
    )
    .bind(user_ids)
    .fetch_one(db_pool)
    .await?;
    if admin_count > 0 {
        return Err(anyhow::anyhow!("超级管理员不允许删除"));
    }

    let mut tx = db_pool.begin().await?;
    sqlx::query("DELETE FROM user_roles WHERE user_id = ANY($1)")
        .bind(user_ids)
        .execute(&mut *tx)
        .await?;

    let result = sqlx::query(
        r#"
        UPDATE user_info
        SET deleted = TRUE,
            token_version = token_version + 1
        WHERE id = ANY($1)
          AND deleted = FALSE
        "#,
    )
    .bind(user_ids)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(result.rows_affected())
}

pub async fn get_user_primary_role_id(
    user_id: i64,
    db_pool: &PgPool,
) -> anyhow::Result<Option<i64>> {
    selected_user_role_id(user_id, db_pool).await
}

pub fn parse_id_list(value: &str, field_name: &str) -> anyhow::Result<Vec<i64>> {
    let ids: Result<Vec<_>, _> = value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::parse::<i64>)
        .collect();

    let ids = ids.map_err(|_| anyhow::anyhow!("{field_name} 必须为数字"))?;
    if ids.is_empty() {
        return Err(anyhow::anyhow!("{field_name} 不能为空"));
    }
    if ids.iter().any(|id| *id <= 0) {
        return Err(anyhow::anyhow!("{field_name} 必须为正整数"));
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_dept, normalize_post, normalize_user, parse_id_list, parse_status_value,
        status_label, trim_optional, user_status_to_login_status, validate_user_status,
    };

    #[test]
    fn status_labels_match_frontend_dictionary() {
        assert_eq!(status_label(1), "正常");
        assert_eq!(status_label(0), "停用");
    }

    #[test]
    fn user_status_maps_to_login_status() {
        assert_eq!(user_status_to_login_status(1), "active");
        assert_eq!(user_status_to_login_status(0), "inactive");
    }

    #[test]
    fn parse_status_value_accepts_string_and_number() {
        assert_eq!(
            parse_status_value(Some(&serde_json::json!("1")), 0).unwrap(),
            1
        );
        assert_eq!(
            parse_status_value(Some(&serde_json::json!(0)), 1).unwrap(),
            0
        );
        assert!(parse_status_value(Some(&serde_json::json!("2")), 1).is_err());
    }

    #[test]
    fn normalize_dept_rejects_invalid_parent_or_name() {
        assert!(normalize_dept(-1, "研发", 1, None, None, None, Some(1)).is_err());
        assert!(normalize_dept(0, "", 1, None, None, None, Some(1)).is_err());
    }

    #[test]
    fn normalize_post_rejects_blank_code_or_name() {
        assert!(normalize_post("", "董事长", 1, None, None).is_err());
        assert!(normalize_post("ceo", "", 1, None, None).is_err());
    }

    #[test]
    fn normalize_user_rejects_invalid_status_and_sex() {
        assert!(validate_user_status(3).is_ok());
        assert!(validate_user_status(4).is_err());
        assert!(
            normalize_user(
                Some("admin"),
                None,
                None,
                None,
                Some(4),
                None,
                Some(1),
                None,
                None,
                None,
                None
            )
            .is_err()
        );
    }

    #[test]
    fn parse_id_list_accepts_comma_separated_values() {
        assert_eq!(parse_id_list("1,2", "ids").unwrap(), vec![1, 2]);
        assert!(parse_id_list("", "ids").is_err());
        assert!(parse_id_list("0", "ids").is_err());
        assert!(parse_id_list("x", "ids").is_err());
    }

    #[test]
    fn trim_optional_turns_blank_values_into_none() {
        assert_eq!(trim_optional(Some(" Alice ")).as_deref(), Some("Alice"));
        assert!(trim_optional(Some("   ")).is_none());
        assert!(trim_optional(None).is_none());
    }
}
