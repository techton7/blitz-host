use std::fmt;
use std::sync::{Arc, RwLock};

type WakeCallback = Arc<dyn Fn() + Send + Sync>;

/// Thread-safe callback hook to wake the host/UI thread when an incoming request arrives.
#[derive(Clone, Default)]
pub struct ServiceWaker(Arc<RwLock<Option<WakeCallback>>>);

impl ServiceWaker {
    /// Install or replace the wake callback.
    pub fn set(&self, wake: impl Fn() + Send + Sync + 'static) {
        let mut guard = match self.0.write() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        };
        *guard = Some(Arc::new(wake));
    }

    /// Clear the wake callback.
    pub fn clear(&self) {
        let mut guard = match self.0.write() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        };
        *guard = None;
    }

    /// Invoke the installed wake callback if present.
    ///
    /// The read lock is dropped before the callback is invoked so that the callback
    /// can re-enter `ServiceWaker` methods (such as `set` or `clear`) without deadlocking.
    pub fn wake(&self) {
        let callback = {
            let guard = match self.0.read() {
                Ok(guard) => guard,
                Err(poison) => poison.into_inner(),
            };
            guard.clone()
        };
        if let Some(wake) = callback {
            wake();
        }
    }
}

impl fmt::Debug for ServiceWaker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let installed = match self.0.read() {
            Ok(guard) => guard.is_some(),
            Err(poison) => poison.into_inner().is_some(),
        };
        f.debug_struct("ServiceWaker")
            .field("installed", &installed)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    #[test]
    fn test_set_and_wake() {
        let waker = ServiceWaker::default();
        let counter = Arc::new(AtomicUsize::new(0));

        {
            let counter = Arc::clone(&counter);
            waker.set(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            });
        }

        assert_eq!(counter.load(Ordering::SeqCst), 0);
        waker.wake();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        waker.wake();
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_clear_disables_wake() {
        let waker = ServiceWaker::default();
        let counter = Arc::new(AtomicUsize::new(0));

        {
            let counter = Arc::clone(&counter);
            waker.set(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            });
        }

        waker.wake();
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        waker.clear();
        waker.wake();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_replace_waker() {
        let waker = ServiceWaker::default();
        let first_counter = Arc::new(AtomicUsize::new(0));
        let second_counter = Arc::new(AtomicUsize::new(0));

        {
            let first_counter = Arc::clone(&first_counter);
            waker.set(move || {
                first_counter.fetch_add(1, Ordering::SeqCst);
            });
        }

        waker.wake();
        assert_eq!(first_counter.load(Ordering::SeqCst), 1);
        assert_eq!(second_counter.load(Ordering::SeqCst), 0);

        {
            let second_counter = Arc::clone(&second_counter);
            waker.set(move || {
                second_counter.fetch_add(1, Ordering::SeqCst);
            });
        }

        waker.wake();
        assert_eq!(first_counter.load(Ordering::SeqCst), 1);
        assert_eq!(second_counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_invocation_outside_lock() {
        let waker = ServiceWaker::default();
        let waker_for_cb = waker.clone();
        let replaced_called = Arc::new(AtomicBool::new(false));

        let replaced_called_cb = Arc::clone(&replaced_called);
        waker.set(move || {
            // Calling set from within the wake callback must not deadlock.
            let flag = Arc::clone(&replaced_called_cb);
            waker_for_cb.set(move || {
                flag.store(true, Ordering::SeqCst);
            });
        });

        assert!(!replaced_called.load(Ordering::SeqCst));
        waker.wake();
        assert!(!replaced_called.load(Ordering::SeqCst));
        waker.wake();
        assert!(replaced_called.load(Ordering::SeqCst));
    }
}
