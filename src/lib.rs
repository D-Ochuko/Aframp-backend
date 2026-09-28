use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

pub fn router(state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/", axum::routing::get(|| async { "aframp" }))
        .route(
            "/health",
            axum::routing::get(|| async { axum::http::StatusCode::NO_CONTENT }),
        )
        .route("/signup", axum::routing::post(api::auth::signup))
        .route("/login", axum::routing::post(api::auth::login))
        .route("/verify-otp", axum::routing::post(api::auth::verify_otp))
        .route("/logout", axum::routing::post(api::auth::logout))
        .route("/webhooks/termii", axum::routing::post(api::webhooks::termii))
        .route("/me", axum::routing::get(api::me::get))
        .route("/wallet/create", axum::routing::post(api::wallets::create))
        .route("/wallet", axum::routing::get(api::wallets::get))
        .route("/balance", axum::routing::get(api::balances::get))
        .route("/transactions", axum::routing::get(api::transactions::list))
        .route("/withdraw", axum::routing::post(api::withdrawals::create))
        .route("/withdrawals", axum::routing::get(api::withdrawals::list))
        .route(
            "/payment-requests",
            axum::routing::post(api::payment_requests::create)
                .get(api::payment_requests::list),
        )
        .route(
            "/payment-requests/{id}/status",
            axum::routing::get(api::payment_requests::status),
        )
        .route(
            "/payment-requests/{id}",
            axum::routing::get(api::payment_requests::get),
        )
        .route("/admin", axum::routing::get(api::admin::dashboard))
        .route("/admin/overview", axum::routing::get(api::admin::overview))
        .route("/admin/merchants", axum::routing::get(api::admin::merchants))
        .route("/admin/users", axum::routing::get(api::admin::users))
        .route("/admin/wallets", axum::routing::get(api::admin::wallets))
        .route("/admin/transactions", axum::routing::get(api::admin::transactions))
        .route("/admin/withdrawals", axum::routing::get(api::admin::withdrawals))
        .route(
            "/admin/payment-requests",
            axum::routing::get(api::admin::payment_requests),
        )
        .route(
            "/admin/merchants/{id}/suspend",
            axum::routing::post(api::admin::suspend_merchant),
        )
        .route(
            "/admin/merchants/{id}/unsuspend",
            axum::routing::post(api::admin::unsuspend_merchant),
        )
        .route(
            "/admin/users/{id}/unlock",
            axum::routing::post(api::admin::unlock_user),
        )
        .with_state(state)
        .layer(axum::middleware::from_fn(middleware::require_json_content_type))
pub fn app() -> Router {
    Router::new().route("/health", get(health))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_returns_200_with_json_body() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(value["status"], "ok");
        assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
    }
}

#[cfg(test)]
mod cors_tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Method, Request, StatusCode};
    use tower::ServiceExt;

    fn test_state() -> AppState {
        let db = PgPoolOptions::new()
            .connect_lazy("postgres://user:pass@localhost:5432/aframp")
            .expect("lazy pool");
        AppState {
            db,
            jwt_secret: SecretString::new("test-jwt-secret".to_string()),
            webhook_secret: SecretString::new("test-webhook-secret".to_string()),
            wallet_encryption_key: std::sync::Arc::new([0u8; 32]),
            payment_provider: std::sync::Arc::new(payments::paystack::PaystackProvider::new(
                "test-paystack-key".to_string(),
            )),
            otp_provider: std::sync::Arc::new(otp::mock::MockOtpProvider),
            otp_hmac_secret: SecretString::new("test-otp-hmac-secret".to_string()),
            cookie: CookieConfig::default(),
        }
    }

    async fn preflight(method: Method) -> StatusCode {
        let app = router(test_state());
        let request = Request::builder()
            .method(Method::OPTIONS)
            .uri("/me")
            .header(header::ORIGIN, "https://app.aframp.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, method.as_str())
            .body(Body::empty())
            .expect("request");
        app.oneshot(request).await.expect("response").status()
    }

    #[tokio::test]
    async fn cors_preflight_allows_patch() {
        let status = preflight(Method::PATCH).await;
        assert!(
            status.is_success(),
            "PATCH preflight should be allowed, got {status}"
        );
    }

    #[tokio::test]
    async fn cors_preflight_allows_delete() {
        let status = preflight(Method::DELETE).await;
        assert!(
            status.is_success(),
            "DELETE preflight should be allowed, got {status}"
        );
    }
}
