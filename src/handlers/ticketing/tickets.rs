use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::header::CACHE_CONTROL;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use serde::Deserialize;

use crate::domain::ticketing::ticket::{PaymentMethod, PaymentStatus};
use crate::handlers::ticketing::TicketingError;
use crate::handlers::ticketing::dto::{
    CheckInRequest, ListTicketsResponse, SellTicketsRequest, SellTicketsResponse, TicketDto,
    TicketResponse, UpdateTicketContactRequest,
};
use crate::ports::ticketing::{CheckInOutcome, SellTicketsInput};
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
    Json(payload): Json<SellTicketsRequest>,
) -> Result<(StatusCode, Json<SellTicketsResponse>), TicketingError> {
    let medio_pago = PaymentMethod::parse(&payload.medio_pago)?;
    let estado_pago = PaymentStatus::parse(&payload.estado_pago)?;

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

    let tickets = state.ticketing.sell_tickets.execute(input)?;
    let entradas = tickets
        .iter()
        .map(|t| TicketDto::from_domain(t, true))
        .collect();

    Ok((StatusCode::CREATED, Json(SellTicketsResponse { entradas })))
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
    Json(payload): Json<UpdateTicketContactRequest>,
) -> Result<Json<TicketResponse>, TicketingError> {
    let ticket = state.ticketing.update_ticket_contact.execute(
        &id,
        &payload.nombre_comprador,
        &payload.contacto,
    )?;

    Ok(Json(TicketResponse {
        entrada: TicketDto::from_domain(&ticket, false),
    }))
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
    Json(payload): Json<CheckInRequest>,
) -> Result<Json<CheckInOutcome>, TicketingError> {
    let outcome = state
        .ticketing
        .check_in_ticket
        .execute(&payload.evento_id, &payload.codigo)?;

    Ok(Json(outcome))
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
