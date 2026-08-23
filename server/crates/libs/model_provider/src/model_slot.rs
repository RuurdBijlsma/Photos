use core::fmt;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::info;

struct ModelSlotInner<T> {
    instance: Option<Arc<T>>,
    idle_since: Option<Instant>,
}

pub struct ModelSlot<T> {
    name: String,
    inner: Mutex<ModelSlotInner<T>>,
    idle_timeout: Duration,
}

impl<T> fmt::Debug for ModelSlot<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModelSlot")
            .field("name", &self.name)
            .field("inner", &"<model_slot_inner>")
            .field("idle_timeout", &self.idle_timeout)
            .finish()
    }
}

impl<T: Send + Sync> ModelSlot<T> {
    #[must_use]
    pub fn new(name: &str, idle_timeout: Duration) -> Self {
        Self {
            name: name.to_owned(),
            inner: Mutex::new(ModelSlotInner {
                instance: None,
                idle_since: None,
            }),
            idle_timeout,
        }
    }

    pub async fn get_or_load<F, Fut>(&self, loader: F) -> color_eyre::Result<Arc<T>>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = color_eyre::Result<Arc<T>>> + Send,
    {
        let mut guard = self.inner.lock().await;
        if let Some(arc) = guard.instance.clone() {
            guard.idle_since = None;
            return Ok(arc);
        }

        let arc = loader().await?;
        guard.instance = Some(arc.clone());
        guard.idle_since = None;
        drop(guard);

        Ok(arc)
    }

    pub async fn cleanup_idle(&self) {
        let mut guard = self.inner.lock().await;
        let is_in_use = guard
            .instance
            .as_ref()
            .is_some_and(|arc| Arc::strong_count(arc) > 1);

        if is_in_use {
            guard.idle_since = None;
        } else if guard.instance.is_some() {
            let idle_since = *guard.idle_since.get_or_insert_with(Instant::now);
            if idle_since.elapsed() >= self.idle_timeout {
                info!(
                    "Unloading {} after {:?} of inactivity.",
                    &self.name, self.idle_timeout
                );
                guard.instance = None;
                guard.idle_since = None;
            }
        }
    }
}