#![deny(clippy::unwrap_used)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::large_futures
)]

pub mod app;

pub use app::*;
