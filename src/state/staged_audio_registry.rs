use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};
use uuid::Uuid;

pub const DEFAULT_STAGED_AUDIO_TTL: Duration = Duration::from_secs(900); // 15 minutes

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StagedAudioState {
    Available,
    Reserved { lease_id: String },
    Consumed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedAudioDescriptor {
    pub staged_audio_id: String,
    pub stage_id: String,
    pub title: String,
    pub duration_secs: u32,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedAudioLease {
    pub staged_audio_id: String,
    pub lease_id: String,
    pub stage_id: String,
    pub file_path: PathBuf,
    pub title: String,
    pub duration_secs: u32,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StagedAudioRegistryError {
    #[error("Archivo de audio temporal no encontrado")]
    NotFound,
    #[error("El archivo de audio ya fue consumido")]
    AlreadyConsumed,
    #[error("El archivo de audio se encuentra reservado")]
    InUse,
    #[error("El identificador de reserva no es válido")]
    InvalidLease,
}

impl StagedAudioRegistryError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound => "staged_audio_not_found",
            Self::AlreadyConsumed => "audio_already_consumed",
            Self::InUse => "staged_audio_in_use",
            Self::InvalidLease => "invalid_lease",
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::NotFound => "Archivo de audio temporal no encontrado",
            Self::AlreadyConsumed => "El archivo de audio ya fue consumido",
            Self::InUse => "El archivo de audio se encuentra reservado",
            Self::InvalidLease => "El identificador de reserva no es válido",
        }
    }
}

#[derive(Debug)]
struct StagedAudioEntry {
    staged_audio_id: String,
    stage_id: String,
    file_path: PathBuf,
    title: String,
    duration_secs: u32,
    size_bytes: u64,
    #[allow(dead_code)]
    created_at: Instant,
    expires_at: Instant,
    state: StagedAudioState,
    expired_pending_release: bool,
}

#[derive(Debug, Clone)]
pub struct StagedAudioRegistry {
    inner: Arc<Mutex<HashMap<String, StagedAudioEntry>>>,
    ttl: Duration,
}

