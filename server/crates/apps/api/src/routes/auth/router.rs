use crate::api_state::ApiContext;
use crate::auth::handlers::{
    generate_invite_handler, get_me, login, logout, refresh_session, register,
};
use app_state::RateLimitingSettings;
use axum::{
    Router,
    routing::{get, post},
};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

pub fn auth_public_router(rate_limiting: &RateLimitingSettings) -> Router<ApiContext> {
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(rate_limiting.req_per_second)
        .burst_size(rate_limiting.burst_size)
        .finish()
        .expect("Could not create rate-limiting governor.");

    Router::new()
        .route("/auth/refresh", post(refresh_session))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .layer(GovernorLayer::new(governor_conf))
}

pub fn auth_protected_router() -> Router<ApiContext> {
    Router::new().route("/auth/me", get(get_me))
}

pub fn auth_admin_routes() -> Router<ApiContext> {
    Router::new().route("/auth/generate-invite", post(generate_invite_handler))
}
