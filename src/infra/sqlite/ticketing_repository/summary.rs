use crate::domain::ticketing::{
    EventSummary, ExpenseCategory, ExpenseCategoryBreakdown, Money, PaymentMethodBreakdown,
};
use crate::error::AppError;
use crate::infra::sqlite::ticketing_repository::SqliteTicketingRepository;
use crate::ports::ticketing::{EventRepository, EventSummaryReader};

impl EventSummaryReader for SqliteTicketingRepository {
    fn get_event_summary(&self, evento_id: &str) -> Result<EventSummary, AppError> {
        let event = self
            .get_event(evento_id)?
            .ok_or_else(|| AppError::not_found("event_not_found", "Evento no encontrado"))?;

        let capacidad_total = event.total_capacity();

        let conn = self.pool.get().map_err(|e| {
            AppError::Database(format!(
                "Failed to get connection for get_event_summary: {e}"
            ))
        })?;

        // 1. Entradas metrics (incluyendo asistencia en puerta)
        let (cobrado_cents, pendiente_cents, entradas_emitidas, entradas_ingresadas): (i64, i64, u32, u32) = {
            let mut stmt = conn
                .prepare(
                    "SELECT
                        COALESCE(SUM(CASE WHEN estado_pago = 'Pagado' THEN precio_centimos ELSE 0 END), 0),
                        COALESCE(SUM(CASE WHEN estado_pago = 'Pendiente' THEN precio_centimos ELSE 0 END), 0),
                        COUNT(*),
                        COALESCE(SUM(CASE WHEN estado_checkin = 'Ingresó' THEN 1 ELSE 0 END), 0)
                     FROM entradas
                     WHERE evento_id = ?1;",
                )
                .map_err(|e| AppError::Database(format!("Failed to prepare entradas summary query: {e}")))?;

            stmt.query_row([evento_id], |row| {
                let cobrado: i64 = row.get(0)?;
                let pendiente: i64 = row.get(1)?;
                let count: u32 = row.get(2)?;
                let ingresadas: u32 = row.get(3)?;
                Ok((cobrado, pendiente, count, ingresadas))
            })
            .map_err(|e| {
                AppError::Database(format!("Failed to execute entradas summary query: {e}"))
            })?
        };

        // Payment method breakdown
        let mut metodos_pago = PaymentMethodBreakdown::default();
        {
            let mut stmt = conn
                .prepare(
                    "SELECT
                        medio_pago,
                        estado_pago,
                        COUNT(*),
                        COALESCE(SUM(precio_centimos), 0)
                     FROM entradas
                     WHERE evento_id = ?1
                     GROUP BY medio_pago, estado_pago;",
                )
                .map_err(|e| {
                    AppError::Database(format!("Failed to prepare payment breakdown query: {e}"))
                })?;

            let rows = stmt
                .query_map([evento_id], |row| {
                    let medio: String = row.get(0)?;
                    let estado: String = row.get(1)?;
                    let count: u32 = row.get(2)?;
                    let sum_cents: i64 = row.get(3)?;
                    Ok((medio, estado, count, sum_cents))
                })
                .map_err(|e| {
                    AppError::Database(format!("Failed to query payment breakdown: {e}"))
                })?;

            for row in rows {
                let (medio, estado, count, sum_cents) = row
                    .map_err(|e| AppError::Database(format!("Failed to read payment row: {e}")))?;

                match (medio.as_str(), estado.as_str()) {
                    ("Yape", "Pagado") => {
                        metodos_pago.yape_cobrado = Money::from_cents(sum_cents);
                        metodos_pago.yape_entradas += count;
                    }
                    ("Yape", "Pendiente") => {
                        metodos_pago.yape_pendiente = Money::from_cents(sum_cents);
                        metodos_pago.yape_entradas += count;
                    }
                    ("Efectivo", "Pagado") => {
                        metodos_pago.efectivo_cobrado = Money::from_cents(sum_cents);
                        metodos_pago.efectivo_entradas += count;
                    }
                    ("Efectivo", "Pendiente") => {
                        metodos_pago.efectivo_pendiente = Money::from_cents(sum_cents);
                        metodos_pago.efectivo_entradas += count;
                    }
                    ("Cortesía", _) => {
                        metodos_pago.cortesia_entradas += count;
                    }
                    _ => {}
                }
            }
        }

        // 2. Interesados confirmados (personas)
        let personas_confirmadas: u32 = {
            let mut stmt = conn
                .prepare(
                    "SELECT COALESCE(SUM(personas), 0)
                     FROM interesados
                     WHERE evento_id = ?1 AND estado = 'Confirmado';",
                )
                .map_err(|e| {
                    AppError::Database(format!("Failed to prepare interesados summary query: {e}"))
                })?;

            stmt.query_row([evento_id], |row| row.get(0)).map_err(|e| {
                AppError::Database(format!("Failed to execute interesados summary query: {e}"))
            })?
        };

        let aforo_proyectado = entradas_emitidas + personas_confirmadas;

        // 3. Gastos
        let gastos_total_cents: i64 = {
            let mut stmt = conn
                .prepare(
                    "SELECT COALESCE(SUM(monto_centimos), 0)
                     FROM gastos
                     WHERE evento_id = ?1;",
                )
                .map_err(|e| {
                    AppError::Database(format!("Failed to prepare gastos summary query: {e}"))
                })?;

            stmt.query_row([evento_id], |row| row.get(0)).map_err(|e| {
                AppError::Database(format!("Failed to execute gastos summary query: {e}"))
            })?
        };

        // Categorias gasto breakdown
        let mut categorias_gasto = ExpenseCategoryBreakdown::default();
        {
            let mut stmt = conn
                .prepare(
                    "SELECT categoria, COALESCE(SUM(monto_centimos), 0)
                     FROM gastos
                     WHERE evento_id = ?1
                     GROUP BY categoria;",
                )
                .map_err(|e| {
                    AppError::Database(format!("Failed to prepare gastos breakdown query: {e}"))
                })?;

            let rows = stmt
                .query_map([evento_id], |row| {
                    let cat: String = row.get(0)?;
                    let sum_cents: i64 = row.get(1)?;
                    Ok((cat, sum_cents))
                })
                .map_err(|e| {
                    AppError::Database(format!("Failed to query gastos breakdown: {e}"))
                })?;

            for row in rows {
                let (cat_str, sum_cents) =
                    row.map_err(|e| AppError::Database(format!("Failed to read gastos row: {e}")))?;

                if let Ok(cat) = ExpenseCategory::parse(&cat_str) {
                    let m = Money::from_cents(sum_cents);
                    match cat {
                        ExpenseCategory::LocalAlquiler => categorias_gasto.local_alquiler = m,
                        ExpenseCategory::SonidoLuces => categorias_gasto.sonido_luces = m,
                        ExpenseCategory::MusicosPersonal => categorias_gasto.musicos_personal = m,
                        ExpenseCategory::PublicidadMarketing => {
                            categorias_gasto.publicidad_marketing = m
                        }
                        ExpenseCategory::BebidasCatering => categorias_gasto.bebidas_catering = m,
                        ExpenseCategory::Transporte => categorias_gasto.transporte = m,
                        ExpenseCategory::Otros => categorias_gasto.otros = m,
                    }
                }
            }
        }

        Ok(EventSummary::new(
            Money::from_cents(cobrado_cents),
            Money::from_cents(pendiente_cents),
            Money::from_cents(gastos_total_cents),
            capacidad_total,
            entradas_emitidas,
            aforo_proyectado,
            entradas_ingresadas,
            metodos_pago,
            categorias_gasto,
        ))
    }
}
