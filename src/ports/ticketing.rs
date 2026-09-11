use serde::{Deserialize, Serialize};

use crate::domain::ticketing::ticket::{PaymentMethod, PaymentStatus, Ticket};
use crate::domain::ticketing::{Event, EventSummary, Expense, QrSecret, ScanToken};
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct GeneratedCredentials {
    pub id: String,
    pub scan_token: ScanToken,
    pub qr_secret: QrSecret,
}

pub trait CredentialGenerator: Send + Sync {
    fn generate(&self) -> Result<GeneratedCredentials, AppError>;
}

#[derive(Debug, Clone)]
pub struct SellTicketsInput {
    pub evento_id: String,
    pub interesado_id: Option<String>,
    pub nombre_comprador: String,
    pub contacto: String,
    pub tipo_entrada: String,
    pub cantidad: u32,
    pub medio_pago: PaymentMethod,
    pub estado_pago: PaymentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "resultado")]
pub enum CheckInOutcome {
    #[serde(rename = "Valida")]
    Valida {
        nombre_comprador: String,
        tipo_entrada: String,
        hora_ingreso: String,
    },
    #[serde(rename = "Ya Usada")]
    YaUsada {
        hora_ingreso: String,
    },
    #[serde(rename = "Invalida")]
    Invalida {
        motivo: String,
    },
}

pub trait TicketRepository: Send + Sync {
    fn list_tickets(
        &self,
        evento_id: &str,
        q: Option<&str>,
        estado_pago: Option<&str>,
        tipo_entrada: Option<&str>,
    ) -> Result<Vec<Ticket>, AppError>;
    fn get_ticket(&self, id: &str) -> Result<Option<Ticket>, AppError>;
    fn get_ticket_by_scan_token(&self, scan_token: &str) -> Result<Option<Ticket>, AppError>;
    fn get_ticket_by_qr_secret(&self, qr_secret: &str) -> Result<Option<Ticket>, AppError>;
    fn update_ticket_contact(
        &self,
        id: &str,
        nombre_comprador: &str,
        contacto: &str,
    ) -> Result<Option<Ticket>, AppError>;
    fn confirm_ticket_payment(&self, id: &str) -> Result<Option<Ticket>, AppError>;
    fn delete_ticket(&self, id: &str) -> Result<bool, AppError>;
}

pub trait TicketingTransactions: Send + Sync {
    fn issue_sale(
        &self,
        input: &SellTicketsInput,
        credentials: Vec<GeneratedCredentials>,
    ) -> Result<Vec<Ticket>, AppError>;

    fn check_in(
        &self,
        evento_id: &str,
        codigo_qr: &str,
        now: &str,
    ) -> Result<CheckInOutcome, AppError>;
}

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
