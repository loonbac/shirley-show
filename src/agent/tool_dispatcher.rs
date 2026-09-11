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
            state.download_youtube_audio.clone(),
            state.staged_audio_registry.clone(),
            state.separate_vocals.clone(),
        )
    }

    pub async fn dispatch(&self, call: &ToolCall, context: &mut TurnToolContext) -> Value {
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

            _ => {
                json!({
                    "error": "unknown_tool",
                    "message": "Herramienta no permitida"
                })
            }
        }
    }
}
