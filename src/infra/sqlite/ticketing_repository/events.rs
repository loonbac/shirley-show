use rusqlite::OptionalExtension;

use crate::domain::ticketing::{Event, TicketType};
use crate::error::AppError;
use crate::infra::sqlite::ticketing_repository::SqliteTicketingRepository;
use crate::ports::ticketing::EventRepository;

fn db_err(e: impl std::fmt::Display) -> AppError {
    AppError::Database(e.to_string())
}

impl EventRepository for SqliteTicketingRepository {
    fn create_event(&self, event: &Event) -> Result<(), AppError> {
        let conn = self.pool.get().map_err(|e| {
            AppError::Database(format!("Failed to get connection for create_event: {e}"))
        })?;

        let tipos_json = serde_json::to_string(&event.tipos_de_entrada).map_err(|e| {
            AppError::Internal(format!("Failed to serialize tipos_de_entrada: {e}"))
        })?;

        conn.execute(
            "INSERT INTO eventos (id, nombre, fecha_hora, lugar, tipos_de_entrada, finalizado, imagen, fecha_creacion, fecha_actualizacion)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9);",
            rusqlite::params![
                event.id,
                event.nombre,
                event.fecha_hora,
                event.lugar,
                tipos_json,
                if event.finalizado { 1 } else { 0 },
                event.imagen,
                event.fecha_creacion,
                event.fecha_actualizacion,
            ],
        )
        .map_err(|e| AppError::Database(format!("Failed to insert event: {e}")))?;

        Ok(())
    }

