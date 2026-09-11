use rusqlite::OptionalExtension;

use crate::domain::ticketing::interest::{Interest, InterestStatus};
use crate::domain::ticketing::ticket::{CheckinStatus, PaymentMethod, PaymentStatus, Ticket};
use crate::domain::ticketing::{Money, TicketType};
use crate::error::AppError;
use crate::infra::sqlite::connect::DbPool;
use crate::infra::sqlite::ticketing_repository::SqliteTicketingRepository;
use crate::ports::ticketing::{GeneratedCredentials, InterestRepository};

fn db_err(e: impl std::fmt::Display) -> AppError {
    AppError::Database(e.to_string())
}

fn int_err(e: impl std::fmt::Display) -> AppError {
    AppError::Internal(e.to_string())
}

fn row_to_interest(row: &rusqlite::Row) -> Result<Interest, rusqlite::Error> {
    let id: String = row.get(0)?;
    let evento_id: String = row.get(1)?;
    let nombre: String = row.get(2)?;
    let contacto: String = row.get(3)?;
    let canal: String = row.get(4)?;
    let promotora: String = row.get(5)?;
    let personas: u32 = row.get(6)?;
    let estado_str: String = row.get(7)?;
    let convertido_at: Option<String> = row.get(8)?;
    let fecha_creacion: String = row.get(9)?;
    let fecha_actualizacion: String = row.get(10)?;

    let estado = InterestStatus::parse(&estado_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(e))
    })?;

    Interest::new(
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
    )
    .map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

const INTEREST_SELECT_COLUMNS: &str = "id, evento_id, nombre, contacto, canal, promotora, personas, estado, convertido_at, fecha_creacion, fecha_actualizacion";

impl InterestRepository for SqliteTicketingRepository {
    fn create_interest(&self, interest: &Interest) -> Result<(), AppError> {
        let conn = self.pool.get().map_err(db_err)?;
        conn.execute(
            "INSERT INTO interesados (
                id, evento_id, nombre, contacto, canal, promotora, personas,
                estado, convertido_at, fecha_creacion, fecha_actualizacion
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                interest.id,
                interest.evento_id,
                interest.nombre,
                interest.contacto,
                interest.canal,
                interest.promotora,
                interest.personas,
                interest.estado.as_str(),
                interest.convertido_at,
                interest.fecha_creacion,
                interest.fecha_actualizacion,
            ],
        )
        .map_err(db_err)?;
        Ok(())
    }

    fn list_interests(
        &self,
        evento_id: &str,
        estado: Option<&str>,
    ) -> Result<Vec<Interest>, AppError> {
        let conn = self.pool.get().map_err(db_err)?;
        let mut sql = format!(
            "SELECT {} FROM interesados WHERE evento_id = ?1",
            INTEREST_SELECT_COLUMNS
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> =
            vec![Box::new(evento_id.to_string())];

        if let Some(st) = estado {
            let trimmed = st.trim();
            if !trimmed.is_empty() {
                sql.push_str(" AND estado = ?2");
                params.push(Box::new(trimmed.to_string()));
            }
        }

        sql.push_str(" ORDER BY fecha_creacion DESC, id DESC");

        let mut stmt = conn.prepare(&sql).map_err(db_err)?;
        let rusqlite_params: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|b| b.as_ref()).collect();

        let rows = stmt
            .query_map(&rusqlite_params[..], row_to_interest)
            .map_err(db_err)?;

        let mut interests = Vec::new();
        for row in rows {
            interests.push(row.map_err(db_err)?);
        }
        Ok(interests)
    }

    fn get_interest(&self, id: &str) -> Result<Option<Interest>, AppError> {
        let conn = self.pool.get().map_err(db_err)?;
        let sql = format!(
            "SELECT {} FROM interesados WHERE id = ?1",
            INTEREST_SELECT_COLUMNS
        );
        conn.query_row(&sql, [id], row_to_interest)
            .optional()
            .map_err(db_err)
    }

    fn update_interest(&self, interest: &Interest) -> Result<(), AppError> {
        let conn = self.pool.get().map_err(db_err)?;
        let rows = conn
            .execute(
                "UPDATE interesados
                 SET nombre = ?1, contacto = ?2, canal = ?3, promotora = ?4,
                     personas = ?5, estado = ?6, convertido_at = ?7, fecha_actualizacion = ?8
                 WHERE id = ?9",
                rusqlite::params![
                    interest.nombre,
                    interest.contacto,
                    interest.canal,
                    interest.promotora,
                    interest.personas,
                    interest.estado.as_str(),
                    interest.convertido_at,
                    interest.fecha_actualizacion,
                    interest.id,
                ],
            )
            .map_err(db_err)?;

        if rows == 0 {
            return Err(AppError::not_found(
                "interest_not_found",
                "Interesado no encontrado",
            ));
        }
        Ok(())
    }

    fn delete_interest(&self, id: &str) -> Result<bool, AppError> {
        let conn = self.pool.get().map_err(db_err)?;
        let rows = conn
            .execute("DELETE FROM interesados WHERE id = ?1", [id])
            .map_err(db_err)?;
        Ok(rows > 0)
    }
}