impl Default for StagedAudioRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl StagedAudioRegistry {
    pub fn new() -> Self {
        Self::with_ttl(DEFAULT_STAGED_AUDIO_TTL)
    }

    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            ttl,
        }
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    pub fn register(
        &self,
        stage_id: String,
        file_path: PathBuf,
        title: String,
        duration_secs: u32,
        size_bytes: u64,
    ) -> StagedAudioDescriptor {
        let staged_audio_id = format!("staged-{}", Uuid::new_v4());
        let now = Instant::now();
        let expires_at = now + self.ttl;

        let entry = StagedAudioEntry {
            staged_audio_id: staged_audio_id.clone(),
            stage_id: stage_id.clone(),
            file_path: file_path.clone(),
            title: title.clone(),
            duration_secs,
            size_bytes,
            created_at: now,
            expires_at,
            state: StagedAudioState::Available,
            expired_pending_release: false,
        };

        {
            let mut map = self.inner.lock().expect("lock poisoned");
            map.insert(staged_audio_id.clone(), entry);
        }

        // Spawn expiration task holding Weak reference if a tokio runtime is active
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let weak_inner: Weak<Mutex<HashMap<String, StagedAudioEntry>>> =
                Arc::downgrade(&self.inner);
            let id_for_task = staged_audio_id.clone();
            let ttl_for_task = self.ttl;

            handle.spawn(async move {
                tokio::time::sleep(ttl_for_task).await;
                if let Some(inner) = weak_inner.upgrade() {
                    let mut map = match inner.lock() {
                        Ok(m) => m,
                        Err(_) => return,
                    };

                    if let Some(entry) = map.get_mut(&id_for_task) {
                        match &entry.state {
                            StagedAudioState::Available => {
                                let path = entry.file_path.clone();
                                map.remove(&id_for_task);
                                let _ = std::fs::remove_file(path);
                            }
                            StagedAudioState::Consumed => {
                                // Expire tombstone after TTL
                                map.remove(&id_for_task);
                            }
                            StagedAudioState::Reserved { .. } => {
                                // Mark expired pending release so active operation can finish safely
                                entry.expired_pending_release = true;
                            }
                        }
                    }
                }
            });
        }

        StagedAudioDescriptor {
            staged_audio_id,
            stage_id,
            title,
            duration_secs,
            size_bytes,
        }
    }

    pub fn reserve(
        &self,
        staged_audio_id: &str,
    ) -> Result<StagedAudioLease, StagedAudioRegistryError> {
        let mut map = self.inner.lock().expect("lock poisoned");
        let now = Instant::now();

        // Check if expired and available
        if let Some(entry) = map.get(staged_audio_id) {
            if entry.expires_at <= now && entry.state == StagedAudioState::Available {
                let path = entry.file_path.clone();
                map.remove(staged_audio_id);
                let _ = std::fs::remove_file(path);
                return Err(StagedAudioRegistryError::NotFound);
            }
        }

        let entry = map
            .get_mut(staged_audio_id)
            .ok_or(StagedAudioRegistryError::NotFound)?;

        match &entry.state {
            StagedAudioState::Consumed => Err(StagedAudioRegistryError::AlreadyConsumed),
            StagedAudioState::Reserved { .. } => Err(StagedAudioRegistryError::InUse),
            StagedAudioState::Available => {
                let lease_id = Uuid::new_v4().to_string();
                entry.state = StagedAudioState::Reserved {
                    lease_id: lease_id.clone(),
                };

                Ok(StagedAudioLease {
                    staged_audio_id: entry.staged_audio_id.clone(),
                    lease_id,
                    stage_id: entry.stage_id.clone(),
                    file_path: entry.file_path.clone(),
                    title: entry.title.clone(),
                    duration_secs: entry.duration_secs,
                    size_bytes: entry.size_bytes,
                })
            }
        }
    }

    pub fn release(
        &self,
        staged_audio_id: &str,
        lease_id: &str,
    ) -> Result<(), StagedAudioRegistryError> {
        let mut map = self.inner.lock().expect("lock poisoned");
        let now = Instant::now();

        let entry = map
            .get_mut(staged_audio_id)
            .ok_or(StagedAudioRegistryError::NotFound)?;

        match &entry.state {
            StagedAudioState::Reserved {
                lease_id: current_lease,
            } if current_lease == lease_id => {
                if entry.expired_pending_release || entry.expires_at <= now {
                    let path = entry.file_path.clone();
                    map.remove(staged_audio_id);
                    let _ = std::fs::remove_file(path);
                    Ok(())
                } else if !entry.file_path.exists() {
                    map.remove(staged_audio_id);
                    Ok(())
                } else {
                    entry.state = StagedAudioState::Available;
                    Ok(())
                }
            }
            _ => Err(StagedAudioRegistryError::InvalidLease),
        }
    }

    pub fn commit_consumed(
        &self,
        staged_audio_id: &str,
        lease_id: &str,
    ) -> Result<(), StagedAudioRegistryError> {
        let mut map = self.inner.lock().expect("lock poisoned");

        let entry = map
            .get_mut(staged_audio_id)
            .ok_or(StagedAudioRegistryError::NotFound)?;

        match &entry.state {
            StagedAudioState::Reserved {
                lease_id: current_lease,
            } if current_lease == lease_id => {
                entry.state = StagedAudioState::Consumed;
                Ok(())
            }
            _ => Err(StagedAudioRegistryError::InvalidLease),
        }
    }

    pub fn invalidate(&self, staged_audio_id: &str) -> Result<(), StagedAudioRegistryError> {
        let mut map = self.inner.lock().expect("lock poisoned");
        if let Some(entry) = map.remove(staged_audio_id) {
            let _ = std::fs::remove_file(entry.file_path);
        }
        Ok(())
    }

    pub fn purge_expired(&self) -> usize {
        let mut map = self.inner.lock().expect("lock poisoned");
        let now = Instant::now();
        let mut to_remove = Vec::new();

        for (id, entry) in map.iter_mut() {
            if entry.expires_at <= now {
                match &entry.state {
                    StagedAudioState::Available => {
                        let _ = std::fs::remove_file(&entry.file_path);
                        to_remove.push(id.clone());
                    }
                    StagedAudioState::Consumed => {
                        to_remove.push(id.clone());
                    }
                    StagedAudioState::Reserved { .. } => {
                        entry.expired_pending_release = true;
                    }
                }
            }
        }

        let count = to_remove.len();
        for id in to_remove {
            map.remove(&id);
        }
        count
    }

    pub fn get_state(&self, staged_audio_id: &str) -> Option<StagedAudioState> {
        let map = self.inner.lock().expect("lock poisoned");
        map.get(staged_audio_id).map(|e| e.state.clone())
    }

    pub fn get_descriptor(&self, staged_audio_id: &str) -> Option<StagedAudioDescriptor> {
        let map = self.inner.lock().expect("lock poisoned");
        map.get(staged_audio_id).map(|e| StagedAudioDescriptor {
            staged_audio_id: e.staged_audio_id.clone(),
            stage_id: e.stage_id.clone(),
            title: e.title.clone(),
            duration_secs: e.duration_secs,
            size_bytes: e.size_bytes,
        })
    }
}
