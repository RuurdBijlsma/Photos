#![deny(clippy::unwrap_used)]
#![allow(
    clippy::cognitive_complexity,
    clippy::cast_sign_loss,
    clippy::missing_errors_doc,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::missing_panics_doc
)]

pub mod context;
pub mod handlers;
pub mod jobs;
pub mod macros;
pub mod worker;
