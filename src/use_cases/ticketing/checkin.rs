use std::sync::Arc;

use crate::error::AppError;
use crate::ports::ticketing::{CheckInOutcome, TicketingTransactions};

#[derive(Clone)]
pub struct CheckInTicket<T> {
    pub tx_repo: Arc<T>,
}

impl<T: TicketingTransactions> CheckInTicket<T> {
    pub fn new(tx_repo: Arc<T>) -> Self {
        Self { tx_repo }
    }

    pub fn execute(&self, evento_id: &str, codigo: &str) -> Result<CheckInOutcome, AppError> {
        let evento_id = evento_id.trim();
        if evento_id.is_empty() {
            return Err(AppError::bad_request(
                "empty_evento_id",
                "evento_id no puede estar vacío",
            ));
        }
        let codigo = codigo.trim();
        if codigo.is_empty() {
            return Err(AppError::bad_request(
                "empty_codigo",
                "codigo no puede estar vacío",
            ));
        }

        let now = chrono::Utc::now().to_rfc3339();
        self.tx_repo.check_in(evento_id, codigo, &now)
    }
}
