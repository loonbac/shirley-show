use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::domain::ticketing::interest::InterestStatus;
use crate::domain::ticketing::ticket::{PaymentMethod, PaymentStatus};
use crate::error::{ErrorBody, ErrorResponse};
use crate::handlers::ticketing::dto::{
    ConvertInterestRequest, CreateInterestRequest, InterestDto, InterestResponse,
    ListInterestsResponse, SellTicketsResponse, TicketDto, UpdateInterestRequest,
};
use crate::handlers::ticketing::ticketing_error_response;
use crate::state::AppState;
use crate::use_cases::ticketing::interests::{
    ConvertInterestInput, CreateInterestInput, UpdateInterestInput,
};

#[derive(Debug, Deserialize)]
pub struct ListInterestsParams {
    pub estado: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    Path(evento_id): Path<String>,
    Query(params): Query<ListInterestsParams>,
) -> Response {
    match state
        .ticketing
        .list_interests
        .execute(&evento_id, params.estado.as_deref())
    {
        Ok(interests) => {
            let dtos = interests.iter().map(InterestDto::from_domain).collect();
            (
                StatusCode::OK,
                Json(ListInterestsResponse { interesados: dtos }),
            )
                .into_response()
        }
        Err(err) => ticketing_error_response(err),
    }
}

pub async fn create(
    State(state): State<AppState>,
    Path(evento_id): Path<String>,
    payload: Result<Json<CreateInterestRequest>, JsonRejection>,
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

    let input = CreateInterestInput {
        evento_id,
        nombre: request.nombre,
        contacto: request.contacto,
        canal: request.canal,
        promotora: request.promotora,
        personas: request.personas,
    };

    match state.ticketing.create_interest.execute(input) {
        Ok(interest) => (
            StatusCode::CREATED,
            Json(InterestResponse {
                interesado: InterestDto::from_domain(&interest),
            }),
        )
            .into_response(),
        Err(err) => ticketing_error_response(err),
    }
}

pub async fn get(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.ticketing.get_interest.execute(&id) {
        Ok(interest) => (
            StatusCode::OK,
            Json(InterestResponse {
                interesado: InterestDto::from_domain(&interest),
            }),
        )
            .into_response(),
        Err(err) => ticketing_error_response(err),
    }
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    payload: Result<Json<UpdateInterestRequest>, JsonRejection>,
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

    let estado = match InterestStatus::parse(&request.estado) {
        Ok(st) => st,
        Err(err) => return ticketing_error_response(err),
    };

    let input = UpdateInterestInput {
        id,
        nombre: request.nombre,
        contacto: request.contacto,
        canal: request.canal,
        promotora: request.promotora,
        personas: request.personas,
        estado,
    };

    match state.ticketing.update_interest.execute(input) {
        Ok(interest) => (
            StatusCode::OK,
            Json(InterestResponse {
                interesado: InterestDto::from_domain(&interest),
            }),
        )
            .into_response(),
        Err(err) => ticketing_error_response(err),
    }
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.ticketing.delete_interest.execute(&id) {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => ticketing_error_response(err),
    }
}

pub async fn convert(
    State(state): State<AppState>,
    Path(id): Path<String>,
    payload: Result<Json<ConvertInterestRequest>, JsonRejection>,
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

    let lead = match state.ticketing.get_interest.execute(&id) {
        Ok(l) => l,
        Err(err) => return ticketing_error_response(err),
    };

    let cantidad = request.cantidad.unwrap_or(lead.personas);

    let medio_pago = match PaymentMethod::parse(&request.medio_pago) {
        Ok(mp) => mp,
        Err(err) => return ticketing_error_response(err),
    };

    let estado_pago = match PaymentStatus::parse(&request.estado_pago) {
        Ok(ep) => ep,
        Err(err) => return ticketing_error_response(err),
    };

    let input = ConvertInterestInput {
        interesado_id: id,
        tipo_entrada: request.tipo_entrada,
        cantidad,
        medio_pago,
        estado_pago,
    };

    match state.ticketing.convert_interest_to_sale.execute(input) {
        Ok(tickets) => {
            let dtos = tickets
                .iter()
                .map(|t| TicketDto::from_domain(t, true, state.public_base_url.as_deref()))
                .collect();
            (
                StatusCode::CREATED,
                Json(SellTicketsResponse { entradas: dtos }),
            )
                .into_response()
        }
        Err(err) => ticketing_error_response(err),
    }
}
