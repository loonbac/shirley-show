use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::domain::agent_tool::ToolAction;
use crate::domain::audio_format::AudioFormat;
use crate::infra::filesystem::StagingFileGuard;

pub enum StagedAudioHandle {
    Available {
        guard: StagingFileGuard,
        format: AudioFormat,
        byte_size: i64,
    },
    Consumed,
    Failed,
}

#[derive(Default, Clone)]
pub struct CommittedTurnState {
    pub actions_taken: Vec<ToolAction>,
    pub active_presentation_id: Option<String>,
    pub staged_audio_id: Option<String>,
}

pub struct TurnToolContext {
    pub known_presentation_ids: HashSet<String>,
    pub known_staged_audio_ids: HashSet<String>,
    pub executed_call_ids: HashSet<String>,
    pub executed_signatures: HashMap<String, serde_json::Value>,
    pub staged_audio: Option<StagedAudioHandle>,
    pub staged_audio_id: Option<String>,
    pub actions_taken: Vec<ToolAction>,
    pub active_presentation_id: Option<String>,
    pub committed_tracker: Option<Arc<Mutex<CommittedTurnState>>>,
}

impl Default for TurnToolContext {
    fn default() -> Self {
        Self::new(None, None)
    }
}

impl TurnToolContext {
    pub fn new(
        active_presentation_id: Option<String>,
        staged_audio: Option<StagedAudioHandle>,
    ) -> Self {
        Self::with_full_context(active_presentation_id, staged_audio, None, None)
    }

    pub fn with_staged_audio_id(
        active_presentation_id: Option<String>,
        staged_audio: Option<StagedAudioHandle>,
        staged_audio_id: Option<String>,
    ) -> Self {
        Self::with_full_context(active_presentation_id, staged_audio, staged_audio_id, None)
    }

    pub fn with_tracker(
        active_presentation_id: Option<String>,
        staged_audio: Option<StagedAudioHandle>,
        committed_tracker: Option<Arc<Mutex<CommittedTurnState>>>,
    ) -> Self {
        Self::with_full_context(
            active_presentation_id,
            staged_audio,
            None,
            committed_tracker,
        )
    }

    pub fn with_full_context(
        active_presentation_id: Option<String>,
        staged_audio: Option<StagedAudioHandle>,
        staged_audio_id: Option<String>,
        committed_tracker: Option<Arc<Mutex<CommittedTurnState>>>,
    ) -> Self {
        let mut known_presentation_ids = HashSet::new();
        if let Some(ref id) = active_presentation_id {
            let trimmed = id.trim();
            if !trimmed.is_empty() {
                known_presentation_ids.insert(trimmed.to_string());
            }
        }

        let mut known_staged_audio_ids = HashSet::new();
        if let Some(ref id) = staged_audio_id {
            let trimmed = id.trim();
            if !trimmed.is_empty() {
                known_staged_audio_ids.insert(trimmed.to_string());
            }
        }

        if let Some(ref tracker) = committed_tracker {
            tracker.lock().unwrap().staged_audio_id = staged_audio_id.clone();
        }

        Self {
            known_presentation_ids,
            known_staged_audio_ids,
            executed_call_ids: HashSet::new(),
            executed_signatures: HashMap::new(),
            staged_audio,
            staged_audio_id,
            actions_taken: Vec::new(),
            active_presentation_id,
            committed_tracker,
        }
    }

    pub fn record_action(&mut self, action: ToolAction) {
        if let Some(ref tracker) = self.committed_tracker {
            tracker.lock().unwrap().actions_taken.push(action.clone());
        }
        self.actions_taken.push(action);
    }

    pub fn set_active_presentation_id(&mut self, id: Option<String>) {
        if let Some(ref tracker) = self.committed_tracker {
            tracker.lock().unwrap().active_presentation_id = id.clone();
        }
        self.active_presentation_id = id;
    }

    pub fn set_staged_audio_id(&mut self, id: Option<String>) {
        if let Some(ref tracker) = self.committed_tracker {
            tracker.lock().unwrap().staged_audio_id = id.clone();
        }
        if let Some(ref val) = id {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                self.known_staged_audio_ids.insert(trimmed.to_string());
            }
        }
        self.staged_audio_id = id;
    }
}
