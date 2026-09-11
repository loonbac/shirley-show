use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::agent::tool_signature::canonical_tool_signature;
use crate::agent::turn_context::{StagedAudioHandle, TurnToolContext};
use crate::domain::agent_tool::{ToolAction, ToolCall};
use crate::handlers::dto::ShowItemResponse;
use crate::infra::sqlite::presentation_repository::SqlitePresentationRepository;
use crate::infra::sqlite::presentation_summary_repository::PresentationSummaryRepository;
use crate::infra::sqlite::show_item_repository::SqliteShowItemRepository;
use crate::ports::show_item_reader::ShowItemReader;
use crate::state::AppState;
use crate::use_cases::add_lyrics::AddLyrics;
use crate::use_cases::add_note::AddNote;
use crate::use_cases::create_presentation::CreatePresentation;
use crate::use_cases::get_presentation::GetPresentation;
use crate::use_cases::list_presentations::ListPresentations;
use crate::use_cases::upload_track::UploadTrack;

#[derive(Deserialize)]
struct CreatePresentationArgs {
    name: String,
}

#[derive(Deserialize)]
struct PresentationIdArgs {
    presentation_id: String,
}

#[derive(Deserialize)]
struct AddItemTextArgs {
    presentation_id: String,
    text: String,
}

#[derive(Deserialize)]
struct AddTrackArgs {
    presentation_id: String,
    title: String,
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
}

impl AgentToolDispatcher {
    pub fn new(state: AppState) -> Self {
        Self::from_state(&state)
    }

    pub fn from_components(
        create_presentation: Arc<CreatePresentation<SqlitePresentationRepository>>,
        list_presentations: Arc<ListPresentations<PresentationSummaryRepository>>,
        get_presentation: Arc<GetPresentation<SqlitePresentationRepository>>,
        add_lyrics: Arc<AddLyrics<SqliteShowItemRepository>>,
        add_note: Arc<AddNote<SqliteShowItemRepository>>,
        upload_track: Arc<UploadTrack<SqliteShowItemRepository>>,
        item_repo: Arc<SqliteShowItemRepository>,
    ) -> Self {
        Self {
            create_presentation,
            list_presentations,
            get_presentation,
            add_lyrics,
            add_note,
            upload_track,
            item_repo,
        }
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
        )
    }

    pub fn dispatch(&self, call: &ToolCall, context: &mut TurnToolContext) -> Value {
        let is_read_tool = matches!(
            call.name.as_str(),
            "list_presentations" | "get_presentation"
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
                            tool: "create_presentation".to_string(),
                            presentation_id: pres.id,
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

            "list_presentations" => {
                match self.list_presentations.execute() {
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
                }
            }

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
                        let items = self
                            .item_repo
                            .list_by_presentation(pres_id)
                            .unwrap_or_default();
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
                            tool: "add_lyrics".to_string(),
                            presentation_id: pres_id.to_string(),
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
                            tool: "add_note".to_string(),
                            presentation_id: pres_id.to_string(),
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

                match context.staged_audio.take() {
                    None => {
                        json!({
                            "error": "no_audio_staged",
                            "message": "No hay archivo de audio adjunto en esta solicitud"
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
                                    tool: "add_track_from_staged".to_string(),
                                    presentation_id: pres_id.to_string(),
                                    summary: format!("Pista “{}” añadida a la presentación", trimmed_title),
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

            _ => {
                json!({
                    "error": "unknown_tool",
                    "message": "Herramienta no permitida"
                })
            }
        }
    }
}
