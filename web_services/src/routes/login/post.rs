use crate::common::{AppState, ExtractToken};
use crate::routes::{build_token_dto, user_role_keys};
use actix_web::cookie::{Cookie, SameSite};
use actix_web::{HttpRequest, HttpResponse, Responder, post, web};
use chrono::Utc;
use common::api::{ApiError, ApiResponse};
use common::utils::{
    CommonUser, decrypt_login_password_or_plain, generate_jwt, generate_refresh_token, verify_jwt,
};
use common::{ACCESS_TOKEN, REFRESH_TOKEN};
use infra::{
    browser_from_user_agent, build_login_log, build_session_login_info,
    operation_system_from_user_agent, private_ip_location, record_login_info,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

fn keystone_business_response(code: i32, msg: &str) -> HttpResponse {
    HttpResponse::Ok().json(ApiResponse::<()> {
        code,
        msg: msg.to_string(),
        status: "error".into(),
        message: Some(msg.to_string()),
        data: None,
    })
}

fn token_window_days(
    app_state: &web::Data<AppState>,
    token_key: &'static str,
) -> Result<i64, ApiError> {
    app_state
        .configuration
        .token
        .get(token_key)
        .copied()
        .ok_or_else(|| {
            tracing::error!("token 配置缺失: {token_key}");
            ApiError::Internal("token 配置缺失".into())
        })
}

fn request_ip(req: &HttpRequest) -> String {
    req.connection_info()
        .realip_remote_addr()
        .unwrap_or_default()
        .to_string()
}

fn user_agent(req: &HttpRequest) -> String {
    req.headers()
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string()
}

fn login_identity(req: &LoginRequest) -> String {
    req.username
        .as_deref()
        .or(req.email.as_deref())
        .unwrap_or_default()
        .trim()
        .to_string()
}

async fn record_login_attempt(
    app_state: &web::Data<AppState>,
    http_req: &HttpRequest,
    username: &str,
    status: i16,
    msg: &str,
) {
    let log = build_login_log(
        username,
        &request_ip(http_req),
        &user_agent(http_req),
        status,
        msg,
    );
    if let Err(e) = record_login_info(&log, &app_state.db_pool).await {
        tracing::warn!("写入登录日志失败: {e}");
    }
}

async fn validate_login_captcha_if_enabled(
    app_state: &web::Data<AppState>,
    http_req: &HttpRequest,
    req: &LoginRequest,
) -> Result<Option<HttpResponse>, ApiError> {
    let is_captcha_on = infra::is_captcha_on(&app_state.db_pool)
        .await
        .map_err(|e| {
            tracing::error!("读取验证码开关失败: {e}");
            ApiError::Internal("读取验证码开关失败".into())
        })?;
    if !is_captcha_on {
        return Ok(None);
    }

    match app_state
        .captcha_store
        .validate(req.captcha_code_key.as_deref(), req.captcha_code.as_deref())
    {
        Ok(()) => Ok(None),
        Err(error) => {
            record_login_attempt(
                app_state,
                http_req,
                &login_identity(req),
                0,
                error.message(),
            )
            .await;
            Ok(Some(keystone_business_response(
                error.code(),
                error.message(),
            )))
        }
    }
}

fn session_login_info(http_req: &HttpRequest) -> infra::SessionLoginInfo {
    let ip_address = request_ip(http_req);
    let user_agent = user_agent(http_req);
    build_session_login_info(
        &ip_address,
        &private_ip_location(&ip_address),
        &browser_from_user_agent(&user_agent),
        &operation_system_from_user_agent(&user_agent),
    )
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: String,
    #[serde(default)]
    pub captcha_code: Option<String>,
    #[serde(default)]
    pub captcha_code_key: Option<String>,
    #[serde(default)]
    pub force_login: Option<bool>,
}

#[derive(Debug, Serialize, FromRow)]
struct LoginUser {
    id: i64,
    email: String,
    username: String,
    password: String,
    avatar_url: Option<String>,
    token_version: i64,
    status: String,
}

#[post("/login")]
async fn login(
    app_state: web::Data<AppState>,
    http_req: HttpRequest,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, ApiError> {
    let req = body.into_inner();
    if let Some(response) = validate_login_captcha_if_enabled(&app_state, &http_req, &req).await? {
        return Ok(response);
    }

    let password = decrypt_login_password_or_plain(&req.password);
    if password.len() < 8 {
        record_login_attempt(
            &app_state,
            &http_req,
            &login_identity(&req),
            0,
            "用户名/邮箱或密码错误",
        )
        .await;
        return Err(ApiError::InvalidData("用户名/邮箱或密码错误".into()));
    }

    let username = req
        .username
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let email = req
        .email
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    if username.is_none() && email.is_none() {
        record_login_attempt(&app_state, &http_req, "", 0, "请提供 username 或 email").await;
        return Err(ApiError::InvalidData("请提供 username 或 email".into()));
    }

    let attempted_username = login_identity(&req);
    let user = match sqlx::query_as::<_, LoginUser>(
        r#"
            SELECT id, email, username, password, avatar_url, token_version, status
            FROM user_info
            WHERE ($1::text IS NULL OR username = $1)
              AND ($2::text IS NULL OR email = $2)
            ORDER BY id DESC
            LIMIT 1
        "#,
    )
    .bind(username)
    .bind(email)
    .fetch_optional(&app_state.db_pool)
    .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            record_login_attempt(
                &app_state,
                &http_req,
                &attempted_username,
                0,
                "用户名/邮箱或密码错误",
            )
            .await;
            return Err(ApiError::Unauthorized("用户名/邮箱或密码错误".into()));
        }
        Err(e) => {
            tracing::error!("查询登录用户失败: {e}");
            return Err(ApiError::Internal("服务器内部错误".into()));
        }
    };

    if user.status != "active" {
        record_login_attempt(&app_state, &http_req, &user.username, 0, "账号不可用").await;
        return Err(ApiError::Forbidden("账号不可用".into()));
    }

    let verify_ok = bcrypt::verify(&password, &user.password).map_err(|e| {
        tracing::error!("校验密码失败: {e}");
        ApiError::Internal("服务器内部错误".into())
    })?;

    if !verify_ok {
        record_login_attempt(
            &app_state,
            &http_req,
            &user.username,
            0,
            "用户名/邮箱或密码错误",
        )
        .await;
        return Err(ApiError::Unauthorized("用户名/邮箱或密码错误".into()));
    }

    let roles = user_role_keys(user.id, &app_state).await?;

    let access_token_mins = token_window_days(&app_state, ACCESS_TOKEN)?;
    let refresh_token_days = token_window_days(&app_state, REFRESH_TOKEN)?;

    let common_user = CommonUser {
        id: user.id,
        sub: user.username.clone(),
        uid: user.id,
        email: Some(user.email.clone()),
        avatar_url: user.avatar_url,
        r#type: "local".to_string(),
        roles,
        ver: user.token_version,
    };

    let access_token = generate_jwt(&common_user, access_token_mins).map_err(|e| {
        tracing::error!("access token 生成失败: {e}");
        ApiError::Internal("access token 生成失败".into())
    })?;

    let refresh_token = generate_refresh_token(refresh_token_days).map_err(|e| {
        tracing::error!("refresh token 生成失败: {e}");
        ApiError::Internal("refresh token 生成失败".into())
    })?;

    let session_expires_at = Utc::now() + chrono::Duration::days(30);

    let login_info = session_login_info(&http_req);

    sqlx::query(
        r#"
            INSERT INTO refresh_tokens (
                user_id, token, expires_at, session_expires_at,
                login_ip, login_location, browser, operation_system
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(user.id)
    .bind(&refresh_token.token)
    .bind(refresh_token.expires_at)
    .bind(session_expires_at)
    .bind(&login_info.ip_address)
    .bind(&login_info.login_location)
    .bind(&login_info.browser)
    .bind(&login_info.operation_system)
    .execute(&app_state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("持久化 refresh_token 失败: {e}");
        ApiError::Internal("服务器内部错误".into())
    })?;

    record_login_attempt(&app_state, &http_req, &user.username, 1, "登录成功").await;

    let is_prod = app_state.configuration.is_production;
    let access_cookie = Cookie::build(ACCESS_TOKEN, access_token.token.clone())
        .http_only(true)
        .secure(is_prod)
        .same_site(SameSite::None)
        .path("/")
        .finish();

    let refresh_cookie = Cookie::build(REFRESH_TOKEN, refresh_token.token.clone())
        .http_only(true)
        .secure(is_prod)
        .same_site(SameSite::None)
        .path("/")
        .finish();

    let token_dto = build_token_dto(
        access_token.token.clone(),
        access_token.expires_at,
        Some(refresh_token.token),
        Some(refresh_token.expires_at),
        Some(user.id),
        &app_state,
    )
    .await?;

    Ok(HttpResponse::Ok()
        .cookie(access_cookie)
        .cookie(refresh_cookie)
        .json(ApiResponse::ok(token_dto)))
}

#[post("/login/keylo")]
async fn keylo_login_compat() -> Result<HttpResponse, ApiError> {
    Err(ApiError::BadRequest(
        "Keylo token 登录未启用，请使用 /login".into(),
    ))
}

#[post("/logout")]
async fn logout(app_state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let username = req
        .get_access_token()
        .and_then(|token| verify_jwt(&token).ok())
        .map(|claims| claims.sub)
        .unwrap_or_default();

    if let Some(refresh_token) = req.get_refresh_token() {
        if let Err(e) = sqlx::query(
            r#"
                UPDATE refresh_tokens SET revoked = true WHERE token = $1;
            "#,
        )
        .bind(refresh_token)
        .execute(&app_state.db_pool)
        .await
        {
            tracing::error!("注销 refresh_token 失败: {e}");
        }
    } else {
        tracing::warn!("用户登出时未携带 refresh_token");
    }

    let is_prod = app_state.configuration.is_production;
    let expired_cookie = |name: String| {
        Cookie::build(name, "")
            .path("/")
            .http_only(true)
            .secure(is_prod)
            .same_site(SameSite::None)
            .expires(time::OffsetDateTime::now_utc() - time::Duration::seconds(1))
            .finish()
    };

    let access_cookie = expired_cookie("access_token".to_string());
    let refresh_cookie = expired_cookie("refresh_token".to_string());

    record_login_attempt(&app_state, &req, &username, 2, "退出成功").await;

    HttpResponse::Ok()
        .cookie(access_cookie)
        .cookie(refresh_cookie)
        .json(ApiResponse::<()>::ok(()))
}

#[cfg(test)]
mod tests {
    use super::keystone_business_response;
    use actix_web::body::to_bytes;
    use serde_json::Value;

    #[actix_web::test]
    async fn captcha_business_error_response_matches_keystone_shape() {
        let response = keystone_business_response(10203, "验证码错误");
        let body = to_bytes(response.into_body()).await.unwrap();
        let data: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(data["code"], 10203);
        assert_eq!(data["msg"], "验证码错误");
        assert_eq!(data["status"], "error");
        assert_eq!(data["message"], "验证码错误");
        assert!(data.get("data").is_none());
    }
}
