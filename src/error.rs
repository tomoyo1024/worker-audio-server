use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

#[derive(Debug)]
pub struct AppError(pub String);

impl From<worker::Error> for AppError {
    fn from(err: worker::Error) -> Self {
        AppError(err.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = format!(
            r#"{{\"error\":{}}}"#,
            serde_json::to_string(&self.0).unwrap_or_else(|_| "\"\"".into())
        );

        let mut headers = HeaderMap::new();
        headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());

        (StatusCode::INTERNAL_SERVER_ERROR, headers, body).into_response()
    }
}
