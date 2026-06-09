use crate::common::AppState;
use actix_web::{HttpRequest, HttpResponse, delete, get, web};
use chrono::Utc;
use common::OnlineUserQuery;
use common::api::{ApiError, ApiResponse};
use common::dto::{
    CpuInfoDTO, DiskInfoDTO, JvmInfoDTO, MemoryInfoDTO, RedisCacheInfoDTO, RedisCommandStatusDTO,
    ServerInfoDTO, SystemInfoDTO,
};
use common::po::ApiResult;
use infra::{active_refresh_token_count, list_online_users, revoke_online_user};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Instant;
use sysinfo::{Disks, MINIMUM_CPU_UPDATE_INTERVAL, System};

static STARTED_AT: OnceLock<Instant> = OnceLock::new();

fn app_started_at() -> Instant {
    *STARTED_AT.get_or_init(Instant::now)
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn percent(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        round2(numerator as f64 * 100.0 / denominator as f64)
    }
}

fn gb(bytes: u64) -> f64 {
    round2(bytes as f64 / 1024.0 / 1024.0 / 1024.0)
}

fn mb(bytes: u64) -> f64 {
    round2(bytes as f64 / 1024.0 / 1024.0)
}

fn format_file_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let value = bytes as f64;
    if value >= GB {
        format!("{:.1} GB", value / GB)
    } else if value >= MB {
        format!("{:.1} MB", value / MB)
    } else {
        format!("{:.1} KB", value / KB)
    }
}

fn run_time_string(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;
    let seconds = seconds % 60;
    format!("{days}d {hours}h {minutes}m {seconds}s")
}

fn local_ip_address() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .ok()
        .and_then(|socket| {
            socket.connect("8.8.8.8:80").ok()?;
            socket.local_addr().ok()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "127.0.0.1".to_string())
}

fn path_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| path.display().to_string())
}

fn build_disk_infos() -> Vec<DiskInfoDTO> {
    let disks = Disks::new_with_refreshed_list();
    disks
        .iter()
        .map(|disk| {
            let total = disk.total_space();
            let free = disk.available_space();
            let used = total.saturating_sub(free);
            DiskInfoDTO {
                dir_name: disk.mount_point().display().to_string(),
                sys_type_name: disk.file_system().to_string_lossy().to_string(),
                type_name: path_file_name(disk.name().as_ref()),
                total: format_file_size(total),
                free: format_file_size(free),
                used: format_file_size(used),
                usage: percent(used, total),
            }
        })
        .collect()
}

fn build_server_info() -> ServerInfoDTO {
    let mut system = System::new_all();
    system.refresh_all();
    std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
    system.refresh_cpu_usage();

    let cpu_count = system.cpus().len();
    let cpu_used = if cpu_count == 0 {
        0.0
    } else {
        let total_usage: f32 = system.cpus().iter().map(|cpu| cpu.cpu_usage()).sum();
        round2((total_usage / cpu_count as f32) as f64)
    };

    let total_memory = system.total_memory();
    let used_memory = system.used_memory();
    let free_memory = total_memory.saturating_sub(used_memory);

    let process_memory = std::env::current_exe()
        .ok()
        .and_then(|_| sysinfo::get_current_pid().ok())
        .and_then(|pid| system.process(pid).map(|process| process.memory()))
        .unwrap_or(0);

    let started_elapsed = app_started_at().elapsed().as_secs();
    let start_time =
        Utc::now() - chrono::Duration::from_std(app_started_at().elapsed()).unwrap_or_default();

    ServerInfoDTO {
        cpu_info: CpuInfoDTO {
            cpu_num: cpu_count,
            total: 100.0,
            sys: 0.0,
            used: cpu_used,
            wait: 0.0,
            free: round2((100.0 - cpu_used).max(0.0)),
        },
        memory_info: MemoryInfoDTO {
            total: gb(total_memory),
            used: gb(used_memory),
            free: gb(free_memory),
            usage: percent(used_memory, total_memory),
        },
        jvm_info: JvmInfoDTO {
            total: mb(process_memory),
            max: mb(total_memory),
            free: mb(free_memory),
            used: mb(process_memory),
            usage: percent(process_memory, total_memory),
            name: "Rust runtime".to_string(),
            version: format!("rust {}", env!("CARGO_PKG_VERSION")),
            home: std::env::current_exe()
                .ok()
                .and_then(|path| path.parent().map(|parent| parent.display().to_string()))
                .unwrap_or_default(),
            start_time: start_time.format("%Y-%m-%d %H:%M:%S").to_string(),
            run_time: run_time_string(started_elapsed),
            input_args: format!("{:?}", std::env::args().collect::<Vec<_>>()),
        },
        system_info: SystemInfoDTO {
            computer_name: System::host_name().unwrap_or_else(|| "unknown".to_string()),
            computer_ip: local_ip_address(),
            user_dir: std::env::current_dir()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            os_name: System::long_os_version()
                .or_else(System::name)
                .unwrap_or_else(|| std::env::consts::OS.to_string()),
            os_arch: std::env::consts::ARCH.to_string(),
        },
        disk_infos: build_disk_infos(),
    }
}

