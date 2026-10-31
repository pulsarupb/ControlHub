use crate::state::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::json;

pub async fn frame(State(state): State<AppState>, Path(kind): Path<String>) -> Response {
    let crosshair = match kind.as_str() {
        "color" => false,
        "crosshair" => true,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "expected color or crosshair"})),
            )
                .into_response();
        }
    };
    if state.simulate {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "SIMULATED · no D435 frames"})),
        )
            .into_response();
    }
    match state.d435.snapshot(crosshair) {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "image/jpeg"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            bytes,
        )
            .into_response(),
        Err(error) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": error})),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn validates_sources_and_reports_unavailable_without_capture() {
        for simulate in [false, true] {
            let (state, _) =
                AppState::new(crate::config::Config::default(), String::new(), simulate);
            let router = crate::api::router(state);
            for (kind, expected) in [
                ("color", StatusCode::SERVICE_UNAVAILABLE),
                ("crosshair", StatusCode::SERVICE_UNAVAILABLE),
                ("depth", StatusCode::BAD_REQUEST),
            ] {
                let response = router
                    .clone()
                    .oneshot(
                        Request::builder()
                            .uri(format!("/api/d435/frame/{kind}"))
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(response.status(), expected);
            }
        }
    }
}
