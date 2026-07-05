#![allow(
    clippy::needless_for_each,
    clippy::cognitive_complexity,
    clippy::cast_sign_loss,
    clippy::struct_excessive_bools,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::cast_possible_truncation
)]
use crate::api_state::ApiContext;
use crate::create_router;
use crate::timeline::websocket::create_media_item_transmitter;
use app_state::AppSettings;
use app_state::constants::HOSTED_FOLDER;
use axum::routing::get_service;
use color_eyre::Result;
use common_services::s2s_client::S2SClient;
use http::{HeaderValue, header};
use open_clip_inference::{TextEmbedder, VisionEmbedder};
use reqwest::Client;
use sqlx::PgPool;
use std::iter::once;
use std::net::SocketAddr;
use std::sync::Arc;
use tasks::task_runner::init_task_scheduler;
use tower_http::compression::CompressionLayer;
use tower_http::cors;
use tower_http::cors::CorsLayer;
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, info};

pub async fn serve(pool: PgPool, settings: AppSettings, run_task_scheduler: bool) -> Result<()> {
    if run_task_scheduler {
        init_task_scheduler(&pool, &settings)?;
    }
    info!("Loading CLIP text embedder...");
    let text_embedder = TextEmbedder::from_hf(&settings.ingest.analyzer.search.embedder_model_id)
        .build()
        .await?;
    info!("Loading CLIP vision embedder...");
    let vision_embedder =
        VisionEmbedder::from_hf(&settings.ingest.analyzer.search.embedder_model_id)
            .build()
            .await?;
    // --- Server Startup ---
    info!("🚀 Initializing server...");
    let api_state = ApiContext {
        pool: pool.clone(),
        s2s_client: S2SClient::new(Client::new()),
        settings: settings.clone(),
        timeline_broadcaster: create_media_item_transmitter(&pool)?,
        text_embedder: Arc::new(text_embedder),
        vision_embedder: Arc::new(vision_embedder),
    };

    // --- CORS Configuration ---
    let allowed_origins: Vec<HeaderValue> = settings
        .api
        .allowed_origins
        .iter()
        .filter_map(|s| match s.parse() {
            Ok(hv) => Some(hv),
            Err(e) => {
                error!("Invalid CORS origin configured: {} - Error: {}", s, e);
                None
            }
        })
        .collect();

    let cors = CorsLayer::new()
        .expose_headers([header::CONTENT_DISPOSITION])
        .allow_methods(cors::Any)
        .allow_origin(allowed_origins)
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::ORIGIN,
            header::USER_AGENT,
            header::CACHE_CONTROL,
            header::PRAGMA,
        ]);

    // Static file serving
    let serve_thumbnails = ServeDir::new(&settings.ingest.thumbnails_root);
    let thumbnail_cache_layer = SetResponseHeaderLayer::if_not_present(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );

    let serve_hosted = ServeDir::new(settings.ingest.app_data_root.join(HOSTED_FOLDER));
    let hosted_cache_layer = SetResponseHeaderLayer::if_not_present(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=86400"),
    );
    let serve_hosted = get_service(serve_hosted)
        .layer::<_, std::convert::Infallible>(hosted_cache_layer)
        .layer::<_, std::convert::Infallible>(cors.clone());

    // --- Create Router ---
    let app = create_router(api_state)
        .layer(TraceLayer::new_for_http().on_request(()))
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(SetSensitiveRequestHeadersLayer::new(once(
            header::AUTHORIZATION,
        )))
        .nest_service(
            "/thumbnails",
            get_service(serve_thumbnails).layer(thumbnail_cache_layer),
        )
        .nest_service("/hosted", serve_hosted);

    // --- Start Server ---
    let listen_address = format!("{}:{}", settings.api.host, settings.api.port);
    let listener = tokio::net::TcpListener::bind(&listen_address).await?;

    info!("📚 Docs available at http://{listen_address}/docs");
    info!("✅ Server listening on http://{listen_address}");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
