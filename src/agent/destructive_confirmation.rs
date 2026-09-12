use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const PENDING_DESTRUCTIVE_TTL: Duration = Duration::from_secs(15 * 60); // 15 minutes

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestructiveTool {
    DeletePresentation,
    DeleteItem,
    DeleteEvent,
    FinalizeEvent,
    DeleteTicket,
    DeleteExpense,
    DeleteInterest,
}

impl DestructiveTool {
    pub fn name(&self) -> &'static str {
        match self {
            Self::DeletePresentation => "eliminar_presentacion",
            Self::DeleteItem => "eliminar_elemento",
            Self::DeleteEvent => "eliminar_evento",
            Self::FinalizeEvent => "finalizar_evento",
            Self::DeleteTicket => "eliminar_entrada",
            Self::DeleteExpense => "eliminar_gasto",
            Self::DeleteInterest => "eliminar_interesado",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "eliminar_presentacion" => Some(Self::DeletePresentation),
            "eliminar_elemento" => Some(Self::DeleteItem),
            "eliminar_evento" => Some(Self::DeleteEvent),
            "finalizar_evento" => Some(Self::FinalizeEvent),
            "eliminar_entrada" => Some(Self::DeleteTicket),
            "eliminar_gasto" => Some(Self::DeleteExpense),
            "eliminar_interesado" => Some(Self::DeleteInterest),
            _ => None,
        }
    }

    pub fn is_finalize(&self) -> bool {
        matches!(self, Self::FinalizeEvent)
    }

    pub fn required_phrase(&self) -> &'static str {
        if self.is_finalize() {
            "sí, finalízalo"
        } else {
            "sí, elimínalo"
        }
    }

    pub fn default_consequence(&self) -> &'static str {
        match self {
            Self::DeletePresentation => {
                "eliminará permanentemente la presentación y todos sus elementos (canciones, letras y notas)"
            }
            Self::DeleteItem => {
                "eliminará permanentemente el elemento de la presentación (y su audio asociado si aplica)"
            }
            Self::DeleteEvent => {
                "eliminará permanentemente el evento y todas sus entradas vendidas, gastos, interesados y su afiche promocional"
            }
            Self::FinalizeEvent => {
                "cerrará definitivamente la venta de entradas y bloqueará modificaciones operativas en el evento"
            }
            Self::DeleteTicket => "anulará y eliminará definitivamente esta entrada emitida",
            Self::DeleteExpense => {
                "eliminará permanentemente este registro de gasto y actualizará el resumen financiero del evento"
            }
            Self::DeleteInterest => "eliminará definitivamente este interesado/lead del evento",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DestructiveTarget {
    Presentation {
        id: String,
    },
    ShowItem {
        presentation_id: String,
        item_id: String,
    },
    Event {
        id: String,
    },
    Ticket {
        id: String,
    },
    Expense {
        id: String,
    },
    Interest {
        id: String,
    },
}

impl DestructiveTarget {
    pub fn id_label(&self) -> &str {
        match self {
            Self::Presentation { id } => id,
            Self::ShowItem { item_id, .. } => item_id,
            Self::Event { id } => id,
            Self::Ticket { id } => id,
            Self::Expense { id } => id,
            Self::Interest { id } => id,
        }
    }

    pub fn to_canonical_json_value(&self) -> serde_json::Value {
        match self {
            Self::Presentation { id } => serde_json::json!({ "presentacion_id": id }),
            Self::ShowItem {
                presentation_id,
                item_id,
            } => serde_json::json!({
                "presentacion_id": presentation_id,
                "elemento_id": item_id,
            }),
            Self::Event { id } => serde_json::json!({ "evento_id": id }),
            Self::Ticket { id } => serde_json::json!({ "entrada_id": id }),
            Self::Expense { id } => serde_json::json!({ "gasto_id": id }),
            Self::Interest { id } => serde_json::json!({ "interesado_id": id }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PendingDestructiveAction {
    pub tool: DestructiveTool,
    pub target: DestructiveTarget,
    pub target_label: String,
    pub created_at: Instant,
}

impl PendingDestructiveAction {
    pub fn new(tool: DestructiveTool, target: DestructiveTarget, target_label: String) -> Self {
        Self {
            tool,
            target,
            target_label,
            created_at: Instant::now(),
        }
    }

    pub fn with_created_at(
        tool: DestructiveTool,
        target: DestructiveTarget,
        target_label: String,
        created_at: Instant,
    ) -> Self {
        Self {
            tool,
            target,
            target_label,
            created_at,
        }
    }

    pub fn is_expired(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.created_at) > PENDING_DESTRUCTIVE_TTL
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationReply {
    ExactDelete,
    ExactFinalize,
    Cancel,
    Ambiguous,
    Other,
}

impl ConfirmationReply {
    pub fn classify(text: &str) -> Self {
        let trimmed = text.trim();
        let lower = trimmed.to_lowercase();

        // SOLO frases exactas canónicas:
        // "sí, elimínalo" o "sí elimínalo"
        // "sí, finalízalo" o "sí finalízalo"
        // Rechaza explícitamente "si" sin tilde, "eliminalo"/"finalizalo" sin tilde, y puntuación extra.
        if lower == "sí, elimínalo" || lower == "sí elimínalo" {
            return Self::ExactDelete;
        }
        if lower == "sí, finalízalo" || lower == "sí finalízalo" {
            return Self::ExactFinalize;
        }

        let stripped = lower
            .trim_matches(|c: char| {
                c == '.' || c == '!' || c == '¡' || c == '?' || c == '¿' || c == '"' || c == '\''
            })
            .trim();

        // Cancellation phrases
        // "cancela", "cancelar", "mejor no", "déjalo así", "espera", "ya no", "no lo hagas", "déjalo"
        let norm_cancel = stripped
            .replace('á', "a")
            .replace('é', "e")
            .replace('í', "i")
            .replace('ó', "o")
            .replace('ú', "u");

        if norm_cancel == "cancela"
            || norm_cancel == "cancelar"
            || norm_cancel == "mejor no"
            || norm_cancel == "dejalo asi"
            || norm_cancel == "dejalo"
            || norm_cancel == "espera"
            || norm_cancel == "ya no"
            || norm_cancel == "no lo hagas"
            || norm_cancel == "no hagas nada"
            || norm_cancel.starts_with("cancela")
            || norm_cancel.starts_with("mejor no")
            || norm_cancel.starts_with("dejalo asi")
        {
            return Self::Cancel;
        }

        // Ambiguous phrases:
        // "dale", "ok", "hazlo", "bueno", "procede", "adelante", "va", "si", "sí", "elimínalo", "finalízalo"
        if norm_cancel == "dale"
            || norm_cancel == "ok"
            || norm_cancel == "hazlo"
            || norm_cancel == "bueno"
            || norm_cancel == "procede"
            || norm_cancel == "adelante"
            || norm_cancel == "dale pues"
            || norm_cancel == "va"
            || norm_cancel == "si"
            || norm_cancel == "eliminalo"
            || norm_cancel == "finalizalo"
            || norm_cancel == "si eliminalo"
            || norm_cancel == "si, eliminalo"
            || norm_cancel == "si finalizalo"
            || norm_cancel == "si, finalizalo"
        {
            return Self::Ambiguous;
        }

        // Also classify any near-confirmation (e.g. unaccented or punctuated) as Ambiguous
        let norm_words: Vec<&str> = stripped.split_whitespace().collect();
        if norm_words.len() == 2 {
            let w0 = norm_words[0].trim_end_matches(',');
            let w1 = norm_words[1];
            let is_near_si = w0 == "sí" || w0 == "si";
            let is_near_elim = w1 == "elimínalo" || w1 == "eliminalo";
            let is_near_fin = w1 == "finalízalo" || w1 == "finalizalo";
            if is_near_si && (is_near_elim || is_near_fin) {
                return Self::Ambiguous;
            }
        }

        Self::Other
    }
}

#[derive(Debug, Default)]
pub struct PendingDestructiveRegistry {
    entries: Mutex<HashMap<String, PendingDestructiveAction>>,
}

impl PendingDestructiveRegistry {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn arm(&self, conversation_id: String, action: PendingDestructiveAction) {
        let mut guard = self.entries.lock().unwrap();
        guard.insert(conversation_id, action);
    }

    pub fn take(&self, conversation_id: &str) -> Option<PendingDestructiveAction> {
        let mut guard = self.entries.lock().unwrap();
        if let Some(action) = guard.remove(conversation_id) {
            if !action.is_expired(Instant::now()) {
                return Some(action);
            }
        }
        None
    }

    pub fn peek(&self, conversation_id: &str) -> Option<PendingDestructiveAction> {
        let guard = self.entries.lock().unwrap();
        if let Some(action) = guard.get(conversation_id) {
            if !action.is_expired(Instant::now()) {
                return Some(action.clone());
            }
        }
        None
    }

    pub fn clear(&self, conversation_id: &str) {
        let mut guard = self.entries.lock().unwrap();
        guard.remove(conversation_id);
    }

    pub fn purge_expired(&self) {
        let mut guard = self.entries.lock().unwrap();
        let now = Instant::now();
        guard.retain(|_, action| !action.is_expired(now));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confirmation_reply_exact_phrases_and_rejections() {
        // EXACT POSITIVE: must classify as ExactDelete
        assert_eq!(
            ConfirmationReply::classify("sí, elimínalo"),
            ConfirmationReply::ExactDelete
        );
        assert_eq!(
            ConfirmationReply::classify("sí elimínalo"),
            ConfirmationReply::ExactDelete
        );
        assert_eq!(
            ConfirmationReply::classify("Sí, elimínalo"),
            ConfirmationReply::ExactDelete
        );
        assert_eq!(
            ConfirmationReply::classify("SÍ, ELIMÍNALO"),
            ConfirmationReply::ExactDelete
        );
        assert_eq!(
            ConfirmationReply::classify("  sí, elimínalo  "),
            ConfirmationReply::ExactDelete
        );

        // EXACT POSITIVE: must classify as ExactFinalize
        assert_eq!(
            ConfirmationReply::classify("sí, finalízalo"),
            ConfirmationReply::ExactFinalize
        );
        assert_eq!(
            ConfirmationReply::classify("sí finalízalo"),
            ConfirmationReply::ExactFinalize
        );
        assert_eq!(
            ConfirmationReply::classify("Sí, finalízalo"),
            ConfirmationReply::ExactFinalize
        );
        assert_eq!(
            ConfirmationReply::classify("SÍ, FINALÍZALO"),
            ConfirmationReply::ExactFinalize
        );

        // NEGATIVE: Missing accent on "si" -> REJECTED from ExactDelete / ExactFinalize
        assert_ne!(
            ConfirmationReply::classify("si, elimínalo"),
            ConfirmationReply::ExactDelete
        );
        assert_ne!(
            ConfirmationReply::classify("si elimínalo"),
            ConfirmationReply::ExactDelete
        );
        assert_ne!(
            ConfirmationReply::classify("si, finalízalo"),
            ConfirmationReply::ExactFinalize
        );
        assert_ne!(
            ConfirmationReply::classify("si finalízalo"),
            ConfirmationReply::ExactFinalize
        );

        // NEGATIVE: Missing accent on "eliminalo" / "finalizalo" -> REJECTED
        assert_ne!(
            ConfirmationReply::classify("sí, eliminalo"),
            ConfirmationReply::ExactDelete
        );
        assert_ne!(
            ConfirmationReply::classify("sí eliminalo"),
            ConfirmationReply::ExactDelete
        );
        assert_ne!(
            ConfirmationReply::classify("si, eliminalo"),
            ConfirmationReply::ExactDelete
        );
        assert_ne!(
            ConfirmationReply::classify("si eliminalo"),
            ConfirmationReply::ExactDelete
        );
        assert_ne!(
            ConfirmationReply::classify("sí, finalizalo"),
            ConfirmationReply::ExactFinalize
        );
        assert_ne!(
            ConfirmationReply::classify("sí finalizalo"),
            ConfirmationReply::ExactFinalize
        );

        // NEGATIVE: Extra punctuation -> REJECTED
        assert_ne!(
            ConfirmationReply::classify("sí, elimínalo!"),
            ConfirmationReply::ExactDelete
        );
        assert_ne!(
            ConfirmationReply::classify("¡sí, elimínalo!"),
            ConfirmationReply::ExactDelete
        );
        assert_ne!(
            ConfirmationReply::classify("sí, elimínalo."),
            ConfirmationReply::ExactDelete
        );
        assert_ne!(
            ConfirmationReply::classify("¿sí, elimínalo?"),
            ConfirmationReply::ExactDelete
        );
        assert_ne!(
            ConfirmationReply::classify("sí, finalízalo!"),
            ConfirmationReply::ExactFinalize
        );
        assert_ne!(
            ConfirmationReply::classify("¡sí, finalízalo!"),
            ConfirmationReply::ExactFinalize
        );

        // NEGATIVE: Partial phrases
        assert_ne!(
            ConfirmationReply::classify("sí"),
            ConfirmationReply::ExactDelete
        );
        assert_ne!(
            ConfirmationReply::classify("si"),
            ConfirmationReply::ExactDelete
        );
        assert_ne!(
            ConfirmationReply::classify("elimínalo"),
            ConfirmationReply::ExactDelete
        );
        assert_ne!(
            ConfirmationReply::classify("finalízalo"),
            ConfirmationReply::ExactFinalize
        );
    }
}
