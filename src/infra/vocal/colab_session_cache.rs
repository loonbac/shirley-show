//! Keeps the Colab session alive briefly after a separation, so a second
//! request does not pay another cold start.
//!
//! The product rule is explicit: after a request finishes, the machine stays up
//! for ten minutes, and anything asked inside that window reuses it. Reusing
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
    /// When the pending release would fire, and which generation owns it.
    ///
    /// A release is superseded rather than cancelled: a detached task cannot be
    /// cancelled cheaply, so instead it re-checks the generation before stopping
    /// anything. An old task that wakes up finds a newer generation and does
    /// nothing, which is what keeps a reused session from being stopped
    /// underneath an active request.
    release_deadline: Option<Instant>,
    release_generation: u64,
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
    /// Exists so the retention boundary can be tested without sleeping for ten
    /// minutes: aging is the whole behaviour, and a test that cannot move the
    /// clock cannot observe it.
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
    /// Refreshes only the age of the cached session.
    ///
    /// Called when the work finishes, not when it starts. Stamping at start made
    /// a separation longer than the retention window expire before it completed,
    /// so the next request paid a cold start while the previous machine was still
    /// running. The window the product asks for counts from completion.
    pub fn refresh_at(&mut self, at: Instant) {
        if let Ok(mut guard) = self.inner.try_lock() {
            if guard.session_id.is_some() {
                guard.stored_at = Some(at);
            }
        }
    }

    /// Arms (or re-arms) the pending release and returns its generation.
    ///
    /// Re-arming bumps the generation. The task that owns generation N refuses to
    /// stop the session once the generation has moved past N, so a second request
    /// that reuses the machine cannot be killed by the first request's timer.
    pub fn arm_release(&mut self, deadline: Instant) -> u64 {
        if let Ok(mut guard) = self.inner.try_lock() {
            guard.release_deadline = Some(deadline);
            guard.release_generation = guard.release_generation.wrapping_add(1);
            return guard.release_generation;
        }
        0
    }

    /// The instant the current release is due, if one is armed.
    pub fn release_deadline(&self) -> Option<Instant> {
        self.inner.try_lock().ok()?.release_deadline
    }

    /// The generation that currently owns the release.
    pub fn release_generation(&self) -> u64 {
        self.inner
            .try_lock()
            .map(|g| g.release_generation)
            .unwrap_or(0)
    }

    /// True when `generation` still owns the release, so the task may stop it.
    pub fn should_release(&self, generation: u64) -> bool {
        match self.inner.try_lock() {
            Ok(g) => generation != 0 && g.release_generation == generation,
            Err(_) => false,
        }
    }
}
