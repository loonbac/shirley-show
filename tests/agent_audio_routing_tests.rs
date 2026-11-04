//! The attached-file path was unreachable: a user could attach an MP3, the
//! server wrote it to staging, and the agent was never told. These tests pin the
//! routing decisions so the gap cannot reopen.
//!
//! Three independent defects combined, and each one alone was enough to break the
//! feature:
//!
//! 1. The tool filter only looked at `staged_audio_id` (an id from a *previous*
//!    turn), never at `staged_audio` (the file just attached).
//! 2. Even with the right tools, the attached file lives in the context as a
//!    `StagingFileGuard`, not as an id in the registry, and `separate_vocals`
//!    only resolves ids from the registry.
//! 3. `separate_vocals` with `track_id` only accepts ids in
//!    `context.known_track_ids`, which is filled exclusively by
//!    `get_presentation`; without it in the filter the agent had the tool and no
//!    valid id to pass.

use shirley_show::agent::tool_routing::{
    AudioTurnKind, AudioTurnState, audio_turn_kind, audio_turn_tool_names,
};

// T1. An attached file is its own case. It used to be classified as "no audio",
// so the agent received exactly one tool and could not even list presentations.
#[test]
fn test_attached_file_is_classified_as_attached_audio() {
    assert_eq!(
        audio_turn_kind(AudioTurnState {
            has_attached_file: true,
            has_previous_staged_id: false,
            has_confirmed_destination: false,
            mentions_audio: true,
            has_youtube_url: false,
        }),
        AudioTurnKind::AttachedFile,
        "a file attached in this turn must be recognised as audio"
    );
}

// T2. An id from a previous turn keeps its case.
#[test]
fn test_previous_staged_id_is_classified_as_prepared_audio() {
    assert_eq!(
        audio_turn_kind(AudioTurnState {
            has_attached_file: false,
            has_previous_staged_id: true,
            has_confirmed_destination: false,
            mentions_audio: true,
            has_youtube_url: false,
        }),
        AudioTurnKind::PreparedAudio
    );
}

// T3. When both exist, the freshly attached file wins: it is the most recent
// work and the one the user is talking about.
#[test]
fn test_attached_file_wins_over_previous_id() {
    assert_eq!(
        audio_turn_kind(AudioTurnState {
            has_attached_file: true,
            has_previous_staged_id: true,
            has_confirmed_destination: false,
            mentions_audio: true,
            has_youtube_url: false,
        }),
        AudioTurnKind::AttachedFile
    );
}

// T4. No audio at all stays a YouTube turn.
#[test]
fn test_no_audio_is_a_youtube_turn() {
    assert_eq!(
        audio_turn_kind(AudioTurnState {
            has_attached_file: false,
            has_previous_staged_id: false,
            has_confirmed_destination: false,
            mentions_audio: true,
            has_youtube_url: false,
        }),
        AudioTurnKind::Youtube
    );
}

// T5. The attached-file case must expose the whole audio toolset, including
// `get_presentation` (the only source of `known_track_ids`) and
// `separate_vocals` itself. This is the assertion that would have caught the
// defect: before, this case returned only `download_youtube_track`.
#[test]
fn test_attached_audio_exposes_the_tools_needed_to_separate() {
    let names = audio_turn_tool_names(AudioTurnKind::AttachedFile);
    for required in [
        "get_presentation",
        "list_presentations",
        "separate_vocals",
        "add_track_from_staged",
    ] {
        assert!(
            names.contains(&required),
            "attached audio must expose {required}, got {names:?}"
        );
    }
    assert!(
        !names.contains(&"download_youtube_track"),
        "the YouTube downloader is not needed when the file is already here"
    );
}

// T6. Prepared audio keeps the same set, because it is the same situation one
// turn later.
#[test]
fn test_prepared_audio_exposes_the_same_toolset() {
    assert_eq!(
        audio_turn_tool_names(AudioTurnKind::PreparedAudio),
        audio_turn_tool_names(AudioTurnKind::AttachedFile)
    );
}

// T7. The YouTube case is narrow in one specific way: it must not separate.
// Separation needs audio that does not exist until the download finishes, and
// it costs GPU time; keeping it in the same turn would let the model decide two
// paid operations in one step.
//
// It does include the downloader and the means to place the result, because the
// requirement is "send a link and that is it": leaving the audio in staging made
// the user come back for a second request.
#[test]
fn test_youtube_turn_downloads_and_places_but_does_not_separate() {
    let names = audio_turn_tool_names(AudioTurnKind::Youtube);
    assert!(names.contains(&"download_youtube_track"));
    assert!(names.contains(&"add_track_from_staged"));
    assert!(!names.contains(&"separate_vocals"));
}

// T8. The routing helper must not claim separability where the destination is
// unknown: separating without a confirmed presentation produces an orphan file.
// The filter is about tools, not about the prompt, so this is documented here as
// the reason `get_presentation` is mandatory in the set.
#[test]
fn test_attached_audio_includes_the_presentation_reader() {
    let names = audio_turn_tool_names(AudioTurnKind::AttachedFile);
    assert!(
        names.contains(&"get_presentation"),
        "known_track_ids is only populated by get_presentation, so the agent          cannot pass a valid track_id without it"
    );
}

