#![deny(clippy::unwrap_used)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]

mod api;

pub use api::*;