    fn list_events(&self) -> Result<Vec<Event>, AppError> {
        let conn = self.pool.get().map_err(|e| {
            AppError::Database(format!("Failed to get connection for list_events: {e}"))
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT id, nombre, fecha_hora, lugar, tipos_de_entrada, finalizado, imagen, fecha_creacion, fecha_actualizacion
                 FROM eventos
                 ORDER BY fecha_hora ASC;",
            )
            .map_err(|e| AppError::Database(format!("Failed to prepare list_events query: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let nombre: String = row.get(1)?;
                let fecha_hora: String = row.get(2)?;
                let lugar: String = row.get(3)?;
                let tipos_json: String = row.get(4)?;
                let finalizado_int: i32 = row.get(5)?;
                let imagen: Option<String> = row.get(6)?;
                let fecha_creacion: String = row.get(7)?;
                let fecha_actualizacion: String = row.get(8)?;

                Ok((
                    id,
                    nombre,
                    fecha_hora,
                    lugar,
                    tipos_json,
                    finalizado_int != 0,
                    imagen,
                    fecha_creacion,
                    fecha_actualizacion,
                ))
            })
            .map_err(|e| AppError::Database(format!("Failed to execute list_events query: {e}")))?;

        let mut events = Vec::new();
        for row in rows {
            let (
                id,
                nombre,
                fecha_hora,
                lugar,
                tipos_json,
                finalizado,
                imagen,
                fecha_creacion,
                fecha_actualizacion,
            ) = row.map_err(|e| AppError::Database(format!("Failed to read event row: {e}")))?;

            let tipos_de_entrada: Vec<TicketType> = serde_json::from_str(&tipos_json)
                .map_err(|e| AppError::Database(format!("Corrupt tipos_de_entrada JSON: {e}")))?;

            events.push(Event {
                id,
                nombre,
                fecha_hora,
                lugar,
                tipos_de_entrada,
                finalizado,
                imagen,
                fecha_creacion,
                fecha_actualizacion,
            });
        }

        Ok(events)
    }

    fn get_event(&self, id: &str) -> Result<Option<Event>, AppError> {
        let conn = self.pool.get().map_err(|e| {
            AppError::Database(format!("Failed to get connection for get_event: {e}"))
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT id, nombre, fecha_hora, lugar, tipos_de_entrada, finalizado, imagen, fecha_creacion, fecha_actualizacion
                 FROM eventos
                 WHERE id = ?1;",
            )
            .map_err(|e| AppError::Database(format!("Failed to prepare get_event query: {e}")))?;

        let result = stmt
            .query_row([id], |row| {
                let id: String = row.get(0)?;
                let nombre: String = row.get(1)?;
                let fecha_hora: String = row.get(2)?;
                let lugar: String = row.get(3)?;
                let tipos_json: String = row.get(4)?;
                let finalizado_int: i32 = row.get(5)?;
                let imagen: Option<String> = row.get(6)?;
                let fecha_creacion: String = row.get(7)?;
                let fecha_actualizacion: String = row.get(8)?;

                Ok((
                    id,
                    nombre,
                    fecha_hora,
                    lugar,
                    tipos_json,
                    finalizado_int != 0,
                    imagen,
                    fecha_creacion,
                    fecha_actualizacion,
                ))
            })
            .optional()
            .map_err(|e| AppError::Database(format!("Failed to execute get_event query: {e}")))?;

        match result {
            Some((
                id,
                nombre,
                fecha_hora,
                lugar,
                tipos_json,
                finalizado,
                imagen,
                fecha_creacion,
                fecha_actualizacion,
            )) => {
                let tipos_de_entrada: Vec<TicketType> =
                    serde_json::from_str(&tipos_json).map_err(|e| {
                        AppError::Database(format!("Corrupt tipos_de_entrada JSON: {e}"))
                    })?;

                Ok(Some(Event {
                    id,
                    nombre,
                    fecha_hora,
                    lugar,
                    tipos_de_entrada,
                    finalizado,
                    imagen,
                    fecha_creacion,
                    fecha_actualizacion,
                }))
            }
            None => Ok(None),
        }
    }

    fn update_event(&self, event: &Event) -> Result<(), AppError> {
        let mut conn = self.pool.get().map_err(|e| {
            AppError::Database(format!("Failed to get connection for update_event: {e}"))
        })?;

        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(db_err)?;

        // Invariant C4: verify sold ticket types and ensure capacity is not lowered below emitted count
        {
            let mut stmt = tx
                .prepare("SELECT tipo_entrada, COUNT(*) FROM entradas WHERE evento_id = ?1 GROUP BY tipo_entrada;")
                .map_err(db_err)?;

            let emitted_counts: Vec<(String, u32)> = stmt
                .query_map(rusqlite::params![event.id], |row| {
                    let name: String = row.get(0)?;
                    let count: u32 = row.get(1)?;
                    Ok((name, count))
                })
                .map_err(db_err)?
                .collect::<Result<_, _>>()
                .map_err(db_err)?;

            for (sold_tipo, count) in emitted_counts {
                if count > 0 {
                    let matching_new_type = event
                        .tipos_de_entrada
                        .iter()
                        .find(|t| t.nombre == sold_tipo);

                    match matching_new_type {
                        None => {
                            return Err(AppError::bad_request(
                                "cannot_remove_sold_ticket_type",
                                format!(
                                    "No se puede eliminar o renombrar el tipo de entrada '{}' porque ya tiene {} entrada(s) emitida(s)",
                                    sold_tipo, count
                                ),
                            ));
                        }
                        Some(new_type) => {
                            if new_type.capacidad < count {
                                return Err(AppError::bad_request(
                                    "capacity_below_emitted",
                                    format!(
                                        "La capacidad del tipo de entrada '{}' ({}) no puede ser menor a las entradas ya emitidas ({})",
                                        sold_tipo, new_type.capacidad, count
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }

        let tipos_json = serde_json::to_string(&event.tipos_de_entrada).map_err(|e| {
            AppError::Internal(format!("Failed to serialize tipos_de_entrada: {e}"))
        })?;

        let rows = tx
            .execute(
                "UPDATE eventos
                 SET nombre = ?2, fecha_hora = ?3, lugar = ?4, tipos_de_entrada = ?5, finalizado = ?6, imagen = ?7, fecha_actualizacion = ?8
                 WHERE id = ?1;",
                rusqlite::params![
                    event.id,
                    event.nombre,
                    event.fecha_hora,
                    event.lugar,
                    tipos_json,
                    if event.finalizado { 1 } else { 0 },
                    event.imagen,
                    event.fecha_actualizacion,
                ],
            )
            .map_err(|e| AppError::Database(format!("Failed to update event: {e}")))?;

        if rows == 0 {
            return Err(AppError::not_found(
                "event_not_found",
                "Evento no encontrado",
            ));
        }

        tx.commit().map_err(db_err)?;

        Ok(())
    }

    fn set_finalizado(&self, id: &str, finalizado: bool) -> Result<(), AppError> {
        let conn = self.pool.get().map_err(|e| {
            AppError::Database(format!("Failed to get connection for set_finalizado: {e}"))
        })?;

        let rows = conn
            .execute(
                "UPDATE eventos SET finalizado = ?2 WHERE id = ?1;",
                rusqlite::params![id, if finalizado { 1 } else { 0 }],
            )
            .map_err(|e| AppError::Database(format!("Failed to set_finalizado: {e}")))?;

        if rows == 0 {
            return Err(AppError::not_found(
                "event_not_found",
                "Evento no encontrado",
            ));
        }

        Ok(())
    }

    fn delete_event(&self, id: &str) -> Result<bool, AppError> {
        let conn = self.pool.get().map_err(|e| {
            AppError::Database(format!("Failed to get connection for delete_event: {e}"))
        })?;

        let rows = conn
            .execute("DELETE FROM eventos WHERE id = ?1;", [id])
            .map_err(|e| AppError::Database(format!("Failed to delete event: {e}")))?;

        Ok(rows > 0)
    }
}
