use axum::{
    Json, Router,
    extract::{Path, State},
    http::{StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;

#[derive(Clone)]
struct AppState {
    initialized: bool,
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

#[derive(Serialize)]
struct Problem {
    r#type: &'static str,
    title: &'static str,
    status: u16,
    code: &'static str,
    detail: String,
    retryable: bool,
}

/// Builds the initial, dependency-free HTTP API router.
///
/// Readiness reports that the process has initialized. It does not check
/// PostgreSQL, object storage, identity, or any other external dependency.
pub fn app() -> Router {
    router(AppState { initialized: true })
}

fn router(state: AppState) -> Router {
    Router::new()
        .route("/health/live", get(liveness))
        .route("/health/ready", get(readiness))
        .route("/v1/operations/{operation_id}", get(operation_not_found))
        .with_state(state)
}

async fn liveness() -> Json<Health> {
    Json(Health { status: "alive" })
}

async fn readiness(State(state): State<AppState>) -> Response {
    if state.initialized {
        (StatusCode::OK, Json(Health { status: "ready" })).into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            [(CONTENT_TYPE, "application/problem+json")],
            Json(Problem {
                r#type: "about:blank",
                title: "Service not ready",
                status: StatusCode::SERVICE_UNAVAILABLE.as_u16(),
                code: "service_not_ready",
                detail: "The service has not completed initialization.".to_owned(),
                retryable: true,
            }),
        )
            .into_response()
    }
}

async fn operation_not_found(Path(operation_id): Path<String>) -> Response {
    (
        StatusCode::NOT_FOUND,
        [(CONTENT_TYPE, "application/problem+json")],
        Json(Problem {
            r#type: "about:blank",
            title: "Operation not found",
            status: StatusCode::NOT_FOUND.as_u16(),
            code: "operation_not_found",
            detail: format!("No operation exists with id `{operation_id}`."),
            retryable: false,
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn readiness_returns_unavailable_before_initialization()
    -> Result<(), Box<dyn std::error::Error>> {
        let response = router(AppState { initialized: false })
            .oneshot(
                Request::builder()
                    .uri("/health/ready")
                    .body(Body::empty())?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.headers()[CONTENT_TYPE], "application/problem+json");
        Ok(())
    }

    #[tokio::test]
    async fn unknown_operation_returns_typed_not_found_problem()
    -> Result<(), Box<dyn std::error::Error>> {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/v1/operations/op-missing")
                    .body(Body::empty())?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response.headers()[CONTENT_TYPE], "application/problem+json");
        let body = to_bytes(response.into_body(), usize::MAX).await?;
        let problem: serde_json::Value = serde_json::from_slice(&body)?;
        assert_eq!(problem["status"], StatusCode::NOT_FOUND.as_u16());
        assert_eq!(problem["code"], "operation_not_found");
        assert_eq!(problem["retryable"], false);
        Ok(())
    }
}
