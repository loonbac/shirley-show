//! Keeps the Colab session alive briefly after a separation, so a second
//! request does not pay another cold start.
//!
//! The product rule is explicit: after a request finishes, the machine stays up
//! for three minutes, and anything asked inside that window reuses it. Reusing
//! needs two things that did not exist: a place to remember the session id, and
//! a decision that is not "stop immediately".
//!
//! The cache is deliberately dumb. It stores one id and the instant it was
//! stored; the policy lives in the caller, which is the only place that knows
//! whether the session is still running and whether a new request arrived.

use std::time::{Duration, Instant};

use tokio::sync::Mutex;

#[derive(Debug, Default)]
struct Inner {
    session_id: Option<String>,
    stored_at: Option<Instant>,
}

/// Remembers the most recent Colab session and how old that memory is.
#[derive(Debug, Default)]
pub struct ColabSessionCache {
    inner: Mutex<Inner>,
}

impl ColabSessionCache {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner::default()),
        }
    }

    /// Records a session as the current one, restarting its age.
    pub fn store(&mut self, session_id: String) {
        self.store_at(session_id, Instant::now());
    }

    /// Records a session as if it had been stored at `at`.
    ///
    /// Exists so the retention boundary can be tested without sleeping for
    /// three minutes: aging is the whole behaviour, and a test that cannot move
    /// the clock cannot observe it.
    pub fn store_at(&mut self, session_id: String, at: Instant) {
        if let Ok(mut guard) = self.inner.try_lock() {
            guard.session_id = Some(session_id);
            guard.stored_at = Some(at);
            return;
        }
        // The store path is synchronous, so a contended lock means another task
        // is reading; dropping the update is safe because a missing cached id
        // only costs a cold start, never correctness.
    }

    /// Returns the cached session id if it was stored less than `retention` ago.
    pub fn reusable(&self, retention: Duration) -> Option<String> {
        let guard = self.inner.try_lock().ok()?;
        let id = guard.session_id.as_ref()?;
        let stored_at = guard.stored_at?;
        if stored_at.elapsed() < retention {
            Some(id.clone())
        } else {
            None
        }
    }

    /// The cached id regardless of age, or `None` when nothing is cached.
    pub fn current(&self) -> Option<String> {
        self.inner.try_lock().ok()?.session_id.clone()
    }

    /// Forgets the session. Called when Colab reports it gone, or when it was
    /// stopped on purpose.
    pub fn invalidate(&mut self) {
        if let Ok(mut guard) = self.inner.try_lock() {
            guard.session_id = None;
            guard.stored_at = None;
        }
    }
}
