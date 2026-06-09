use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AniInfoDto {
    pub id: i64,
    pub title: String,
    pub update_count: String,
    pub update_info: String,
    pub image_url: String,
    pub detail_url: String,
    pub update_time: String,
    pub platform: String,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserDto {
    pub id: i64,
    pub email: String,
    pub username: String,
    pub password: String,
    pub display_name: String,
    pub avatar_url: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub tenant_id: String,
    pub org_id: String,
    pub plan: String,
    pub token_version: i64,
    pub status: String,
    pub locked_until: Option<chrono::DateTime<Utc>>,
    pub failed_login_attempts: i64,
}

#[derive(Debug, Clone)]
pub struct NewUser {
    pub email: String,
    pub username: String,
    pub password: String,
    pub display_name: String,
    pub avatar_url: String,
}

/// 用户身份Dto,用于关联第三方登录认证的数据
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserIdentityDto {
    pub provider_user_id: String,
    pub provider: String,
    pub email: Option<String>,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsInfoDTO {
    pub id: i64,
    pub news_from: String,
    pub news_date: chrono::NaiveDate,
    pub data: serde_json::Value,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: Option<chrono::DateTime<Utc>>,
    pub name: String,
    pub extracted: bool,
    pub extracted_at: Option<chrono::DateTime<Utc>>,
}

pub struct NewsItemDTO {
    pub id: String,
    pub title: String,
    pub url: String,
    pub content: serde_json::Value,
    pub source: chrono::DateTime<Utc>,
    pub published_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledTasksDTO {
    pub id: i64,
    pub name: String,
    pub cron: String,
    pub params: serde_json::Value,
    pub is_enabled: bool,
    pub retry_times: u8,
    pub last_run: Option<chrono::NaiveDateTime>,
    pub next_run: Option<chrono::NaiveDateTime>,
    pub last_status: String,
}

/// 创建定时任务的请求体
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateScheduledTaskDTO {
    pub name: String,
    pub cron: String,
    pub params: serde_json::Value,
    #[serde(default = "bool::default")]
    pub is_enabled: bool,
    #[serde(default = "default_retry_times")]
    pub retry_times: u8,
}

/// 更新定时任务的请求体（所有字段均可选）
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateScheduledTaskDTO {
    pub name: Option<String>,
    pub cron: Option<String>,
    pub params: Option<serde_json::Value>,
    pub retry_times: Option<u8>,
}

/// 切换定时任务启停状态的请求体
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToggleScheduledTaskDTO {
    pub is_enabled: bool,
}

fn default_retry_times() -> u8 {
    3
}

/// 收藏番剧的请求体
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAniCollectDTO {
    pub ani_item_id: i64,
    pub ani_title: String,
}

/// 标记番剧观看状态的请求体
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchedAniCollectDTO {
    pub is_watched: bool,
}

/// 番剧收藏 Response DTO
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AniCollectDTO {
    pub id: i64,
    pub ani_item_id: i64,
    pub ani_title: String,
    pub collect_time: chrono::DateTime<chrono::Utc>,
    pub is_watched: bool,
}

/// 新闻条目 Response DTO
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct NewsItemResponseDTO {
    pub id: i64,
    pub item_id: String,
    pub title: String,
    pub url: String,
    pub source: Option<String>,
    pub published_at: chrono::NaiveDate,
    pub cluster_id: Option<i64>,
    pub extracted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// 新闻热点事件 Response DTO
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct NewsEventDTO {
    pub id: i64,
    pub event_date: chrono::NaiveDate,
    pub cluster_id: i64,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub news_count: i32,
    pub score: Option<f32>,
    pub status: i16,
    pub parent_event_id: Option<i64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Keystone-compatible dictionary type response.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct DictTypeDTO {
    pub dict_id: i64,
    pub dict_name: String,
    pub dict_type: String,
    pub status: i16,
    pub remark: Option<String>,
    pub create_time: chrono::DateTime<Utc>,
}

/// Keystone-compatible dictionary data response.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct DictDataDTO {
    pub dict_code: i64,
    pub dict_type: String,
    pub dict_label: String,
    pub dict_value: String,
    pub dict_sort: i32,
    pub is_default: i16,
    pub css_class: Option<String>,
    pub list_class: Option<String>,
    pub status: i16,
    pub remark: Option<String>,
    pub create_time: chrono::DateTime<Utc>,
}

/// Compact dictionary value used by Keystone's /getConfig response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryDataDTO {
    pub label: String,
    pub value: i32,
    pub css_tag: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDTO {
    pub is_captcha_on: bool,
    pub dictionary: BTreeMap<String, Vec<DictionaryDataDTO>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptchaDTO {
    pub is_captcha_on: bool,
    pub captcha_code_key: String,
    pub captcha_code_img: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RsaPublicKeyDTO {
    pub public_key: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDictTypeDTO {
    pub dict_name: String,
    pub dict_type: String,
    pub status: i16,
    pub remark: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDictTypeDTO {
    pub dict_name: String,
    pub dict_type: String,
    pub status: i16,
    pub remark: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDictDataDTO {
    pub dict_type: String,
    pub dict_label: String,
    pub dict_value: String,
    #[serde(default)]
    pub dict_sort: i32,
    #[serde(default)]
    pub is_default: i16,
    pub css_class: Option<String>,
    pub list_class: Option<String>,
    pub status: i16,
    pub remark: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDictDataDTO {
    pub dict_type: String,
    pub dict_label: String,
    pub dict_value: String,
    #[serde(default)]
    pub dict_sort: i32,
    #[serde(default)]
    pub is_default: i16,
    pub css_class: Option<String>,
    pub list_class: Option<String>,
    pub status: i16,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct SystemConfigDTO {
    pub config_id: String,
    pub config_name: String,
    pub config_key: String,
    pub config_value: String,
    pub config_options: Vec<String>,
    pub is_allow_change: i16,
    pub is_allow_change_str: String,
    pub remark: Option<String>,
    pub create_time: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemConfigDetailDTO {
    pub config_id: Option<String>,
    pub config_name: Option<String>,
    pub config_key: Option<String>,
    pub config_value: Option<String>,
    pub config_options: Option<Vec<String>>,
    pub is_allow_change: Option<i16>,
    pub is_allow_change_str: Option<String>,
    pub remark: Option<String>,
    pub create_time: Option<chrono::DateTime<Utc>>,
}

impl SystemConfigDetailDTO {
    pub fn empty() -> Self {
        Self {
            config_id: None,
            config_name: None,
            config_key: None,
            config_value: None,
            config_options: None,
            is_allow_change: None,
            is_allow_change_str: None,
            remark: None,
            create_time: None,
        }
    }
}

impl From<SystemConfigDTO> for SystemConfigDetailDTO {
    fn from(value: SystemConfigDTO) -> Self {
        Self {
            config_id: Some(value.config_id),
            config_name: Some(value.config_name),
            config_key: Some(value.config_key),
            config_value: Some(value.config_value),
            config_options: Some(value.config_options),
            is_allow_change: Some(value.is_allow_change),
            is_allow_change_str: Some(value.is_allow_change_str),
            remark: value.remark,
            create_time: Some(value.create_time),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSystemConfigDTO {
    pub config_value: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct NoticeDTO {
    pub notice_id: String,
    pub notice_title: String,
    pub notice_type: i16,
    pub notice_content: String,
    pub status: i16,
    pub create_time: chrono::DateTime<Utc>,
    pub creator_name: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoticeDTO {
    pub notice_title: String,
    pub notice_type: String,
    pub notice_content: String,
    pub status: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNoticeDTO {
    pub notice_title: String,
    pub notice_type: String,
    pub notice_content: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MenuMetaDTO {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_link: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_parent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_src: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_frame_src_internal: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_icon: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_loading: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden_tag: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_level: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuDTO {
    pub id: i64,
    pub parent_id: i64,
    pub menu_name: String,
    pub router_name: String,
    pub path: String,
    pub rank: Option<i32>,
    pub menu_type: i16,
    pub menu_type_str: Option<String>,
    pub is_button: bool,
    pub status: i16,
    pub status_str: String,
    pub create_time: chrono::DateTime<Utc>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuDetailDTO {
    #[serde(flatten)]
    pub menu: MenuDTO,
    pub permission: String,
    pub meta: MenuMetaDTO,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuDetailResponseDTO {
    pub id: Option<i64>,
    pub parent_id: Option<i64>,
    pub menu_name: Option<String>,
    pub router_name: Option<String>,
    pub path: Option<String>,
    pub rank: Option<i32>,
    pub menu_type: Option<i16>,
    pub menu_type_str: Option<String>,
    pub is_button: Option<bool>,
    pub status: Option<i16>,
    pub status_str: Option<String>,
    pub create_time: Option<chrono::DateTime<Utc>>,
    pub icon: Option<String>,
    pub permission: Option<String>,
    pub meta: Option<MenuMetaDTO>,
}

impl MenuDetailResponseDTO {
    pub fn empty() -> Self {
        Self {
            id: None,
            parent_id: None,
            menu_name: None,
            router_name: None,
            path: None,
            rank: None,
            menu_type: None,
            menu_type_str: None,
            is_button: None,
            status: None,
            status_str: None,
            create_time: None,
            icon: None,
            permission: None,
            meta: None,
        }
    }
}

impl From<MenuDetailDTO> for MenuDetailResponseDTO {
    fn from(value: MenuDetailDTO) -> Self {
        Self {
            id: Some(value.menu.id),
            parent_id: Some(value.menu.parent_id),
            menu_name: Some(value.menu.menu_name),
            router_name: Some(value.menu.router_name),
            path: Some(value.menu.path),
            rank: value.menu.rank,
            menu_type: Some(value.menu.menu_type),
            menu_type_str: value.menu.menu_type_str,
            is_button: Some(value.menu.is_button),
            status: Some(value.menu.status),
            status_str: Some(value.menu.status_str),
            create_time: Some(value.menu.create_time),
            icon: value.menu.icon,
            permission: Some(value.permission),
            meta: Some(value.meta),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuDropdownDTO {
    pub id: i64,
    pub parent_id: i64,
    pub label: String,
    pub children: Vec<MenuDropdownDTO>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMenuDTO {
    pub parent_id: Option<i64>,
    pub menu_name: String,
    pub router_name: Option<String>,
    pub path: Option<String>,
    pub status: Option<i16>,
    pub menu_type: Option<i16>,
    pub is_button: Option<bool>,
    pub permission: Option<String>,
    pub meta: Option<MenuMetaDTO>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMenuDTO {
    pub parent_id: Option<i64>,
    pub menu_name: String,
    pub router_name: Option<String>,
    pub path: Option<String>,
    pub status: Option<i16>,
    pub menu_type: Option<i16>,
    pub is_button: Option<bool>,
    pub permission: Option<String>,
    pub meta: Option<MenuMetaDTO>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleDTO {
    pub role_id: i64,
    pub role_name: String,
    pub role_key: String,
    pub role_sort: i32,
    pub status: i16,
    pub remark: Option<String>,
    pub create_time: chrono::DateTime<Utc>,
    pub data_scope: i16,
    pub selected_menu_list: Vec<i64>,
    pub selected_dept_list: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoleDTO {
    pub role_name: String,
    pub role_key: String,
    pub role_sort: i32,
    pub remark: Option<String>,
    pub data_scope: Option<String>,
    pub status: Option<String>,
    pub menu_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoleDTO {
    pub role_id: i64,
    pub role_name: String,
    pub role_key: String,
    pub role_sort: i32,
    pub remark: Option<String>,
    pub data_scope: Option<String>,
    pub status: Option<String>,
    pub menu_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoleStatusDTO {
    pub status: i16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoleDataScopeDTO {
    pub dept_ids: Vec<i64>,
    pub data_scope: Option<i16>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct DeptDTO {
    pub id: i64,
    pub parent_id: i64,
    pub dept_name: String,
    pub order_num: i32,
    pub leader_name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub status: i16,
    pub status_str: String,
    pub create_time: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDeptDTO {
    pub parent_id: i64,
    pub dept_name: String,
    pub order_num: i32,
    pub leader_name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub status: Option<i16>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDeptDTO {
    pub dept_id: Option<i64>,
    pub id: Option<i64>,
    pub parent_id: i64,
    pub dept_name: String,
    pub order_num: i32,
    pub leader_name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub status: Option<i16>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PostDTO {
    pub post_id: i64,
    pub post_code: String,
    pub post_name: String,
    pub post_sort: i32,
    pub remark: Option<String>,
    pub status: i16,
    pub status_str: String,
    pub create_time: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostResponseDTO {
    pub post_id: Option<i64>,
    pub post_code: Option<String>,
    pub post_name: Option<String>,
    pub post_sort: Option<i32>,
    pub remark: Option<String>,
    pub status: Option<i16>,
    pub status_str: Option<String>,
    pub create_time: Option<chrono::DateTime<Utc>>,
}

impl PostResponseDTO {
    pub fn empty() -> Self {
        Self {
            post_id: None,
            post_code: None,
            post_name: None,
            post_sort: None,
            remark: None,
            status: None,
            status_str: None,
            create_time: None,
        }
    }
}

impl From<PostDTO> for PostResponseDTO {
    fn from(value: PostDTO) -> Self {
        Self {
            post_id: Some(value.post_id),
            post_code: Some(value.post_code),
            post_name: Some(value.post_name),
            post_sort: Some(value.post_sort),
            remark: value.remark,
            status: Some(value.status),
            status_str: Some(value.status_str),
            create_time: Some(value.create_time),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePostDTO {
    pub post_code: String,
    pub post_name: String,
    pub post_sort: i32,
    pub remark: Option<String>,
    pub status: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePostDTO {
    pub post_id: i64,
    pub post_code: String,
    pub post_name: String,
    pub post_sort: i32,
    pub remark: Option<String>,
    pub status: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct SystemUserDTO {
    pub user_id: i64,
    pub post_id: Option<i64>,
    pub post_name: Option<String>,
    pub role_id: Option<i64>,
    pub role_name: Option<String>,
    pub dept_id: Option<i64>,
    pub dept_name: Option<String>,
    pub username: String,
    pub nickname: Option<String>,
    pub user_type: Option<i16>,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub sex: Option<i16>,
    pub avatar: Option<String>,
    pub status: i16,
    pub login_ip: Option<String>,
    pub login_date: Option<chrono::DateTime<Utc>>,
    pub creator_id: Option<i64>,
    pub creator_name: Option<String>,
    pub create_time: chrono::DateTime<Utc>,
    pub updater_id: Option<i64>,
    pub updater_name: Option<String>,
    pub update_time: Option<chrono::DateTime<Utc>>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDetailDTO {
    pub user: Option<SystemUserDTO>,
    pub role_options: Vec<RoleDTO>,
    pub post_options: Vec<PostDTO>,
    pub post_id: Option<i64>,
    pub role_id: Option<i64>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileDTO {
    pub user: SystemUserDTO,
    pub role_name: Option<String>,
    pub post_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentLoginUserDTO {
    pub user_info: SystemUserDTO,
    pub role_key: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenDTO {
    pub token: String,
    pub refresh_token: Option<String>,
    pub expires_in: i64,
    pub refresh_expires_in: Option<i64>,
    pub current_user: Option<CurrentLoginUserDTO>,
    pub keylo_access_token: Option<String>,
    pub keylo_refresh_token: Option<String>,
    pub keylo_expires_in: Option<i64>,
    pub keylo_token_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouterDTO {
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<i32>,
    pub meta: MenuMetaDTO,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<RouterDTO>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileDTO {
    pub sex: Option<i16>,
    #[serde(alias = "nickName")]
    pub nickname: Option<String>,
    pub phone_number: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOwnPasswordDTO {
    pub user_id: Option<i64>,
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadFileDTO {
    pub img_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadDTO {
    pub url: String,
    pub file_name: String,
    pub new_file_name: String,
    pub original_filename: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSystemUserDTO {
    pub dept_id: Option<i64>,
    pub username: String,
    pub nickname: Option<String>,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub sex: Option<i16>,
    pub avatar: Option<String>,
    pub password: String,
    pub status: Option<i16>,
    pub role_id: Option<i64>,
    pub post_id: Option<i64>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSystemUserDTO {
    pub user_id: Option<i64>,
    pub dept_id: Option<i64>,
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub sex: Option<i16>,
    pub avatar: Option<String>,
    pub password: Option<String>,
    pub status: Option<i16>,
    pub role_id: Option<i64>,
    pub post_id: Option<i64>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetUserPasswordDTO {
    pub user_id: Option<i64>,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserStatusDTO {
    pub user_id: Option<i64>,
    pub status: i16,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct LoginLogDTO {
    pub log_id: String,
    pub username: String,
    pub ip_address: String,
    pub login_location: String,
    pub operation_system: String,
    pub browser: String,
    pub status: i16,
    pub status_str: String,
    pub msg: String,
    pub login_time: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct OperationLogDTO {
    pub operation_id: i64,
    pub business_type: i16,
    pub business_type_str: String,
    pub request_method: String,
    pub request_module: String,
    pub request_url: String,
    pub called_method: String,
    pub operator_type: i16,
    pub operator_type_str: String,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub operator_ip: Option<String>,
    pub operator_location: Option<String>,
    pub dept_id: Option<i64>,
    pub dept_name: Option<String>,
    pub operation_param: Option<String>,
    pub operation_result: Option<String>,
    pub status: i16,
    pub status_str: String,
    pub error_stack: Option<String>,
    pub operation_time: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct OnlineUserDTO {
    pub token_id: String,
    pub dept_name: Option<String>,
    pub username: String,
    pub ip_address: String,
    pub login_location: String,
    pub browser: String,
    pub operation_system: String,
    pub login_time: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedisCommandStatusDTO {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedisCacheInfoDTO {
    pub info: HashMap<String, String>,
    pub db_size: i64,
    pub command_stats: Vec<RedisCommandStatusDTO>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfoDTO {
    pub cpu_num: usize,
    pub total: f64,
    pub sys: f64,
    pub used: f64,
    pub wait: f64,
    pub free: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfoDTO {
    pub total: f64,
    pub used: f64,
    pub free: f64,
    pub usage: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JvmInfoDTO {
    pub total: f64,
    pub max: f64,
    pub free: f64,
    pub used: f64,
    pub usage: f64,
    pub name: String,
    pub version: String,
    pub home: String,
    pub start_time: String,
    pub run_time: String,
    pub input_args: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfoDTO {
    pub computer_name: String,
    pub computer_ip: String,
    pub user_dir: String,
    pub os_name: String,
    pub os_arch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfoDTO {
    pub dir_name: String,
    pub sys_type_name: String,
    pub type_name: String,
    pub total: String,
    pub free: String,
    pub used: String,
    pub usage: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfoDTO {
    pub cpu_info: CpuInfoDTO,
    pub memory_info: MemoryInfoDTO,
    pub jvm_info: JvmInfoDTO,
    pub system_info: SystemInfoDTO,
    pub disk_infos: Vec<DiskInfoDTO>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddOperationLogDTO {
    pub business_type: Option<i16>,
    pub request_method: Option<i16>,
    pub request_module: Option<String>,
    pub request_url: Option<String>,
    pub called_method: Option<String>,
    pub operator_type: Option<i16>,
    pub operation_param: Option<String>,
    pub operation_result: Option<String>,
    pub status: Option<i16>,
    pub error_stack: Option<String>,
    pub operation_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct JobDTO {
    pub job_id: i64,
    pub job_name: String,
    pub job_group: String,
    pub invoke_target: String,
    pub cron_expression: String,
    pub concurrent: i16,
    pub concurrent_str: String,
    pub status: i16,
    pub status_str: String,
    pub remark: Option<String>,
    pub create_time: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateJobDTO {
    pub job_name: String,
    pub job_group: String,
    pub invoke_target: String,
    pub cron_expression: String,
    pub concurrent: i16,
    pub status: i16,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateJobDTO {
    pub job_name: String,
    pub job_group: String,
    pub invoke_target: String,
    pub cron_expression: String,
    pub concurrent: i16,
    pub status: i16,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateJobStatusDTO {
    pub status: i16,
}
