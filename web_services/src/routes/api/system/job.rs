use crate::common::AppState;
use actix_web::{HttpRequest, HttpResponse, delete, get, post, put, web};
use common::JobQuery;
use common::api::{ApiError, ApiResponse};
use common::dto::{CreateJobDTO, UpdateJobDTO, UpdateJobStatusDTO};
use common::po::ApiResult;
use infra::{
    create_job, delete_jobs, extract_invoke_target_method, get_job, list_jobs, parse_id_list,
    update_job, update_job_status,
};
use serde::Deserialize;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteJobsQuery {
    #[serde(default)]
    job_ids: String,
}

fn validate_job_id(job_id: i64) -> Result<(), ApiError> {
    if job_id <= 0 {
        return Err(ApiError::BadRequest("jobId 必须为正整数".into()));
    }
    Ok(())
}

fn validate_job_ids(job_ids: &[i64]) -> Result<(), ApiError> {
    if job_ids.is_empty() {
        return Err(ApiError::BadRequest("jobIds 不能为空".into()));
    }
    if job_ids.iter().any(|id| *id <= 0) {
        return Err(ApiError::BadRequest("jobIds 必须为正整数".into()));
    }
    Ok(())
}

fn validate_cron_expression(cron_expression: &str) -> Result<(), ApiError> {
    cron::Schedule::from_str(cron_expression)
        .map(|_| ())
        .map_err(|_| ApiError::BadRequest("Cron 表达式无效".into()))
}

fn validate_invoke_target(
    invoke_target: &str,
    app_state: &web::Data<AppState>,
) -> Result<(), ApiError> {
    let method = extract_invoke_target_method(invoke_target)
        .map_err(|_| ApiError::BadRequest("调用目标格式必须为 bean.method()".into()))?;
    if !app_state.task_manager.has_task_command(&method) {
        return Err(ApiError::BadRequest(format!(
            "调用目标方法 {method} 未注册"
        )));
    }
    Ok(())
}

fn validate_create_job(
    data: &CreateJobDTO,
    app_state: &web::Data<AppState>,
) -> Result<(), ApiError> {
    validate_cron_expression(&data.cron_expression)?;
    validate_invoke_target(&data.invoke_target, app_state)
}

fn validate_update_job(
    data: &UpdateJobDTO,
    app_state: &web::Data<AppState>,
) -> Result<(), ApiError> {
    validate_cron_expression(&data.cron_expression)?;
    validate_invoke_target(&data.invoke_target, app_state)
}

#[get("/system/jobs")]
async fn jobs_list(
    req: HttpRequest,
    query: web::Query<JobQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    match list_jobs(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询定时任务列表失败: {e:?}");
            Err(ApiError::Database("查询定时任务列表失败".into()))
        }
    }
}

#[get("/system/jobs/{job_id}")]
async fn job_get(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let job_id = path.into_inner();
    validate_job_id(job_id)?;

    match get_job(job_id, &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询定时任务 {job_id} 失败: {e:?}");
            Err(ApiError::NotFound(format!("定时任务 {job_id} 不存在")))
        }
    }
}

#[post("/system/jobs")]
async fn job_create(
    req: HttpRequest,
    body: web::Json<CreateJobDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let data = body.into_inner();
    validate_create_job(&data, &app_state)?;

    match create_job(&data, &app_state.db_pool).await {
        Ok(()) => {
            app_state.task_manager.refresh_config().await.map_err(|e| {
                tracing::error!("定时任务调度器刷新失败: {e:?}");
                ApiError::Internal("定时任务配置刷新失败".into())
            })?;
            Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(())))
        }
        Err(e) => {
            tracing::error!("新增定时任务失败: {e:?}");
            Err(ApiError::BadRequest("新增定时任务失败".into()))
        }
    }
}

#[put("/system/jobs/{job_id}")]
async fn job_update(
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateJobDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let job_id = path.into_inner();
    validate_job_id(job_id)?;
    let data = body.into_inner();
    validate_update_job(&data, &app_state)?;

    match update_job(job_id, &data, &app_state.db_pool).await {
        Ok(()) => {
            app_state.task_manager.refresh_config().await.map_err(|e| {
                tracing::error!("定时任务调度器刷新失败: {e:?}");
                ApiError::Internal("定时任务配置刷新失败".into())
            })?;
            Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(())))
        }
        Err(e) => {
            tracing::error!("更新定时任务 {job_id} 失败: {e:?}");
            Err(ApiError::BadRequest(format!("更新定时任务 {job_id} 失败")))
        }
    }
}

#[put("/system/jobs/{job_id}/status")]
async fn job_status_update(
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateJobStatusDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let job_id = path.into_inner();
    validate_job_id(job_id)?;
    let data = body.into_inner();

    match update_job_status(job_id, &data, &app_state.db_pool).await {
        Ok(()) => {
            app_state.task_manager.refresh_config().await.map_err(|e| {
                tracing::error!("定时任务调度器刷新失败: {e:?}");
                ApiError::Internal("定时任务配置刷新失败".into())
            })?;
            Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(())))
        }
        Err(e) => {
            tracing::error!("更新定时任务 {job_id} 状态失败: {e:?}");
            Err(ApiError::BadRequest(format!(
                "更新定时任务 {job_id} 状态失败"
            )))
        }
    }
}

#[post("/system/jobs/{job_id}/run")]
async fn job_run(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let job_id = path.into_inner();
    validate_job_id(job_id)?;

    match app_state.task_manager.run_task_once_by_id(job_id).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("执行定时任务 {job_id} 失败: {e:?}");
            Err(ApiError::BadRequest(format!("执行定时任务 {job_id} 失败")))
        }
    }
}

#[delete("/system/jobs")]
async fn jobs_delete(
    req: HttpRequest,
    query: web::Query<DeleteJobsQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::api::scheduled_tasks::ensure_admin_access(&req, &app_state).await?;
    let job_ids = parse_id_list(&query.into_inner().job_ids, "jobIds")
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    validate_job_ids(&job_ids)?;

    match delete_jobs(&job_ids, &app_state.db_pool).await {
        Ok(_) => {
            app_state.task_manager.refresh_config().await.map_err(|e| {
                tracing::error!("定时任务调度器刷新失败: {e:?}");
                ApiError::Internal("定时任务配置刷新失败".into())
            })?;
            Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(())))
        }
        Err(e) => {
            tracing::error!("删除定时任务失败: {e:?}");
            Err(ApiError::BadRequest("删除定时任务失败".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DeleteJobsQuery, validate_cron_expression, validate_job_id, validate_job_ids};

    #[test]
    fn delete_jobs_query_accepts_comma_separated_ids() {
        let query = actix_web::web::Query::<DeleteJobsQuery>::from_query("jobIds=1,2")
            .expect("job query parses");

        assert_eq!(query.into_inner().job_ids, "1,2");
    }

    #[test]
    fn validate_job_id_rejects_non_positive_ids() {
        assert!(validate_job_id(0).is_err());
        assert!(validate_job_id(-1).is_err());
        assert!(validate_job_id(1).is_ok());
    }

    #[test]
    fn validate_job_ids_rejects_empty_or_invalid_ids() {
        assert!(validate_job_ids(&[]).is_err());
        assert!(validate_job_ids(&[1, 0]).is_err());
        assert!(validate_job_ids(&[1, 2]).is_ok());
    }

    #[test]
    fn validate_cron_expression_accepts_valid_cron() {
        assert!(validate_cron_expression("0 */10 * * * *").is_ok());
        assert!(validate_cron_expression("invalid").is_err());
    }
}
