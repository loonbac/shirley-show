use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::header::CACHE_CONTROL;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use super::error::ticketing_error_response;
use crate::domain::ticketing::ticket::{PaymentMethod, PaymentStatus};
use crate::error::{ErrorBody, ErrorResponse};
use crate::handlers::ticketing::TicketingError;
use crate::handlers::ticketing::dto::{
    CheckInRequest, ListTicketsResponse, SellTicketsRequest, SellTicketsResponse, TicketDto,
    TicketResponse, UpdateTicketContactRequest,
};
use crate::ports::ticketing::SellTicketsInput;
use crate::state::AppState;
use crate::use_cases::ticketing::tickets::PublicTicketDto;

#[derive(Debug, Deserialize)]
pub struct ListTicketsQuery {
    pub q: Option<String>,
    pub estado_pago: Option<String>,
    pub tipo_entrada: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    Path(evento_id): Path<String>,
    Query(query): Query<ListTicketsQuery>,
) -> Result<Json<ListTicketsResponse>, TicketingError> {
    let tickets = state.ticketing.list_tickets.execute(
        &evento_id,
        query.q.as_deref(),
        query.estado_pago.as_deref(),
        query.tipo_entrada.as_deref(),
    )?;

    let entradas = tickets
        .iter()
        .map(|t| TicketDto::from_domain(t, false))
        .collect();

    Ok(Json(ListTicketsResponse { entradas }))
}

pub async fn sell(
    State(state): State<AppState>,
    Path(evento_id): Path<String>,
    payload: Result<Json<SellTicketsRequest>, JsonRejection>,
) -> Response {
    let Json(payload) = match payload {
        Ok(p) => p,
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

    let medio_pago = match PaymentMethod::parse(&payload.medio_pago) {
        Ok(m) => m,
        Err(err) => return ticketing_error_response(err),
    };
    let estado_pago = match PaymentStatus::parse(&payload.estado_pago) {
        Ok(s) => s,
        Err(err) => return ticketing_error_response(err),
    };

    let input = SellTicketsInput {
        evento_id,
        interesado_id: None,
        nombre_comprador: payload.nombre_comprador,
        contacto: payload.contacto,
        tipo_entrada: payload.tipo_entrada,
        cantidad: payload.cantidad,
        medio_pago,
        estado_pago,
    };

    let tickets = match state.ticketing.sell_tickets.execute(input) {
        Ok(t) => t,
        Err(err) => return ticketing_error_response(err),
    };
    let entradas = tickets
        .iter()
        .map(|t| TicketDto::from_domain(t, true))
        .collect();

    (StatusCode::CREATED, Json(SellTicketsResponse { entradas })).into_response()
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<TicketResponse>, TicketingError> {
    let ticket = state.ticketing.get_ticket.execute(&id)?;
    Ok(Json(TicketResponse {
        entrada: TicketDto::from_domain(&ticket, true),
    }))
}

pub async fn update_contact(
    State(state): State<AppState>,
    Path(id): Path<String>,
    payload: Result<Json<UpdateTicketContactRequest>, JsonRejection>,
) -> Response {
    let Json(payload) = match payload {
        Ok(p) => p,
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

    let ticket = match state.ticketing.update_ticket_contact.execute(
        &id,
        &payload.nombre_comprador,
        &payload.contacto,
    ) {
        Ok(t) => t,
        Err(err) => return ticketing_error_response(err),
    };

    Json(TicketResponse {
        entrada: TicketDto::from_domain(&ticket, false),
    })
    .into_response()
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, TicketingError> {
    state.ticketing.delete_ticket.execute(&id)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn confirm_payment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<TicketResponse>, TicketingError> {
    let ticket = state.ticketing.confirm_ticket_payment.execute(&id)?;
    Ok(Json(TicketResponse {
        entrada: TicketDto::from_domain(&ticket, true),
    }))
}

pub async fn check_in(
    State(state): State<AppState>,
    payload: Result<Json<CheckInRequest>, JsonRejection>,
) -> Response {
    let Json(payload) = match payload {
        Ok(p) => p,
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

    let outcome = match state
        .ticketing
        .check_in_ticket
        .execute(&payload.evento_id, &payload.codigo)
    {
        Ok(o) => o,
        Err(err) => return ticketing_error_response(err),
    };

    Json(outcome).into_response()
}

pub async fn get_public_ticket(
    State(state): State<AppState>,
    Path(scan_token): Path<String>,
) -> Result<(HeaderMap, Json<PublicTicketDto>), TicketingError> {
    let ticket = state.ticketing.get_public_ticket.execute(&scan_token)?;

    let mut headers = HeaderMap::new();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));

    Ok((headers, Json(ticket)))
}
