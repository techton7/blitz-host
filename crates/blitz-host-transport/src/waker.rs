use std::sync::{Arc, OnceLock};

/// Thread-safe callback hook to wake the host/UI thread when an incoming request arrives.
#[derive(Clone, Default)]
pub struct ServiceWaker(Arc<OnceLock<Box<dyn Fn() + Send + Sync>>>);

impl ServiceWaker {
    /// Install the wake callback. Only the first call takes effect.
    pub fn set(&self, wake: impl Fn() + Send + Sync + 'static) {
        let _ = self.0.set(Box::new(wake));
    }

    /// Invoke the installed wake callback if present.
    pub fn wake(&self) {
        if let Some(wake) = self.0.get() {
            wake();
        }
    }
}

impl std::fmt::Debug for ServiceWaker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceWaker")
            .field("installed", &self.0.get().is_some())
            .finish()
    }
}
