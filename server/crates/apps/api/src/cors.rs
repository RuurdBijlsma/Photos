use http::{HeaderName, HeaderValue, Method, header};
use tower_http::cors::CorsLayer;
use tracing::error;

pub fn get_cors(allowed_origins: &[String]) -> CorsLayer {
    let allowed_origins: Vec<HeaderValue> = allowed_origins
        .iter()
        .filter_map(|s| match s.parse() {
            Ok(hv) => Some(hv),
            Err(e) => {
                error!("Invalid CORS origin configured: {} - Error: {}", s, e);
                None
            }
        })
        .collect();

    CorsLayer::new()
        .expose_headers([
            header::CONTENT_DISPOSITION,
            HeaderName::from_static("location"),
            HeaderName::from_static("tus-resumable"),
            HeaderName::from_static("tus-version"),
            HeaderName::from_static("tus-max-size"),
            HeaderName::from_static("tus-extension"),
            HeaderName::from_static("upload-offset"),
            HeaderName::from_static("upload-length"),
            HeaderName::from_static("upload-metadata"),
        ])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_origin(allowed_origins)
        .allow_credentials(true)
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::ORIGIN,
            header::USER_AGENT,
            header::CACHE_CONTROL,
            header::PRAGMA,
            HeaderName::from_static("upload-length"),
            HeaderName::from_static("upload-offset"),
            HeaderName::from_static("tus-resumable"),
            HeaderName::from_static("upload-metadata"),
            HeaderName::from_static("upload-defer-length"),
            HeaderName::from_static("upload-checksum"),
            HeaderName::from_static("x-requested-with"),
            HeaderName::from_static("x-http-method-override"),
        ])
}
