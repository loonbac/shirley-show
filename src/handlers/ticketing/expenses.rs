use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::domain::ticketing::{ExpenseCategory, Money};
use crate::error::{ErrorBody, ErrorResponse};
use crate::handlers::ticketing::dto::{
    CreateExpenseRequest, ExpenseDto, ExpenseResponse, ListExpensesResponse, UpdateExpenseRequest,
};
use crate::state::AppState;
use crate::use_cases::ticketing::expenses::{CreateExpenseInput, UpdateExpenseInput};

pub async fn list(State(state): State<AppState>, Path(evento_id): Path<String>) -> Response {
    match state.ticketing.list_expenses.execute(&evento_id) {
        Ok(expenses) => {
            let dtos = expenses.iter().map(ExpenseDto::from_domain).collect();
            (StatusCode::OK, Json(ListExpensesResponse { gastos: dtos })).into_response()
        }
        Err(err) => err.into_response(),
    }
}

pub async fn create(
    State(state): State<AppState>,
    Path(evento_id): Path<String>,
    payload: Result<Json<CreateExpenseRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(body) => body,
        Err(rejection) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: ErrorBody {
                        code: "invalid_request".to_string(),
                        message: rejection.body_text(),
                        field: None,
                    },
                }),
            )
                .into_response();
        }
    };

    let categoria = match ExpenseCategory::parse(&request.categoria) {
        Ok(c) => c,
        Err(err) => return err.into_response(),
    };

    let monto = match Money::from_soles(request.monto) {
        Ok(m) => m,
        Err(err) => return err.into_response(),
    };

    let input = CreateExpenseInput {
        evento_id,
        categoria,
        monto,
        descripcion: request.descripcion,
        fecha: request.fecha,
    };

    match state.ticketing.create_expense.execute(input) {
        Ok(expense) => (
            StatusCode::CREATED,
            Json(ExpenseResponse {
                gasto: ExpenseDto::from_domain(&expense),
            }),
        )
            .into_response(),
        Err(err) => err.into_response(),
    }
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    payload: Result<Json<UpdateExpenseRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(body) => body,
        Err(rejection) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: ErrorBody {
                        code: "invalid_request".to_string(),
                        message: rejection.body_text(),
                        field: None,
                    },
                }),
            )
                .into_response();
        }
    };

    let categoria = match ExpenseCategory::parse(&request.categoria) {
        Ok(c) => c,
        Err(err) => return err.into_response(),
    };

    let monto = match Money::from_soles(request.monto) {
        Ok(m) => m,
        Err(err) => return err.into_response(),
    };

    let input = UpdateExpenseInput {
        id,
        categoria,
        monto,
        descripcion: request.descripcion,
        fecha: request.fecha,
    };

    match state.ticketing.update_expense.execute(input) {
        Ok(expense) => (
            StatusCode::OK,
            Json(ExpenseResponse {
                gasto: ExpenseDto::from_domain(&expense),
            }),
        )
            .into_response(),
        Err(err) => err.into_response(),
    }
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.ticketing.delete_expense.execute(&id) {
        Ok(()) => StatusCode::OK.into_response(),
        Err(err) => err.into_response(),
    }
}
