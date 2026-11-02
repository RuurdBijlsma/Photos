use ort::environment::Environment;
use std::sync::{Mutex, PoisonError};

static ENVIRONMENT: Mutex<Option<Environment>> = Mutex::new(None);

/// Returns the ONNX Runtime environment shared by all server models.
pub fn shared_onnx_environment() -> color_eyre::Result<Environment> {
    let mut slot = ENVIRONMENT.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(environment) = slot.as_ref() {
        return Ok(environment.clone());
    }

    let environment = ort::init().with_name("ruurd_photos").build()?;
    *slot = Some(environment.clone());
    drop(slot);
    Ok(environment)
}
