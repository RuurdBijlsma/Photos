#[allow(clippy::all)]
#[allow(clippy::pedantic)]
#[allow(clippy::nursery)]
#[allow(clippy::restriction)]
pub mod api {
    include!(concat!(env!("OUT_DIR"), "/api.rs"));
}
