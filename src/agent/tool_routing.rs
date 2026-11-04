//! Which tools the agent is allowed to see on an audio turn.
//!
//! This lives outside `agent_chat` because the rule has four cases, a dependency
//! between them, and a history: the original version lived inline in a `match` on
//! a single boolean and collapsed every audio situation into two. The result was
//! that a user could attach an MP3 or have a track in her presentation and the
//! agent would be handed exactly one tool — the YouTube downloader — and answer,
//! honestly, that it had no way to separate vocals.

/// The state an audio turn is decided from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioTurnState {
    /// A file was attached in this very request.
    pub has_attached_file: bool,
    /// An id carried over from a previous turn's staging.
    pub has_previous_staged_id: bool,
    /// A presentation is already selected, so its tracks are reachable.
    pub has_confirmed_destination: bool,
    /// The message talks about audio at all.
    pub mentions_audio: bool,
}

/// What kind of audio work this turn is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioTurnKind {
    /// A file was attached now.
    ///
    /// Held by the server as a `StagingFileGuard`, not yet a registry entry, so it
    /// must be registered before separating.
    AttachedFile,
    /// An id left over from a previous turn.
    PreparedAudio,
    /// Nothing attached or staged, but the presentation is known and may already
    /// contain the track the user names. This is the case that broke: it used to
    /// fall into the YouTube branch, so the agent could not see the presentation
    /// at all and answered that separation did not exist.
    ExistingTrack,
    /// No audio anywhere: the only useful action is fetching one.
    Youtube,
}

/// Decides the case.
///
/// Order matters and is the whole point:
/// 1. an attached file is the most recent work, so it wins;
/// 2. then a staged id from an earlier turn;
/// 3. then a known destination, whose tracks are reachable through
///    `get_presentation`;
/// 4. otherwise a YouTube download.
pub fn audio_turn_kind(state: AudioTurnState) -> AudioTurnKind {
    if state.has_attached_file {
        AudioTurnKind::AttachedFile
    } else if state.has_previous_staged_id {
        AudioTurnKind::PreparedAudio
    } else if state.has_confirmed_destination && state.mentions_audio {
        AudioTurnKind::ExistingTrack
    } else {
        AudioTurnKind::Youtube
    }
}

/// The tool names exposed for a case.
///
/// The three audio-work cases share one set on purpose: each ends in the same
/// sequence — find the track, separate it, add the result — and keeping them
/// equal is what makes the routing auditable.
///
/// `get_presentation` is mandatory even though separation does not read it
/// directly. `separate_vocals` with a `track_id` only accepts ids present in
/// `known_track_ids`, and that set is populated exclusively by
/// `get_presentation`. Exposing `separate_vocals` without it gives the agent a
/// tool it can never call with a valid id.
///
/// The YouTube case stays narrow deliberately: one turn, one action. Letting it
/// also separate would mean two paid operations decided by the model in a single
/// step, and a failed download would already have burned a GPU session.
pub fn audio_turn_tool_names(kind: AudioTurnKind) -> Vec<&'static str> {
    match kind {
        AudioTurnKind::AttachedFile
        | AudioTurnKind::PreparedAudio
        | AudioTurnKind::ExistingTrack => vec![
            "list_presentations",
            "get_presentation",
            "add_track_from_staged",
            "separate_vocals",
        ],
        AudioTurnKind::Youtube => vec!["download_youtube_track"],
    }
}

/// Whether the dispatcher must create a registry entry before separating.
///
/// `separate_vocals` resolves its input as `staged_audio_id -> track_id ->
/// youtube_url`, and every id it accepts must exist in the registry. A file
/// attached this turn is a `StagingFileGuard`, so it must be registered first or
/// the call fails with `staged_audio_not_found` for audio sitting on disk.
pub fn needs_registry_entry(kind: AudioTurnKind) -> bool {
    matches!(kind, AudioTurnKind::AttachedFile)
}

/// The instruction handed to the model for an audio turn.
///
/// This is part of the routing contract, not decoration. The previous text for
/// the "no prepared audio" branch described only a YouTube download, so even
/// with a tool available the model had no way to know a track already existed and
/// answered that it could not find one.
pub fn audio_turn_instruction(kind: AudioTurnKind, has_destination: bool) -> String {
    match kind {
        AudioTurnKind::AttachedFile | AudioTurnKind::PreparedAudio => {
            let source = if matches!(kind, AudioTurnKind::AttachedFile) {
                "El audio esta adjunto en esta solicitud y ya fue guardado: no lo descargues ni lo pidas."
            } else {
                "El audio ya esta preparado y disponible temporalmente: no lo descargues ni lo pidas."
            };
            format!(
                "{source} Destino confirmado: {has_destination}. Si no esta confirmado, llama a \
                 list_presentations, identifica por el nombre escrito por la usuaria una unica \
                 presentacion y llama a get_presentation con su ID. Despues llama a \
                 get_presentation para obtener el track_id de la pista y llama a separate_vocals \
                 con ese track_id. Para quitar solo la voz principal usa mode=chorus; para quitar \
                 todas las voces usa mode=instrumental. Finalmente agrega el resultado con \
                 add_track_from_staged."
            )
        }
        AudioTurnKind::ExistingTrack => format!(
            "La presentacion ya tiene pistas. Destino confirmado: {has_destination}. Llama a \
             get_presentation con el ID de la presentacion para obtener el track_id de la pista \
             que la usuaria nombra, y despues llama a separate_vocals con ese track_id. Para \
             quitar solo la voz principal usa mode=chorus; para quitar todas las voces usa \
             mode=instrumental. Finalmente agrega el resultado con add_track_from_staged. No \
             pidas enlaces de YouTube: la pista ya esta en la presentacion."
        ),
        AudioTurnKind::Youtube => format!(
            "Interpreta la solicitud completa de audio. Para un enlace de YouTube llama primero \
             a download_youtube_track. Destino confirmado: {has_destination}. No inventes IDs ni \
             afirmes acciones sin herramientas."
        ),
    }
}
