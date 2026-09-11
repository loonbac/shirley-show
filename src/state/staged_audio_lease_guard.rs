use std::ops::Deref;

use crate::state::staged_audio_registry::{
    StagedAudioLease, StagedAudioRegistry, StagedAudioRegistryError,
};

/// RAII guard that automatically releases a staged audio reservation when dropped
/// unless explicitly committed via `commit_consumed`, `commit_processed`, or `defuse`.
#[derive(Debug)]
pub struct StagedAudioLeaseGuard {
    registry: StagedAudioRegistry,
    lease: Option<StagedAudioLease>,
}

impl StagedAudioLeaseGuard {
    pub fn new(registry: StagedAudioRegistry, lease: StagedAudioLease) -> Self {
        Self {
            registry,
            lease: Some(lease),
        }
    }

    /// Commits the staged audio as consumed while deleting the temporary source file from disk.
    /// Retains the single-use tombstone in the registry.
    pub fn commit_processed(mut self) -> Result<(), StagedAudioRegistryError> {
        let lease = self
            .lease
            .take()
            .ok_or(StagedAudioRegistryError::InvalidLease)?;
        self.registry
            .commit_processed(&lease.staged_audio_id, &lease.lease_id)
    }

    /// Commits the staged audio as consumed assuming the file was moved to permanent storage.
    /// Retains the single-use tombstone in the registry without deleting the path.
    pub fn commit_consumed(mut self) -> Result<(), StagedAudioRegistryError> {
        let lease = self
            .lease
            .take()
            .ok_or(StagedAudioRegistryError::InvalidLease)?;
        self.registry
            .commit_consumed(&lease.staged_audio_id, &lease.lease_id)
    }

    /// Defuses the guard without performing release or deletion.
    pub fn defuse(mut self) {
        self.lease = None;
    }

    /// Explicitly releases the reservation back to available state.
    pub fn release(mut self) -> Result<(), StagedAudioRegistryError> {
        let lease = self
            .lease
            .take()
            .ok_or(StagedAudioRegistryError::InvalidLease)?;
        self.registry
            .release(&lease.staged_audio_id, &lease.lease_id)
    }

    pub fn lease(&self) -> &StagedAudioLease {
        self.lease.as_ref().expect("lease must be present")
    }
}

impl Deref for StagedAudioLeaseGuard {
    type Target = StagedAudioLease;

    fn deref(&self) -> &Self::Target {
        self.lease.as_ref().expect("lease must be present")
    }
}

impl Drop for StagedAudioLeaseGuard {
    fn drop(&mut self) {
        if let Some(ref lease) = self.lease {
            let _ = self
                .registry
                .release(&lease.staged_audio_id, &lease.lease_id);
        }
    }
}
