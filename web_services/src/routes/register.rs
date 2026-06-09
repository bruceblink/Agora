use actix_web::{HttpResponse, post};
use common::api::ApiResponse;

const KEYSTONE_UNSUPPORTED_OPERATION_CODE: i32 = 10002;
const KEYSTONE_UNSUPPORTED_OPERATION_MSG: &str = "不支持的操作";

fn unsupported_register_response() -> ApiResponse<()> {
    ApiResponse {
        code: KEYSTONE_UNSUPPORTED_OPERATION_CODE,
        msg: KEYSTONE_UNSUPPORTED_OPERATION_MSG.into(),
        status: "error".into(),
        message: Some(KEYSTONE_UNSUPPORTED_OPERATION_MSG.into()),
        data: None,
    }
}

#[post("/register")]
pub async fn register() -> HttpResponse {
    HttpResponse::Ok().json(unsupported_register_response())
}

#[cfg(test)]
mod tests {
    use super::{KEYSTONE_UNSUPPORTED_OPERATION_CODE, unsupported_register_response};

    #[test]
    fn register_response_matches_keystone_unsupported_operation() {
        let response = unsupported_register_response();

        assert_eq!(response.code, KEYSTONE_UNSUPPORTED_OPERATION_CODE);
        assert_eq!(response.msg, "不支持的操作");
        assert_eq!(response.status, "error");
        assert_eq!(response.message.as_deref(), Some("不支持的操作"));
        assert!(response.data.is_none());
    }
}