// T9. The attached file is a `StagingFileGuard` held by the turn, not an id in
// the registry, and `separate_vocals` only resolves ids from the registry. So
// even with the right tool exposed, the call would fail with
// `staged_audio_not_found`. The turn must register the attachment before the
// dispatcher runs, which is what this contract states.
//
// The behaviour lives in the handler wiring (async, needs a registry and a file),
// so the assertion here is on the decision function: given an attached handle,
// the turn reports that a registry entry is required.
#[test]
fn test_attached_file_requires_registration_before_separating() {
    use shirley_show::agent::tool_routing::{AudioTurnKind, audio_turn_kind, needs_registry_entry};

    assert!(
        needs_registry_entry(AudioTurnKind::AttachedFile),
        "the attached file is not in the registry yet, so separation would fail \
         with staged_audio_not_found"
    );
    assert!(
        !needs_registry_entry(AudioTurnKind::PreparedAudio),
        "a prepared id is already a registry entry"
    );
    assert!(!needs_registry_entry(AudioTurnKind::Youtube));
    let _ = audio_turn_kind(AudioTurnState {
        has_attached_file: true,
        has_previous_staged_id: false,
        has_confirmed_destination: false,
        mentions_audio: true,
        has_youtube_url: false,
    });
}

// T10. The prompt must tell the model the audio exists. The old prompt for the
// "no prepared audio" branch talked only about YouTube, so the model had no way
// to know a file was attached even if a tool had been present.
#[test]
fn test_attached_audio_branch_mentions_the_attached_file() {
    use shirley_show::agent::tool_routing::{AudioTurnKind, audio_turn_kind};

    let kind = audio_turn_kind(AudioTurnState {
        has_attached_file: true,
        has_previous_staged_id: false,
        has_confirmed_destination: false,
        mentions_audio: true,
        has_youtube_url: false,
    });
    assert_eq!(kind, AudioTurnKind::AttachedFile);
    let prompt = shirley_show::agent::tool_routing::audio_turn_instruction(kind, true);
    assert!(
        prompt.to_lowercase().contains("adjunt"),
        "the instruction must say the audio is attached: {prompt}"
    );
    assert!(
        prompt.contains("separate_vocals"),
        "the instruction must name the tool to call: {prompt}"
    );
    assert!(
        !prompt.to_lowercase().contains("descarga"),
        "an attached file must not be described as something to download: {prompt}"
    );
}

// T11. The case that actually broke: the track is already in the presentation,
// nothing is attached, and nothing was staged. The user says "separate the
// Medicine track". Before, this fell into the YouTube branch and the agent
// answered that it had no separation tool.
//
// A confirmed destination plus an audio request is enough to expose the audio
// toolset: `get_presentation` is what reveals the track ids the agent needs.
#[test]
fn test_existing_track_in_destination_is_its_own_case() {
    use shirley_show::agent::tool_routing::{AudioTurnKind, AudioTurnState, audio_turn_kind};

    let state = AudioTurnState {
        has_attached_file: false,
        has_previous_staged_id: false,
        has_confirmed_destination: true,
        mentions_audio: true,
        has_youtube_url: false,
    };
    assert_eq!(
        audio_turn_kind(state),
        AudioTurnKind::ExistingTrack,
        "a track already in the presentation must not be treated as a YouTube request"
    );

    let names =
        shirley_show::agent::tool_routing::audio_turn_tool_names(AudioTurnKind::ExistingTrack);
    assert!(names.contains(&"get_presentation"));
    assert!(names.contains(&"separate_vocals"));
}

// T12. Without a destination and without audio there is genuinely nothing to
// separate yet, so the narrow YouTube branch stays.
#[test]
fn test_youtube_branch_requires_no_destination_and_no_audio() {
    use shirley_show::agent::tool_routing::{AudioTurnKind, AudioTurnState, audio_turn_kind};

    let state = AudioTurnState {
        has_attached_file: false,
        has_previous_staged_id: false,
        has_confirmed_destination: false,
        mentions_audio: true,
        has_youtube_url: false,
    };
    assert_eq!(audio_turn_kind(state), AudioTurnKind::Youtube);
}

// T13. Every audio case except pure YouTube must expose the same set, because
// each of them ends in the same operation: find the track, separate it, add the
// result. Keeping them equal is what makes the routing auditable.
#[test]
fn test_all_audio_work_cases_share_the_toolset() {
    use shirley_show::agent::tool_routing::{AudioTurnKind, audio_turn_tool_names};

    let audio_set = audio_turn_tool_names(AudioTurnKind::AttachedFile);
    for kind in [AudioTurnKind::PreparedAudio, AudioTurnKind::ExistingTrack] {
        assert_eq!(
            audio_turn_tool_names(kind),
            audio_set,
            "{kind:?} must expose the same tools as an attached file"
        );
    }
}

