#![deny(clippy::unwrap_used)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::cast_sign_loss,
    clippy::module_inception,
    clippy::struct_excessive_bools,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap
)]

pub mod api;
pub mod binary_setup;
pub mod caching;
pub mod database;
pub mod graceful_exit;
pub mod job_queue;
pub mod media_mount;
pub mod s2s_client;
pub mod utils;
