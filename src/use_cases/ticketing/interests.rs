use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::ticketing::interest::{Interest, InterestStatus};
use crate::error::AppError;
use crate::ports::ticketing::InterestRepository;

#[derive(Debug, Clone)]
pub struct CreateInterestInput {
    pub evento_id: String,
    pub nombre: String,
    pub contacto: String,
    pub canal: String,
    pub promotora: String,
    pub personas: u32,
}

pub struct CreateInterest<R: InterestRepository> {
    repo: Arc<R>,
}

impl<R: InterestRepository> CreateInterest<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub fn execute(&self, input: CreateInterestInput) -> Result<Interest, AppError> {
        let now = Utc::now().to_rfc3339();
        let interest = Interest::new(
            Uuid::new_v4().to_string(),
            input.evento_id,
            input.nombre,
            input.contacto,
            input.canal,
            input.promotora,
            input.personas,
            InterestStatus::Pendiente,
            None,
            now.clone(),
            now,
        )?;

        self.repo.create_interest(&interest)?;
        Ok(interest)
    }
}

pub struct ListInterests<R: InterestRepository> {
    repo: Arc<R>,
}

impl<R: InterestRepository> ListInterests<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub fn execute(
        &self,
        evento_id: &str,
        estado: Option<&str>,
    ) -> Result<Vec<Interest>, AppError> {
        self.repo.list_interests(evento_id, estado)
    }
}

pub struct GetInterest<R: InterestRepository> {
    repo: Arc<R>,
}

impl<R: InterestRepository> GetInterest<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub fn execute(&self, id: &str) -> Result<Interest, AppError> {
        self.repo
            .get_interest(id)?
            .ok_or_else(|| AppError::not_found("interest_not_found", "Interesado no encontrado"))
    }
}

#[derive(Debug, Clone)]
pub struct UpdateInterestInput {
    pub id: String,
    pub nombre: String,
    pub contacto: String,
    pub canal: String,
    pub promotora: String,
    pub personas: u32,
    pub estado: InterestStatus,
}

pub struct UpdateInterest<R: InterestRepository> {
    repo: Arc<R>,
}

impl<R: InterestRepository> UpdateInterest<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub fn execute(&self, input: UpdateInterestInput) -> Result<Interest, AppError> {
        let existing = self
            .repo
            .get_interest(&input.id)?
            .ok_or_else(|| AppError::not_found("interest_not_found", "Interesado no encontrado"))?;

        let now = Utc::now().to_rfc3339();
        let updated = Interest::new(
            input.id,
            existing.evento_id,
            input.nombre,
            input.contacto,
            input.canal,
            input.promotora,
            input.personas,
            input.estado,
            existing.convertido_at,
            existing.fecha_creacion,
            now,
        )?;

        self.repo.update_interest(&updated)?;
        Ok(updated)
    }
}

pub struct DeleteInterest<R: InterestRepository> {
    repo: Arc<R>,
}

impl<R: InterestRepository> DeleteInterest<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub fn execute(&self, id: &str) -> Result<bool, AppError> {
        let deleted = self.repo.delete_interest(id)?;
        if !deleted {
            return Err(AppError::not_found(
                "interest_not_found",
                "Interesado no encontrado",
            ));
        }
        Ok(true)
    }
}

use crate::domain::ticketing::ticket::{PaymentMethod, PaymentStatus, Ticket};
use crate::ports::ticketing::{CredentialGenerator, TicketingTransactions};

#[derive(Debug, Clone)]
pub struct ConvertInterestInput {
    pub interesado_id: String,
    pub tipo_entrada: String,
    pub cantidad: u32,
    pub medio_pago: PaymentMethod,
    pub estado_pago: PaymentStatus,
}

pub struct ConvertInterestToSale<
    R: InterestRepository,
    T: TicketingTransactions,
    C: CredentialGenerator,
> {
    _repo: Arc<R>,
    tx: Arc<T>,
    creds_gen: Arc<C>,
}

impl<R: InterestRepository, T: TicketingTransactions, C: CredentialGenerator>
    ConvertInterestToSale<R, T, C>
{
    pub fn new(repo: Arc<R>, tx: Arc<T>, creds_gen: Arc<C>) -> Self {
        Self {
            _repo: repo,
            tx,
            creds_gen,
        }
    }

    pub fn execute(&self, input: ConvertInterestInput) -> Result<Vec<Ticket>, AppError> {
        if input.cantidad < 1 {
            return Err(AppError::validation(
                "invalid_quantity",
                "La cantidad de entradas debe ser de al menos 1",
                "cantidad",
            ));
        }

        let mut credentials = Vec::with_capacity(input.cantidad as usize);
        for _ in 0..input.cantidad {
            credentials.push(self.creds_gen.generate()?);
        }

        self.tx.convert_interest(
            &input.interesado_id,
            &input.tipo_entrada,
            input.cantidad,
            input.medio_pago,
            input.estado_pago,
            credentials,
        )
    }
}
