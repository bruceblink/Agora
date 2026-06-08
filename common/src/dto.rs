use chrono::Utc;
use serde::Serialize;
use std::collections::BTreeMap;

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
