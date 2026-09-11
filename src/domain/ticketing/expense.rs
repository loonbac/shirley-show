use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::domain::ticketing::money::Money;
use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExpenseCategory {
    #[serde(rename = "Local/Alquiler")]
    LocalAlquiler,
    #[serde(rename = "Sonido/Luces")]
    SonidoLuces,
    #[serde(rename = "Músicos/Personal")]
    MusicosPersonal,
    #[serde(rename = "Publicidad/Marketing")]
    PublicidadMarketing,
    #[serde(rename = "Bebidas/Catering")]
    BebidasCatering,
    #[serde(rename = "Transporte")]
    Transporte,
    #[serde(rename = "Otros")]
    Otros,
}

impl ExpenseCategory {
    pub const ALL: &'static [Self] = &[
        Self::LocalAlquiler,
        Self::SonidoLuces,
        Self::MusicosPersonal,
        Self::PublicidadMarketing,
        Self::BebidasCatering,
        Self::Transporte,
        Self::Otros,
    ];

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::LocalAlquiler => "Local/Alquiler",
            Self::SonidoLuces => "Sonido/Luces",
            Self::MusicosPersonal => "Músicos/Personal",
            Self::PublicidadMarketing => "Publicidad/Marketing",
            Self::BebidasCatering => "Bebidas/Catering",
            Self::Transporte => "Transporte",
            Self::Otros => "Otros",
        }
    }

    pub fn parse(s: &str) -> Result<Self, AppError> {
        match s {
            "Local/Alquiler" => Ok(Self::LocalAlquiler),
            "Sonido/Luces" => Ok(Self::SonidoLuces),
            "Músicos/Personal" => Ok(Self::MusicosPersonal),
            "Publicidad/Marketing" => Ok(Self::PublicidadMarketing),
            "Bebidas/Catering" => Ok(Self::BebidasCatering),
            "Transporte" => Ok(Self::Transporte),
            "Otros" => Ok(Self::Otros),
            _ => Err(AppError::validation(
                "invalid_expense_category",
                format!("Categoría de gasto inválida: {s}"),
                "categoria",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Expense {
    pub id: String,
    pub evento_id: String,
    pub categoria: ExpenseCategory,
    pub monto: Money,
    pub descripcion: String,
    pub fecha: String,
    pub fecha_creacion: String,
    pub fecha_actualizacion: String,
}

impl Expense {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        evento_id: String,
        categoria: ExpenseCategory,
        monto: Money,
        descripcion: String,
        fecha: String,
        fecha_creacion: String,
        fecha_actualizacion: String,
    ) -> Result<Self, AppError> {
        let descripcion = descripcion.trim().to_string();
        if descripcion.is_empty() {
            return Err(AppError::validation(
                "empty_description",
                "La descripción del gasto no puede estar vacía",
                "descripcion",
            ));
        }

        NaiveDate::parse_from_str(&fecha, "%Y-%m-%d").map_err(|e| {
            AppError::validation(
                "invalid_date",
                format!("Fecha inválida (debe ser YYYY-MM-DD): {e}"),
                "fecha",
            )
        })?;

        Ok(Self {
            id,
            evento_id,
            categoria,
            monto,
            descripcion,
            fecha,
            fecha_creacion,
            fecha_actualizacion,
        })
    }
}
