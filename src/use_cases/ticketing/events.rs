use std::sync::Arc;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::ticketing::{Event, TicketType};
use crate::error::AppError;
use crate::ports::ticketing::EventRepository;

#[derive(Debug, Clone)]
pub struct CreateEventInput {
    pub nombre: String,
    pub fecha_hora: String,
    pub lugar: String,
    pub tipos_de_entrada: Vec<TicketType>,
}

pub struct CreateEvent<R: EventRepository> {
    repo: Arc<R>,
}

impl<R: EventRepository> CreateEvent<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub fn execute(&self, input: CreateEventInput) -> Result<Event, AppError> {
        let now = Utc::now().to_rfc3339();
        let event = Event::new(
            Uuid::new_v4().to_string(),
            input.nombre,
            input.fecha_hora,
            input.lugar,
            input.tipos_de_entrada,
            false,
            None,
            now.clone(),
            now,
        )?;

        self.repo.create_event(&event)?;
        Ok(event)
    }
}

pub struct ListEvents<R: EventRepository> {
    repo: Arc<R>,
}

impl<R: EventRepository> ListEvents<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub fn execute(&self) -> Result<Vec<Event>, AppError> {
        self.repo.list_events()
    }
}

pub struct GetEvent<R: EventRepository> {
    repo: Arc<R>,
}

impl<R: EventRepository> GetEvent<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub fn execute(&self, id: &str) -> Result<Event, AppError> {
        self.repo
            .get_event(id)?
            .ok_or_else(|| AppError::not_found("event_not_found", "Evento no encontrado"))
    }
}

#[derive(Debug, Clone)]
pub struct UpdateEventInput {
    pub id: String,
    pub nombre: String,
    pub fecha_hora: String,
    pub lugar: String,
    pub tipos_de_entrada: Vec<TicketType>,
}

pub struct UpdateEvent<R: EventRepository> {
    repo: Arc<R>,
}

impl<R: EventRepository> UpdateEvent<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub fn execute(&self, input: UpdateEventInput) -> Result<Event, AppError> {
        let existing = self
            .repo
            .get_event(&input.id)?
            .ok_or_else(|| AppError::not_found("event_not_found", "Evento no encontrado"))?;

        let now = Utc::now().to_rfc3339();
        let updated = Event::new(
            input.id,
            input.nombre,
            input.fecha_hora,
            input.lugar,
            input.tipos_de_entrada,
            existing.finalizado,
            existing.imagen,
            existing.fecha_creacion,
            now,
        )?;

        self.repo.update_event(&updated)?;
        Ok(updated)
    }
}

pub struct FinalizeEvent<R: EventRepository> {
    repo: Arc<R>,
}

impl<R: EventRepository> FinalizeEvent<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub fn execute(&self, id: &str) -> Result<(), AppError> {
        self.repo.set_finalizado(id, true)
    }
}

pub struct ReopenEvent<R: EventRepository> {
    repo: Arc<R>,
}

impl<R: EventRepository> ReopenEvent<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub fn execute(&self, id: &str) -> Result<(), AppError> {
        self.repo.set_finalizado(id, false)
    }
}

pub struct DeleteEvent<R: EventRepository> {
    repo: Arc<R>,
}

impl<R: EventRepository> DeleteEvent<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub fn execute(&self, id: &str) -> Result<(), AppError> {
        let deleted = self.repo.delete_event(id)?;
        if !deleted {
            return Err(AppError::not_found("event_not_found", "Evento no encontrado"));
        }
        Ok(())
    }
}
