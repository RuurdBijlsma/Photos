#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use app_state::{database_url, load_app_settings};
use common_services::api::admin::service::admin_update_user_media_folder;
use common_services::api::auth::interfaces::CreateUser;
use common_services::api::auth::service::{create_user, generate_invite};
use common_services::database::get_db_pool;
use common_services::database::user_store::UserStore;
use common_types::dev_constants::{EMAIL, PASSWORD, USERNAME};

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let settings = load_app_settings()?;
    let pool = get_db_pool(database_url(), true).await?;

    let create_user_payload = CreateUser {
        name: USERNAME.to_owned(),
        email: EMAIL.to_owned(),
        password: PASSWORD.to_owned(),
        token: None,
    };
    let user_result = create_user(&pool, &settings.ingest, &create_user_payload).await;
    let user = if let Ok(u) = user_result {
        u
    } else {
        UserStore::list_users(&pool)
            .await?
            .first()
            .expect("No dev user found")
            .clone()
    };
    println!("Created user {user:?}");
    let user_invite = generate_invite(&pool, &settings.ingest, "otheruser").await?;
    create_user(
        &pool,
        &settings.ingest,
        &CreateUser {
            name: "other_user".to_owned(),
            email: "other@example.com".to_owned(),
            password: PASSWORD.to_owned(),
            token: Some(user_invite.token),
        },
    )
    .await?;
    let user = admin_update_user_media_folder(&pool, &settings.ingest, user.id, "Ruurd").await?;
    println!(
        "Initiated processing, media folder: {:?}",
        user.media_folder
    );

    Ok(())
}
