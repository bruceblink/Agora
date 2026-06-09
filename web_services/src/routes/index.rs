use actix_web::http::header;
use actix_web::{HttpResponse, get};
use common::api::ApiResponse;
use common::po::ApiResult;

const DEFAULT_KEYSTONE_FRONTEND_URL: &str = "http://localhost:80";

fn frontend_url() -> String {
    std::env::var("KEYSTONE_FRONTEND_URL")
        .ok()
        .or_else(|| std::env::var("FRONTEND_URL").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_KEYSTONE_FRONTEND_URL.to_string())
}

#[get("/")]
async fn index() -> ApiResult {
    Ok(HttpResponse::Found()
        .insert_header((header::LOCATION, frontend_url()))
        .finish())
}

#[get("/health")]
async fn health() -> ApiResult {
    Ok(HttpResponse::Ok().json(ApiResponse::ok("is alive")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, body::to_bytes, http::StatusCode, test};
    use serde_json::Value;

    #[actix_web::test]
    async fn index_redirects_to_configured_frontend_url() {
        unsafe {
            std::env::set_var("KEYSTONE_FRONTEND_URL", "http://frontend.example");
            std::env::remove_var("FRONTEND_URL");
        }
        assert_eq!(frontend_url(), "http://frontend.example");

        let app = test::init_service(App::new().service(index)).await;
        let req = test::TestRequest::get().uri("/").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::FOUND);
        assert_eq!(
            resp.headers()
                .get(header::LOCATION)
                .and_then(|v| v.to_str().ok()),
            Some("http://frontend.example")
        );

        unsafe {
            std::env::remove_var("KEYSTONE_FRONTEND_URL");
            std::env::set_var("FRONTEND_URL", "http://legacy.example");
        }
        assert_eq!(frontend_url(), "http://legacy.example");

        unsafe {
            std::env::remove_var("FRONTEND_URL");
            std::env::remove_var("KEYSTONE_FRONTEND_URL");
        }
        assert_eq!(frontend_url(), DEFAULT_KEYSTONE_FRONTEND_URL);
    }

    #[actix_web::test]
    async fn health_returns_ok_status() {
        let app = test::init_service(App::new().service(health)).await;
        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);

        let body = to_bytes(resp.into_body()).await.unwrap();
        let data: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(data["code"], 0);
        assert_eq!(data["msg"], "操作成功");
        assert_eq!(data["status"], "ok");
        assert_eq!(data["data"], "is alive");
    }
}