fn cache_info(active_sessions: i64) -> RedisCacheInfoDTO {
    let uptime = app_started_at().elapsed().as_secs();
    let mut info = HashMap::new();
    info.insert("redis_version".to_string(), "Agora runtime".to_string());
    info.insert("redis_mode".to_string(), "standalone".to_string());
    info.insert("tcp_port".to_string(), "0".to_string());
    info.insert("connected_clients".to_string(), active_sessions.to_string());
    info.insert("uptime_in_days".to_string(), (uptime / 86_400).to_string());
    info.insert("used_memory_human".to_string(), "0 MB".to_string());
    info.insert("used_cpu_user_children".to_string(), "0".to_string());
    info.insert("total_system_memory_human".to_string(), "0 MB".to_string());
    info.insert("aof_enabled".to_string(), "0".to_string());
    info.insert("rdb_last_bgsave_status".to_string(), "ok".to_string());
    info.insert("instantaneous_input_kbps".to_string(), "0".to_string());
    info.insert("instantaneous_output_kbps".to_string(), "0".to_string());

    RedisCacheInfoDTO {
        info,
        db_size: active_sessions,
        command_stats: vec![
            RedisCommandStatusDTO {
                name: "online_sessions".to_string(),
                value: active_sessions.to_string(),
            },
            RedisCommandStatusDTO {
                name: "uptime_seconds".to_string(),
                value: uptime.to_string(),
            },
        ],
    }
}

#[get("/monitor/cacheInfo")]
async fn monitor_cache_info(req: HttpRequest, app_state: web::Data<AppState>) -> ApiResult {
    crate::routes::ensure_admin_access(&req, &app_state).await?;
    match active_refresh_token_count(&app_state.db_pool).await {
        Ok(count) => Ok(HttpResponse::Ok().json(ApiResponse::ok(cache_info(count)))),
        Err(e) => {
            tracing::error!("查询缓存监控信息失败: {e:?}");
            Err(ApiError::Database("查询缓存监控信息失败".into()))
        }
    }
}

#[get("/monitor/serverInfo")]
async fn monitor_server_info(req: HttpRequest, app_state: web::Data<AppState>) -> ApiResult {
    crate::routes::ensure_admin_access(&req, &app_state).await?;
    let server_info = tokio::task::spawn_blocking(build_server_info)
        .await
        .map_err(|e| {
            tracing::error!("采集服务器监控信息失败: {e}");
            ApiError::Internal("采集服务器监控信息失败".into())
        })?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(server_info)))
}

#[get("/monitor/onlineUsers")]
async fn monitor_online_users(
    req: HttpRequest,
    query: web::Query<OnlineUserQuery>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::ensure_admin_access(&req, &app_state).await?;
    match list_online_users(&query.into_inner(), &app_state.db_pool).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::ok(data))),
        Err(e) => {
            tracing::error!("查询在线用户失败: {e:?}");
            Err(ApiError::Database("查询在线用户失败".into()))
        }
    }
}

#[delete("/monitor/onlineUser/{token_id}")]
async fn monitor_online_user_delete(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    crate::routes::ensure_admin_access(&req, &app_state).await?;
    match revoke_online_user(&path.into_inner(), &app_state.db_pool).await {
        Ok(_) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("强退在线用户失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{format_file_size, run_time_string};

    #[test]
    fn format_file_size_matches_keystone_style() {
        assert_eq!(format_file_size(512), "0.5 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1_572_864), "1.5 MB");
        assert_eq!(format_file_size(1_610_612_736), "1.5 GB");
    }

    #[test]
    fn run_time_string_formats_duration_parts() {
        assert_eq!(run_time_string(90_061), "1d 1h 1m 1s");
    }
}
