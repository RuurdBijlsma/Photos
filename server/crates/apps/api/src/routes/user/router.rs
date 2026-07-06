use crate::api_state::ApiContext;
use crate::user::handlers::{get_user_profile_handler, list_users_handler, update_my_profile};
use axum::{
    Router,
    routing::{get, put},
};

pub fn user_protected_router() -> Router<ApiContext> {
    Router::new()
        .route("/user/{user_id}/profile", get(get_user_profile_handler))
        .route("/user/profile", put(update_my_profile))
        .route("/user", get(list_users_handler))
}
