use crate::common::{AppState, ExtractToken, build_captcha_dto};
use actix_web::cookie::{Cookie, SameSite};
use actix_web::{HttpRequest, HttpResponse, get, post, web};
use chrono::Utc;
use common::api::{ApiError, ApiResponse};
use common::dto::{CurrentLoginUserDTO, RsaPublicKeyDTO, TokenDTO};
use common::po::ApiResult;
use common::utils::{CommonUser, JwtClaims, generate_jwt, login_rsa_public_key_base64, verify_jwt};
use common::{ACCESS_TOKEN, REFRESH_TOKEN};
use infra::{get_system_user, is_captcha_on, list_user_router_tree};
use serde::Deserialize;
use sqlx::FromRow;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshTokenRequest {
    pub refresh_token: Option<String>,
}

#[derive(Debug, FromRow)]
struct RefreshTokenRecord {
    user_id: i64,
    session_expires_at: chrono::DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct CurrentRoleRow {
    role_key: String,
    role_name: String,
}

#[derive(Debug, FromRow)]
struct LoginIdentityRow {
    id: i64,
    email: Option<String>,
    username: Option<String>,
    avatar_url: Option<String>,
    token_version: i64,
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

fn access_cookie(app_state: &web::Data<AppState>, token: String) -> Cookie<'static> {
    Cookie::build(ACCESS_TOKEN, token)
        .http_only(true)
        .secure(app_state.configuration.is_production)
        .same_site(SameSite::None)
        .path("/")
        .finish()
}

fn refresh_cookie(app_state: &web::Data<AppState>, token: String) -> Cookie<'static> {
    Cookie::build(REFRESH_TOKEN, token)
        .http_only(true)
        .secure(app_state.configuration.is_production)
        .same_site(SameSite::None)
        .path("/")
        .finish()
}

pub async fn user_role_keys(
    user_id: i64,
    app_state: &web::Data<AppState>,
) -> Result<Vec<String>, ApiError> {
    sqlx::query_scalar(
        r#"
        SELECT COALESCE(NULLIF(r.role_key, ''), r.name)
        FROM roles r
        JOIN user_roles ur ON ur.role_id = r.id
        WHERE ur.user_id = $1
          AND r.status = 1
          AND r.deleted = FALSE
        ORDER BY r.role_sort ASC NULLS LAST, r.id ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(&app_state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("查询用户角色标识失败 user_id={user_id}: {e}");
        ApiError::Internal("服务器内部错误".into())
    })
}

async fn current_role(
    user_id: i64,
    app_state: &web::Data<AppState>,
) -> Result<CurrentRoleRow, ApiError> {
    sqlx::query_as::<_, CurrentRoleRow>(
        r#"
        SELECT COALESCE(NULLIF(r.role_key, ''), r.name) AS role_key,
               COALESCE(NULLIF(r.role_name, ''), r.name) AS role_name
        FROM roles r
        JOIN user_roles ur ON ur.role_id = r.id
        WHERE ur.user_id = $1
          AND r.status = 1
          AND r.deleted = FALSE
        ORDER BY r.role_sort ASC NULLS LAST, r.id ASC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&app_state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("查询当前用户主角色失败 user_id={user_id}: {e}");
        ApiError::Internal("服务器内部错误".into())
    })?
    .ok_or_else(|| ApiError::Forbidden("账号未分配可用角色".into()))
}

pub async fn current_user_dto(
    user_id: i64,
    app_state: &web::Data<AppState>,
) -> Result<CurrentLoginUserDTO, ApiError> {
    let mut user_info = get_system_user(user_id, &app_state.db_pool)
        .await
        .map_err(|e| {
            tracing::error!("查询当前用户信息失败 user_id={user_id}: {e:?}");
            ApiError::Internal("服务器内部错误".into())
        })?;
    let role = current_role(user_id, app_state).await?;
    user_info.role_name = Some(role.role_name.clone());
    let permissions = user_permissions(user_id, app_state).await?;

    Ok(CurrentLoginUserDTO {
        user_info,
        role_key: role.role_key,
        permissions,
    })
}

pub async fn user_permissions(
    user_id: i64,
    app_state: &web::Data<AppState>,
) -> Result<Vec<String>, ApiError> {
    sqlx::query_scalar(
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
    .fetch_all(&app_state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("查询用户菜单权限失败 user_id={user_id}: {e}");
        ApiError::Internal("服务器内部错误".into())
    })
}

