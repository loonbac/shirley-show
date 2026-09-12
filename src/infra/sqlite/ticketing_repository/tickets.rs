use rusqlite::OptionalExtension;

use crate::domain::ticketing::ticket::{
    CheckinStatus, PaymentMethod, PaymentStatus, QrSecret, ScanToken, Ticket,
};
use crate::domain::ticketing::{Money, TicketType};
use crate::error::AppError;
use crate::infra::sqlite::ticketing_repository::SqliteTicketingRepository;
use crate::ports::ticketing::{
    CheckInOutcome, GeneratedCredentials, SellTicketsInput, TicketRepository, TicketingTransactions,
};

pub const DEFAULT_TICKETS_LIMIT: usize = 500;

fn db_err(e: impl std::fmt::Display) -> AppError {
    AppError::Database(e.to_string())
}

fn int_err(e: impl std::fmt::Display) -> AppError {
    AppError::Internal(e.to_string())
}

fn row_to_ticket(row: &rusqlite::Row) -> Result<Ticket, rusqlite::Error> {
    let id: String = row.get(0)?;
    let evento_id: String = row.get(1)?;
    let interesado_id: Option<String> = row.get(2)?;
    let nombre_comprador: String = row.get(3)?;
    let contacto: String = row.get(4)?;
    let tipo_entrada: String = row.get(5)?;
    let precio_centimos: i64 = row.get(6)?;
    let medio_pago_str: String = row.get(7)?;
    let estado_pago_str: String = row.get(8)?;
    let codigo_qr_str: String = row.get(9)?;
    let scan_token_str: String = row.get(10)?;
    let estado_checkin_str: String = row.get(11)?;
    let hora_ingreso: Option<String> = row.get(12)?;
    let fecha_creacion: String = row.get(13)?;
    let fecha_actualizacion: String = row.get(14)?;

    let precio = Money::from_cents(precio_centimos);
    let medio_pago = PaymentMethod::parse(&medio_pago_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let estado_pago = PaymentStatus::parse(&estado_pago_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let scan_token = ScanToken::new(scan_token_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(10, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let codigo_qr = QrSecret::new(codigo_qr_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(9, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let estado_checkin = CheckinStatus::parse(&estado_checkin_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(11, rusqlite::types::Type::Text, Box::new(e))
    })?;

    Ticket::new(
        id,
        evento_id,
        interesado_id,
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
    )
    .map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

const TICKET_SELECT_COLUMNS: &str = "id, evento_id, interesado_id, nombre_comprador, contacto, tipo_entrada, precio_centimos, medio_pago, estado_pago, codigo_qr, scan_token, estado_checkin, hora_ingreso, fecha_creacion, fecha_actualizacion";

impl TicketRepository for SqliteTicketingRepository {
    fn list_tickets(
        &self,
        evento_id: &str,
        q: Option<&str>,
        estado_pago: Option<&str>,
        tipo_entrada: Option<&str>,
    ) -> Result<Vec<Ticket>, AppError> {
        let conn = self.pool.get().map_err(db_err)?;

        let mut sql = format!(
            "SELECT {} FROM entradas WHERE evento_id = ?1",
            TICKET_SELECT_COLUMNS
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> =
            vec![Box::new(evento_id.to_string())];

        if let Some(query) = q {
            let trimmed = query.trim();
            if !trimmed.is_empty() {
                let param_idx = params.len() + 1;
                sql.push_str(&format!(
                    " AND (nombre_comprador LIKE ?{0} COLLATE NOCASE OR contacto LIKE ?{0} COLLATE NOCASE)",
                    param_idx
                ));
                params.push(Box::new(format!("%{}%", trimmed)));
            }
        }

        if let Some(pago) = estado_pago {
            let trimmed = pago.trim();
            if !trimmed.is_empty() {
                let param_idx = params.len() + 1;
                sql.push_str(&format!(" AND estado_pago = ?{}", param_idx));
                params.push(Box::new(trimmed.to_string()));
            }
        }

        if let Some(tipo) = tipo_entrada {
            let trimmed = tipo.trim();
            if !trimmed.is_empty() {
                let param_idx = params.len() + 1;
                sql.push_str(&format!(" AND tipo_entrada = ?{}", param_idx));
                params.push(Box::new(trimmed.to_string()));
            }
        }

        sql.push_str(&format!(
            " ORDER BY fecha_creacion DESC, id DESC LIMIT {DEFAULT_TICKETS_LIMIT}"
        ));

        let mut stmt = conn.prepare(&sql).map_err(db_err)?;
        let rusqlite_params: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|b| b.as_ref()).collect();

        let rows = stmt
            .query_map(&rusqlite_params[..], row_to_ticket)
            .map_err(db_err)?;

        let mut tickets = Vec::new();
        for row in rows {
            tickets.push(row.map_err(db_err)?);
        }
        Ok(tickets)
    }

    fn get_ticket(&self, id: &str) -> Result<Option<Ticket>, AppError> {
        let conn = self.pool.get().map_err(db_err)?;
        let sql = format!(
            "SELECT {} FROM entradas WHERE id = ?1",
            TICKET_SELECT_COLUMNS
        );
        conn.query_row(&sql, [id], row_to_ticket)
            .optional()
            .map_err(db_err)
    }

    fn get_ticket_by_scan_token(&self, scan_token: &str) -> Result<Option<Ticket>, AppError> {
        let conn = self.pool.get().map_err(db_err)?;
        let sql = format!(
            "SELECT {} FROM entradas WHERE scan_token = ?1",
            TICKET_SELECT_COLUMNS
        );
        conn.query_row(&sql, [scan_token], row_to_ticket)
            .optional()
            .map_err(db_err)
    }

    fn get_ticket_by_qr_secret(&self, qr_secret: &str) -> Result<Option<Ticket>, AppError> {
        let conn = self.pool.get().map_err(db_err)?;
        let sql = format!(
            "SELECT {} FROM entradas WHERE codigo_qr = ?1",
            TICKET_SELECT_COLUMNS
        );
        conn.query_row(&sql, [qr_secret], row_to_ticket)
            .optional()
            .map_err(db_err)
    }

    fn update_ticket_contact(
        &self,
        id: &str,
        nombre_comprador: &str,
        contacto: &str,
    ) -> Result<Option<Ticket>, AppError> {
        let nombre = nombre_comprador.trim();
        if nombre.is_empty() {
            return Err(AppError::validation(
                "empty_nombre_comprador",
                "El nombre del comprador no puede estar vacío",
                "nombre_comprador",
            ));
        }
        let contacto = contacto.trim();
        if contacto.is_empty() {
            return Err(AppError::validation(
                "empty_contacto",
                "El contacto no puede estar vacío",
                "contacto",
            ));
        }

        let conn = self.pool.get().map_err(db_err)?;
        let now = chrono::Utc::now().to_rfc3339();

        let rows_affected = conn
            .execute(
                "UPDATE entradas SET nombre_comprador = ?1, contacto = ?2, fecha_actualizacion = ?3 WHERE id = ?4",
                rusqlite::params![nombre, contacto, now, id],
            )
            .map_err(db_err)?;

        if rows_affected == 0 {
            return Ok(None);
        }

        self.get_ticket(id)
    }

    fn confirm_ticket_payment(&self, id: &str) -> Result<Option<Ticket>, AppError> {
        let conn = self.pool.get().map_err(db_err)?;
        let now = chrono::Utc::now().to_rfc3339();

        // Idempotent update: if already Pagado, leaves it as Pagado
        let rows_affected = conn
            .execute(
                "UPDATE entradas SET estado_pago = 'Pagado', fecha_actualizacion = ?1 WHERE id = ?2 AND estado_pago = 'Pendiente'",
                rusqlite::params![now, id],
            )
            .map_err(db_err)?;

        if rows_affected == 0 {
            return self.get_ticket(id);
        }

        self.get_ticket(id)
    }

    fn delete_ticket(&self, id: &str) -> Result<bool, AppError> {
        let conn = self.pool.get().map_err(db_err)?;

        // First check check-in status
        let ticket_opt: Option<(String,)> = conn
            .query_row(
                "SELECT estado_checkin FROM entradas WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?,)),
            )
            .optional()
            .map_err(db_err)?;

        let (estado_checkin,) = match ticket_opt {
            Some(row) => row,
            None => return Ok(false),
        };

        if estado_checkin == "Ingresó" {
            return Err(AppError::conflict(
                "ticket_already_entered",
                "No se puede eliminar una entrada que ya completó ingreso en puerta",
            ));
        }

        let rows = conn
            .execute("DELETE FROM entradas WHERE id = ?1", [id])
            .map_err(db_err)?;

        Ok(rows > 0)
    }
}

impl TicketingTransactions for SqliteTicketingRepository {
    fn issue_sale(
        &self,
        input: &SellTicketsInput,
        credentials: Vec<GeneratedCredentials>,
    ) -> Result<Vec<Ticket>, AppError> {
        if input.cantidad < 1 {
            return Err(AppError::validation(
                "invalid_quantity",
                "La cantidad de entradas a emitir debe ser de al menos 1",
                "cantidad",
            ));
        }

        if credentials.len() != input.cantidad as usize {
            return Err(int_err(
                "Mismatch between requested ticket quantity and generated credentials",
            ));
        }

        let mut conn = self.pool.get().map_err(db_err)?;
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(db_err)?;

        // 1. Load event and verify it exists and is not finalized
        let event_row: Option<(i64, String)> = tx
            .query_row(
                "SELECT finalizado, tipos_de_entrada FROM eventos WHERE id = ?1",
                [&input.evento_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(db_err)?;

        let (finalizado, tipos_json) = match event_row {
            Some(row) => row,
            None => {
                return Err(AppError::not_found(
                    "event_not_found",
                    "El evento especificado no existe",
                ));
            }
        };

        if finalizado == 1 {
            return Err(AppError::conflict(
                "event_finalized",
                "No se pueden vender entradas para un evento finalizado",
            ));
        }

        // 2. Parse ticket types and locate the requested ticket type
        let tipos: Vec<TicketType> = serde_json::from_str(&tipos_json).map_err(int_err)?;

        let tipo = match tipos
            .into_iter()
            .find(|t| t.nombre.eq_ignore_ascii_case(&input.tipo_entrada))
        {
            Some(t) => t,
            None => {
                return Err(AppError::validation(
                    "ticket_type_not_found",
                    format!(
                        "El tipo de entrada '{}' no existe en este evento",
                        input.tipo_entrada
                    ),
                    "tipo_entrada",
                ));
            }
        };

        // 3. Count already emitted tickets of this type
        let emitidas: u32 = tx
            .query_row(
                "SELECT COUNT(*) FROM entradas WHERE evento_id = ?1 AND tipo_entrada = ?2",
                rusqlite::params![input.evento_id, tipo.nombre],
                |r| r.get(0),
            )
            .map_err(db_err)?;

        if emitidas + input.cantidad > tipo.capacidad {
            return Err(AppError::conflict(
                "capacity_exceeded",
                format!(
                    "Capacidad excedida para el tipo '{}'. Disponibles: {}, solicitadas: {}",
                    tipo.nombre,
                    tipo.capacidad.saturating_sub(emitidas),
                    input.cantidad
                ),
            ));
        }

        // 4. Invariants for Courtesy vs Normal sale
        let (precio, estado_pago) = if input.medio_pago == PaymentMethod::Cortesia {
            (Money::zero(), PaymentStatus::Pagado)
        } else {
            (tipo.precio, input.estado_pago)
        };

        let now = chrono::Utc::now().to_rfc3339();
        let mut created_tickets = Vec::with_capacity(credentials.len());

        {
            let mut insert_stmt = tx
                .prepare(
                    "INSERT INTO entradas (
                        id, evento_id, interesado_id, nombre_comprador, contacto, tipo_entrada,
                        precio_centimos, medio_pago, estado_pago, codigo_qr, scan_token,
                        estado_checkin, hora_ingreso, fecha_creacion, fecha_actualizacion
                    ) VALUES (
                        ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15
                    )",
                )
                .map_err(db_err)?;

            for cred in credentials {
                insert_stmt
                    .execute(rusqlite::params![
                        cred.id,
                        input.evento_id,
                        input.interesado_id,
                        input.nombre_comprador,
                        input.contacto,
                        tipo.nombre,
                        precio.cents(),
                        input.medio_pago.as_str(),
                        estado_pago.as_str(),
                        cred.qr_secret.as_str(),
                        cred.scan_token.as_str(),
                        CheckinStatus::NoIngreso.as_str(),
                        Option::<String>::None,
                        now,
                        now,
                    ])
                    .map_err(db_err)?;

                let ticket = Ticket::new(
                    cred.id,
                    input.evento_id.clone(),
                    input.interesado_id.clone(),
                    input.nombre_comprador.clone(),
                    input.contacto.clone(),
                    tipo.nombre.clone(),
                    precio,
                    input.medio_pago,
                    estado_pago,
                    cred.scan_token,
                    cred.qr_secret,
                    CheckinStatus::NoIngreso,
                    None,
                    now.clone(),
                    now.clone(),
                )?;
                created_tickets.push(ticket);
            }
        }

        tx.commit().map_err(db_err)?;
        Ok(created_tickets)
    }

    fn convert_interest(
        &self,
        interesado_id: &str,
        tipo_entrada: &str,
        cantidad: u32,
        medio_pago: PaymentMethod,
        estado_pago: PaymentStatus,
        credentials: Vec<GeneratedCredentials>,
    ) -> Result<Vec<Ticket>, AppError> {
        super::interests::convert_interest_impl(
            &self.pool,
            interesado_id,
            tipo_entrada,
            cantidad,
            medio_pago,
            estado_pago,
            credentials,
        )
    }

    fn check_in(
        &self,
        evento_id: &str,
        codigo_qr: &str,
        now: &str,
    ) -> Result<CheckInOutcome, AppError> {
        let trimmed = codigo_qr.trim();
        if trimmed.len() != 32 || !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(CheckInOutcome::Invalida {
                motivo: "Formato de código inválido".to_string(),
            });
        }
        let normalized_qr = trimmed.to_ascii_lowercase();

        let mut conn = self.pool.get().map_err(db_err)?;
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(db_err)?;

        // Conditional atomic update
        let rows_affected = tx
            .execute(
                "UPDATE entradas
                 SET estado_checkin = 'Ingresó', hora_ingreso = ?1, fecha_actualizacion = ?1
                 WHERE evento_id = ?2
                   AND codigo_qr = ?3
                   AND estado_pago = 'Pagado'
                   AND estado_checkin = 'No ingresó'",
                rusqlite::params![now, evento_id, normalized_qr],
            )
            .map_err(db_err)?;

        if rows_affected == 1 {
            let (nombre_comprador, tipo_entrada, hora_ingreso): (String, String, String) = tx
                .query_row(
                    "SELECT nombre_comprador, tipo_entrada, hora_ingreso FROM entradas WHERE evento_id = ?1 AND codigo_qr = ?2",
                    rusqlite::params![evento_id, normalized_qr],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .map_err(db_err)?;

            tx.commit().map_err(db_err)?;
            return Ok(CheckInOutcome::Valida {
                nombre_comprador,
                tipo_entrada,
                hora_ingreso,
            });
        }

        // rows_affected == 0: re-read ticket to classify reason
        let ticket_opt: Option<(String, String, String, Option<String>)> = tx
            .query_row(
                "SELECT evento_id, estado_pago, estado_checkin, hora_ingreso FROM entradas WHERE codigo_qr = ?1",
                [&normalized_qr],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()
            .map_err(db_err)?;

        tx.commit().map_err(db_err)?;

        match ticket_opt {
            None => Ok(CheckInOutcome::Invalida {
                motivo: "Codigo no encontrado".to_string(),
            }),
            Some((ticket_event_id, estado_pago, estado_checkin, hora_ingreso)) => {
                if ticket_event_id != evento_id {
                    Ok(CheckInOutcome::Invalida {
                        motivo: "Codigo pertenece a otro evento".to_string(),
                    })
                } else if estado_pago == "Pendiente" {
                    Ok(CheckInOutcome::Invalida {
                        motivo: "Pago pendiente".to_string(),
                    })
                } else if estado_checkin == "Ingresó" {
                    Ok(CheckInOutcome::YaUsada {
                        hora_ingreso: hora_ingreso.unwrap_or_default(),
                    })
                } else {
                    Ok(CheckInOutcome::Invalida {
                        motivo: "Entrada no válida para ingreso".to_string(),
                    })
                }
            }
        }
    }
}
