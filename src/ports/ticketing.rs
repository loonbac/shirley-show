use serde::{Deserialize, Serialize};

use crate::domain::ticketing::ticket::{PaymentMethod, PaymentStatus, Ticket};
use crate::domain::ticketing::{Event, EventSummary, Expense, Interest, QrSecret, ScanToken};
use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Webp,
}

impl ImageFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Webp => "webp",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Webp => "image/webp",
        }
    }

    pub fn detect(bytes: &[u8]) -> Result<Self, AppError> {
        if bytes.len() >= 3 && bytes[0..3] == [0xFF, 0xD8, 0xFF] {
            return Ok(Self::Jpeg);
        }
        if bytes.len() >= 8 && bytes[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
            return Ok(Self::Png);
        }
        if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            return Ok(Self::Webp);
        }
        Err(AppError::unsupported_media_type(
            "unsupported_image_format",
            "Formato de imagen no soportado. Debe ser JPEG, PNG o WebP",
        ))
    }
}

pub trait PosterStorage: Send + Sync {
    fn store_poster(&self, evento_id: &str, data: &[u8]) -> Result<String, AppError>;
    fn get_poster(&self, filename: &str) -> Result<Option<(Vec<u8>, ImageFormat)>, AppError>;
    fn delete_poster(&self, filename: &str) -> Result<bool, AppError>;
}

#[derive(Debug, Default, Clone)]
pub struct NoopPosterStorage;

impl PosterStorage for NoopPosterStorage {
    fn store_poster(&self, _evento_id: &str, _data: &[u8]) -> Result<String, AppError> {
        Ok(String::new())
    }
    fn get_poster(&self, _filename: &str) -> Result<Option<(Vec<u8>, ImageFormat)>, AppError> {
        Ok(None)
    }
    fn delete_poster(&self, _filename: &str) -> Result<bool, AppError> {
        Ok(true)
    }
}

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
    YaUsada { hora_ingreso: String },
    #[serde(rename = "Invalida")]
    Invalida { motivo: String },
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

    fn convert_interest(
        &self,
        interesado_id: &str,
        tipo_entrada: &str,
        cantidad: u32,
        medio_pago: PaymentMethod,
        estado_pago: PaymentStatus,
        credentials: Vec<GeneratedCredentials>,
    ) -> Result<Vec<Ticket>, AppError>;

    fn check_in(
        &self,
        evento_id: &str,
        codigo_qr: &str,
        now: &str,
    ) -> Result<CheckInOutcome, AppError>;
}

pub trait InterestRepository: Send + Sync {
    fn create_interest(&self, interest: &Interest) -> Result<(), AppError>;
    fn list_interests(
        &self,
        evento_id: &str,
        estado: Option<&str>,
    ) -> Result<Vec<Interest>, AppError>;
    fn get_interest(&self, id: &str) -> Result<Option<Interest>, AppError>;
    fn update_interest(&self, interest: &Interest) -> Result<(), AppError>;
    fn delete_interest(&self, id: &str) -> Result<bool, AppError>;
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