pub async fn build_token_dto(
    access_token: String,
    access_expires_at: chrono::DateTime<Utc>,
    refresh_token_value: Option<String>,
    refresh_expires_at: Option<chrono::DateTime<Utc>>,
    user_id: Option<i64>,
    app_state: &web::Data<AppState>,
) -> Result<TokenDTO, ApiError> {
    let now = Utc::now();
    let current_user = match user_id {
        Some(user_id) => Some(current_user_dto(user_id, app_state).await?),
        None => None,
    };

    Ok(TokenDTO {
        token: access_token,
        refresh_token: refresh_token_value,
        expires_in: (access_expires_at - now).num_seconds().max(0),
        refresh_expires_in: refresh_expires_at
            .map(|expires_at| (expires_at - now).num_seconds().max(0)),
        current_user,
        keylo_access_token: None,
        keylo_refresh_token: None,
        keylo_expires_in: None,
        keylo_token_type: None,
    })
}

pub fn claims_from_request(req: &HttpRequest) -> Result<JwtClaims, ApiError> {
    let token = req
        .get_access_token()
        .ok_or_else(|| ApiError::Unauthorized("缺少 access token".into()))?;
    verify_jwt(&token).map_err(|e| {
        tracing::warn!("JWT 校验失败: {e}");
        ApiError::Unauthorized("access token 无效或已过期".into())
    })
}

async fn identity_for_token(
    user_id: i64,
    app_state: &web::Data<AppState>,
) -> Result<LoginIdentityRow, ApiError> {
    sqlx::query_as::<_, LoginIdentityRow>(
        r#"
        SELECT id, email, username, avatar_url, token_version
        FROM user_info
        WHERE id = $1
          AND status = 'active'
          AND COALESCE(user_status, 1) = 1
          AND deleted = FALSE
        "#,
    )
    .bind(user_id)
    .fetch_optional(&app_state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("查询 token 用户失败 user_id={user_id}: {e}");
        ApiError::Internal("服务器内部错误".into())
    })?
    .ok_or_else(|| ApiError::Unauthorized("账号不可用".into()))
}

async fn issue_access_token_for_user(
    user_id: i64,
    app_state: &web::Data<AppState>,
) -> Result<(String, chrono::DateTime<Utc>), ApiError> {
    let user = identity_for_token(user_id, app_state).await?;
    let roles = user_role_keys(user_id, app_state).await?;
    let common_user = CommonUser {
        id: user.id,
        sub: user.username.unwrap_or_default(),
        uid: user.id,
        email: user.email,
        avatar_url: user.avatar_url,
        r#type: "local".to_string(),
        roles,
        ver: user.token_version,
    };
    let access_token_minutes = token_window_days(app_state, ACCESS_TOKEN)?;
    let access_token = generate_jwt(&common_user, access_token_minutes).map_err(|e| {
        tracing::error!("access token 生成失败 user_id={user_id}: {e}");
        ApiError::Internal("access token 生成失败".into())
    })?;
    Ok((access_token.token, access_token.expires_at))
}

#[get("/captchaImage")]
async fn captcha_image(app_state: web::Data<AppState>) -> ApiResult {
    let is_captcha_on = is_captcha_on(&app_state.db_pool).await.map_err(|e| {
        tracing::error!("读取验证码开关失败: {e}");
        ApiError::Internal("读取验证码开关失败".into())
    })?;
    let captcha = build_captcha_dto(is_captcha_on, &app_state.captcha_store).map_err(|e| {
        tracing::error!("生成验证码失败: {e}");
        ApiError::Internal("验证码生成失败".into())
    })?;

    Ok(HttpResponse::Ok().json(ApiResponse::ok(captcha)))
}

#[get("/login/rsa-public-key")]
async fn login_rsa_public_key() -> ApiResult {
    let public_key = login_rsa_public_key_base64().map_err(|e| {
        tracing::error!("获取登录 RSA 公钥失败: {e}");
        ApiError::Internal("获取登录 RSA 公钥失败".into())
    })?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(RsaPublicKeyDTO { public_key })))
}

#[get("/getLoginUserInfo")]
async fn get_login_user_info(req: HttpRequest, app_state: web::Data<AppState>) -> ApiResult {
    let claims = claims_from_request(&req)?;
    let current_user = current_user_dto(claims.uid, &app_state).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(current_user)))
}

