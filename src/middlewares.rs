use axum::{
    body::Body, extract::Request, http::Response, middleware::Next, response::IntoResponse,
};
use std::time::Instant;

pub async fn track_time(req: Request<Body>, next: Next) -> impl IntoResponse {
    let method = req.method().to_string();
    let path = req.uri().path().to_owned();

    let start = Instant::now();
    let response: Response<Body> = next.run(req).await;
    let elapsed = start.elapsed();
    let status = response.status();

    if status.is_success() {
        tracing::info!("🌐 {method}:{path} - {:?}", elapsed);
    } else {
        tracing::error!("🚨 {method}:{path} - Status: {status} - {:?}", elapsed);
    }
    response
}
