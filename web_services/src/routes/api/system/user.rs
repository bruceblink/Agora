use crate::common::AppState;
use crate::routes::api::export::{
    EXPORT_PAGE_SIZE, datetime, optional, optional_datetime, xlsx_from_rows, xlsx_response,
};
use actix_multipart::Multipart;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, delete, get, post, put, web};
use calamine::{Data, Reader, open_workbook_auto_from_rs};
use common::api::{ApiError, ApiResponse};
use common::dto::{
    CreateDeptDTO, CreatePostDTO, CreateSystemUserDTO, DeptDTO, DeptResponseDTO, PostDTO,
    PostResponseDTO, ResetUserPasswordDTO, SystemUserDTO, UpdateDeptDTO, UpdatePostDTO,
    UpdateSystemUserDTO, UpdateUserStatusDTO,
};
use common::po::ApiResult;
use common::utils::JwtClaims;
use common::{DeptQuery, PostQuery, SystemUserQuery};
use futures_util::StreamExt;
use infra::{
    SystemDeptBusinessError, SystemPostBusinessError, SystemUserBusinessError, create_dept,
    create_post, create_system_user, delete_dept, delete_posts, delete_system_users, get_dept,
    get_post, get_user_detail, list_depts, list_posts, list_system_users, parse_id_list,
    update_dept, update_post, update_system_user, update_system_user_password,
    update_system_user_status,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;

const USER_IMPORT_MAX_BYTES: usize = 5 * 1024 * 1024;
const USER_IMPORT_HEADERS: [&str; 12] = [
    "部门ID",
    "用户名",
    "昵称",
    "邮件",
    "电话号码",
    "性别",
    "头像",
    "密码",
    "状态",
    "角色ID",
    "职位ID",
    "备注",
];

const KEYSTONE_OBJECT_NOT_FOUND_CODE: i32 = 10001;
const KEYSTONE_POST_NAME_NOT_UNIQUE_CODE: i32 = 10701;
const KEYSTONE_POST_CODE_NOT_UNIQUE_CODE: i32 = 10702;
const KEYSTONE_POST_ASSIGNED_TO_USER_CODE: i32 = 10703;
const KEYSTONE_DEPT_NAME_NOT_UNIQUE_CODE: i32 = 10801;
const KEYSTONE_DEPT_PARENT_SELF_CODE: i32 = 10802;
const KEYSTONE_DEPT_STATUS_CHANGE_CODE: i32 = 10803;
const KEYSTONE_DEPT_HAS_CHILD_CODE: i32 = 10804;
const KEYSTONE_DEPT_LINKED_USER_CODE: i32 = 10805;
const KEYSTONE_DEPT_PARENT_UNAVAILABLE_CODE: i32 = 10806;
const KEYSTONE_USER_PHONE_NOT_UNIQUE_CODE: i32 = 10508;
const KEYSTONE_USER_EMAIL_NOT_UNIQUE_CODE: i32 = 10509;
const KEYSTONE_USER_NAME_NOT_UNIQUE_CODE: i32 = 10513;
const KEYSTONE_USER_CURRENT_USER_CANNOT_DELETE_CODE: i32 = 10514;

fn business_error_response(code: i32, msg: String) -> ApiResponse<()> {
    ApiResponse {
        code,
        msg: msg.clone(),
        status: "error".into(),
        message: Some(msg),
        data: None,
    }
}

fn post_business_error_response(error: &SystemPostBusinessError) -> ApiResponse<()> {
    let code = match error {
        SystemPostBusinessError::ObjectNotFound { .. } => KEYSTONE_OBJECT_NOT_FOUND_CODE,
        SystemPostBusinessError::NameNotUnique { .. } => KEYSTONE_POST_NAME_NOT_UNIQUE_CODE,
        SystemPostBusinessError::CodeNotUnique { .. } => KEYSTONE_POST_CODE_NOT_UNIQUE_CODE,
        SystemPostBusinessError::AlreadyAssignedToUser => KEYSTONE_POST_ASSIGNED_TO_USER_CODE,
    };
    business_error_response(code, error.to_string())
}

fn dept_business_error_response(error: &SystemDeptBusinessError) -> ApiResponse<()> {
    let code = match error {
        SystemDeptBusinessError::ObjectNotFound { .. } => KEYSTONE_OBJECT_NOT_FOUND_CODE,
        SystemDeptBusinessError::NameNotUnique { .. } => KEYSTONE_DEPT_NAME_NOT_UNIQUE_CODE,
        SystemDeptBusinessError::ParentIdNotAllowSelf => KEYSTONE_DEPT_PARENT_SELF_CODE,
        SystemDeptBusinessError::StatusNotAllowChange => KEYSTONE_DEPT_STATUS_CHANGE_CODE,
        SystemDeptBusinessError::HasChildDept => KEYSTONE_DEPT_HAS_CHILD_CODE,
        SystemDeptBusinessError::HasLinkedUser => KEYSTONE_DEPT_LINKED_USER_CODE,
        SystemDeptBusinessError::ParentDeptUnavailable => KEYSTONE_DEPT_PARENT_UNAVAILABLE_CODE,
    };
    business_error_response(code, error.to_string())
}

fn user_business_error_response(error: &SystemUserBusinessError) -> ApiResponse<()> {
    let code = match error {
        SystemUserBusinessError::ObjectNotFound { .. } => KEYSTONE_OBJECT_NOT_FOUND_CODE,
        SystemUserBusinessError::UsernameNotUnique => KEYSTONE_USER_NAME_NOT_UNIQUE_CODE,
        SystemUserBusinessError::PhoneNumberNotUnique => KEYSTONE_USER_PHONE_NOT_UNIQUE_CODE,
        SystemUserBusinessError::EmailNotUnique => KEYSTONE_USER_EMAIL_NOT_UNIQUE_CODE,
        SystemUserBusinessError::CurrentUserCannotBeDeleted => {
            KEYSTONE_USER_CURRENT_USER_CANNOT_DELETE_CODE
        }
    };
    business_error_response(code, error.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeletePostQuery {
    ids: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DeptTreeDTO {
    id: i64,
    parent_id: i64,
    label: String,
    children: Vec<DeptTreeDTO>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ImportUserRow {
    row_number: usize,
    dept_id: Option<i64>,
    username: String,
    nickname: Option<String>,
    email: Option<String>,
    phone_number: Option<String>,
    sex: Option<i16>,
    avatar: Option<String>,
    password: String,
    status: Option<i16>,
    role_id: Option<i64>,
    post_id: Option<i64>,
    remark: Option<String>,
}

impl From<ImportUserRow> for CreateSystemUserDTO {
    fn from(row: ImportUserRow) -> Self {
        Self {
            dept_id: row.dept_id,
            username: row.username,
            nickname: row.nickname,
            email: row.email,
            phone_number: row.phone_number,
            sex: row.sex,
            avatar: row.avatar,
            password: row.password,
            status: row.status,
            role_id: row.role_id,
            post_id: row.post_id,
            remark: row.remark,
        }
    }
}

fn current_user_id(req: &HttpRequest) -> Option<i64> {
    req.extensions().get::<JwtClaims>().map(|claims| claims.uid)
}

fn validate_positive_id(id: i64, field_name: &str) -> Result<(), ApiError> {
    if id > 0 {
        Ok(())
    } else {
        Err(ApiError::BadRequest(format!("{field_name} 必须为正整数")))
    }
}

fn hash_password(password: &str) -> Result<String, ApiError> {
    if password.len() < 8 {
        return Err(ApiError::BadRequest("密码长度不能少于 8 位".into()));
    }
    bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|e| {
        tracing::error!("密码哈希失败: {e}");
        ApiError::Internal("服务器内部错误".into())
    })
}

fn force_post_export_page(query: &mut PostQuery) {
    query.page = Some(1);
    query.page_num = Some(1);
    query.page_size = Some(EXPORT_PAGE_SIZE);
}

fn force_user_export_page(query: &mut SystemUserQuery) {
    query.page = Some(1);
    query.page_num = Some(1);
    query.page_size = Some(EXPORT_PAGE_SIZE);
}

fn post_export_row(item: &PostDTO) -> Vec<String> {
    vec![
        item.post_id.to_string(),
        item.post_code.clone(),
        item.post_name.clone(),
        item.post_sort.to_string(),
        item.remark.clone().unwrap_or_default(),
        item.status_str.clone(),
    ]
}

fn user_export_row(item: &SystemUserDTO) -> Vec<String> {
    vec![
        item.user_id.to_string(),
        optional(&item.post_id),
        optional(&item.post_name),
        optional(&item.role_id),
        optional(&item.role_name),
        optional(&item.dept_id),
        optional(&item.dept_name),
        item.username.clone(),
        optional(&item.nickname),
        optional(&item.user_type),
        optional(&item.email),
        optional(&item.phone_number),
        optional(&item.sex),
        optional(&item.avatar),
        item.status.to_string(),
        optional(&item.login_ip),
        optional_datetime(&item.login_date),
        optional(&item.creator_id),
        optional(&item.creator_name),
        datetime(&item.create_time),
        optional(&item.updater_id),
        optional(&item.updater_name),
        optional_datetime(&item.update_time),
        optional(&item.remark),
    ]
}

fn dept_tree_nodes(depts: &[DeptDTO]) -> Vec<DeptTreeDTO> {
    let ids = depts
        .iter()
        .map(|dept| dept.id)
        .collect::<std::collections::HashSet<_>>();
    depts
        .iter()
        .filter(|dept| dept.parent_id == 0 || !ids.contains(&dept.parent_id))
        .map(|dept| dept_tree_node(depts, dept))
        .collect()
}

fn dept_tree_node(depts: &[DeptDTO], dept: &DeptDTO) -> DeptTreeDTO {
    DeptTreeDTO {
        id: dept.id,
        parent_id: dept.parent_id,
        label: dept.dept_name.clone(),
        children: depts
            .iter()
            .filter(|child| child.parent_id == dept.id)
            .map(|child| dept_tree_node(depts, child))
            .collect(),
    }
}

fn cell_string(cell: Option<&Data>) -> String {
    cell.map(ToString::to_string)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn blank_to_none(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

fn parse_optional_i64(
    value: String,
    field_name: &str,
    row_number: usize,
) -> Result<Option<i64>, ApiError> {
    if value.is_empty() {
        return Ok(None);
    }
    value
        .parse::<i64>()
        .map(Some)
        .map_err(|_| ApiError::BadRequest(format!("第 {row_number} 行 {field_name} 必须为整数")))
}

fn parse_optional_i16(
    value: String,
    field_name: &str,
    row_number: usize,
) -> Result<Option<i16>, ApiError> {
    if value.is_empty() {
        return Ok(None);
    }
    value
        .parse::<i16>()
        .map(Some)
        .map_err(|_| ApiError::BadRequest(format!("第 {row_number} 行 {field_name} 必须为整数")))
}

fn row_is_empty(row: &[Data]) -> bool {
    row.iter().all(|cell| cell.to_string().trim().is_empty())
}

fn header_indexes(header_row: &[Data]) -> Result<HashMap<&'static str, usize>, ApiError> {
    let headers = header_row
        .iter()
        .enumerate()
        .map(|(idx, cell)| (cell.to_string().trim().to_string(), idx))
        .collect::<HashMap<_, _>>();

    let mut indexes = HashMap::new();
    for expected in USER_IMPORT_HEADERS {
        let Some(index) = headers.get(expected).copied() else {
            return Err(ApiError::BadRequest(format!("Excel 缺少表头：{expected}")));
        };
        indexes.insert(expected, index);
    }
    Ok(indexes)
}

fn required_cell(
    row: &[Data],
    indexes: &HashMap<&'static str, usize>,
    field_name: &'static str,
    row_number: usize,
) -> Result<String, ApiError> {
    let value = cell_string(indexes.get(field_name).and_then(|idx| row.get(*idx)));
    if value.is_empty() {
        Err(ApiError::BadRequest(format!(
            "第 {row_number} 行 {field_name} 不能为空"
        )))
    } else {
        Ok(value)
    }
}

fn optional_cell(
    row: &[Data],
    indexes: &HashMap<&'static str, usize>,
    field_name: &'static str,
) -> String {
    cell_string(indexes.get(field_name).and_then(|idx| row.get(*idx)))
}

fn parse_import_user_row(
    row: &[Data],
    indexes: &HashMap<&'static str, usize>,
    row_number: usize,
) -> Result<ImportUserRow, ApiError> {
    let username = required_cell(row, indexes, "用户名", row_number)?;
    let password = required_cell(row, indexes, "密码", row_number)?;

    Ok(ImportUserRow {
        row_number,
        dept_id: parse_optional_i64(optional_cell(row, indexes, "部门ID"), "部门ID", row_number)?,
        username,
        nickname: blank_to_none(optional_cell(row, indexes, "昵称")),
        email: blank_to_none(optional_cell(row, indexes, "邮件")),
        phone_number: blank_to_none(optional_cell(row, indexes, "电话号码")),
        sex: parse_optional_i16(optional_cell(row, indexes, "性别"), "性别", row_number)?,
        avatar: blank_to_none(optional_cell(row, indexes, "头像")),
        password,
        status: parse_optional_i16(optional_cell(row, indexes, "状态"), "状态", row_number)?,
        role_id: parse_optional_i64(optional_cell(row, indexes, "角色ID"), "角色ID", row_number)?,
        post_id: parse_optional_i64(optional_cell(row, indexes, "职位ID"), "职位ID", row_number)?,
        remark: blank_to_none(optional_cell(row, indexes, "备注")),
    })
}

fn parse_import_users_excel(bytes: &[u8]) -> Result<Vec<ImportUserRow>, ApiError> {
    let cursor = Cursor::new(bytes.to_vec());
    let mut workbook = open_workbook_auto_from_rs(cursor).map_err(|e| {
        tracing::warn!("解析用户导入 Excel 失败: {e}");
        ApiError::BadRequest("用户 Excel 文件格式不正确，请使用系统下载的模板".into())
    })?;
    let range = workbook
        .worksheet_range_at(0)
        .ok_or_else(|| ApiError::BadRequest("用户 Excel 文件缺少工作表".into()))?
        .map_err(|e| {
            tracing::warn!("读取用户导入 Excel 工作表失败: {e}");
            ApiError::BadRequest("用户 Excel 文件格式不正确".into())
        })?;
    let mut rows = range.rows();
    let header_row = rows
        .next()
        .ok_or_else(|| ApiError::BadRequest("用户 Excel 文件缺少表头".into()))?;
    let indexes = header_indexes(header_row)?;

    let mut users = Vec::new();
    for (idx, row) in rows.enumerate() {
        if row_is_empty(row) {
            continue;
        }
        users.push(parse_import_user_row(row, &indexes, idx + 2)?);
    }

    if users.is_empty() {
        return Err(ApiError::BadRequest("用户 Excel 文件没有可导入数据".into()));
    }
    Ok(users)
}

async fn read_user_import_file(mut payload: Multipart) -> Result<Vec<u8>, ApiError> {
    while let Some(item) = payload.next().await {
        let mut field = item.map_err(|e| {
            tracing::warn!("读取用户导入 multipart 字段失败: {e}");
            ApiError::BadRequest("上传文件失败".into())
        })?;
        if field.name() != Some("file") {
            continue;
        }

        let filename = field
            .content_disposition()
            .and_then(|disposition| disposition.get_filename())
            .unwrap_or_default();
        let filename = filename.to_ascii_lowercase();
        if !filename.ends_with(".xlsx") && !filename.ends_with(".xls") {
            return Err(ApiError::BadRequest(
                "用户导入仅支持 xls 或 xlsx 文件".into(),
            ));
        }

        let mut bytes = Vec::new();
        while let Some(chunk) = field.next().await {
            let chunk = chunk.map_err(|e| {
                tracing::warn!("读取用户导入文件失败: {e}");
                ApiError::BadRequest("上传文件失败".into())
            })?;
            bytes.extend_from_slice(&chunk);
            if bytes.len() > USER_IMPORT_MAX_BYTES {
                return Err(ApiError::BadRequest("用户导入文件大小不能超过 5MB".into()));
            }
        }
        if bytes.is_empty() {
            return Err(ApiError::BadRequest("上传文件不能为空".into()));
        }
        return Ok(bytes);
    }

    Err(ApiError::BadRequest("上传文件不能为空".into()))
}

#[get("/system/depts")]
async fn depts_list(
    req: HttpRequest,
    query: web::Query<DeptQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_depts(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询部门列表失败: {e:?}");
            Err(ApiError::Database("查询部门列表失败".into()))
        }
    }
}

#[get("/system/depts/dropdown")]
async fn depts_dropdown(req: HttpRequest, app_state: web::Data<AppState>) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_depts(
        &DeptQuery {
            dept_id: None,
            parent_id: None,
            status: None,
            dept_name: None,
        },
        &app_state.db_pool,
    )
    .await
    {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(dept_tree_nodes(&data)))),
        Err(e) => {
            tracing::error!("查询部门下拉树失败: {e:?}");
            Err(ApiError::Database("查询部门下拉树失败".into()))
        }
    }
}

#[get("/system/dept/{dept_id}")]
async fn dept_get(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let dept_id = path.into_inner();
    validate_positive_id(dept_id, "deptId")?;

    match get_dept(dept_id, &app_state.db_pool).await {
        Ok(Some(data)) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Ok(None) => Ok(HttpResponse::Ok().json(ApiResponse::ok(DeptResponseDTO::empty()))),
        Err(e) => {
            tracing::error!("查询部门 {dept_id} 失败: {e:?}");
            Err(ApiError::Database("查询部门失败".into()))
        }
    }
}

#[post("/system/dept")]
async fn dept_create(
    req: HttpRequest,
    body: web::Json<CreateDeptDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let data = body.into_inner();

    match create_dept(&data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            if let Some(error) = e.downcast_ref::<SystemDeptBusinessError>() {
                return Ok(HttpResponse::Ok().json(dept_business_error_response(error)));
            }
            tracing::error!("新增部门失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[put("/system/dept/{dept_id}")]
async fn dept_update(
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateDeptDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let dept_id = path.into_inner();
    validate_positive_id(dept_id, "deptId")?;
    let data = body.into_inner();

    match update_dept(dept_id, &data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            if let Some(error) = e.downcast_ref::<SystemDeptBusinessError>() {
                return Ok(HttpResponse::Ok().json(dept_business_error_response(error)));
            }
            tracing::error!("更新部门 {dept_id} 失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[delete("/system/dept/{dept_id}")]
async fn dept_delete(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let dept_id = path.into_inner();
    validate_positive_id(dept_id, "deptId")?;

    match delete_dept(dept_id, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            if let Some(error) = e.downcast_ref::<SystemDeptBusinessError>() {
                return Ok(HttpResponse::Ok().json(dept_business_error_response(error)));
            }
            tracing::error!("删除部门 {dept_id} 失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[get("/system/post/list")]
async fn posts_list(
    req: HttpRequest,
    query: web::Query<PostQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_posts(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询岗位列表失败: {e:?}");
            Err(ApiError::Database("查询岗位列表失败".into()))
        }
    }
}

#[get("/system/post/excel")]
async fn posts_export(
    req: HttpRequest,
    query: web::Query<PostQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let mut export_query = query.into_inner();
    force_post_export_page(&mut export_query);

    match list_posts(&export_query, &app_state.db_pool).await {
        Ok(data) => {
            let headers = ["岗位ID", "岗位编码", "岗位名称", "岗位排序", "备注", "状态"];
            let rows = data.items.iter().map(post_export_row).collect::<Vec<_>>();
            let bytes = xlsx_from_rows("Sheet1", &headers, &rows)?;
            Ok(xlsx_response("posts.xlsx", bytes))
        }
        Err(e) => {
            tracing::error!("导出岗位列表失败: {e:?}");
            Err(ApiError::Database("导出岗位列表失败".into()))
        }
    }
}

#[get("/system/post/{post_id}")]
async fn post_get(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let post_id = path.into_inner();
    validate_positive_id(post_id, "postId")?;

    match get_post(post_id, &app_state.db_pool).await {
        Ok(Some(data)) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Ok(None) => Ok(HttpResponse::Ok().json(ApiResponse::ok(PostResponseDTO::empty()))),
        Err(e) => {
            tracing::error!("查询岗位 {post_id} 失败: {e:?}");
            Err(ApiError::Database("查询岗位失败".into()))
        }
    }
}

#[post("/system/post")]
async fn post_create(
    req: HttpRequest,
    body: web::Json<CreatePostDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let data = body.into_inner();

    match create_post(&data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            if let Some(error) = e.downcast_ref::<SystemPostBusinessError>() {
                return Ok(HttpResponse::Ok().json(post_business_error_response(error)));
            }
            tracing::error!("新增岗位失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[put("/system/post")]
async fn post_update(
    req: HttpRequest,
    body: web::Json<UpdatePostDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let data = body.into_inner();

    match update_post(&data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            if let Some(error) = e.downcast_ref::<SystemPostBusinessError>() {
                return Ok(HttpResponse::Ok().json(post_business_error_response(error)));
            }
            tracing::error!("更新岗位失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[delete("/system/post")]
async fn posts_delete(
    req: HttpRequest,
    query: web::Query<DeletePostQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let ids = parse_id_list(&query.into_inner().ids, "ids")
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    match delete_posts(&ids, &app_state.db_pool).await {
        Ok(_) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            if let Some(error) = e.downcast_ref::<SystemPostBusinessError>() {
                return Ok(HttpResponse::Ok().json(post_business_error_response(error)));
            }
            tracing::error!("删除岗位失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[get("/system/users")]
async fn users_list(
    req: HttpRequest,
    query: web::Query<SystemUserQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_system_users(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询用户列表失败: {e:?}");
            Err(ApiError::Database("查询用户列表失败".into()))
        }
    }
}

#[get("/system/users/excel")]
async fn users_export(
    req: HttpRequest,
    query: web::Query<SystemUserQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let mut export_query = query.into_inner();
    force_user_export_page(&mut export_query);

    match list_system_users(&export_query, &app_state.db_pool).await {
        Ok(data) => {
            let headers = [
                "用户ID",
                "职位ID",
                "职位名称",
                "角色ID",
                "角色名称",
                "部门ID",
                "部门名称",
                "用户名",
                "用户昵称",
                "用户类型",
                "邮件",
                "号码",
                "性别",
                "用户头像",
                "状态",
                "IP",
                "登录时间",
                "创建者ID",
                "创建者",
                "创建时间",
                "修改者ID",
                "修改者",
                "修改时间",
                "备注",
            ];
            let rows = data.items.iter().map(user_export_row).collect::<Vec<_>>();
            let bytes = xlsx_from_rows("用户列表", &headers, &rows)?;
            Ok(xlsx_response("users.xlsx", bytes))
        }
        Err(e) => {
            tracing::error!("导出用户列表失败: {e:?}");
            Err(ApiError::Database("导出用户列表失败".into()))
        }
    }
}

#[get("/system/users/excelTemplate")]
async fn users_excel_template(req: HttpRequest, app_state: web::Data<AppState>) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let headers = [
        "部门ID",
        "用户名",
        "昵称",
        "邮件",
        "电话号码",
        "性别",
        "头像",
        "密码",
        "状态",
        "角色ID",
        "职位ID",
        "备注",
    ];
    let rows = vec![vec![
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ]];
    let bytes = xlsx_from_rows("Sheet1", &headers, &rows)?;
    Ok(xlsx_response("user-import-template.xlsx", bytes))
}

#[post("/system/users/excel")]
async fn users_import(
    req: HttpRequest,
    payload: Multipart,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let bytes = read_user_import_file(payload).await?;
    let rows = parse_import_users_excel(&bytes)?;
    let creator_id = current_user_id(&req);

    for row in rows {
        let row_number = row.row_number;
        let data = CreateSystemUserDTO::from(row);
        let password_hash = match hash_password(&data.password) {
            Ok(hash) => hash,
            Err(ApiError::BadRequest(message)) => {
                return Err(ApiError::BadRequest(format!(
                    "第 {row_number} 行 {message}"
                )));
            }
            Err(e) => return Err(e),
        };

        create_system_user(&data, &password_hash, creator_id, &app_state.db_pool)
            .await
            .map_err(|e| {
                tracing::error!("导入用户第 {row_number} 行失败: {e:?}");
                ApiError::BadRequest(format!("第 {row_number} 行导入失败: {e}"))
            })?;
    }

    Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(())))
}

#[get("/system/users/{user_id}")]
async fn user_get(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let user_id = path.into_inner();
    validate_positive_id(user_id, "userId")?;

    match get_user_detail(Some(user_id), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询用户 {user_id} 失败: {e:?}");
            Err(ApiError::Internal("服务器内部错误".into()))
        }
    }
}

#[post("/system/users")]
async fn user_create(
    req: HttpRequest,
    body: web::Json<CreateSystemUserDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let data = body.into_inner();
    let password_hash = hash_password(&data.password)?;

    match create_system_user(
        &data,
        &password_hash,
        current_user_id(&req),
        &app_state.db_pool,
    )
    .await
    {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            if let Some(error) = e.downcast_ref::<SystemUserBusinessError>() {
                return Ok(HttpResponse::Ok().json(user_business_error_response(error)));
            }
            tracing::error!("新增用户失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[put("/system/users/{user_id}")]
async fn user_update(
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateSystemUserDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let user_id = path.into_inner();
    validate_positive_id(user_id, "userId")?;
    let data = body.into_inner();

    match update_system_user(user_id, &data, current_user_id(&req), &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            if let Some(error) = e.downcast_ref::<SystemUserBusinessError>() {
                return Ok(HttpResponse::Ok().json(user_business_error_response(error)));
            }
            tracing::error!("更新用户 {user_id} 失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[put("/system/users/{user_id}/password")]
async fn user_password_update(
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<ResetUserPasswordDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let user_id = path.into_inner();
    validate_positive_id(user_id, "userId")?;
    let data = body.into_inner();
    let password_hash = hash_password(&data.password)?;

    match update_system_user_password(user_id, &data, &password_hash, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            if let Some(error) = e.downcast_ref::<SystemUserBusinessError>() {
                return Ok(HttpResponse::Ok().json(user_business_error_response(error)));
            }
            tracing::error!("重置用户 {user_id} 密码失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[put("/system/users/{user_id}/status")]
async fn user_status_update(
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateUserStatusDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let user_id = path.into_inner();
    validate_positive_id(user_id, "userId")?;
    let data = body.into_inner();

    match update_system_user_status(user_id, &data, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            if let Some(error) = e.downcast_ref::<SystemUserBusinessError>() {
                return Ok(HttpResponse::Ok().json(user_business_error_response(error)));
            }
            tracing::error!("更新用户 {user_id} 状态失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[delete("/system/users/{user_ids}")]
async fn users_delete(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let user_ids = parse_id_list(&path.into_inner(), "userIds")
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    match delete_system_users(&user_ids, current_user_id(&req), &app_state.db_pool).await {
        Ok(_) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            if let Some(error) = e.downcast_ref::<SystemUserBusinessError>() {
                return Ok(HttpResponse::Ok().json(user_business_error_response(error)));
            }
            tracing::error!("删除用户失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        KEYSTONE_DEPT_HAS_CHILD_CODE, KEYSTONE_DEPT_LINKED_USER_CODE,
        KEYSTONE_DEPT_NAME_NOT_UNIQUE_CODE, KEYSTONE_DEPT_PARENT_SELF_CODE,
        KEYSTONE_DEPT_PARENT_UNAVAILABLE_CODE, KEYSTONE_DEPT_STATUS_CHANGE_CODE,
        KEYSTONE_OBJECT_NOT_FOUND_CODE, KEYSTONE_POST_ASSIGNED_TO_USER_CODE,
        KEYSTONE_POST_CODE_NOT_UNIQUE_CODE, KEYSTONE_POST_NAME_NOT_UNIQUE_CODE,
        KEYSTONE_USER_CURRENT_USER_CANNOT_DELETE_CODE, KEYSTONE_USER_EMAIL_NOT_UNIQUE_CODE,
        KEYSTONE_USER_NAME_NOT_UNIQUE_CODE, KEYSTONE_USER_PHONE_NOT_UNIQUE_CODE,
        USER_IMPORT_HEADERS, dept_business_error_response, dept_tree_nodes, hash_password,
        parse_import_users_excel, post_business_error_response, user_business_error_response,
        user_export_row, validate_positive_id,
    };
    use crate::routes::api::export::xlsx_from_rows;
    use chrono::Utc;
    use common::api::{ApiError, ApiResponse};
    use common::dto::{DeptDTO, DeptResponseDTO, PostResponseDTO, SystemUserDTO, UserDetailDTO};
    use infra::{SystemDeptBusinessError, SystemPostBusinessError, SystemUserBusinessError};

    #[test]
    fn validate_positive_id_rejects_non_positive_values() {
        assert!(validate_positive_id(0, "userId").is_err());
        assert!(validate_positive_id(-1, "userId").is_err());
    }

    #[test]
    fn hash_password_rejects_short_password() {
        assert!(hash_password("short").is_err());
    }

    #[test]
    fn missing_dept_detail_matches_keystone_empty_success() {
        let value = serde_json::to_value(ApiResponse::ok(DeptResponseDTO::empty()))
            .unwrap_or(serde_json::Value::Null);

        assert_eq!(value["code"], 0);
        assert_eq!(value["msg"], "操作成功");
        assert_eq!(value["status"], "ok");
        assert!(value["data"].is_object());
        assert!(value["data"]["id"].is_null());
        assert!(value["data"]["deptName"].is_null());
    }

    #[test]
    fn dept_business_responses_match_keystone_errors() {
        let cases = [
            (
                SystemDeptBusinessError::ObjectNotFound { id: 9 },
                KEYSTONE_OBJECT_NOT_FOUND_CODE,
                "找不到ID为 9 的 部门",
            ),
            (
                SystemDeptBusinessError::NameNotUnique {
                    dept_name: "研发部".to_string(),
                },
                KEYSTONE_DEPT_NAME_NOT_UNIQUE_CODE,
                "部门名称:研发部, 已存在",
            ),
            (
                SystemDeptBusinessError::ParentIdNotAllowSelf,
                KEYSTONE_DEPT_PARENT_SELF_CODE,
                "父级部门不能选择自己",
            ),
            (
                SystemDeptBusinessError::StatusNotAllowChange,
                KEYSTONE_DEPT_STATUS_CHANGE_CODE,
                "子部门还有正在启用的部门，暂时不能停用该部门",
            ),
            (
                SystemDeptBusinessError::HasChildDept,
                KEYSTONE_DEPT_HAS_CHILD_CODE,
                "该部门存在下级部门不允许删除",
            ),
            (
                SystemDeptBusinessError::HasLinkedUser,
                KEYSTONE_DEPT_LINKED_USER_CODE,
                "该部门存在关联的用户不允许删除",
            ),
            (
                SystemDeptBusinessError::ParentDeptUnavailable,
                KEYSTONE_DEPT_PARENT_UNAVAILABLE_CODE,
                "该父级部门不存在或已停用",
            ),
        ];

        for (error, code, message) in cases {
            let value = serde_json::to_value(dept_business_error_response(&error))
                .unwrap_or(serde_json::Value::Null);
            assert_eq!(value["code"], code);
            assert_eq!(value["msg"], message);
            assert_eq!(value["message"], message);
            assert_eq!(value["status"], "error");
            assert!(value.get("data").is_none());
        }
    }

    #[test]
    fn missing_post_detail_matches_keystone_empty_success() {
        let value = serde_json::to_value(ApiResponse::ok(PostResponseDTO::empty()))
            .unwrap_or(serde_json::Value::Null);

        assert_eq!(value["code"], 0);
        assert_eq!(value["msg"], "操作成功");
        assert_eq!(value["status"], "ok");
        assert!(value["data"].is_object());
        assert!(value["data"]["postId"].is_null());
        assert!(value["data"]["postName"].is_null());
    }

    #[test]
    fn post_business_responses_match_keystone_errors() {
        let cases = [
            (
                SystemPostBusinessError::ObjectNotFound { id: 8 },
                KEYSTONE_OBJECT_NOT_FOUND_CODE,
                "找不到ID为 8 的 职位",
            ),
            (
                SystemPostBusinessError::NameNotUnique {
                    post_name: "研发工程师".to_string(),
                },
                KEYSTONE_POST_NAME_NOT_UNIQUE_CODE,
                "岗位名称:研发工程师, 已存在",
            ),
            (
                SystemPostBusinessError::CodeNotUnique {
                    post_code: "rd".to_string(),
                },
                KEYSTONE_POST_CODE_NOT_UNIQUE_CODE,
                "岗位编号:rd, 已存在",
            ),
            (
                SystemPostBusinessError::AlreadyAssignedToUser,
                KEYSTONE_POST_ASSIGNED_TO_USER_CODE,
                "职位已分配给用户，请先取消分配再删除",
            ),
        ];

        for (error, code, message) in cases {
            let value = serde_json::to_value(post_business_error_response(&error))
                .unwrap_or(serde_json::Value::Null);
            assert_eq!(value["code"], code);
            assert_eq!(value["msg"], message);
            assert_eq!(value["message"], message);
            assert_eq!(value["status"], "error");
            assert!(value.get("data").is_none());
        }
    }

    #[test]
    fn missing_user_detail_matches_keystone_empty_success() {
        let value = serde_json::to_value(ApiResponse::ok(UserDetailDTO {
            user: None,
            role_options: Vec::new(),
            post_options: Vec::new(),
            post_id: None,
            role_id: None,
            permissions: Vec::new(),
        }))
        .unwrap_or(serde_json::Value::Null);

        assert_eq!(value["code"], 0);
        assert_eq!(value["msg"], "操作成功");
        assert_eq!(value["status"], "ok");
        assert!(value["data"].is_object());
        assert!(value["data"]["user"].is_null());
        assert!(value["data"]["postId"].is_null());
        assert!(value["data"]["roleId"].is_null());
    }

    #[test]
    fn user_business_responses_match_keystone_errors() {
        let cases = [
            (
                SystemUserBusinessError::ObjectNotFound {
                    id: 9,
                    object_name: "用户",
                },
                KEYSTONE_OBJECT_NOT_FOUND_CODE,
                "找不到ID为 9 的 用户",
            ),
            (
                SystemUserBusinessError::ObjectNotFound {
                    id: 3,
                    object_name: "角色",
                },
                KEYSTONE_OBJECT_NOT_FOUND_CODE,
                "找不到ID为 3 的 角色",
            ),
            (
                SystemUserBusinessError::UsernameNotUnique,
                KEYSTONE_USER_NAME_NOT_UNIQUE_CODE,
                "用户名已被其他用户占用",
            ),
            (
                SystemUserBusinessError::PhoneNumberNotUnique,
                KEYSTONE_USER_PHONE_NOT_UNIQUE_CODE,
                "该电话号码已被其他用户占用",
            ),
            (
                SystemUserBusinessError::EmailNotUnique,
                KEYSTONE_USER_EMAIL_NOT_UNIQUE_CODE,
                "该邮件地址已被其他用户占用",
            ),
            (
                SystemUserBusinessError::CurrentUserCannotBeDeleted,
                KEYSTONE_USER_CURRENT_USER_CANNOT_DELETE_CODE,
                "当前用户不允许被删除",
            ),
        ];

        for (error, code, message) in cases {
            let value = serde_json::to_value(user_business_error_response(&error))
                .unwrap_or(serde_json::Value::Null);
            assert_eq!(value["code"], code);
            assert_eq!(value["msg"], message);
            assert_eq!(value["message"], message);
            assert_eq!(value["status"], "error");
            assert!(value.get("data").is_none());
        }
    }

    #[test]
    fn user_export_row_keeps_keystone_raw_numeric_columns() {
        let row = user_export_row(&SystemUserDTO {
            user_id: 1,
            post_id: Some(2),
            post_name: Some("研发".into()),
            role_id: Some(3),
            role_name: Some("管理员".into()),
            dept_id: Some(4),
            dept_name: Some("总部".into()),
            username: "admin".into(),
            nickname: Some("Admin".into()),
            user_type: Some(1),
            email: Some("admin@example.com".into()),
            phone_number: Some("15800000000".into()),
            sex: Some(2),
            avatar: None,
            status: 1,
            login_ip: None,
            login_date: None,
            creator_id: None,
            creator_name: None,
            create_time: Utc::now(),
            updater_id: None,
            updater_name: None,
            update_time: None,
            remark: None,
        });

        assert_eq!(row[12], "2");
        assert_eq!(row[14], "1");
    }

    #[test]
    fn parse_import_users_excel_maps_keystone_template_headers() {
        let rows = vec![vec![
            "1".to_string(),
            "demo".to_string(),
            "演示用户".to_string(),
            "demo@example.com".to_string(),
            "15800000000".to_string(),
            "2".to_string(),
            String::new(),
            "password123".to_string(),
            "1".to_string(),
            "2".to_string(),
            "4".to_string(),
            "备注".to_string(),
        ]];
        let bytes = xlsx_from_rows("Sheet1", &USER_IMPORT_HEADERS, &rows).unwrap();

        let users = parse_import_users_excel(&bytes).unwrap();

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].row_number, 2);
        assert_eq!(users[0].dept_id, Some(1));
        assert_eq!(users[0].username, "demo");
        assert_eq!(users[0].nickname.as_deref(), Some("演示用户"));
        assert_eq!(users[0].sex, Some(2));
        assert_eq!(users[0].status, Some(1));
        assert_eq!(users[0].role_id, Some(2));
        assert_eq!(users[0].post_id, Some(4));
    }

    #[test]
    fn parse_import_users_excel_rejects_missing_headers() {
        let rows = vec![vec!["demo".to_string()]];
        let bytes = xlsx_from_rows("Sheet1", &["用户名"], &rows).unwrap();

        let err = parse_import_users_excel(&bytes).unwrap_err();

        match err {
            ApiError::BadRequest(message) => assert!(message.contains("Excel 缺少表头")),
            _ => panic!("expected bad request"),
        }
    }

    #[test]
    fn dept_tree_nodes_builds_keystone_dropdown_shape() {
        let now = Utc::now();
        let depts = vec![
            DeptDTO {
                id: 1,
                parent_id: 0,
                dept_name: "总部".into(),
                order_num: 1,
                leader_name: None,
                phone: None,
                email: None,
                status: 1,
                status_str: "正常".into(),
                create_time: now,
            },
            DeptDTO {
                id: 2,
                parent_id: 1,
                dept_name: "研发部".into(),
                order_num: 2,
                leader_name: None,
                phone: None,
                email: None,
                status: 1,
                status_str: "正常".into(),
                create_time: now,
            },
        ];

        let tree = dept_tree_nodes(&depts);

        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].label, "总部");
        assert_eq!(tree[0].children[0].label, "研发部");
        assert_eq!(tree[0].children[0].parent_id, 1);
    }
}
