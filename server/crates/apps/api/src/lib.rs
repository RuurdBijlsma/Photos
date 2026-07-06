#![deny(clippy::unwrap_used)]
#![allow(
    clippy::needless_for_each,
    clippy::cognitive_complexity,
    clippy::cast_sign_loss,
    clippy::struct_excessive_bools,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::cast_possible_truncation
)]

pub mod api_state;
mod routes;
mod server;

pub use routes::*;
pub use server::*;