// T14. REGRESSION. Adding the ExistingTrack case broke the YouTube path: a message
// with a link and a confirmed destination satisfied "destination + mentions audio",
// so the existing-track branch won and the downloader was not exposed at all. The
// agent then answered that the presentation was empty and it could not download.
//
// A link in the message is unambiguous: it must reach the downloader.
#[test]
fn test_youtube_link_beats_existing_destination() {
    use shirley_show::agent::tool_routing::{AudioTurnKind, AudioTurnState, audio_turn_kind};

    let state = AudioTurnState {
        has_attached_file: false,
        has_previous_staged_id: false,
        has_confirmed_destination: true,
        mentions_audio: true,
        has_youtube_url: true,
    };
    assert_eq!(
        audio_turn_kind(state),
        AudioTurnKind::Youtube,
        "a YouTube link must not be routed to the existing-track branch"
    );
}

// T15. The user's requirement, stated plainly: send a link and the agent downloads
// it in the best quality. "And that's it" means the track must end up in the
// presentation without a second request, so the YouTube case must expose the
// downloader AND the means to add what it produced.
#[test]
fn test_youtube_turn_can_download_and_add_in_one_step() {
    use shirley_show::agent::tool_routing::{AudioTurnKind, audio_turn_tool_names};

    let names = audio_turn_tool_names(AudioTurnKind::Youtube);
    assert!(
        names.contains(&"download_youtube_track"),
        "the downloader is the whole point: {names:?}"
    );
    assert!(
        names.contains(&"add_track_from_staged"),
        "without it the audio stays in staging and the user needs a second request: {names:?}"
    );
    assert!(
        names.contains(&"get_presentation"),
        "the agent needs the track list before adding, to avoid duplicating: {names:?}"
    );
    assert!(
        !names.contains(&"separate_vocals"),
        "separation needs audio that does not exist yet: that is a second turn"
    );
}

// T16. A link with no destination still works: the agent asks where to put it.
#[test]
fn test_youtube_link_without_destination_still_exposes_the_downloader() {
    use shirley_show::agent::tool_routing::{AudioTurnKind, AudioTurnState, audio_turn_kind};

    let state = AudioTurnState {
        has_attached_file: false,
        has_previous_staged_id: false,
        has_confirmed_destination: false,
        mentions_audio: false,
        has_youtube_url: true,
    };
    assert_eq!(audio_turn_kind(state), AudioTurnKind::Youtube);
}

// T17. El prompt de AttachedFile pedía un `track_id` que no puede existir: un
// audio recién adjunto no es una pista. El código ya registra el adjunto y le da
// un `staged_audio_id`, pero el prompt le decía al modelo que pidiera
// `get_presentation` para obtener el `track_id` de la pista. El agente obedecía,
// veía `tracks_count: 0` en una presentación vacía y se detenía con
// `actions_taken: []`.
//
// Reportado en producción por la sesión de despliegue, con la cita textual.
#[test]
fn test_attached_file_prompt_uses_the_staged_id_not_a_track_id() {
    use shirley_show::agent::tool_routing::{AudioTurnKind, audio_turn_instruction};

    let prompt = audio_turn_instruction(AudioTurnKind::AttachedFile, true);
    assert!(
        prompt.contains("staged_audio_id"),
        "el prompt debe nombrar el id que sí existe: {prompt}"
    );
    assert!(
        !prompt.contains("track_id de la pista"),
        "un audio adjunto no es una pista, así que no puede tener track_id: {prompt}"
    );
    assert!(
        !prompt.contains("para obtener el track_id"),
        "pedir get_presentation para obtener un track_id inexistente es lo que \
         rompía el turno: {prompt}"
    );
}

// T18. Un audio preparado sí es un id de staging, igual que el adjunto. El
// prompt no debe hablarle de pistas tampoco.
#[test]
fn test_prepared_audio_prompt_uses_the_staged_id() {
    use shirley_show::agent::tool_routing::{AudioTurnKind, audio_turn_instruction};

    let prompt = audio_turn_instruction(AudioTurnKind::PreparedAudio, true);
    assert!(prompt.contains("staged_audio_id"), "{prompt}");
    assert!(!prompt.contains("track_id de la pista"), "{prompt}");
}

// T19. El caso de pista existente sí debe hablar de track_id: ahí es el único id
// que hay. Es la razón por la que los prompts no pueden compartirse.
#[test]
fn test_existing_track_prompt_uses_track_id() {
    use shirley_show::agent::tool_routing::{AudioTurnKind, audio_turn_instruction};

    let prompt = audio_turn_instruction(AudioTurnKind::ExistingTrack, true);
    assert!(
        prompt.contains("track_id"),
        "cuando la pista ya está en la presentación, ese es el id correcto: {prompt}"
    );
    assert!(
        !prompt.contains("descargues"),
        "no hay nada que descargar: {prompt}"
    );
}
