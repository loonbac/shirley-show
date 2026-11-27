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
    /// The message carries a YouTube link.
    ///
    /// This is the strongest signal there is: it names the exact audio source.
    /// It used to lose against "destination confirmed + mentions audio", so a
    /// link sent to a presentation that already had tracks never reached the
    /// downloader and the agent answered that it could not download.
    pub has_youtube_url: bool,
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
    } else if state.has_youtube_url {
        // A link names its own source, so it beats "there is a destination with
        // tracks": the user is not asking to work on what is there, she is asking
        // to bring something new in.
        AudioTurnKind::Youtube
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
        // The user's requirement, in her words: "le envío un link y ya". "Y ya"
        // means the track must land in the presentation, not sit in staging
        // waiting for a second request. The downloader alone was not enough: it
        // produced a staged id and nothing consumed it.
        //
        // `separate_vocals` stays out on purpose: there is no audio to separate
        // yet, and separating needs a track id that only exists after adding.
        AudioTurnKind::Youtube => vec![
            "download_youtube_track",
            "get_presentation",
            "add_track_from_staged",
        ],
    }
}

/// Tools offered on an audio turn, taking the destination into account.
///
/// `audio_turn_tool_names` describes each source kind on its own and is kept
/// unchanged for callers that only know the kind. A turn that starts from the
/// main screen has no `presentation_id`: it needs the catalogue readers to find
/// both the song (in *any* repertoire) and the destination the user names, and
/// it needs the separation tool even when the source is a YouTube link. Once a
/// destination exists, the established sets are returned untouched.
pub fn audio_turn_tool_names_for_turn(
    kind: AudioTurnKind,
    has_destination: bool,
) -> Vec<&'static str> {
    let mut names = audio_turn_tool_names(kind);
    if has_destination {
        return names;
    }
    for tool in [
        "list_presentations",
        "get_presentation",
        "add_track_from_staged",
        "separate_vocals",
    ] {
        if !names.contains(&tool) {
            names.push(tool);
        }
    }
    if matches!(kind, AudioTurnKind::Youtube) && !names.contains(&"download_youtube_track") {
        names.insert(0, "download_youtube_track");
    }
    names
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
    let instruction = match kind {
        AudioTurnKind::AttachedFile | AudioTurnKind::PreparedAudio => {
            let source = if matches!(kind, AudioTurnKind::AttachedFile) {
                "El audio esta adjunto en esta solicitud y ya fue guardado: no lo descargues ni lo pidas."
            } else {
                "El audio ya esta preparado y disponible temporalmente: no lo descargues ni lo pidas."
            };
            // El id que existe aquí es de staging, no de pista: un audio recién
            // adjunto no es una pista de la presentación. La versión anterior
            // pedía `get_presentation` «para obtener el track_id de la pista»
            // —un id que no puede existir—, así que el agente llamaba, veía
            // `tracks_count: 0` y se detenía sin separar.
            format!(
                "{source} Destino confirmado: {has_destination}. Si no esta confirmado, llama a \
                 list_presentations, identifica por el nombre escrito por la usuaria una unica \
                 presentacion y llama a get_presentation con su ID. Para separar, llama a \
                 separate_vocals SIN pasar track_id: el sistema ya conoce el audio adjunto y usa \
                 su staged_audio_id automaticamente. No busques un track_id de pista, porque un \
                 audio recien adjunto todavia no es una pista de la presentacion. Para quitar \
                 solo la voz principal usa mode=chorus; para quitar todas las voces usa \
                 mode=instrumental. Finalmente agrega el resultado con add_track_from_staged."
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
            "La solicitud trae un enlace de YouTube. Llama a download_youtube_track con ese \
             enlace: descarga el audio en la mejor calidad disponible. Destino confirmado: \
             {has_destination}. Si el destino no esta confirmado, llama a list_presentations, \
             identifica por el nombre escrito por la usuaria una unica presentacion y llama a \
             get_presentation con su ID. Despues llama a add_track_from_staged con el \
             staged_audio_id que devolvio la descarga y el titulo del audio, para que la pista \
             quede en la presentacion en este mismo turno: no pidas una segunda confirmacion. \
             No inventes IDs ni afirmes acciones sin herramientas."
        ),
    };

    if has_destination {
        instruction
    } else {
        let instruction = if matches!(kind, AudioTurnKind::Youtube) {
            format!("{YOUTUBE_DESTINATION_FIRST_INSTRUCTION} {instruction}")
        } else {
            instruction
        };
        format!("{instruction} {}", NO_DESTINATION_AUDIO_INSTRUCTION)
    }
}

/// Con un enlace y sin repertorio abierto, fijar el destino ANTES de descargar
/// evita que el turno se detenga a preguntar justo después de la descarga.
const YOUTUBE_DESTINATION_FIRST_INSTRUCTION: &str = "Si la usuaria nombra el destino, \
     resuelvelo primero con list_presentations y get_presentation, antes de descargar; \
     despues descarga y agrega la pista en el mismo turno.";

/// Regla adicional para el modo catálogo (sin repertorio abierto). Se agrega
/// al prompt de cada caso; no sustituye las instrucciones de la fuente, porque
/// un archivo adjunto no tiene `track_id` y un enlace todavía no tiene audio.
const NO_DESTINATION_AUDIO_INSTRUCTION: &str = "Trabajas desde la pantalla principal, sin \
     presentacion activa. No elijas un destino al azar: usa list_presentations y resuelve por \
     el nombre escrito por la usuaria. Si coincide con una sola presentacion, usa \
     get_presentation con su ID para fijarla; si no aparece o hay varias, pregunta y muestra \
     las presentaciones existentes antes de separar. La fuente puede estar en CUALQUIER \
     repertorio: si la usuaria nombra una cancion y no hay archivo adjunto ni audio \
     preparado, busca esa cancion con list_presentations y get_presentation en las \
     presentaciones y usa el track_id devuelto; no hace falta abrir primero ese repertorio en \
     la interfaz. El destino puede ser distinto del repertorio de origen. Guarda siempre el \
     resultado en la presentacion de destino con add_track_from_staged.";
