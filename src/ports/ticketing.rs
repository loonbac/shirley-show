use crate::domain::ticketing::{Event, EventSummary, Expense};
use crate::error::AppError;

pub trait EventRepository: Send + Sync {
    fn create_event(&self, event: &Event) -> Result<(), AppError>;
    fn list_events(&self) -> Result<Vec<Event>, AppError>;
    fn get_event(&self, id: &str) -> Result<Option<Event>, AppError>;
    fn update_event(&self, event: &Event) -> Result<(), AppError>;
    fn set_finalizado(&self, id: &str, finalizado: bool) -> Result<(), AppError>;
    fn delete_event(&self, id: &str) -> Result<bool, AppError>;
}

pub trait ExpenseRepository: Send + Sync {
    fn create_expense(&self, expense: &Expense) -> Result<(), AppError>;
    fn list_expenses(&self, evento_id: &str) -> Result<Vec<Expense>, AppError>;
    fn get_expense(&self, id: &str) -> Result<Option<Expense>, AppError>;
    fn update_expense(&self, expense: &Expense) -> Result<(), AppError>;
    fn delete_expense(&self, id: &str) -> Result<bool, AppError>;
}

pub trait EventSummaryReader: Send + Sync {
    fn get_event_summary(&self, evento_id: &str) -> Result<EventSummary, AppError>;
}
