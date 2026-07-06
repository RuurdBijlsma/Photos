pub mod admin;
pub mod album;
pub mod auth;
pub mod camera;
pub mod daily_cards;
pub mod jobs;
pub mod people;
pub mod photos;
pub mod root;
pub mod s2s;
pub mod search;
pub mod storage;
pub mod system;
pub mod theme;
pub mod timeline;
pub mod trash;
pub mod user;

use crate::album::router::{album_auth_optional_router, album_protected_router};
use crate::people::router::{people_protected_router, people_public_router};
use crate::user::router::user_protected_router;

use crate::admin::router::admin_admin_routes;
use crate::api_state::ApiContext;
use crate::auth::middlewares::optional_user::OptionalUser;
use crate::auth::middlewares::require_role::require_role;
use crate::auth::middlewares::user::ApiUser;
use crate::auth::middlewares::websocket::WsUser;
use crate::auth::router::{auth_admin_routes, auth_protected_router, auth_public_router};
use crate::camera::router::camera_protected_router;
use crate::daily_cards::router::daily_cards_protected_router;
use crate::jobs::router::{jobs_admin_router, jobs_protected_router};
use crate::photos::router::{photos_protected_router, photos_public_router};
use crate::root::router::root_public_router;
use crate::s2s::router::s2s_public_router;
use crate::search::router::search_protected_router;
use crate::storage::router::storage_protected_router;
use crate::system::router::system_protected_router;
use crate::theme::router::theme_protected_router;
use crate::timeline::router::{timeline_protected_router, timeline_websocket_router};
use crate::trash::router::trash_protected_router;
use app_state::RateLimitingSettings;
use axum::Router;
use axum::middleware::{from_extractor_with_state, from_fn_with_state};
use common_services::database::app_user::UserRole;

// --- Router Construction ---
pub fn create_router(api_state: ApiContext) -> Router {
    Router::new()
        .merge(public_routes(&api_state.settings.api.rate_limiting))
        .merge(websocket_routes(api_state.clone()))
        .merge(protected_routes(api_state.clone()))
        .merge(auth_optional_routes(api_state.clone()))
        .merge(admin_routes(api_state.clone()))
        .with_state(api_state)
}

fn public_routes(rate_limiting: &RateLimitingSettings) -> Router<ApiContext> {
    Router::new()
        .merge(auth_public_router(rate_limiting))
        .merge(root_public_router())
        .merge(s2s_public_router())
        .merge(people_public_router())
        .merge(photos_public_router())
}

// New WebSocket Route Group
fn websocket_routes(api_state: ApiContext) -> Router<ApiContext> {
    Router::new()
        .merge(timeline_websocket_router())
        .route_layer(from_extractor_with_state::<WsUser, ApiContext>(api_state))
}

fn auth_optional_routes(api_state: ApiContext) -> Router<ApiContext> {
    Router::new()
        .merge(album_auth_optional_router())
        .route_layer(from_extractor_with_state::<OptionalUser, ApiContext>(
            api_state,
        ))
}

fn protected_routes(api_state: ApiContext) -> Router<ApiContext> {
    Router::new()
        .merge(auth_protected_router())
        .merge(photos_protected_router())
        .merge(theme_protected_router())
        .merge(timeline_protected_router())
        .merge(search_protected_router())
        .merge(storage_protected_router())
        .merge(album_protected_router())
        .merge(camera_protected_router())
        .merge(people_protected_router())
        .merge(user_protected_router())
        .merge(system_protected_router())
        .merge(daily_cards_protected_router())
        .merge(trash_protected_router())
        .merge(jobs_protected_router())
        .route_layer(from_extractor_with_state::<ApiUser, ApiContext>(api_state))
}

fn admin_routes(api_state: ApiContext) -> Router<ApiContext> {
    Router::new()
        .merge(admin_admin_routes())
        .merge(auth_admin_routes())
        .merge(jobs_admin_router())
        .route_layer(from_fn_with_state(UserRole::Admin, require_role))
        .route_layer(from_extractor_with_state::<ApiUser, ApiContext>(api_state))
}
