use chrono::DateTime;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::domain::ticketing::money::Money;
use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TicketType {
    pub nombre: String,
    pub precio: Money,
    pub capacidad: u32,
}

impl TicketType {
    pub fn new(nombre: impl Into<String>, precio: Money, capacidad: u32) -> Result<Self, AppError> {
        let trimmed = nombre.into().trim().to_string();
        if trimmed.is_empty() {
            return Err(AppError::validation(
                "empty_ticket_type_name",
                "El nombre del tipo de entrada no puede estar vacío",
                "nombre",
            ));
        }
        if capacidad == 0 {
            return Err(AppError::validation(
                "invalid_capacity",
                "La capacidad debe ser de al menos 1",
                "capacidad",
            ));
        }
        Ok(Self {
            nombre: trimmed,
            precio,
            capacidad,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub nombre: String,
    pub fecha_hora: String,
    pub lugar: String,
    pub tipos_de_entrada: Vec<TicketType>,
    pub finalizado: bool,
    pub imagen: Option<String>,
    pub fecha_creacion: String,
    pub fecha_actualizacion: String,
}

impl Event {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        nombre: String,
        fecha_hora: String,
        lugar: String,
        tipos_de_entrada: Vec<TicketType>,
        finalizado: bool,
        imagen: Option<String>,
        fecha_creacion: String,
        fecha_actualizacion: String,
    ) -> Result<Self, AppError> {
        let nombre = nombre.trim().to_string();
        if nombre.is_empty() {
            return Err(AppError::validation(
                "empty_name",
                "El nombre del evento no puede estar vacío",
                "nombre",
            ));
        }

        let lugar = lugar.trim().to_string();
        if lugar.is_empty() {
            return Err(AppError::validation(
                "empty_venue",
                "El lugar del evento no puede estar vacío",
                "lugar",
            ));
        }

        DateTime::parse_from_rfc3339(&fecha_hora).map_err(|e| {
            AppError::validation(
                "invalid_date",
                format!("Fecha y hora inválida (debe ser RFC3339): {e}"),
                "fecha_hora",
            )
        })?;

        if tipos_de_entrada.is_empty() {
            return Err(AppError::validation(
                "empty_ticket_types",
                "El evento debe tener al menos un tipo de entrada",
                "tipos_de_entrada",
            ));
        }

        let mut seen = HashSet::new();
        for t in &tipos_de_entrada {
            let lower = t.nombre.to_lowercase();
            if !seen.insert(lower) {
                return Err(AppError::validation(
                    "duplicate_ticket_type",
                    format!("Tipo de entrada duplicado: {}", t.nombre),
                    "tipos_de_entrada",
                ));
            }
        }

        Ok(Self {
            id,
            nombre,
            fecha_hora,
            lugar,
            tipos_de_entrada,
            finalizado,
            imagen,
            fecha_creacion,
            fecha_actualizacion,
        })
    }

    pub fn total_capacity(&self) -> u32 {
        self.tipos_de_entrada.iter().map(|t| t.capacidad).sum()
    }
}
