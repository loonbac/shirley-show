use std::sync::Arc;

use crate::domain::ticketing::EventSummary;
use crate::error::AppError;
use crate::ports::ticketing::EventSummaryReader;

pub struct GetEventSummary<R: EventSummaryReader> {
    reader: Arc<R>,
}

impl<R: EventSummaryReader> GetEventSummary<R> {
    pub fn new(reader: Arc<R>) -> Self {
        Self { reader }
    }

    pub fn execute(&self, evento_id: &str) -> Result<EventSummary, AppError> {
        self.reader.get_event_summary(evento_id)
    }
}
