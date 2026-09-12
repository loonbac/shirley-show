use crate::agent::destructive_confirmation::{
    ConfirmationReply, DestructiveTarget, DestructiveTool, PendingDestructiveAction,
    PendingDestructiveRegistry,
};
use crate::domain::move_direction::MoveDirection;
use crate::domain::show_item::ShowItemPayload;
use crate::domain::ticketing::{
    ExpenseCategory, InterestStatus, Money, PaymentMethod, PaymentStatus, TicketType,
};
use crate::infra::sqlite::cleanup_repository::SqliteCleanupRepository;
use crate::ports::ticketing::{CheckInOutcome, SellTicketsInput};
use crate::state::ticketing_services::TicketingServices;
use crate::use_cases::delete_item::DeleteItem;
use crate::use_cases::delete_presentation::DeletePresentation;
use crate::use_cases::rename_presentation::RenamePresentation;
use crate::use_cases::reorder_item::ReorderItem;
use crate::use_cases::ticketing::events::{CreateEventInput, UpdateEventInput};
use crate::use_cases::ticketing::expenses::UpdateExpenseInput;
use crate::use_cases::ticketing::interests::{ConvertInterestInput, UpdateInterestInput};
use crate::use_cases::ticketing::{CreateExpenseInput, CreateInterestInput};
use crate::use_cases::update_show_item::UpdateShowItem;
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenombrarPresentacionArgs {
    presentacion_id: String,
    nuevo_nombre: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReordenarElementoArgs {
    presentacion_id: String,
    elemento_id: String,
    #[serde(default)]
    posicion_destino: Option<i64>,
    #[serde(default)]
    direccion: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EditarElementoArgs {
    presentacion_id: String,
    elemento_id: String,
    #[serde(default)]
    texto: Option<String>,
    #[serde(default)]
    titulo: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EditarEventoArgs {
    evento_id: String,
    nombre: String,
    fecha_hora: String,
    lugar: String,
    tipos_de_entrada: Vec<CrearEventoTipoEntradaArgs>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EliminarPresentacionArgs {
    presentacion_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EliminarElementoArgs {
    presentacion_id: String,
    elemento_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EventoIdArgs {
    evento_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EliminarEntradaArgs {
    entrada_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EliminarGastoArgs {
    gasto_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EliminarInteresadoArgs {
    interesado_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReabrirEventoArgs {
    evento_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EditarGastoArgs {
    gasto_id: String,
    categoria: String,
    monto: f64,
    descripcion: String,
    fecha: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EditarInteresadoArgs {
    interesado_id: String,
    nombre: String,
    contacto: String,
    canal: String,
    personas: u32,
    promotora: String,
    estado: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfirmarInteresadoArgs {
    interesado_id: String,
    tipo_entrada: String,
    cantidad: u32,
    medio_pago: String,
    estado_pago: String,
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

fn is_authorized_presentation(context: &TurnToolContext, pres_id: &str) -> bool {
    let trimmed = pres_id.trim();
    if trimmed.is_empty() {
        return false;
    }
    context.known_presentation_ids.contains(trimmed)
        || context
            .active_presentation_id
            .as_deref()
            .map(|id| id.trim() == trimmed)
            .unwrap_or(false)
}

fn validation_err(msg: impl Into<String>) -> Value {
    json!({
        "error": "validation_error",
        "message": msg.into()
    })
}

fn unauthorized_presentation_err() -> Value {
    json!({
        "error": "unauthorized_presentation_id",
        "message": "Identificador de presentación no autorizado en el contexto actual"
    })
}

fn unauthorized_event_err() -> Value {
    json!({
        "error": "unauthorized_event_id",
        "message": "Identificador de evento no autorizado en el contexto actual"
    })
}

fn service_unavailable_err() -> Value {
    json!({
        "error": "service_unavailable",
        "message": "El servicio de ticketing no está disponible"
    })
}

fn normalize_event_fecha_hora(s: &str) -> String {
    let trimmed = s.trim();
    if chrono::DateTime::parse_from_rfc3339(trimmed).is_ok() {
        return trimmed.to_string();
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M") {
        return format!("{}:00Z", naive.format("%Y-%m-%dT%H:%M"));
    }
    trimmed.to_string()
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

fn format_tickets_response(
    tickets: &[crate::domain::ticketing::Ticket],
    public_base_url: Option<&str>,
    context: &mut TurnToolContext,
) -> Vec<Value> {
    let mut items = Vec::with_capacity(tickets.len());
    for ticket in tickets {
        context.known_ticket_ids.insert(ticket.id.clone());
        let ticket_url = format_ticket_url(public_base_url, ticket.scan_token.as_str());
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
    items
}

fn parse_ticket_types(
    tipos_de_entrada: Vec<CrearEventoTipoEntradaArgs>,
) -> Result<Vec<TicketType>, Value> {
    if tipos_de_entrada.is_empty() {
        return Err(validation_err("Debe incluir al menos un tipo de entrada"));
    }
    let mut tipos = Vec::with_capacity(tipos_de_entrada.len());
    for t in tipos_de_entrada {
        let precio = match Money::from_soles(t.precio) {
            Ok(m) => m,
            Err(e) => return Err(validation_err(format!("Precio inválido: {e}"))),
        };
        match TicketType::new(t.nombre, precio, t.capacidad) {
            Ok(tipo) => tipos.push(tipo),
            Err(e) => return Err(validation_err(format!("Tipo de entrada inválido: {e}"))),
        }
    }
    Ok(tipos)
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
    rename_presentation: Arc<RenamePresentation<SqlitePresentationRepository>>,
    reorder_item: Arc<ReorderItem<SqliteShowItemRepository>>,
    update_show_item: Arc<UpdateShowItem<SqliteShowItemRepository, SqliteShowItemRepository>>,
    ticketing: Option<TicketingServices>,
    public_base_url: Option<String>,
    delete_presentation: Option<Arc<DeletePresentation<SqlitePresentationRepository>>>,
    delete_item: Option<Arc<DeleteItem<SqliteShowItemRepository, SqliteCleanupRepository>>>,
    destructive_registry: Option<Arc<PendingDestructiveRegistry>>,
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
        rename_presentation: Arc<RenamePresentation<SqlitePresentationRepository>>,
        reorder_item: Arc<ReorderItem<SqliteShowItemRepository>>,
        update_show_item: Arc<UpdateShowItem<SqliteShowItemRepository, SqliteShowItemRepository>>,
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
            rename_presentation,
            reorder_item,
            update_show_item,
            ticketing: None,
            public_base_url: None,
            delete_presentation: None,
            delete_item: None,
            destructive_registry: None,
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

    pub fn with_destructive_presentation(
        mut self,
        delete_presentation: Arc<DeletePresentation<SqlitePresentationRepository>>,
        delete_item: Arc<DeleteItem<SqliteShowItemRepository, SqliteCleanupRepository>>,
    ) -> Self {
        self.delete_presentation = Some(delete_presentation);
        self.delete_item = Some(delete_item);
        self
    }

    pub fn with_destructive_registry(mut self, registry: Arc<PendingDestructiveRegistry>) -> Self {
        self.destructive_registry = Some(registry);
        self
    }

    pub fn destructive_registry(&self) -> Option<&Arc<PendingDestructiveRegistry>> {
        self.destructive_registry.as_ref()
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
            state.rename_presentation.clone(),
            state.reorder_item.clone(),
            state.update_show_item.clone(),
        )
        .with_ticketing(state.ticketing.clone())
        .with_public_base_url(std::env::var("SHIRLEY_PUBLIC_BASE_URL").ok())
        .with_destructive_presentation(state.delete_presentation.clone(), state.delete_item.clone())
        .with_destructive_registry(state.destructive_registry.clone())
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
                    Err(_) => return validation_err("Parámetros de herramienta inválidos"),
                };

                let pres_id = args.presentation_id.trim();
                if !is_authorized_presentation(context, pres_id) {
                    return unauthorized_presentation_err();
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
                    Err(_) => return validation_err("Parámetros de herramienta inválidos"),
                };

                let pres_id = args.presentation_id.trim();
                if !is_authorized_presentation(context, pres_id) {
                    return unauthorized_presentation_err();
                }

                let trimmed_text = args.text.trim();
                if trimmed_text.is_empty() {
                    return validation_err("El texto no puede estar vacío");
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
                    Err(_) => return validation_err("Parámetros de herramienta inválidos"),
                };

                let pres_id = args.presentation_id.trim();
                if !is_authorized_presentation(context, pres_id) {
                    return unauthorized_presentation_err();
                }

                let trimmed_text = args.text.trim();
                if trimmed_text.is_empty() {
                    return validation_err("El texto no puede estar vacío");
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
                    Err(_) => return validation_err("Parámetros de herramienta inválidos"),
                };

                let pres_id = args.presentation_id.trim();
                if !is_authorized_presentation(context, pres_id) {
                    return unauthorized_presentation_err();
                }

                let trimmed_title = args.title.trim();
                if trimmed_title.is_empty() {
                    return validation_err("El título no puede estar vacío");
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

                let tipos = match parse_ticket_types(args.tipos_de_entrada) {
                    Ok(t) => t,
                    Err(err_val) => return err_val,
                };

                match ticketing.create_event.execute(CreateEventInput {
                    nombre: nombre.to_string(),
                    fecha_hora: normalize_event_fecha_hora(&args.fecha_hora),
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
                    Err(e) => app_error_to_json(&e),
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
                    Err(e) => app_error_to_json(&e),
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
                        let items = format_tickets_response(
                            &tickets,
                            self.public_base_url.as_deref(),
                            context,
                        );

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
                    Err(e) => app_error_to_json(&e),
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
                    Err(e) => app_error_to_json(&e),
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
                    Err(e) => app_error_to_json(&e),
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
                    Err(e) => app_error_to_json(&e),
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
                    Err(e) => app_error_to_json(&e),
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

                match ticketing
                    .check_in_ticket
                    .execute(evento_id, args.codigo.trim())
                {
                    Ok(outcome) => {
                        let res = serde_json::to_value(&outcome).unwrap_or_default();

                        let summary = match &outcome {
                            CheckInOutcome::Valida {
                                nombre_comprador,
                                tipo_entrada,
                                ..
                            } => {
                                format!(
                                    "Check-in VÁLIDO para «{}» ({})",
                                    nombre_comprador, tipo_entrada
                                )
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
                    Err(e) => app_error_to_json(&e),
                }
            }

            "renombrar_presentacion" => {
                let args: RenombrarPresentacionArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => return validation_err("Parámetros de herramienta inválidos"),
                };
                let pres_id = args.presentacion_id.trim();
                if !is_authorized_presentation(context, pres_id) {
                    return unauthorized_presentation_err();
                }
                let trimmed_title = args.nuevo_nombre.trim();
                if trimmed_title.is_empty() {
                    return validation_err("El nombre no puede estar vacío");
                }
                match self.rename_presentation.execute(pres_id, trimmed_title) {
                    Ok(pres) => {
                        let res = json!({
                            "id": pres.id,
                            "title": pres.title,
                            "updated_at": pres.updated_at
                        });
                        context.record_action(ToolAction {
                            tool: "renombrar_presentacion".to_string(),
                            presentation_id: Some(pres.id),
                            event_id: None,
                            summary: format!("Presentación renombrada a “{}”", pres.title),
                            status: "success".to_string(),
                            details: Some(res.clone()),
                        });
                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => app_error_to_json(&e),
                }
            }

            "reordenar_elemento" => {
                let args: ReordenarElementoArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => return validation_err("Parámetros de herramienta inválidos"),
                };
                let pres_id = args.presentacion_id.trim();
                if !is_authorized_presentation(context, pres_id) {
                    return unauthorized_presentation_err();
                }
                let item_id = args.elemento_id.trim();
                if item_id.is_empty() {
                    return validation_err("El identificador del elemento no puede estar vacío");
                }
                if (args.posicion_destino.is_some() && args.direccion.is_some())
                    || (args.posicion_destino.is_none() && args.direccion.is_none())
                {
                    return validation_err(
                        "Debe proporcionar exactamente un selector: 'posicion_destino' o 'direccion'",
                    );
                }

                let outcome = if let Some(target_pos) = args.posicion_destino {
                    if target_pos < 1 {
                        return validation_err("La posición destino debe ser al menos 1");
                    }
                    let target_idx = target_pos - 1;
                    self.reorder_item
                        .execute_target_position(pres_id, item_id, target_idx)
                } else if let Some(ref dir_str) = args.direccion {
                    let dir = match dir_str.trim().to_lowercase().as_str() {
                        "arriba" => MoveDirection::Up,
                        "abajo" => MoveDirection::Down,
                        _ => return validation_err("La dirección debe ser 'arriba' o 'abajo'"),
                    };
                    self.reorder_item.execute(pres_id, item_id, dir)
                } else {
                    unreachable!()
                };

                match outcome {
                    Ok(items) => {
                        let new_pos = items
                            .iter()
                            .find(|i| i.id == item_id)
                            .map(|i| (i.position + 1) as u32)
                            .unwrap_or(1);
                        let res = json!({
                            "item_id": item_id,
                            "position": new_pos,
                            "presentation_id": pres_id
                        });
                        context.record_action(ToolAction {
                            tool: "reordenar_elemento".to_string(),
                            presentation_id: Some(pres_id.to_string()),
                            event_id: None,
                            summary: format!(
                                "Elemento {} movido a la posición {}",
                                item_id, new_pos
                            ),
                            status: "success".to_string(),
                            details: Some(res.clone()),
                        });
                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => app_error_to_json(&e),
                }
            }

            "editar_elemento" => {
                let args: EditarElementoArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => return validation_err("Parámetros de herramienta inválidos"),
                };
                let pres_id = args.presentacion_id.trim();
                if !is_authorized_presentation(context, pres_id) {
                    return unauthorized_presentation_err();
                }
                let item_id = args.elemento_id.trim();
                if item_id.is_empty() {
                    return validation_err("El identificador del elemento no puede estar vacío");
                }
                if (args.texto.is_some() && args.titulo.is_some())
                    || (args.texto.is_none() && args.titulo.is_none())
                {
                    return validation_err(
                        "Debe proporcionar exactamente uno de 'texto' o 'titulo'",
                    );
                }

                let title_opt = if let Some(ref t) = args.titulo {
                    let trimmed = t.trim();
                    if trimmed.is_empty() {
                        return validation_err("El título no puede estar vacío");
                    }
                    Some(trimmed.to_string())
                } else {
                    None
                };

                let body_opt = if let Some(ref b) = args.texto {
                    let trimmed = b.trim();
                    if trimmed.is_empty() {
                        return validation_err("El texto no puede estar vacío");
                    }
                    Some(trimmed.to_string())
                } else {
                    None
                };

                match self
                    .update_show_item
                    .execute(pres_id, item_id, title_opt, body_opt)
                {
                    Ok(item) => {
                        let kind = item.kind().as_str().to_string();
                        let title = match &item.payload {
                            ShowItemPayload::Track(t) => Some(t.title.clone()),
                            _ => None,
                        };
                        let res = json!({
                            "item_id": item.id,
                            "kind": kind,
                            "title": title,
                            "updated_at": chrono::Utc::now().to_rfc3339()
                        });
                        context.record_action(ToolAction {
                            tool: "editar_elemento".to_string(),
                            presentation_id: Some(pres_id.to_string()),
                            event_id: None,
                            summary: format!(
                                "Elemento {} actualizado ({})",
                                item.id,
                                item.kind().as_str()
                            ),
                            status: "success".to_string(),
                            details: Some(res.clone()),
                        });
                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => app_error_to_json(&e),
                }
            }

            "editar_evento" => {
                let Some(ticketing) = self.ticketing.as_ref() else {
                    return service_unavailable_err();
                };
                let args: EditarEventoArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => return validation_err("Parámetros de herramienta inválidos"),
                };
                let evento_id = args.evento_id.trim();
                if !is_authorized_event(context, evento_id) {
                    return unauthorized_event_err();
                }
                if args.nombre.trim().is_empty() {
                    return validation_err("El nombre del evento no puede estar vacío");
                }
                if args.fecha_hora.trim().is_empty() {
                    return validation_err("La fecha/hora del evento no puede estar vacía");
                }
                if args.lugar.trim().is_empty() {
                    return validation_err("El lugar del evento no puede estar vacío");
                }
                if args.tipos_de_entrada.is_empty() {
                    return validation_err("Debe incluir al menos un tipo de entrada");
                }

                let tipos = match parse_ticket_types(args.tipos_de_entrada) {
                    Ok(t) => t,
                    Err(err_val) => return err_val,
                };

                match ticketing.update_event.execute(UpdateEventInput {
                    id: evento_id.to_string(),
                    nombre: args.nombre.trim().to_string(),
                    fecha_hora: normalize_event_fecha_hora(&args.fecha_hora),
                    lugar: args.lugar.trim().to_string(),
                    tipos_de_entrada: tipos,
                }) {
                    Ok(event) => {
                        let res = json!({
                            "id": event.id,
                            "nombre": event.nombre,
                            "fecha_hora": event.fecha_hora,
                            "lugar": event.lugar,
                            "estado": if event.finalizado { "Finalizado" } else { "Activo" }
                        });
                        context.record_action(ToolAction {
                            tool: "editar_evento".to_string(),
                            presentation_id: None,
                            event_id: Some(event.id.clone()),
                            summary: format!("Evento «{}» actualizado", event.nombre),
                            status: "success".to_string(),
                            details: Some(res.clone()),
                        });
                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => app_error_to_json(&e),
                }
            }

            "reabrir_evento" => {
                let Some(ticketing) = self.ticketing.as_ref() else {
                    return service_unavailable_err();
                };
                let args: ReabrirEventoArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => return validation_err("Parámetros de herramienta inválidos"),
                };
                let evento_id = args.evento_id.trim();
                if !is_authorized_event(context, evento_id) {
                    return unauthorized_event_err();
                }
                match ticketing.reopen_event.execute(evento_id) {
                    Ok(()) => {
                        let res = json!({
                            "id": evento_id,
                            "estado": "Activo"
                        });
                        context.record_action(ToolAction {
                            tool: "reabrir_evento".to_string(),
                            presentation_id: None,
                            event_id: Some(evento_id.to_string()),
                            summary: format!("Evento {} reabierto como Activo", evento_id),
                            status: "success".to_string(),
                            details: Some(res.clone()),
                        });
                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => app_error_to_json(&e),
                }
            }

            "editar_gasto" => {
                let Some(ticketing) = self.ticketing.as_ref() else {
                    return service_unavailable_err();
                };
                let args: EditarGastoArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => return validation_err("Parámetros de herramienta inválidos"),
                };
                let categoria = match ExpenseCategory::parse(&args.categoria) {
                    Ok(c) => c,
                    Err(e) => return validation_err(format!("Categoría inválida: {e}")),
                };
                let monto = match Money::from_soles(args.monto) {
                    Ok(m) => m,
                    Err(e) => return validation_err(format!("Monto inválido: {e}")),
                };
                let desc = args.descripcion.trim().to_string();
                if desc.is_empty() {
                    return validation_err("La descripción no puede estar vacía");
                }
                let fecha = args.fecha.trim().to_string();
                if fecha.is_empty() {
                    return validation_err("La fecha no puede estar vacía");
                }
                match ticketing.update_expense.execute(UpdateExpenseInput {
                    id: args.gasto_id.trim().to_string(),
                    categoria,
                    monto,
                    descripcion: desc,
                    fecha,
                }) {
                    Ok(expense) => {
                        let res = json!({
                            "id": expense.id,
                            "categoria": expense.categoria.as_str(),
                            "monto": expense.monto.to_soles(),
                            "descripcion": expense.descripcion,
                            "fecha": expense.fecha
                        });
                        context.record_action(ToolAction {
                            tool: "editar_gasto".to_string(),
                            presentation_id: None,
                            event_id: Some(expense.evento_id.clone()),
                            summary: format!(
                                "Gasto de S/ {:.2} actualizado ({})",
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
                    Err(e) => app_error_to_json(&e),
                }
            }

            "editar_interesado" => {
                let Some(ticketing) = self.ticketing.as_ref() else {
                    return service_unavailable_err();
                };
                let args: EditarInteresadoArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => return validation_err("Parámetros de herramienta inválidos"),
                };
                let nombre = args.nombre.trim().to_string();
                if nombre.is_empty() {
                    return validation_err("El nombre no puede estar vacío");
                }
                let contacto = args.contacto.trim().to_string();
                if contacto.is_empty() {
                    return validation_err("El contacto no puede estar vacío");
                }
                let canal = args.canal.trim().to_string();
                if canal.is_empty() {
                    return validation_err("El canal no puede estar vacío");
                }
                if args.personas < 1 {
                    return validation_err("El número de personas debe ser al menos 1");
                }
                let promotora = args.promotora.trim().to_string();
                if promotora.is_empty() {
                    return validation_err("La promotora no puede estar vacía");
                }
                let estado = match InterestStatus::parse(&args.estado) {
                    Ok(s) => s,
                    Err(e) => return validation_err(format!("Estado inválido: {e}")),
                };
                match ticketing.update_interest.execute(UpdateInterestInput {
                    id: args.interesado_id.trim().to_string(),
                    nombre,
                    contacto,
                    canal,
                    personas: args.personas,
                    promotora,
                    estado,
                }) {
                    Ok(lead) => {
                        let res = json!({
                            "id": lead.id,
                            "nombre": lead.nombre,
                            "personas": lead.personas,
                            "estado": lead.estado.as_str()
                        });
                        context.record_action(ToolAction {
                            tool: "editar_interesado".to_string(),
                            presentation_id: None,
                            event_id: Some(lead.evento_id.clone()),
                            summary: format!("Interesado «{}» actualizado", lead.nombre),
                            status: "success".to_string(),
                            details: Some(res.clone()),
                        });
                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => app_error_to_json(&e),
                }
            }

            "confirmar_interesado" => {
                let Some(ticketing) = self.ticketing.as_ref() else {
                    return service_unavailable_err();
                };
                let args: ConfirmarInteresadoArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => return validation_err("Parámetros de herramienta inválidos"),
                };
                if args.tipo_entrada.trim().is_empty() {
                    return validation_err("El tipo de entrada no puede estar vacío");
                }
                if args.cantidad < 1 {
                    return validation_err("La cantidad debe ser al menos 1");
                }
                let medio_pago = match PaymentMethod::parse(&args.medio_pago) {
                    Ok(m) => m,
                    Err(e) => return validation_err(format!("Medio de pago inválido: {e}")),
                };
                let estado_pago = match PaymentStatus::parse(&args.estado_pago) {
                    Ok(s) => s,
                    Err(e) => return validation_err(format!("Estado de pago inválido: {e}")),
                };

                match ticketing
                    .convert_interest_to_sale
                    .execute(ConvertInterestInput {
                        interesado_id: args.interesado_id.trim().to_string(),
                        tipo_entrada: args.tipo_entrada.trim().to_string(),
                        cantidad: args.cantidad,
                        medio_pago,
                        estado_pago,
                    }) {
                    Ok(tickets) => {
                        let items = format_tickets_response(
                            &tickets,
                            self.public_base_url.as_deref(),
                            context,
                        );
                        let event_id = tickets.first().map(|t| t.evento_id.clone());
                        let res = json!({
                            "interesado_id": args.interesado_id.trim(),
                            "tickets_creados": tickets.len(),
                            "tickets": items
                        });
                        context.record_action(ToolAction {
                            tool: "confirmar_interesado".to_string(),
                            presentation_id: None,
                            event_id,
                            summary: format!(
                                "{} entrada(s) confirmada(s) para interesado",
                                tickets.len()
                            ),
                            status: "success".to_string(),
                            details: Some(res.clone()),
                        });
                        context.executed_call_ids.insert(call.id.clone());
                        context.executed_signatures.insert(signature, res.clone());
                        res
                    }
                    Err(e) => app_error_to_json(&e),
                }
            }

            "eliminar_presentacion" => {
                let args: EliminarPresentacionArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => return validation_err("Parámetros de herramienta inválidos"),
                };
                let pres_id = args.presentacion_id.trim().to_string();
                if pres_id.is_empty() {
                    return validation_err("Identificador de presentación inválido");
                }
                let target = DestructiveTarget::Presentation {
                    id: pres_id.clone(),
                };
                self.dispatch_destructive(
                    call,
                    signature,
                    context,
                    DestructiveTool::DeletePresentation,
                    target,
                    |_active_ev| {
                        match self.get_presentation.execute(&pres_id) {
                            Ok(p) => Ok(p.title),
                            Err(e) => Err(app_error_to_json(&e)),
                        }
                    },
                    |_ctx, label| {
                        let delete_uc = self.delete_presentation.as_ref().ok_or_else(|| {
                            json!({ "error": "unconfigured", "message": "delete_presentation no configurado" })
                        })?;
                        match delete_uc.execute(&pres_id) {
                            Ok(_) => {
                                let action = ToolAction {
                                    tool: "eliminar_presentacion".to_string(),
                                    presentation_id: Some(pres_id.clone()),
                                    event_id: None,
                                    summary: format!("Presentación “{}” eliminada", label),
                                    status: "success".to_string(),
                                    details: Some(json!({ "id": pres_id, "status": "deleted" })),
                                };
                                Ok((action, json!({ "id": pres_id, "status": "deleted" })))
                            }
                            Err(e) => Err(app_error_to_json(&e)),
                        }
                    },
                )
            }

            "eliminar_elemento" => {
                let args: EliminarElementoArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => return validation_err("Parámetros de herramienta inválidos"),
                };
                let pres_id = args.presentacion_id.trim().to_string();
                let item_id = args.elemento_id.trim().to_string();
                if pres_id.is_empty() || item_id.is_empty() {
                    return validation_err("Identificadores inválidos");
                }
                let target = DestructiveTarget::ShowItem {
                    presentation_id: pres_id.clone(),
                    item_id: item_id.clone(),
                };
                self.dispatch_destructive(
                    call,
                    signature,
                    context,
                    DestructiveTool::DeleteItem,
                    target,
                    |_active_ev| {
                        let item = self.item_repo.get(&pres_id, &item_id).map_err(|e| app_error_to_json(&e))?.ok_or_else(|| json!({ "error": "item_not_found", "message": "Elemento no encontrado" }))?;
                        let label = match &item.payload {
                            ShowItemPayload::Track(t) => format!("Pista \"{}\"", t.title),
                            ShowItemPayload::Lyrics(l) => {
                                let snippet: String = l.body.lines().next().unwrap_or("Letra").chars().take(30).collect();
                                format!("Letra \"{}\"", snippet)
                            }
                            ShowItemPayload::Note(n) => {
                                let snippet: String = n.body.lines().next().unwrap_or("Nota").chars().take(30).collect();
                                format!("Nota \"{}\"", snippet)
                            }
                        };
                        Ok(label)
                    },
                    |_ctx, label| {
                        let delete_uc = self.delete_item.as_ref().ok_or_else(|| {
                            json!({ "error": "unconfigured", "message": "delete_item no configurado" })
                        })?;
                        match delete_uc.execute(&pres_id, &item_id) {
                            Ok(_) => {
                                let action = ToolAction {
                                    tool: "eliminar_elemento".to_string(),
                                    presentation_id: Some(pres_id.clone()),
                                    event_id: None,
                                    summary: format!("Elemento “{}” eliminado de la presentación", label),
                                    status: "success".to_string(),
                                    details: Some(json!({ "item_id": item_id, "status": "deleted" })),
                                };
                                Ok((action, json!({ "item_id": item_id, "status": "deleted" })))
                            }
                            Err(e) => Err(app_error_to_json(&e)),
                        }
                    },
                )
            }

            "eliminar_evento" => {
                let args: EventoIdArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => return validation_err("Parámetros de herramienta inválidos"),
                };
                let evento_id = args.evento_id.trim().to_string();
                if evento_id.is_empty() {
                    return validation_err("Identificador de evento inválido");
                }
                let target = DestructiveTarget::Event {
                    id: evento_id.clone(),
                };
                self.dispatch_destructive(
                    call,
                    signature,
                    context,
                    DestructiveTool::DeleteEvent,
                    target,
                    |_active_ev| {
                        let tix = self.ticketing.as_ref().ok_or_else(|| json!({ "error": "ticketing_unconfigured", "message": "Ticketing no configurado" }))?;
                        let event = tix.get_event.execute(&evento_id).map_err(|e| app_error_to_json(&e))?;
                        Ok(event.nombre)
                    },
                    |_ctx, label| {
                        let tix = self.ticketing.as_ref().ok_or_else(|| json!({ "error": "ticketing_unconfigured", "message": "Ticketing no configurado" }))?;
                        match tix.delete_event.execute(&evento_id) {
                            Ok(_) => {
                                let action = ToolAction {
                                    tool: "eliminar_evento".to_string(),
                                    presentation_id: None,
                                    event_id: Some(evento_id.clone()),
                                    summary: format!("Evento “{}” eliminado correctamente", label),
                                    status: "success".to_string(),
                                    details: Some(json!({ "id": evento_id, "status": "deleted" })),
                                };
                                Ok((action, json!({ "id": evento_id, "status": "deleted" })))
                            }
                            Err(e) => Err(app_error_to_json(&e)),
                        }
                    },
                )
            }

            "finalizar_evento" => {
                let args: EventoIdArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => return validation_err("Parámetros de herramienta inválidos"),
                };
                let evento_id = args.evento_id.trim().to_string();
                if evento_id.is_empty() {
                    return validation_err("Identificador de evento inválido");
                }
                let target = DestructiveTarget::Event {
                    id: evento_id.clone(),
                };
                self.dispatch_destructive(
                    call,
                    signature,
                    context,
                    DestructiveTool::FinalizeEvent,
                    target,
                    |_active_ev| {
                        let tix = self.ticketing.as_ref().ok_or_else(|| json!({ "error": "ticketing_unconfigured", "message": "Ticketing no configurado" }))?;
                        let event = tix.get_event.execute(&evento_id).map_err(|e| app_error_to_json(&e))?;
                        Ok(event.nombre)
                    },
                    |_ctx, label| {
                        let tix = self.ticketing.as_ref().ok_or_else(|| json!({ "error": "ticketing_unconfigured", "message": "Ticketing no configurado" }))?;
                        match tix.finalize_event.execute(&evento_id) {
                            Ok(_) => {
                                let action = ToolAction {
                                    tool: "finalizar_evento".to_string(),
                                    presentation_id: None,
                                    event_id: Some(evento_id.clone()),
                                    summary: format!("Evento “{}” finalizado correctamente", label),
                                    status: "success".to_string(),
                                    details: Some(json!({ "id": evento_id, "estado": "Finalizado" })),
                                };
                                Ok((action, json!({ "id": evento_id, "estado": "Finalizado", "nombre": label })))
                            }
                            Err(e) => Err(app_error_to_json(&e)),
                        }
                    },
                )
            }

            "eliminar_entrada" => {
                let args: EliminarEntradaArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => return validation_err("Parámetros de herramienta inválidos"),
                };
                let entrada_id = args.entrada_id.trim().to_string();
                if entrada_id.is_empty() {
                    return validation_err("Identificador de entrada inválido");
                }
                let target = DestructiveTarget::Ticket {
                    id: entrada_id.clone(),
                };
                self.dispatch_destructive(
                    call,
                    signature,
                    context,
                    DestructiveTool::DeleteTicket,
                    target,
                    |active_ev| {
                        let tix = self.ticketing.as_ref().ok_or_else(|| json!({ "error": "ticketing_unconfigured", "message": "Ticketing no configurado" }))?;
                        let ticket = tix.get_ticket.execute(&entrada_id).map_err(|e| app_error_to_json(&e))?;
                        if let Some(ev) = active_ev {
                            if ticket.evento_id != ev {
                                return Err(json!({ "error": "unauthorized_event_id", "message": "La entrada no pertenece al evento activo" }));
                            }
                        }
                        Ok(format!("Entrada para {} ({})", ticket.nombre_comprador, ticket.tipo_entrada))
                    },
                    |_ctx, label| {
                        let tix = self.ticketing.as_ref().ok_or_else(|| json!({ "error": "ticketing_unconfigured", "message": "Ticketing no configurado" }))?;
                        match tix.delete_ticket.execute(&entrada_id) {
                            Ok(_) => {
                                let action = ToolAction {
                                    tool: "eliminar_entrada".to_string(),
                                    presentation_id: None,
                                    event_id: None,
                                    summary: format!("{} eliminada correctamente", label),
                                    status: "success".to_string(),
                                    details: Some(json!({ "id": entrada_id, "status": "deleted" })),
                                };
                                Ok((action, json!({ "id": entrada_id, "status": "deleted" })))
                            }
                            Err(e) => Err(app_error_to_json(&e)),
                        }
                    },
                )
            }

            "eliminar_gasto" => {
                let args: EliminarGastoArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => return validation_err("Parámetros de herramienta inválidos"),
                };
                let gasto_id = args.gasto_id.trim().to_string();
                if gasto_id.is_empty() {
                    return validation_err("Identificador de gasto inválido");
                }
                let target = DestructiveTarget::Expense {
                    id: gasto_id.clone(),
                };
                self.dispatch_destructive(
                    call,
                    signature,
                    context,
                    DestructiveTool::DeleteExpense,
                    target,
                    |active_ev| {
                        let tix = self.ticketing.as_ref().ok_or_else(|| json!({ "error": "ticketing_unconfigured", "message": "Ticketing no configurado" }))?;
                        let expense = tix.get_expense.execute(&gasto_id).map_err(|e| app_error_to_json(&e))?;
                        if let Some(ev) = active_ev {
                            if expense.evento_id != ev {
                                return Err(json!({ "error": "unauthorized_event_id", "message": "El gasto no pertenece al evento activo" }));
                            }
                        }
                        Ok(format!("Gasto \"{}\"", expense.descripcion))
                    },
                    |_ctx, label| {
                        let tix = self.ticketing.as_ref().ok_or_else(|| json!({ "error": "ticketing_unconfigured", "message": "Ticketing no configurado" }))?;
                        match tix.delete_expense.execute(&gasto_id) {
                            Ok(_) => {
                                let action = ToolAction {
                                    tool: "eliminar_gasto".to_string(),
                                    presentation_id: None,
                                    event_id: None,
                                    summary: format!("{} eliminado correctamente", label),
                                    status: "success".to_string(),
                                    details: Some(json!({ "id": gasto_id, "status": "deleted" })),
                                };
                                Ok((action, json!({ "id": gasto_id, "status": "deleted" })))
                            }
                            Err(e) => Err(app_error_to_json(&e)),
                        }
                    },
                )
            }

            "eliminar_interesado" => {
                let args: EliminarInteresadoArgs = match serde_json::from_str(&call.arguments) {
                    Ok(a) => a,
                    Err(_) => return validation_err("Parámetros de herramienta inválidos"),
                };
                let interesado_id = args.interesado_id.trim().to_string();
                if interesado_id.is_empty() {
                    return validation_err("Identificador de interesado inválido");
                }
                let target = DestructiveTarget::Interest {
                    id: interesado_id.clone(),
                };
                self.dispatch_destructive(
                    call,
                    signature,
                    context,
                    DestructiveTool::DeleteInterest,
                    target,
                    |active_ev| {
                        let tix = self.ticketing.as_ref().ok_or_else(|| json!({ "error": "ticketing_unconfigured", "message": "Ticketing no configurado" }))?;
                        let interest = tix.get_interest.execute(&interesado_id).map_err(|e| app_error_to_json(&e))?;
                        if let Some(ev) = active_ev {
                            if interest.evento_id != ev {
                                return Err(json!({ "error": "unauthorized_event_id", "message": "El interesado no pertenece al evento activo" }));
                            }
                        }
                        Ok(format!("Interesado \"{}\"", interest.nombre))
                    },
                    |_ctx, label| {
                        let tix = self.ticketing.as_ref().ok_or_else(|| json!({ "error": "ticketing_unconfigured", "message": "Ticketing no configurado" }))?;
                        match tix.delete_interest.execute(&interesado_id) {
                            Ok(_) => {
                                let action = ToolAction {
                                    tool: "eliminar_interesado".to_string(),
                                    presentation_id: None,
                                    event_id: None,
                                    summary: format!("{} eliminado correctamente", label),
                                    status: "success".to_string(),
                                    details: Some(json!({ "id": interesado_id, "status": "deleted" })),
                                };
                                Ok((action, json!({ "id": interesado_id, "status": "deleted" })))
                            }
                            Err(e) => Err(app_error_to_json(&e)),
                        }
                    },
                )
            }

            _ => {
                json!({
                    "error": "unknown_tool",
                    "message": "Herramienta no permitida"
                })
            }
        }
    }
    fn dispatch_destructive<FPre, FExec>(
        &self,
        call: &ToolCall,
        signature: String,
        context: &mut TurnToolContext,
        tool: DestructiveTool,
        target: DestructiveTarget,
        preflight: FPre,
        execute: FExec,
    ) -> Value
    where
        FPre: FnOnce(Option<&str>) -> Result<String, Value>,
        FExec: FnOnce(&mut TurnToolContext, &str) -> Result<(ToolAction, Value), Value>,
    {
        let is_authorized = if let Some(ref pending) = context.pending_destructive_action {
            pending.tool == tool
                && pending.target == target
                && match (tool.is_finalize(), context.confirmation_reply) {
                    (false, ConfirmationReply::ExactDelete) => true,
                    (true, ConfirmationReply::ExactFinalize) => true,
                    _ => false,
                }
        } else {
            false
        };

        let active_event = context.active_event_id.clone();

        if is_authorized {
            context.pending_destructive_action = None;
            let label = preflight(active_event.as_deref())
                .unwrap_or_else(|_| target.id_label().to_string());
            match execute(context, &label) {
                Ok((action, res)) => {
                    if tool == DestructiveTool::DeletePresentation {
                        if context.active_presentation_id.as_deref() == Some(target.id_label()) {
                            context.set_active_presentation_id(None);
                        }
                    } else if tool == DestructiveTool::DeleteEvent {
                        if context.active_event_id.as_deref() == Some(target.id_label()) {
                            context.set_active_event_id(None);
                        }
                    }
                    context.record_action(action);
                    context.executed_call_ids.insert(call.id.clone());
                    context.executed_signatures.insert(signature, res.clone());
                    res
                }
                Err(err_val) => err_val,
            }
        } else if context.confirmation_reply == ConfirmationReply::ExactDelete
            || context.confirmation_reply == ConfirmationReply::ExactFinalize
        {
            json!({
                "error": "unauthorized_destructive_call",
                "message": "No existe una autorización pendiente en el servidor para ejecutar esta acción destructiva."
            })
        } else {
            let label = match preflight(active_event.as_deref()) {
                Ok(l) => l,
                Err(err_val) => return err_val,
            };

            let pending = PendingDestructiveAction::new(tool, target.clone(), label.clone());
            if let Some(ref reg) = context.destructive_registry {
                reg.arm(context.conversation_id.clone(), pending.clone());
            } else if let Some(ref reg) = self.destructive_registry {
                reg.arm(context.conversation_id.clone(), pending.clone());
            }
            context.confirmation_requested = Some(pending.clone());

            let res = json!({
                "confirmation_required": true,
                "tool": tool.name(),
                "target_id": target.id_label(),
                "target_label": label,
                "consequence": tool.default_consequence(),
                "required_phrase": tool.required_phrase(),
                "message": format!(
                    "¿Estás segura de que deseas {} \"{}\"? Esta acción {}. Para confirmar, por favor escribe exactamente: {}",
                    if tool.is_finalize() { "finalizar el evento" } else { "eliminar" },
                    label,
                    tool.default_consequence(),
                    tool.required_phrase()
                )
            });
            context.executed_call_ids.insert(call.id.clone());
            context.executed_signatures.insert(signature, res.clone());
            res
        }
    }
}
