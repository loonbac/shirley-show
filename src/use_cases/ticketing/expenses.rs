use std::sync::Arc;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::ticketing::{Expense, ExpenseCategory, Money};
use crate::error::AppError;
use crate::ports::ticketing::ExpenseRepository;

#[derive(Debug, Clone)]
pub struct CreateExpenseInput {
    pub evento_id: String,
    pub categoria: ExpenseCategory,
    pub monto: Money,
    pub descripcion: String,
    pub fecha: String,
}

pub struct CreateExpense<R: ExpenseRepository> {
    repo: Arc<R>,
}

impl<R: ExpenseRepository> CreateExpense<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub fn execute(&self, input: CreateExpenseInput) -> Result<Expense, AppError> {
        let now = Utc::now().to_rfc3339();
        let expense = Expense::new(
            Uuid::new_v4().to_string(),
            input.evento_id,
            input.categoria,
            input.monto,
            input.descripcion,
            input.fecha,
            now.clone(),
            now,
        )?;

        self.repo.create_expense(&expense)?;
        Ok(expense)
    }
}

pub struct ListExpenses<R: ExpenseRepository> {
    repo: Arc<R>,
}

impl<R: ExpenseRepository> ListExpenses<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub fn execute(&self, evento_id: &str) -> Result<Vec<Expense>, AppError> {
        self.repo.list_expenses(evento_id)
    }
}

pub struct GetExpense<R: ExpenseRepository> {
    repo: Arc<R>,
}

impl<R: ExpenseRepository> GetExpense<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub fn execute(&self, id: &str) -> Result<Expense, AppError> {
        self.repo
            .get_expense(id)?
            .ok_or_else(|| AppError::not_found("expense_not_found", "Gasto no encontrado"))
    }
}

#[derive(Debug, Clone)]
pub struct UpdateExpenseInput {
    pub id: String,
    pub categoria: ExpenseCategory,
    pub monto: Money,
    pub descripcion: String,
    pub fecha: String,
}

pub struct UpdateExpense<R: ExpenseRepository> {
    repo: Arc<R>,
}

impl<R: ExpenseRepository> UpdateExpense<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub fn execute(&self, input: UpdateExpenseInput) -> Result<Expense, AppError> {
        let existing = self
            .repo
            .get_expense(&input.id)?
            .ok_or_else(|| AppError::not_found("expense_not_found", "Gasto no encontrado"))?;

        let now = Utc::now().to_rfc3339();
        let updated = Expense::new(
            input.id,
            existing.evento_id,
            input.categoria,
            input.monto,
            input.descripcion,
            input.fecha,
            existing.fecha_creacion,
            now,
        )?;

        self.repo.update_expense(&updated)?;
        Ok(updated)
    }
}

pub struct DeleteExpense<R: ExpenseRepository> {
    repo: Arc<R>,
}

impl<R: ExpenseRepository> DeleteExpense<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub fn execute(&self, id: &str) -> Result<(), AppError> {
        let deleted = self.repo.delete_expense(id)?;
        if !deleted {
            return Err(AppError::not_found("expense_not_found", "Gasto no encontrado"));
        }
        Ok(())
    }
}
