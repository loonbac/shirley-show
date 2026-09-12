use chrono::Utc;
use std::sync::Arc;

use crate::domain::ticketing::Event;
use crate::error::AppError;
use crate::ports::ticketing::{EventRepository, ImageFormat, MAX_POSTER_SIZE_BYTES, PosterStorage};

pub struct ReplaceEventPoster<E: EventRepository, P: PosterStorage> {
    event_repo: Arc<E>,
    poster_storage: Arc<P>,
}

impl<E: EventRepository, P: PosterStorage> ReplaceEventPoster<E, P> {
    pub fn new(event_repo: Arc<E>, poster_storage: Arc<P>) -> Self {
        Self {
            event_repo,
            poster_storage,
        }
    }

    pub fn execute(&self, evento_id: &str, data: &[u8]) -> Result<String, AppError> {
        let event = self
            .event_repo
            .get_event(evento_id)?
            .ok_or_else(|| AppError::not_found("event_not_found", "Evento no encontrado"))?;

        if data.len() > MAX_POSTER_SIZE_BYTES {
            return Err(AppError::payload_too_large(
                "poster_too_large",
                "El afiche excede el límite máximo de 10 MiB",
            ));
        }

        let new_filename = self.poster_storage.store_poster(evento_id, data)?;

        let now = Utc::now().to_rfc3339();
        let updated_event = Event::new(
            event.id.clone(),
            event.nombre.clone(),
            event.fecha_hora.clone(),
            event.lugar.clone(),
            event.tipos_de_entrada.clone(),
            event.finalizado,
            Some(new_filename.clone()),
            event.fecha_creacion.clone(),
            now,
        )?;

        if let Err(e) = self.event_repo.update_event(&updated_event) {
            let _ = self.poster_storage.delete_poster(&new_filename);
            return Err(e);
        }

        if let Some(ref old_filename) = event.imagen {
            match self.poster_storage.delete_poster(old_filename) {
                Ok(_) => {}
                Err(e) => {
                    let _ = self.event_repo.update_event(&event);
                    let _ = self.poster_storage.delete_poster(&new_filename);
                    return Err(AppError::Internal(format!(
                        "Fallo al retirar afiche anterior '{old_filename}', reemplazo revertido: {e}"
                    )));
                }
            }
        }

        Ok(new_filename)
    }
}

pub struct GetEventPoster<E: EventRepository, P: PosterStorage> {
    event_repo: Arc<E>,
    poster_storage: Arc<P>,
}

impl<E: EventRepository, P: PosterStorage> GetEventPoster<E, P> {
    pub fn new(event_repo: Arc<E>, poster_storage: Arc<P>) -> Self {
        Self {
            event_repo,
            poster_storage,
        }
    }

    pub fn execute(&self, evento_id: &str) -> Result<(Vec<u8>, ImageFormat), AppError> {
        let event = self
            .event_repo
            .get_event(evento_id)?
            .ok_or_else(|| AppError::not_found("event_not_found", "Evento no encontrado"))?;

        let filename = event.imagen.ok_or_else(|| {
            AppError::not_found("poster_not_found", "El evento no tiene afiche asignado")
        })?;

        self.poster_storage.get_poster(&filename)?.ok_or_else(|| {
            AppError::not_found("poster_not_found", "Archivo de afiche no encontrado")
        })
    }
}
