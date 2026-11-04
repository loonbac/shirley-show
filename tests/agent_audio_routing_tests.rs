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
            mentions_audio: true
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
            mentions_audio: true
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
            mentions_audio: true
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
            mentions_audio: true
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

// T7. The YouTube case stays deliberately narrow: it must not be able to run a
// download plus a separation in one turn, which would be two paid operations
// decided by the model.
#[test]
fn test_youtube_turn_exposes_only_the_downloader() {
    assert_eq!(
        audio_turn_tool_names(AudioTurnKind::Youtube),
        vec!["download_youtube_track"]
    );
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
