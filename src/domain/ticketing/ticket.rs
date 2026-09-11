use qrcode::QrCode;
use qrcode::render::svg;
use serde::{Deserialize, Serialize};

use crate::domain::ticketing::Money;
use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScanToken(String);

impl ScanToken {
    pub fn new(token: impl Into<String>) -> Result<Self, AppError> {
        let t = token.into();
        let trimmed = t.trim();
        if trimmed.is_empty() {
            return Err(AppError::validation(
                "empty_scan_token",
                "scan_token no puede estar vacío",
                "scan_token",
            ));
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QrSecret(String);

impl QrSecret {
    pub fn new(secret: impl Into<String>) -> Result<Self, AppError> {
        let s = secret.into();
        let trimmed = s.trim();
        if trimmed.len() != 32 || !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(AppError::validation(
                "invalid_qr_secret",
                "codigo_qr debe ser una cadena hexadecimal de 32 caracteres",
                "codigo_qr",
            ));
        }
        Ok(Self(trimmed.to_ascii_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaymentMethod {
    #[serde(rename = "Yape")]
    Yape,
    #[serde(rename = "Efectivo")]
    Efectivo,
    #[serde(rename = "Cortesía")]
    Cortesia,
}

impl PaymentMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Yape => "Yape",
            Self::Efectivo => "Efectivo",
            Self::Cortesia => "Cortesía",
        }
    }

    pub fn parse(s: &str) -> Result<Self, AppError> {
        match s.trim() {
            "Yape" => Ok(Self::Yape),
            "Efectivo" => Ok(Self::Efectivo),
            "Cortesía" | "Cortesia" => Ok(Self::Cortesia),
            other => Err(AppError::validation(
                "invalid_payment_method",
                format!(
                    "medio_pago no válido: '{}'. Permitidos: 'Yape', 'Efectivo', 'Cortesía'",
                    other
                ),
                "medio_pago",
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaymentStatus {
    #[serde(rename = "Pagado")]
    Pagado,
    #[serde(rename = "Pendiente")]
    Pendiente,
}

impl PaymentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pagado => "Pagado",
            Self::Pendiente => "Pendiente",
        }
    }

    pub fn parse(s: &str) -> Result<Self, AppError> {
        match s.trim() {
            "Pagado" => Ok(Self::Pagado),
            "Pendiente" => Ok(Self::Pendiente),
            other => Err(AppError::validation(
                "invalid_payment_status",
                format!(
                    "estado_pago no válido: '{}'. Permitidos: 'Pagado', 'Pendiente'",
                    other
                ),
                "estado_pago",
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CheckinStatus {
    #[serde(rename = "No ingresó")]
    NoIngreso,
    #[serde(rename = "Ingresó")]
    Ingreso,
}

impl CheckinStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoIngreso => "No ingresó",
            Self::Ingreso => "Ingresó",
        }
    }

    pub fn parse(s: &str) -> Result<Self, AppError> {
        match s.trim() {
            "No ingresó" | "No ingreso" => Ok(Self::NoIngreso),
            "Ingresó" | "Ingreso" => Ok(Self::Ingreso),
            other => Err(AppError::validation(
                "invalid_checkin_status",
                format!(
                    "estado_checkin no válido: '{}'. Permitidos: 'No ingresó', 'Ingresó'",
                    other
                ),
                "estado_checkin",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ticket {
    pub id: String,
    pub evento_id: String,
    pub interesado_id: Option<String>,
    pub nombre_comprador: String,
    pub contacto: String,
    pub tipo_entrada: String,
    pub precio: Money,
    pub medio_pago: PaymentMethod,
    pub estado_pago: PaymentStatus,
    pub scan_token: ScanToken,
    pub codigo_qr: QrSecret,
    pub estado_checkin: CheckinStatus,
    pub hora_ingreso: Option<String>,
    pub fecha_creacion: String,
    pub fecha_actualizacion: String,
}

impl Ticket {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        evento_id: String,
        interesado_id: Option<String>,
        nombre_comprador: String,
        contacto: String,
        tipo_entrada: String,
        precio: Money,
        medio_pago: PaymentMethod,
        estado_pago: PaymentStatus,
        scan_token: ScanToken,
        codigo_qr: QrSecret,
        estado_checkin: CheckinStatus,
        hora_ingreso: Option<String>,
        fecha_creacion: String,
        fecha_actualizacion: String,
    ) -> Result<Self, AppError> {
        let id = id.trim().to_string();
        if id.is_empty() {
            return Err(AppError::validation(
                "empty_ticket_id",
                "id de entrada no puede estar vacío",
                "id",
            ));
        }
        let evento_id = evento_id.trim().to_string();
        if evento_id.is_empty() {
            return Err(AppError::validation(
                "empty_evento_id",
                "evento_id de entrada no puede estar vacío",
                "evento_id",
            ));
        }
        let nombre_comprador = nombre_comprador.trim().to_string();
        if nombre_comprador.is_empty() {
            return Err(AppError::validation(
                "empty_nombre_comprador",
                "nombre_comprador no puede estar vacío",
                "nombre_comprador",
            ));
        }
        let contacto = contacto.trim().to_string();
        if contacto.is_empty() {
            return Err(AppError::validation(
                "empty_contacto",
                "contacto no puede estar vacío",
                "contacto",
            ));
        }
        let tipo_entrada = tipo_entrada.trim().to_string();
        if tipo_entrada.is_empty() {
            return Err(AppError::validation(
                "empty_tipo_entrada",
                "tipo_entrada no puede estar vacío",
                "tipo_entrada",
            ));
        }

        // Cortesia invariants: precio MUST be zero, estado_pago MUST be Pagado
        if medio_pago == PaymentMethod::Cortesia {
            if precio != Money::zero() {
                return Err(AppError::validation(
                    "invalid_courtesy_price",
                    "Una entrada de Cortesía debe tener precio 0.0",
                    "precio",
                ));
            }
            if estado_pago != PaymentStatus::Pagado {
                return Err(AppError::validation(
                    "invalid_courtesy_status",
                    "Una entrada de Cortesía debe tener estado Pagado",
                    "estado_pago",
                ));
            }
        }

        // Checkin consistency:
        // (estado_checkin == 'No ingresó' AND hora_ingreso IS NULL)
        // OR (estado_checkin == 'Ingresó' AND hora_ingreso IS NOT NULL)
        match estado_checkin {
            CheckinStatus::NoIngreso => {
                if hora_ingreso.is_some() {
                    return Err(AppError::validation(
                        "invalid_checkin_state",
                        "Una entrada no ingresada no puede tener hora_ingreso",
                        "hora_ingreso",
                    ));
                }
            }
            CheckinStatus::Ingreso => {
                if hora_ingreso.is_none() {
                    return Err(AppError::validation(
                        "invalid_checkin_state",
                        "Una entrada ingresada debe tener hora_ingreso registrada",
                        "hora_ingreso",
                    ));
                }
            }
        }

        Ok(Self {
            id,
            evento_id,
            interesado_id: interesado_id.map(|s| s.trim().to_string()),
            nombre_comprador,
            contacto,
            tipo_entrada,
            precio,
            medio_pago,
            estado_pago,
            scan_token,
            codigo_qr,
            estado_checkin,
            hora_ingreso,
            fecha_creacion,
            fecha_actualizacion,
        })
    }

    pub fn confirm_payment(&mut self, now: String) {
        if self.estado_pago == PaymentStatus::Pendiente {
            self.estado_pago = PaymentStatus::Pagado;
            self.fecha_actualizacion = now;
        }
    }

    pub fn render_qr_svg(&self) -> Option<String> {
        if self.estado_pago != PaymentStatus::Pagado {
            return None;
        }

        let code = QrCode::new(self.codigo_qr.as_str().as_bytes()).ok()?;
        let image = code
            .render::<svg::Color>()
            .min_dimensions(200, 200)
            .quiet_zone(true)
            .build();
        Some(image)
    }
}
