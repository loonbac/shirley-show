use crate::domain::ticketing::{
    ExpenseCategory, Money, PaymentMethod, PaymentStatus, TicketType,
};
use crate::ports::ticketing::{CheckInOutcome, SellTicketsInput};
use crate::use_cases::ticketing::{CreateExpenseInput, CreateInterestInput};
use crate::state::ticketing_services::TicketingServices;
use crate::use_cases::ticketing::events::CreateEventInput;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::agent::tool_signature::canonical_tool_signature;
use crate::agent::turn_context::{StagedAudioHandle, TurnToolContext};
use crate::domain::agent_tool::{ToolAction, ToolCall};
use crate::domain::audio_format::AudioFormat;
use crate::domain::show_item_kind::ShowItemKind;
use crate::domain::vocal_separation_mode::VocalSeparationMode;
use crate::handlers::dto::ShowItemResponse;
use crate::infra::sqlite::presentation_repository::SqlitePresentationRepository;
use crate::infra::sqlite::presentation_summary_repository::PresentationSummaryRepository;
use crate::infra::sqlite::show_item_repository::SqliteShowItemRepository;
use crate::ports::show_item_reader::ShowItemReader;
use crate::state::AppState;
use crate::state::staged_audio_registry::{StagedAudioRegistry, StagedAudioRegistryError};
use crate::use_cases::add_lyrics::AddLyrics;
use crate::use_cases::add_note::AddNote;
use crate::use_cases::create_presentation::CreatePresentation;
use crate::use_cases::download_youtube_audio::DownloadYouTubeAudio;
use crate::use_cases::get_presentation::GetPresentation;
use crate::use_cases::list_presentations::ListPresentations;
use crate::use_cases::separate_vocals::{SeparateVocals, SeparateVocalsInput, VocalAudioSource};
use crate::use_cases::upload_track::UploadTrack;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreatePresentationArgs {
    name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PresentationIdArgs {
    presentation_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AddItemTextArgs {
    presentation_id: String,
    text: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AddTrackArgs {
    presentation_id: String,
    title: String,
    #[serde(default)]
    staged_audio_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DownloadYouTubeTrackArgs {
    url: String,
    #[serde(default)]
    title: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SeparateVocalsArgs {
    mode: String,
    #[serde(default)]
    staged_audio_id: Option<String>,
    #[serde(default)]
    track_id: Option<String>,
    #[serde(default)]
    youtube_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CrearEventoTipoEntradaArgs {
    nombre: String,
    precio: f64,
    capacidad: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CrearEventoArgs {
    nombre: String,
    fecha_hora: String,
    lugar: String,
    tipos_de_entrada: Vec<CrearEventoTipoEntradaArgs>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct ListarEventosArgs {
    #[serde(default)]
    incluir_finalizados: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VenderEntradasArgs {
    evento_id: String,
    nombre_comprador: String,
    contacto: String,
    tipo_entrada: String,
    cantidad: u32,
    medio_pago: String,
    estado_pago: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfirmarPagoEntradaArgs {
    entrada_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistrarGastoArgs {
    evento_id: String,
    categoria: String,
    monto: f64,
    descripcion: String,
    #[serde(default)]
    fecha: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResumenEventoArgs {
    evento_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistrarInteresadoArgs {
    evento_id: String,
    nombre: String,
    contacto: String,
    canal: String,
    personas: u32,
    promotora: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckinEntradaArgs {
    evento_id: String,
    codigo: String,
}

fn app_error_to_json(e: &crate::error::AppError) -> Value {
    match e {
        crate::error::AppError::BadRequest { code, message }
        | crate::error::AppError::Validation { code, message, .. }
        | crate::error::AppError::NotFound { code, message }
        | crate::error::AppError::Conflict { code, message }
        | crate::error::AppError::PayloadTooLarge { code, message }
        | crate::error::AppError::UnsupportedMediaType { code, message } => {
            json!({ "error": code, "message": message })
        }
        crate::error::AppError::Database(msg) => {
            json!({ "error": "database_error", "message": msg })
        }
        crate::error::AppError::Internal(msg) => {
            json!({ "error": "internal_error", "message": msg })
        }
    }
}

fn is_authorized_event(context: &TurnToolContext, evento_id: &str) -> bool {
    let trimmed = evento_id.trim();
    if trimmed.is_empty() {
        return false;
    }
    context.known_event_ids.contains(trimmed)
        || context
            .active_event_id
            .as_deref()
            .map(|id| id.trim() == trimmed)
            .unwrap_or(false)
}

fn format_ticket_url(public_base_url: Option<&str>, scan_token: &str) -> String {
    let scan_token = scan_token.trim();
    if let Some(base) = public_base_url {
        let trimmed = base.trim().trim_end_matches('/');
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            return format!("{trimmed}/#/ticket/{scan_token}");
        }
    }
    format!("/#/ticket/{scan_token}")
}

fn format_whatsapp_url(
    contacto: &str,
    nombre_comprador: &str,
    ticket_url: &str,
    estado_pago: &str,
) -> Option<String> {
    let digits: String = contacto.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let phone = if digits.len() == 9 {
        format!("51{digits}")
    } else {
        digits
    };
    let message = if estado_pago.eq_ignore_ascii_case("Pagado") {
        format!(
            "¡Hola {}! Aquí tienes tu entrada para el evento: {}",
            nombre_comprador.trim(),
            ticket_url
        )
    } else {
        format!(
            "¡Hola {}! Tu entrada para el evento está reservada. Puedes verla aquí: {}",
            nombre_comprador.trim(),
            ticket_url
        )
    };
    let encoded = url::form_urlencoded::byte_serialize(message.as_bytes()).collect::<String>();
    Some(format!("https://wa.me/{phone}?text={encoded}"))
}

#[derive(Clone)]
pub struct AgentToolDispatcher {
    create_presentation: Arc<CreatePresentation<SqlitePresentationRepository>>,
    list_presentations: Arc<ListPresentations<PresentationSummaryRepository>>,
    get_presentation: Arc<GetPresentation<SqlitePresentationRepository>>,
    add_lyrics: Arc<AddLyrics<SqliteShowItemRepository>>,
    add_note: Arc<AddNote<SqliteShowItemRepository>>,
    upload_track: Arc<UploadTrack<SqliteShowItemRepository>>,
    item_repo: Arc<SqliteShowItemRepository>,
    download_youtube_audio: Arc<DownloadYouTubeAudio>,
    staged_audio_registry: Arc<StagedAudioRegistry>,
    separate_vocals: Arc<SeparateVocals<SqliteShowItemRepository>>,
    ticketing: Option<TicketingServices>,
    public_base_url: Option<String>,
}

impl AgentToolDispatcher {
    pub fn new(state: AppState) -> Self {
        Self::from_state(&state)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_components(
        create_presentation: Arc<CreatePresentation<SqlitePresentationRepository>>,
        list_presentations: Arc<ListPresentations<PresentationSummaryRepository>>,
        get_presentation: Arc<GetPresentation<SqlitePresentationRepository>>,
        add_lyrics: Arc<AddLyrics<SqliteShowItemRepository>>,
        add_note: Arc<AddNote<SqliteShowItemRepository>>,
        upload_track: Arc<UploadTrack<SqliteShowItemRepository>>,
        item_repo: Arc<SqliteShowItemRepository>,
        download_youtube_audio: Arc<DownloadYouTubeAudio>,
        staged_audio_registry: Arc<StagedAudioRegistry>,
        separate_vocals: Arc<SeparateVocals<SqliteShowItemRepository>>,
    ) -> Self {
        Self {
            create_presentation,
            list_presentations,
            get_presentation,
            add_lyrics,
            add_note,
            upload_track,
            item_repo,
            download_youtube_audio,
            staged_audio_registry,
            separate_vocals,
            ticketing: None,
            public_base_url: None,
        }
    }

    pub fn with_ticketing(mut self, ticketing: TicketingServices) -> Self {
        self.ticketing = Some(ticketing);
        self
    }

    pub fn with_public_base_url(mut self, url: Option<String>) -> Self {
        self.public_base_url = url;
        self
    }

    pub fn from_state(state: &AppState) -> Self {
        Self::from_components(
            state.create_presentation.clone(),
            state.list_presentations.clone(),
            state.get_presentation.clone(),
            state.add_lyrics.clone(),
            state.add_note.clone(),
            state.upload_track.clone(),
            state.item_repo.clone(),
            state.download_youtube_audio.clone(),
            state.staged_audio_registry.clone(),
            state.separate_vocals.clone(),
        )
        .with_ticketing(state.ticketing.clone())
        .with_public_base_url(std::env::var("SHIRLEY_PUBLIC_BASE_URL").ok())
    }

    pub async fn dispatch(&self, call: &ToolCall, context: &mut TurnToolContext) -> Value {
        let is_read_tool = matches!(
            call.name.as_str(),
            "list_presentations" | "get_presentation" | "listar_eventos" | "resumen_evento"
        );

        // 1. Check if tool call ID was already processed in this turn
        if context.executed_call_ids.contains(&call.id) {
            if is_read_tool {
                let sig = canonical_tool_signature(&call.name, &call.arguments);
                if let Some(cached) = context.executed_signatures.get(&sig) {
                    return cached.clone();
                }
            } else {
                return json!({
                    "warning": "duplicate_call_ignored",
                    "message": "Esta acción ya fue ejecutada en este turno"
                });
            }
        }

        // 2. Check canonical argument signature deduplication
        let signature = canonical_tool_signature(&call.name, &call.arguments);
        if let Some(cached) = context.executed_signatures.get(&signature) {
            if is_read_tool {
                context.executed_call_ids.insert(call.id.clone());
                return cached.clone();
            } else {
                context.executed_call_ids.insert(call.id.clone());
                return json!({
                    "warning": "duplicate_call_ignored",
                    "message": "Esta acción ya fue ejecutada en este turno"
                });
            }
        }

        // 3. Dispatch to allowed tools
        match call.name.as_str() {
            "create_presentation" => {
                let args: CreatePresentationArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => {
                        return json!({
                            "error": "validation_error",
                            "message": "Parámetros de herramienta inválidos"
                        });
                    }
                };

                let trimmed = args.name.trim();
                if trimmed.is_empty() {
                    return json!({
                        "error": "validation_error",
                        "message": "El nombre no puede estar vacío"
                    });
                }

                match self.create_presentation.execute(trimmed) {
                    Ok(pres) => {
                        context.known_presentation_ids.insert(pres.id.clone());
                        context.set_active_presentation_id(Some(pres.id.clone()));

                        let res = json!({
                            "id": pres.id,
                            "title": pres.title,
                            "created_at": pres.created_at
                        });

                        context.record_action(ToolAction {
                            event_id: None,
                            tool: "create_presentation".to_string(),
                            presentation_id: Some(pres.id),
                            summary: format!("Presentación “{}” creada", pres.title),
                            status: "success".to_string(),
                            details: Some(res.clone()),
                        });

                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => {
                        json!({ "error": "domain_error", "message": e.to_string() })
                    }
                }
            }

            "list_presentations" => match self.list_presentations.execute() {
                Ok(summaries) => {
                    for s in &summaries {
                        context.known_presentation_ids.insert(s.id.clone());
                    }

                    let res: Value = json!(
                        summaries
                            .into_iter()
                            .map(|s| {
                                json!({
                                    "id": s.id,
                                    "title": s.title,
                                    "total_items": s.total_items,
                                    "tracks_count": s.tracks_count,
                                    "lyrics_count": s.lyrics_count,
                                    "notes_count": s.notes_count,
                                })
                            })
                            .collect::<Vec<_>>()
                    );

                    context.executed_call_ids.insert(call.id.clone());
                    context.executed_signatures.insert(signature, res.clone());
                    res
                }
                Err(e) => {
                    json!({ "error": "domain_error", "message": e.to_string() })
                }
            },

            "get_presentation" => {
                let args: PresentationIdArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => {
                        return json!({
                            "error": "validation_error",
                            "message": "Parámetros de herramienta inválidos"
                        });
                    }
                };

                let pres_id = args.presentation_id.trim();
                if !context.known_presentation_ids.contains(pres_id) {
                    return json!({
                        "error": "unauthorized_presentation_id",
                        "message": "Identificador de presentación no autorizado en el contexto actual"
                    });
                }

                match self.get_presentation.execute(pres_id) {
                    Ok(pres) => {
                        context.set_confirmed_destination_id(Some(pres.id.clone()));
                        let items = self
                            .item_repo
                            .list_by_presentation(pres_id)
                            .unwrap_or_default();
                        for item in &items {
                            if item.kind() == ShowItemKind::Track {
                                context.known_track_ids.insert(item.id.clone());
                            }
                        }
                        let items_json: Vec<Value> = items
                            .iter()
                            .map(|item| {
                                serde_json::to_value(ShowItemResponse::from(item))
                                    .unwrap_or_default()
                            })
                            .collect();

                        let res = json!({
                            "id": pres.id,
                            "title": pres.title,
                            "items": items_json,
                        });

                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(_) => {
                        json!({
                            "error": "presentation_not_found",
                            "message": "No se encontró la presentación solicitada"
                        })
                    }
                }
            }

            "add_lyrics" => {
                let args: AddItemTextArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => {
                        return json!({
                            "error": "validation_error",
                            "message": "Parámetros de herramienta inválidos"
                        });
                    }
                };

                let pres_id = args.presentation_id.trim();
                if !context.known_presentation_ids.contains(pres_id) {
                    return json!({
                        "error": "unauthorized_presentation_id",
                        "message": "Identificador de presentación no autorizado en el contexto actual"
                    });
                }

                let trimmed_text = args.text.trim();
                if trimmed_text.is_empty() {
                    return json!({
                        "error": "validation_error",
                        "message": "El texto no puede estar vacío"
                    });
                }

                match self.add_lyrics.execute(pres_id, trimmed_text) {
                    Ok(item) => {
                        let res = json!({
                            "item_id": item.id,
                            "kind": "lyrics",
                            "position": item.position,
                            "presentation_id": item.presentation_id
                        });

                        context.record_action(ToolAction {
                            event_id: None,
                            tool: "add_lyrics".to_string(),
                            presentation_id: Some(pres_id.to_string()),
                            summary: "Letra añadida a la presentación".to_string(),
                            status: "success".to_string(),
                            details: Some(res.clone()),
                        });

                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => {
                        json!({ "error": "domain_error", "message": e.to_string() })
                    }
                }
            }

            "add_note" => {
                let args: AddItemTextArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => {
                        return json!({
                            "error": "validation_error",
                            "message": "Parámetros de herramienta inválidos"
                        });
                    }
                };

                let pres_id = args.presentation_id.trim();
                if !context.known_presentation_ids.contains(pres_id) {
                    return json!({
                        "error": "unauthorized_presentation_id",
                        "message": "Identificador de presentación no autorizado en el contexto actual"
                    });
                }

                let trimmed_text = args.text.trim();
                if trimmed_text.is_empty() {
                    return json!({
                        "error": "validation_error",
                        "message": "El texto no puede estar vacío"
                    });
                }

                match self.add_note.execute(pres_id, trimmed_text) {
                    Ok(item) => {
                        let res = json!({
                            "item_id": item.id,
                            "kind": "note",
                            "position": item.position,
                            "presentation_id": item.presentation_id
                        });

                        context.record_action(ToolAction {
                            event_id: None,
                            tool: "add_note".to_string(),
                            presentation_id: Some(pres_id.to_string()),
                            summary: "Nota añadida a la presentación".to_string(),
                            status: "success".to_string(),
                            details: Some(res.clone()),
                        });

                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => {
                        json!({ "error": "domain_error", "message": e.to_string() })
                    }
                }
            }

            "add_track_from_staged" => {
                let args: AddTrackArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => {
                        return json!({
                            "error": "validation_error",
                            "message": "Parámetros de herramienta inválidos"
                        });
                    }
                };

                let pres_id = args.presentation_id.trim();
                if !context.known_presentation_ids.contains(pres_id) {
                    return json!({
                        "error": "unauthorized_presentation_id",
                        "message": "Identificador de presentación no autorizado en el contexto actual"
                    });
                }

                let trimmed_title = args.title.trim();
                if trimmed_title.is_empty() {
                    return json!({
                        "error": "validation_error",
                        "message": "El título no puede estar vacío"
                    });
                }

                // Resolve staged audio source:
                // 1. Explicit staged_audio_id argument or inherited from turn capability
                let staged_id_opt = if let Some(ref arg_id) = args.staged_audio_id {
                    let tr = arg_id.trim();
                    if tr.is_empty() {
                        context.staged_audio_id.clone()
                    } else if !context.known_staged_audio_ids.contains(tr) {
                        return json!({
                            "error": "staged_audio_not_found",
                            "message": "Identificador de audio preparado no encontrado o no autorizado"
                        });
                    } else {
                        Some(tr.to_string())
                    }
                } else {
                    context.staged_audio_id.clone()
                };

                if let Some(ref staged_id) = staged_id_opt {
                    match self.staged_audio_registry.reserve(staged_id) {
                        Ok(lease) => {
                            match self.upload_track.execute(
                                pres_id,
                                trimmed_title,
                                &lease.stage_id,
                                AudioFormat::Mp3,
                                lease.size_bytes as i64,
                            ) {
                                Ok(item) => {
                                    let _ = self
                                        .staged_audio_registry
                                        .commit_consumed(&lease.staged_audio_id, &lease.lease_id);
                                    context.set_staged_audio_id(None);

                                    let res = json!({
                                        "item_id": item.id,
                                        "kind": "track",
                                        "title": trimmed_title,
                                        "position": item.position,
                                        "presentation_id": item.presentation_id
                                    });

                                    context.record_action(ToolAction {
                                        event_id: None,
                                        tool: "add_track_from_staged".to_string(),
                                        presentation_id: Some(pres_id.to_string()),
                                        summary: format!(
                                            "Pista “{}” añadida a la presentación",
                                            trimmed_title
                                        ),
                                        status: "success".to_string(),
                                        details: Some(res.clone()),
                                    });

                                    context.executed_call_ids.insert(call.id.clone());
                                    context.executed_signatures.insert(signature, res.clone());
                                    res
                                }
                                Err(e) => {
                                    let _ = self
                                        .staged_audio_registry
                                        .release(&lease.staged_audio_id, &lease.lease_id);
                                    json!({
                                        "error": "track_upload_failed",
                                        "message": e.to_string()
                                    })
                                }
                            }
                        }
                        Err(StagedAudioRegistryError::AlreadyConsumed) => {
                            json!({
                                "error": "audio_already_consumed",
                                "message": "El archivo de audio ya fue consumido o expiró"
                            })
                        }
                        Err(StagedAudioRegistryError::NotFound) => {
                            json!({
                                "error": "staged_audio_not_found",
                                "message": "Archivo de audio temporal no encontrado"
                            })
                        }
                        Err(e) => {
                                json!({
                                    "error": e.code(),
                                    "message": e.message()
                                })
                        }
                    }
                } else {
                    // 2. Request-scoped attachment fallback
                    match context.staged_audio.take() {
                        None => {
                            json!({
                                "error": "no_audio_staged",
                                "message": "No hay archivo de audio adjunto o preparado en esta solicitud"
                            })
                        }
                        Some(StagedAudioHandle::Consumed) => {
                            context.staged_audio = Some(StagedAudioHandle::Consumed);
                            json!({
                                "error": "audio_already_consumed",
                                "message": "El archivo de audio adjunto ya fue consumido"
                            })
                        }
                        Some(StagedAudioHandle::Failed) => {
                            context.staged_audio = Some(StagedAudioHandle::Failed);
                            json!({
                                "error": "audio_staging_failed",
                                "message": "El procesamiento del archivo de audio falló previamente"
                            })
                        }
                        Some(StagedAudioHandle::Available {
                            guard,
                            format,
                            byte_size,
                        }) => {
                            let stage_id = guard.stage_id().to_string();
                            match self.upload_track.execute(
                                pres_id,
                                trimmed_title,
                                &stage_id,
                                format,
                                byte_size,
                            ) {
                                Ok(item) => {
                                    guard.defuse();
                                    context.staged_audio = Some(StagedAudioHandle::Consumed);

                                    let res = json!({
                                        "item_id": item.id,
                                        "kind": "track",
                                        "title": trimmed_title,
                                        "position": item.position,
                                        "presentation_id": item.presentation_id
                                    });

                                    context.record_action(ToolAction {
                                        event_id: None,
                                        tool: "add_track_from_staged".to_string(),
                                        presentation_id: Some(pres_id.to_string()),
                                        summary: format!(
                                            "Pista “{}” añadida a la presentación",
                                            trimmed_title
                                        ),
                                        status: "success".to_string(),
                                        details: Some(res.clone()),
                                    });

                                    context.executed_call_ids.insert(call.id.clone());
                                    context.executed_signatures.insert(signature, res.clone());
                                    res
                                }
                                Err(e) => {
                                    context.staged_audio = Some(StagedAudioHandle::Failed);
                                    json!({
                                        "error": "track_upload_failed",
                                        "message": e.to_string()
                                    })
                                }
                            }
                        }
                    }
                }
            }

            "download_youtube_track" => {
                let args: DownloadYouTubeTrackArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => {
                        return json!({
                            "error": "validation_error",
                            "message": "Parámetros de herramienta inválidos"
                        });
                    }
                };

                let requested_title = args.title.as_deref().and_then(|t| {
                    let tr = t.trim();
                    if tr.is_empty() { None } else { Some(tr) }
                });

                match self
                    .download_youtube_audio
                    .execute(&args.url, requested_title)
                    .await
                {
                    Ok(output) => {
                        context.set_staged_audio_id(Some(output.staged_audio_id.clone()));

                        let res = json!({
                            "staged_audio_id": output.staged_audio_id,
                            "title": output.title,
                            "duration_secs": output.duration_secs,
                            "size_bytes": output.size_bytes,
                        });

                        context.record_action(ToolAction {
                            event_id: None,
                            tool: "download_youtube_track".to_string(),
                            presentation_id: None,
                            summary: format!("Audio «{}» preparado temporalmente", output.title),
                            status: "success".to_string(),
                            details: Some(json!({
                                "status": "success",
                                "title": output.title,
                                "byte_size": output.size_bytes,
                                "format": "mp3",
                                "staged_audio_id": output.staged_audio_id,
                                "duration_secs": output.duration_secs,
                            })),
                        });

                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => {
                            json!({
                                "error": e.code(),
                                "message": e.message()
                            })
                    }
                }
            }

            "separate_vocals" => {
                let args: SeparateVocalsArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => {
                        return json!({
                            "error": "validation_error",
                            "message": "Parámetros de herramienta inválidos"
                        });
                    }
                };

                let mode = match args.mode.as_str() {
                    "chorus" => VocalSeparationMode::Coros,
                    "instrumental" => VocalSeparationMode::Instrumental,
                    _ => {
                        return json!({
                            "error": "validation_error",
                            "message": "El modo debe ser 'chorus' o 'instrumental'"
                        });
                    }
                };

                if context.confirmed_destination_id.is_none() {
                    return json!({
                        "error": "destination_required",
                        "message": "Se requiere una presentación de destino confirmada antes de separar voces"
                    });
                }

                let source = match VocalAudioSource::resolve(
                    args.staged_audio_id,
                    args.track_id,
                    args.youtube_url,
                ) {
                    Ok(s) => s,
                    Err(e) => {
                        return json!({
                            "error": e.code(),
                            "message": e.message(),
                        });
                    }
                };

                if let VocalAudioSource::TrackId(ref tid) = source
                    && !context.known_track_ids.contains(tid)
                {
                    return json!({
                        "error": "unauthorized_track_id",
                        "message": "Pista no autorizada o no encontrada en la presentación activa"
                    });
                }

                if let VocalAudioSource::StagedAudioId(ref sid) = source
                    && !context.known_staged_audio_ids.contains(sid)
                {
                    return json!({
                        "error": "staged_audio_not_found",
                        "message": "Identificador de audio preparado no encontrado o no autorizado"
                    });
                }

                match self
                    .separate_vocals
                    .execute(SeparateVocalsInput { source, mode })
                    .await
                {
                    Ok(output) => {
                        context.set_staged_audio_id(Some(output.staged_audio_id.clone()));

                        let mode_str = match output.mode {
                            VocalSeparationMode::Coros => "chorus",
                            VocalSeparationMode::Instrumental => "instrumental",
                        };

                        let mode_label = match output.mode {
                            VocalSeparationMode::Coros => "Coros",
                            VocalSeparationMode::Instrumental => "Instrumental",
                        };

                        let res = json!({
                            "staged_audio_id": output.staged_audio_id,
                            "mode": mode_str,
                            "title": output.title,
                            "duration_secs": output.duration_secs,
                            "size_bytes": output.size_bytes,
                        });

                        context.record_action(ToolAction {
                            event_id: None,
                            tool: "separate_vocals".to_string(),
                            presentation_id: context.confirmed_destination_id.clone(),
                            summary: format!(
                                "Separación vocal ({}) completada para «{}»",
                                mode_label, output.title
                            ),
                            status: "success".to_string(),
                            details: Some(res.clone()),
                        });

                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => {
                            json!({
                                "error": e.code(),
                                "message": e.message()
                            })
                    }
                }
            }

            "crear_evento" => {
                let Some(ticketing) = self.ticketing.as_ref() else {
                    return json!({
                        "error": "service_unavailable",
                        "message": "El servicio de ticketing no está disponible"
                    });
                };

                let args: CrearEventoArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => {
                        return json!({
                            "error": "validation_error",
                            "message": "Parámetros de herramienta inválidos"
                        });
                    }
                };

                let nombre = args.nombre.trim();
                if nombre.is_empty() {
                    return json!({
                        "error": "validation_error",
                        "message": "El nombre del evento no puede estar vacío"
                    });
                }

                let mut tipos = Vec::new();
                for t in args.tipos_de_entrada {
                    let precio = match Money::from_soles(t.precio) {
                        Ok(m) => m,
                        Err(e) => {
                            return json!({
                                "error": "validation_error",
                                "message": format!("Precio inválido: {e}")
                            });
                        }
                    };
                    match TicketType::new(t.nombre, precio, t.capacidad) {
                        Ok(tipo) => tipos.push(tipo),
                        Err(e) => {
                            return json!({
                                "error": "validation_error",
                                "message": format!("Tipo de entrada inválido: {e}")
                            });
                        }
                    }
                }

                match ticketing.create_event.execute(CreateEventInput {
                    nombre: nombre.to_string(),
                    fecha_hora: args.fecha_hora.trim().to_string(),
                    lugar: args.lugar.trim().to_string(),
                    tipos_de_entrada: tipos,
                }) {
                    Ok(event) => {
                        context.known_event_ids.insert(event.id.clone());
                        context.set_active_event_id(Some(event.id.clone()));

                        let res = json!({
                            "id": event.id,
                            "nombre": event.nombre,
                            "fecha_hora": event.fecha_hora,
                            "lugar": event.lugar,
                            "finalizado": event.finalizado,
                            "tipos_de_entrada": event.tipos_de_entrada,
                        });

                        context.record_action(ToolAction {
                            tool: "crear_evento".to_string(),
                            presentation_id: None,
                            event_id: Some(event.id.clone()),
                            summary: format!("Evento «{}» creado exitosamente", event.nombre),
                            status: "success".to_string(),
                            details: Some(res.clone()),
                        });

                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => {
                        app_error_to_json(&e)
                    }
                }
            }

            "listar_eventos" => {
                let Some(ticketing) = self.ticketing.as_ref() else {
                    return json!({
                        "error": "service_unavailable",
                        "message": "El servicio de ticketing no está disponible"
                    });
                };

                let args: ListarEventosArgs = if call.arguments.trim().is_empty() {
                    ListarEventosArgs::default()
                } else {
                    match serde_json::from_str(&call.arguments) {
                        Ok(a) => a,
                        Err(_) => {
                            return json!({
                                "error": "validation_error",
                                "message": "Parámetros de herramienta inválidos"
                            });
                        }
                    }
                };

                match ticketing.list_events.execute() {
                    Ok(events) => {
                        let incluir_finalizados = args.incluir_finalizados.unwrap_or(false);
                        let mut list = Vec::new();
                        for event in events {
                            context.known_event_ids.insert(event.id.clone());
                            if !incluir_finalizados && event.finalizado {
                                continue;
                            }
                            list.push(json!({
                                "id": event.id,
                                "nombre": event.nombre,
                                "fecha_hora": event.fecha_hora,
                                "lugar": event.lugar,
                                "finalizado": event.finalizado,
                                "tipos_de_entrada": event.tipos_de_entrada,
                            }));
                        }

                        let res = json!({
                            "eventos": list,
                        });

                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => {
                        app_error_to_json(&e)
                    }
                }
            }

            "vender_entradas" => {
                let Some(ticketing) = self.ticketing.as_ref() else {
                    return json!({
                        "error": "service_unavailable",
                        "message": "El servicio de ticketing no está disponible"
                    });
                };

                let args: VenderEntradasArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => {
                        return json!({
                            "error": "validation_error",
                            "message": "Parámetros de herramienta inválidos"
                        });
                    }
                };

                let evento_id = args.evento_id.trim();
                if !is_authorized_event(context, evento_id) {
                    return json!({
                        "error": "unauthorized_event_id",
                        "message": "Identificador de evento no autorizado o desconocido en el contexto actual. Lista los eventos primero."
                    });
                }

                let medio_pago = match PaymentMethod::parse(&args.medio_pago) {
                    Ok(m) => m,
                    Err(e) => {
                        return app_error_to_json(&e);
                    }
                };

                let estado_pago = if medio_pago == PaymentMethod::Cortesia {
                    PaymentStatus::Pagado
                } else {
                    match PaymentStatus::parse(&args.estado_pago) {
                        Ok(s) => s,
                        Err(e) => {
                            return app_error_to_json(&e);
                        }
                    }
                };

                let input = SellTicketsInput {
                    evento_id: evento_id.to_string(),
                    interesado_id: None,
                    nombre_comprador: args.nombre_comprador.trim().to_string(),
                    contacto: args.contacto.trim().to_string(),
                    tipo_entrada: args.tipo_entrada.trim().to_string(),
                    cantidad: args.cantidad,
                    medio_pago,
                    estado_pago,
                };

                match ticketing.sell_tickets.execute(input) {
                    Ok(tickets) => {
                        let mut items = Vec::new();
                        for ticket in &tickets {
                            context.known_ticket_ids.insert(ticket.id.clone());
                            let ticket_url = format_ticket_url(self.public_base_url.as_deref(), ticket.scan_token.as_str());
                            let whatsapp_url = format_whatsapp_url(
                                &ticket.contacto,
                                &ticket.nombre_comprador,
                                &ticket_url,
                                ticket.estado_pago.as_str(),
                            );

                            items.push(json!({
                                "id": ticket.id,
                                "scan_token": ticket.scan_token.as_str(),
                                "ticket_url": ticket_url,
                                "whatsapp_url": whatsapp_url,
                                "nombre_comprador": ticket.nombre_comprador,
                                "tipo_entrada": ticket.tipo_entrada,
                                "precio": ticket.precio.to_soles(),
                                "medio_pago": ticket.medio_pago.as_str(),
                                "estado_pago": ticket.estado_pago.as_str(),
                            }));
                        }

                        let res = json!({
                            "entradas": items,
                            "total_entradas": tickets.len(),
                        });

                        context.record_action(ToolAction {
                            tool: "vender_entradas".to_string(),
                            presentation_id: None,
                            event_id: Some(evento_id.to_string()),
                            summary: format!(
                                "{} entrada(s) vendida(s) para «{}» ({})",
                                tickets.len(),
                                args.nombre_comprador.trim(),
                                args.tipo_entrada.trim()
                            ),
                            status: "success".to_string(),
                            details: Some(res.clone()),
                        });

                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => {
                        app_error_to_json(&e)
                    }
                }
            }

            "confirmar_pago_entrada" => {
                let Some(ticketing) = self.ticketing.as_ref() else {
                    return json!({
                        "error": "service_unavailable",
                        "message": "El servicio de ticketing no está disponible"
                    });
                };

                let args: ConfirmarPagoEntradaArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => {
                        return json!({
                            "error": "validation_error",
                            "message": "Parámetros de herramienta inválidos"
                        });
                    }
                };

                let entrada_id = args.entrada_id.trim();
                if !context.known_ticket_ids.contains(entrada_id) {
                    return json!({
                        "error": "unauthorized_ticket_id",
                        "message": "Identificador de entrada no autorizado o desconocido en el contexto actual"
                    });
                }

                match ticketing.confirm_ticket_payment.execute(entrada_id) {
                    Ok(ticket) => {
                        let res = json!({
                            "id": ticket.id,
                            "evento_id": ticket.evento_id,
                            "nombre_comprador": ticket.nombre_comprador,
                            "tipo_entrada": ticket.tipo_entrada,
                            "precio": ticket.precio.to_soles(),
                            "estado_pago": ticket.estado_pago.as_str(),
                        });

                        context.record_action(ToolAction {
                            tool: "confirmar_pago_entrada".to_string(),
                            presentation_id: None,
                            event_id: Some(ticket.evento_id.clone()),
                            summary: format!("Pago confirmado para la entrada {}", ticket.id),
                            status: "success".to_string(),
                            details: Some(res.clone()),
                        });

                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => {
                        app_error_to_json(&e)
                    }
                }
            }

            "registrar_gasto" => {
                let Some(ticketing) = self.ticketing.as_ref() else {
                    return json!({
                        "error": "service_unavailable",
                        "message": "El servicio de ticketing no está disponible"
                    });
                };

                let args: RegistrarGastoArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => {
                        return json!({
                            "error": "validation_error",
                            "message": "Parámetros de herramienta inválidos"
                        });
                    }
                };

                let evento_id = args.evento_id.trim();
                if !is_authorized_event(context, evento_id) {
                    return json!({
                        "error": "unauthorized_event_id",
                        "message": "Identificador de evento no autorizado o desconocido en el contexto actual. Lista los eventos primero."
                    });
                }

                let categoria = match ExpenseCategory::parse(&args.categoria) {
                    Ok(c) => c,
                    Err(e) => {
                        return app_error_to_json(&e);
                    }
                };

                let monto = match Money::from_soles(args.monto) {
                    Ok(m) => m,
                    Err(e) => {
                        return json!({ "error": "validation_error", "message": format!("Monto inválido: {e}") });
                    }
                };

                let fecha = match args.fecha.as_deref() {
                    Some(f) if !f.trim().is_empty() => f.trim().to_string(),
                    _ => chrono::Utc::now().format("%Y-%m-%d").to_string(),
                };

                match ticketing.create_expense.execute(CreateExpenseInput {
                    evento_id: evento_id.to_string(),
                    categoria,
                    monto,
                    descripcion: args.descripcion.trim().to_string(),
                    fecha,
                }) {
                    Ok(expense) => {
                        let res = json!({
                            "id": expense.id,
                            "evento_id": expense.evento_id,
                            "categoria": expense.categoria.as_str(),
                            "monto": expense.monto.to_soles(),
                            "descripcion": expense.descripcion,
                            "fecha": expense.fecha,
                        });

                        context.record_action(ToolAction {
                            tool: "registrar_gasto".to_string(),
                            presentation_id: None,
                            event_id: Some(evento_id.to_string()),
                            summary: format!(
                                "Gasto de S/ {:.2} registrado ({})",
                                expense.monto.to_soles(),
                                expense.categoria.as_str()
                            ),
                            status: "success".to_string(),
                            details: Some(res.clone()),
                        });

                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => {
                        app_error_to_json(&e)
                    }
                }
            }

            "resumen_evento" => {
                let Some(ticketing) = self.ticketing.as_ref() else {
                    return json!({
                        "error": "service_unavailable",
                        "message": "El servicio de ticketing no está disponible"
                    });
                };

                let args: ResumenEventoArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => {
                        return json!({
                            "error": "validation_error",
                            "message": "Parámetros de herramienta inválidos"
                        });
                    }
                };

                let evento_id = args.evento_id.trim();
                if !is_authorized_event(context, evento_id) {
                    return json!({
                        "error": "unauthorized_event_id",
                        "message": "Identificador de evento no autorizado o desconocido en el contexto actual. Lista los eventos primero."
                    });
                }

                match ticketing.get_summary.execute(evento_id) {
                    Ok(summary) => {
                        let res = json!({
                            "evento_id": evento_id,
                            "cobrado": summary.cobrado.to_soles(),
                            "pendiente": summary.pendiente.to_soles(),
                            "total_proyectado": summary.total_proyectado.to_soles(),
                            "gastos": summary.gastos.to_soles(),
                            "ganancia_neta": summary.ganancia_neta_soles,
                            "capacidad_total": summary.capacidad_total,
                            "entradas_emitidas": summary.entradas_emitidas,
                            "aforo_proyectado": summary.aforo_proyectado,
                            "total_cobrado": summary.cobrado.to_soles(),
                            "total_pendiente": summary.pendiente.to_soles(),
                            "total_gastos": summary.gastos.to_soles(),
                        });

                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => {
                        app_error_to_json(&e)
                    }
                }
            }

            "registrar_interesado" => {
                let Some(ticketing) = self.ticketing.as_ref() else {
                    return json!({
                        "error": "service_unavailable",
                        "message": "El servicio de ticketing no está disponible"
                    });
                };

                let args: RegistrarInteresadoArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => {
                        return json!({
                            "error": "validation_error",
                            "message": "Parámetros de herramienta inválidos"
                        });
                    }
                };

                let evento_id = args.evento_id.trim();
                if !is_authorized_event(context, evento_id) {
                    return json!({
                        "error": "unauthorized_event_id",
                        "message": "Identificador de evento no autorizado o desconocido en el contexto actual. Lista los eventos primero."
                    });
                }

                let input = CreateInterestInput {
                    evento_id: evento_id.to_string(),
                    nombre: args.nombre.trim().to_string(),
                    contacto: args.contacto.trim().to_string(),
                    canal: args.canal.trim().to_string(),
                    promotora: args.promotora.trim().to_string(),
                    personas: args.personas,
                };

                match ticketing.create_interest.execute(input) {
                    Ok(interest) => {
                        let res = json!({
                            "id": interest.id,
                            "evento_id": interest.evento_id,
                            "nombre": interest.nombre,
                            "contacto": interest.contacto,
                            "canal": interest.canal,
                            "promotora": interest.promotora,
                            "personas": interest.personas,
                            "estado": interest.estado.as_str(),
                        });

                        context.record_action(ToolAction {
                            tool: "registrar_interesado".to_string(),
                            presentation_id: None,
                            event_id: Some(evento_id.to_string()),
                            summary: format!(
                                "Interesado «{}» registrado ({} personas)",
                                interest.nombre, interest.personas
                            ),
                            status: "success".to_string(),
                            details: Some(res.clone()),
                        });

                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => {
                        app_error_to_json(&e)
                    }
                }
            }

            "checkin_entrada" => {
                let Some(ticketing) = self.ticketing.as_ref() else {
                    return json!({
                        "error": "service_unavailable",
                        "message": "El servicio de ticketing no está disponible"
                    });
                };

                let args: CheckinEntradaArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => {
                        return json!({
                            "error": "validation_error",
                            "message": "Parámetros de herramienta inválidos"
                        });
                    }
                };

                let evento_id = args.evento_id.trim();
                if !is_authorized_event(context, evento_id) {
                    return json!({
                        "error": "unauthorized_event_id",
                        "message": "Identificador de evento no autorizado o desconocido en el contexto actual. Lista los eventos primero."
                    });
                }

                match ticketing.check_in_ticket.execute(evento_id, args.codigo.trim()) {
                    Ok(outcome) => {
                        let res = serde_json::to_value(&outcome).unwrap_or_default();

                        let summary = match &outcome {
                            CheckInOutcome::Valida { nombre_comprador, tipo_entrada, .. } => {
                                format!("Check-in VÁLIDO para «{}» ({})", nombre_comprador, tipo_entrada)
                            }
                            CheckInOutcome::YaUsada { hora_ingreso } => {
                                format!("Entrada YA USADA (ingresó a las {})", hora_ingreso)
                            }
                            CheckInOutcome::Invalida { motivo } => {
                                format!("Check-in INVÁLIDO ({})", motivo)
                            }
                        };

                        context.record_action(ToolAction {
                            tool: "checkin_entrada".to_string(),
                            presentation_id: None,
                            event_id: Some(evento_id.to_string()),
                            summary,
                            status: "success".to_string(),
                            details: Some(res.clone()),
                        });

                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => {
                        app_error_to_json(&e)
                    }
                }
            }

            _ => {
                json!({
                    "error": "unknown_tool",
                    "message": "Herramienta no permitida"
                })
            }
        }
    }
}
