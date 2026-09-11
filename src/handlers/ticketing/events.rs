use axum::Json;
use axum::extract::{Path, State};
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::domain::ticketing::TicketType;
use crate::error::{ErrorBody, ErrorResponse};
use crate::handlers::ticketing::dto::{
    CreateEventRequest, EventDto, EventResponse, ListEventsResponse, UpdateEventRequest,
};
use crate::state::AppState;
use crate::use_cases::ticketing::events::{CreateEventInput, UpdateEventInput};

pub async fn list(State(state): State<AppState>) -> Response {
    match state.ticketing.list_events.execute() {
        Ok(events) => {
            let dtos = events.iter().map(EventDto::from_domain).collect();
            (StatusCode::OK, Json(ListEventsResponse { eventos: dtos })).into_response()
        }
        Err(err) => err.into_response(),
    }
}

pub async fn create(
    State(state): State<AppState>,
    payload: Result<Json<CreateEventRequest>, JsonRejection>,
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

    let tipos_result: Result<Vec<TicketType>, _> = request
        .tipos_de_entrada
        .iter()
        .map(|t| t.to_domain())
        .collect();

    let tipos_de_entrada = match tipos_result {
        Ok(t) => t,
        Err(err) => return err.into_response(),
    };

    let input = CreateEventInput {
        nombre: request.nombre,
        fecha_hora: request.fecha_hora,
        lugar: request.lugar,
        tipos_de_entrada,
    };

    match state.ticketing.create_event.execute(input) {
        Ok(event) => (
            StatusCode::CREATED,
            Json(EventResponse {
                evento: EventDto::from_domain(&event),
            }),
        )
            .into_response(),
        Err(err) => err.into_response(),
    }
}

pub async fn get(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.ticketing.get_event.execute(&id) {
        Ok(event) => (
            StatusCode::OK,
            Json(EventResponse {
                evento: EventDto::from_domain(&event),
            }),
        )
            .into_response(),
        Err(err) => err.into_response(),
    }
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    payload: Result<Json<UpdateEventRequest>, JsonRejection>,
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

    let tipos_result: Result<Vec<TicketType>, _> = request
        .tipos_de_entrada
        .iter()
        .map(|t| t.to_domain())
        .collect();

    let tipos_de_entrada = match tipos_result {
        Ok(t) => t,
        Err(err) => return err.into_response(),
    };

    let input = UpdateEventInput {
        id,
        nombre: request.nombre,
        fecha_hora: request.fecha_hora,
        lugar: request.lugar,
        tipos_de_entrada,
    };

    match state.ticketing.update_event.execute(input) {
        Ok(event) => (
            StatusCode::OK,
            Json(EventResponse {
                evento: EventDto::from_domain(&event),
            }),
        )
            .into_response(),
        Err(err) => err.into_response(),
    }
}

pub async fn finalize(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.ticketing.finalize_event.execute(&id) {
        Ok(()) => StatusCode::OK.into_response(),
        Err(err) => err.into_response(),
    }
}

pub async fn reopen(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.ticketing.reopen_event.execute(&id) {
        Ok(()) => StatusCode::OK.into_response(),
        Err(err) => err.into_response(),
    }
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.ticketing.delete_event.execute(&id) {
        Ok(()) => StatusCode::OK.into_response(),
        Err(err) => err.into_response(),
    }
}
