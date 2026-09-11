use rusqlite::OptionalExtension;

use crate::domain::ticketing::{Expense, ExpenseCategory, Money};
use crate::error::AppError;
use crate::infra::sqlite::ticketing_repository::SqliteTicketingRepository;
use crate::ports::ticketing::ExpenseRepository;

impl ExpenseRepository for SqliteTicketingRepository {
    fn create_expense(&self, expense: &Expense) -> Result<(), AppError> {
        let conn = self.pool.get().map_err(|e| {
            AppError::Database(format!("Failed to get connection for create_expense: {e}"))
        })?;

        conn.execute(
            "INSERT INTO gastos (id, evento_id, categoria, monto_centimos, descripcion, fecha, fecha_creacion, fecha_actualizacion)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);",
            rusqlite::params![
                expense.id,
                expense.evento_id,
                expense.categoria.as_str(),
                expense.monto.cents(),
                expense.descripcion,
                expense.fecha,
                expense.fecha_creacion,
                expense.fecha_actualizacion,
            ],
        )
        .map_err(|e| AppError::Database(format!("Failed to insert expense: {e}")))?;

        Ok(())
    }

    fn list_expenses(&self, evento_id: &str) -> Result<Vec<Expense>, AppError> {
        let conn = self.pool.get().map_err(|e| {
            AppError::Database(format!("Failed to get connection for list_expenses: {e}"))
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT id, evento_id, categoria, monto_centimos, descripcion, fecha, fecha_creacion, fecha_actualizacion
                 FROM gastos
                 WHERE evento_id = ?1
                 ORDER BY fecha DESC, fecha_creacion DESC;",
            )
            .map_err(|e| AppError::Database(format!("Failed to prepare list_expenses query: {e}")))?;

        let rows = stmt
            .query_map([evento_id], |row| {
                let id: String = row.get(0)?;
                let evento_id: String = row.get(1)?;
                let categoria_str: String = row.get(2)?;
                let monto_centimos: i64 = row.get(3)?;
                let descripcion: String = row.get(4)?;
                let fecha: String = row.get(5)?;
                let fecha_creacion: String = row.get(6)?;
                let fecha_actualizacion: String = row.get(7)?;

                Ok((
                    id,
                    evento_id,
                    categoria_str,
                    monto_centimos,
                    descripcion,
                    fecha,
                    fecha_creacion,
                    fecha_actualizacion,
                ))
            })
            .map_err(|e| {
                AppError::Database(format!("Failed to execute list_expenses query: {e}"))
            })?;

        let mut expenses = Vec::new();
        for row in rows {
            let (
                id,
                evento_id,
                categoria_str,
                monto_centimos,
                descripcion,
                fecha,
                fecha_creacion,
                fecha_actualizacion,
            ) = row.map_err(|e| AppError::Database(format!("Failed to read expense row: {e}")))?;

            let categoria = ExpenseCategory::parse(&categoria_str)?;
            let monto = Money::from_cents(monto_centimos);

            expenses.push(Expense {
                id,
                evento_id,
                categoria,
                monto,
                descripcion,
                fecha,
                fecha_creacion,
                fecha_actualizacion,
            });
        }

        Ok(expenses)
    }

    fn get_expense(&self, id: &str) -> Result<Option<Expense>, AppError> {
        let conn = self.pool.get().map_err(|e| {
            AppError::Database(format!("Failed to get connection for get_expense: {e}"))
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT id, evento_id, categoria, monto_centimos, descripcion, fecha, fecha_creacion, fecha_actualizacion
                 FROM gastos
                 WHERE id = ?1;",
            )
            .map_err(|e| AppError::Database(format!("Failed to prepare get_expense query: {e}")))?;

        let result = stmt
            .query_row([id], |row| {
                let id: String = row.get(0)?;
                let evento_id: String = row.get(1)?;
                let categoria_str: String = row.get(2)?;
                let monto_centimos: i64 = row.get(3)?;
                let descripcion: String = row.get(4)?;
                let fecha: String = row.get(5)?;
                let fecha_creacion: String = row.get(6)?;
                let fecha_actualizacion: String = row.get(7)?;

                Ok((
                    id,
                    evento_id,
                    categoria_str,
                    monto_centimos,
                    descripcion,
                    fecha,
                    fecha_creacion,
                    fecha_actualizacion,
                ))
            })
            .optional()
            .map_err(|e| AppError::Database(format!("Failed to execute get_expense query: {e}")))?;

        match result {
            Some((
                id,
                evento_id,
                categoria_str,
                monto_centimos,
                descripcion,
                fecha,
                fecha_creacion,
                fecha_actualizacion,
            )) => {
                let categoria = ExpenseCategory::parse(&categoria_str)?;
                let monto = Money::from_cents(monto_centimos);

                Ok(Some(Expense {
                    id,
                    evento_id,
                    categoria,
                    monto,
                    descripcion,
                    fecha,
                    fecha_creacion,
                    fecha_actualizacion,
                }))
            }
            None => Ok(None),
        }
    }

    fn update_expense(&self, expense: &Expense) -> Result<(), AppError> {
        let conn = self.pool.get().map_err(|e| {
            AppError::Database(format!("Failed to get connection for update_expense: {e}"))
        })?;

        let rows = conn
            .execute(
                "UPDATE gastos
                 SET categoria = ?2, monto_centimos = ?3, descripcion = ?4, fecha = ?5, fecha_actualizacion = ?6
                 WHERE id = ?1;",
                rusqlite::params![
                    expense.id,
                    expense.categoria.as_str(),
                    expense.monto.cents(),
                    expense.descripcion,
                    expense.fecha,
                    expense.fecha_actualizacion,
                ],
            )
            .map_err(|e| AppError::Database(format!("Failed to update expense: {e}")))?;

        if rows == 0 {
            return Err(AppError::not_found(
                "expense_not_found",
                "Gasto no encontrado",
            ));
        }

        Ok(())
    }

    fn delete_expense(&self, id: &str) -> Result<bool, AppError> {
        let conn = self.pool.get().map_err(|e| {
            AppError::Database(format!("Failed to get connection for delete_expense: {e}"))
        })?;

        let rows = conn
            .execute("DELETE FROM gastos WHERE id = ?1;", [id])
            .map_err(|e| AppError::Database(format!("Failed to delete expense: {e}")))?;

        Ok(rows > 0)
    }
}
