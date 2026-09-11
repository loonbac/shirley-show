use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterestStatus {
    #[serde(rename = "Pendiente")]
    Pendiente,
    #[serde(rename = "Confirmado")]
    Confirmado,
    #[serde(rename = "Cancelado")]
    Cancelado,
}

impl InterestStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pendiente => "Pendiente",
            Self::Confirmado => "Confirmado",
            Self::Cancelado => "Cancelado",
        }
    }

    pub fn parse(s: &str) -> Result<Self, AppError> {
        match s.trim() {
            "Pendiente" => Ok(Self::Pendiente),
            "Confirmado" => Ok(Self::Confirmado),
            "Cancelado" => Ok(Self::Cancelado),
            _ => Err(AppError::validation(
                "invalid_interest_status",
                format!("Estado de interesado inválido: '{s}'"),
                "estado",
            )),
        }
    }
}

impl std::fmt::Display for InterestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Interest {
    pub id: String,
    pub evento_id: String,
    pub nombre: String,
    pub contacto: String,
    pub canal: String,
    pub promotora: String,
    pub personas: u32,
    pub estado: InterestStatus,
    pub convertido_at: Option<String>,
    pub fecha_creacion: String,
    pub fecha_actualizacion: String,
}

impl Interest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        evento_id: String,
        nombre: String,
        contacto: String,
        canal: String,
        promotora: String,
        personas: u32,
        estado: InterestStatus,
        convertido_at: Option<String>,
        fecha_creacion: String,
        fecha_actualizacion: String,
    ) -> Result<Self, AppError> {
        let nombre = nombre.trim().to_string();
        if nombre.is_empty() {
            return Err(AppError::validation(
                "empty_nombre",
                "El nombre del interesado no puede estar vacío",
                "nombre",
            ));
        }

        let contacto = contacto.trim().to_string();
        if contacto.is_empty() {
            return Err(AppError::validation(
                "empty_contacto",
                "El contacto del interesado no puede estar vacío",
                "contacto",
            ));
        }

        let canal = canal.trim().to_string();
        if canal.is_empty() {
            return Err(AppError::validation(
                "empty_canal",
                "El canal de comunicación no puede estar vacío",
                "canal",
            ));
        }

        let promotora = promotora.trim().to_string();
        if promotora.is_empty() {
            return Err(AppError::validation(
                "empty_promotora",
                "La promotora no puede estar vacía",
                "promotora",
            ));
        }

        if personas < 1 {
            return Err(AppError::validation(
                "invalid_personas",
                "La cantidad de personas debe ser mayor o igual a 1",
                "personas",
            ));
        }

        if convertido_at.is_some() && estado != InterestStatus::Confirmado {
            return Err(AppError::validation(
                "invalid_converted_state",
                "Un interesado convertido debe tener estado Confirmado",
                "estado",
            ));
        }

        Ok(Self {
            id,
            evento_id,
            nombre,
            contacto,
            canal,
            promotora,
            personas,
            estado,
            convertido_at,
            fecha_creacion,
            fecha_actualizacion,
        })
    }
}