#[get("/getRouters")]
async fn get_routers(req: HttpRequest, app_state: web::Data<AppState>) -> ApiResult {
    let claims = claims_from_request(&req)?;
    let routers = list_user_router_tree(claims.uid, &app_state.db_pool)
        .await
        .map_err(|e| {
            tracing::error!("查询用户路由失败 user_id={}: {e:?}", claims.uid);
            ApiError::Internal("查询用户路由失败".into())
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(routers)))
}

#[post("/refresh-token")]
async fn refresh_token_compat(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    body: Option<web::Json<RefreshTokenRequest>>,
) -> ApiResult {
    let old_refresh_token = body
        .as_ref()
        .and_then(|body| {
            body.refresh_token
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(str::trim)
                .map(str::to_string)
        })
        .or_else(|| req.get_refresh_token())
        .ok_or_else(|| ApiError::Unauthorized("缺少 refresh token".into()))?;

    let mut tx = app_state.db_pool.begin().await.map_err(|e| {
        tracing::error!("begin tx failed: {e}");
        ApiError::Internal("服务器错误".into())
    })?;

    let rec = sqlx::query_as::<_, RefreshTokenRecord>(
        r#"
        SELECT user_id, session_expires_at
        FROM refresh_tokens
        WHERE token = $1
          AND revoked = FALSE
          AND expires_at > now()
          AND session_expires_at > now()
        FOR UPDATE
        "#,
    )
    .bind(&old_refresh_token)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("query refresh_token failed: {e}");
        ApiError::Internal("服务器错误".into())
    })?;

    let rec = match rec {
        Some(rec) => rec,
        None => {
            tx.rollback().await.ok();
            return Err(ApiError::Unauthorized("refresh token 无效或已过期".into()));
        }
    };

    sqlx::query("UPDATE refresh_tokens SET revoked = TRUE WHERE token = $1")
        .bind(&old_refresh_token)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("revoke refresh_token failed: {e}");
            ApiError::Internal("服务器错误".into())
        })?;

    let refresh_window_days = token_window_days(&app_state, REFRESH_TOKEN)?;
    let mut new_refresh_token = common::utils::generate_refresh_token(refresh_window_days)
        .map_err(|e| {
            tracing::error!("refresh_token 生成失败: {e}");
            ApiError::Internal("refresh_token 生成失败".into())
        })?;
    if new_refresh_token.expires_at > rec.session_expires_at {
        new_refresh_token.expires_at = rec.session_expires_at;
    }

    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (user_id, token, expires_at, session_expires_at)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(rec.user_id)
    .bind(&new_refresh_token.token)
    .bind(new_refresh_token.expires_at)
    .bind(rec.session_expires_at)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("insert refresh_token failed: {e}");
        ApiError::Internal("服务器错误".into())
    })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("commit tx failed: {e}");
        ApiError::Internal("服务器错误".into())
    })?;

    let (access_token, access_expires_at) =
        issue_access_token_for_user(rec.user_id, &app_state).await?;
    let token_dto = build_token_dto(
        access_token.clone(),
        access_expires_at,
        Some(new_refresh_token.token.clone()),
        Some(new_refresh_token.expires_at),
        None,
        &app_state,
    )
    .await?;

    Ok(HttpResponse::Ok()
        .cookie(access_cookie(&app_state, access_token))
        .cookie(refresh_cookie(&app_state, new_refresh_token.token))
        .json(ApiResponse::ok(token_dto)))
}

#[post("/logout-refresh-token")]
async fn logout_refresh_token(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    body: Option<web::Json<RefreshTokenRequest>>,
) -> ApiResult {
    let refresh_token_value = body
        .as_ref()
        .and_then(|body| {
            body.refresh_token
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(str::trim)
                .map(str::to_string)
        })
        .or_else(|| req.get_refresh_token())
        .ok_or_else(|| ApiError::Unauthorized("缺少 refresh token".into()))?;

    sqlx::query("UPDATE refresh_tokens SET revoked = TRUE WHERE token = $1")
        .bind(refresh_token_value)
        .execute(&app_state.db_pool)
        .await
        .map_err(|e| {
            tracing::error!("注销 refresh_token 失败: {e}");
            ApiError::Internal("服务器错误".into())
        })?;

    Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(())))
}
