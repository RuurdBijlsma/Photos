use crate::api_state::ApiContext;
use axum::extract::ws::{Message, WebSocket};
use color_eyre::Result;
use common_services::database::app_user::User;
use serde::Deserialize;
use serde_json::Value;
use sqlx::{PgPool, postgres::PgListener};
use std::sync::Arc;
use tokio::{select, sync::broadcast};
use tracing::{error, info, warn};

#[derive(Clone, Debug, Deserialize)]
pub struct MediaPayload {
    pub user_id: i32,
    #[serde(skip)]
    pub raw_json: String,
}

pub async fn handle_timeline_socket(mut socket: WebSocket, context: ApiContext, user: User) {
    info!("WS connected for user {}", user.id);

    let mut rx = context.timeline_broadcaster.subscribe();

    loop {
        select! {
            result = rx.recv() => {
                match result {
                    Ok(payload) if payload.user_id == user.id => {
                        if socket.send(Message::Text(payload.raw_json.clone().into())).await.is_err() {
                            break;
                        }
                    }
                    Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(_) => break,
                }
            }

            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_)) | Err(_)) | None => break,
                    _ => {}
                }
            }
        }
    }
}

pub fn create_media_item_transmitter(
    pool: &PgPool,
) -> Result<broadcast::Sender<Arc<MediaPayload>>> {
    let (tx, _) = broadcast::channel(100);
    let listener_pool = pool.clone();
    let listener_tx = tx.clone();

    tokio::spawn(async move {
        let mut listener = match PgListener::connect_with(&listener_pool).await {
            Ok(l) => l,
            Err(e) => {
                error!("PgListener connect failed: {}", e);
                return;
            }
        };

        if let Err(e) = listener.listen("media_item_added").await {
            error!("PgListener subscribe failed: {}", e);
            return;
        }

        loop {
            match listener.recv().await {
                Ok(notification) => {
                    let payload = notification.payload();
                    if let Ok(val) = serde_json::from_str::<Value>(payload)
                        && let Some(user_id) = val.get("user_id").and_then(Value::as_i64)
                    {
                        let _ = listener_tx.send(Arc::new(MediaPayload {
                            user_id: user_id as i32,
                            raw_json: payload.to_owned(),
                        }));
                    }
                }
                Err(e) => warn!("PgListener error: {:?}", e),
            }
        }
    });

    Ok(tx)
}