pub fn convert_interest_impl(
    pool: &DbPool,
    interesado_id: &str,
    tipo_entrada: &str,
    cantidad: u32,
    medio_pago: PaymentMethod,
    estado_pago: PaymentStatus,
    credentials: Vec<GeneratedCredentials>,
) -> Result<Vec<Ticket>, AppError> {
    if cantidad < 1 {
        return Err(AppError::validation(
            "invalid_quantity",
            "La cantidad de entradas a emitir debe ser de al menos 1",
            "cantidad",
        ));
    }

    if credentials.len() != cantidad as usize {
        return Err(int_err(
            "Mismatch between requested ticket quantity and generated credentials",
        ));
    }

    let mut conn = pool.get().map_err(db_err)?;
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(db_err)?;

    // 1. Load lead and check converted_at
    let lead_row: Option<(String, String, String, Option<String>)> = tx
        .query_row(
            "SELECT evento_id, nombre, contacto, convertido_at FROM interesados WHERE id = ?1",
            [interesado_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()
        .map_err(db_err)?;

    let (evento_id, nombre_comprador, contacto, convertido_at) = match lead_row {
        Some(row) => row,
        None => {
            return Err(AppError::not_found(
                "interest_not_found",
                "El interesado especificado no existe",
            ));
        }
    };

    if convertido_at.is_some() {
        return Err(AppError::conflict(
            "interest_already_converted",
            "El interesado ya fue convertido a venta previamente",
        ));
    }

    // 2. Load event and check active
    let event_row: Option<(i64, String)> = tx
        .query_row(
            "SELECT finalizado, tipos_de_entrada FROM eventos WHERE id = ?1",
            [&evento_id],
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

    // 3. Find ticket type and check capacity
    let tipos: Vec<TicketType> = serde_json::from_str(&tipos_json).map_err(int_err)?;
    let tipo = match tipos
        .into_iter()
        .find(|t| t.nombre.eq_ignore_ascii_case(tipo_entrada))
    {
        Some(t) => t,
        None => {
            return Err(AppError::validation(
                "ticket_type_not_found",
                format!("El tipo de entrada '{tipo_entrada}' no existe en este evento"),
                "tipo_entrada",
            ));
        }
    };

    let emitidas: u32 = tx
        .query_row(
            "SELECT COUNT(*) FROM entradas WHERE evento_id = ?1 AND tipo_entrada = ?2",
            rusqlite::params![evento_id, tipo.nombre],
            |r| r.get(0),
        )
        .map_err(db_err)?;

    if emitidas + cantidad > tipo.capacidad {
        return Err(AppError::conflict(
            "capacity_exceeded",
            format!(
                "Capacidad excedida para el tipo '{}'. Disponibles: {}, solicitadas: {}",
                tipo.nombre,
                tipo.capacidad.saturating_sub(emitidas),
                cantidad
            ),
        ));
    }

    // 4. Invariants for Courtesy vs Normal sale
    let (precio, final_estado_pago) = if medio_pago == PaymentMethod::Cortesia {
        (Money::zero(), PaymentStatus::Pagado)
    } else {
        (tipo.precio, estado_pago)
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
                    evento_id,
                    Some(interesado_id),
                    nombre_comprador,
                    contacto,
                    tipo.nombre,
                    precio.cents(),
                    medio_pago.as_str(),
                    final_estado_pago.as_str(),
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
                evento_id.clone(),
                Some(interesado_id.to_string()),
                nombre_comprador.clone(),
                contacto.clone(),
                tipo.nombre.clone(),
                precio,
                medio_pago,
                final_estado_pago,
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

    // 5. Update lead to Confirmado with convertido_at = now
    tx.execute(
        "UPDATE interesados SET estado = 'Confirmado', convertido_at = ?1, fecha_actualizacion = ?1 WHERE id = ?2",
        rusqlite::params![now, interesado_id],
    )
    .map_err(db_err)?;

    tx.commit().map_err(db_err)?;
    Ok(created_tickets)
}
