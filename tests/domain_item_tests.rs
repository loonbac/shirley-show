use shirley_show::domain::audio_format::AudioFormat;
use shirley_show::domain::move_direction::MoveDirection;
use shirley_show::domain::show_item::{LyricsPayload, NotePayload, ShowItem, ShowItemPayload, TrackPayload};
use shirley_show::domain::show_item_kind::ShowItemKind;

#[test]
fn test_audio_format_properties() {
    assert_eq!(AudioFormat::Mp3.extension(), "mp3");
    assert_eq!(AudioFormat::Mp3.media_type(), "audio/mpeg");

    assert_eq!(AudioFormat::Wav.extension(), "wav");
    assert_eq!(AudioFormat::Wav.media_type(), "audio/wav");

    assert_eq!(AudioFormat::M4a.extension(), "m4a");
    assert_eq!(AudioFormat::M4a.media_type(), "audio/mp4");

    assert_eq!(AudioFormat::from_extension("mp3"), Some(AudioFormat::Mp3));
    assert_eq!(AudioFormat::from_extension(".WAV"), Some(AudioFormat::Wav));
    assert_eq!(AudioFormat::from_extension("m4a"), Some(AudioFormat::M4a));
    assert_eq!(AudioFormat::from_extension("flac"), None);
    assert_eq!(AudioFormat::from_extension("ogg"), None);
}

#[test]
fn test_move_direction_parsing() {
    assert_eq!("up".parse::<MoveDirection>().unwrap(), MoveDirection::Up);
    assert_eq!("down".parse::<MoveDirection>().unwrap(), MoveDirection::Down);
    assert_eq!("UP".parse::<MoveDirection>().unwrap(), MoveDirection::Up);
    assert_eq!("Down".parse::<MoveDirection>().unwrap(), MoveDirection::Down);
    assert!("left".parse::<MoveDirection>().is_err());
    assert!("".parse::<MoveDirection>().is_err());
}

#[test]
fn test_show_item_kind_as_str() {
    assert_eq!(ShowItemKind::Track.as_str(), "track");
    assert_eq!(ShowItemKind::Lyrics.as_str(), "lyrics");
    assert_eq!(ShowItemKind::Note.as_str(), "note");

    assert_eq!("track".parse::<ShowItemKind>().unwrap(), ShowItemKind::Track);
    assert_eq!("lyrics".parse::<ShowItemKind>().unwrap(), ShowItemKind::Lyrics);
    assert_eq!("note".parse::<ShowItemKind>().unwrap(), ShowItemKind::Note);
    assert!("invalid".parse::<ShowItemKind>().is_err());
}

#[test]
fn test_lyrics_payload_validation() {
    // Valid multiline lyrics
    let valid = LyricsPayload::new("  Verse 1\nLine 2\n\nChorus  ").unwrap();
    assert_eq!(valid.body, "Verse 1\nLine 2\n\nChorus");

    // Multiline lyrics with Unicode / Spanish diacritics
    let unicode = LyricsPayload::new("  Canción de cuna\nCoro: ¡Oh Shirley!  ").unwrap();
    assert_eq!(unicode.body, "Canción de cuna\nCoro: ¡Oh Shirley!");

    // Empty or whitespace only
    assert!(LyricsPayload::new("").is_err());
    assert!(LyricsPayload::new("   \n\t  ").is_err());
}

#[test]
fn test_note_payload_validation() {
    // Valid note
    let valid = NotePayload::new("  Guitar solo in E minor  ").unwrap();
    assert_eq!(valid.body, "Guitar solo in E minor");

    // Stage cue with punctuation
    let cue = NotePayload::new("  [Pausa dramática de 3s]  ").unwrap();
    assert_eq!(cue.body, "[Pausa dramática de 3s]");

    // Empty or whitespace only
    assert!(NotePayload::new("").is_err());
    assert!(NotePayload::new("   \t  ").is_err());
}

#[test]
fn test_track_payload_validation() {
    // Valid track
    let valid = TrackPayload::new(
        "  Acoustic Intro  ",
        "audio-123.mp3",
        "audio/mpeg",
        1024,
    ).unwrap();
    assert_eq!(valid.title, "Acoustic Intro");
    assert_eq!(valid.audio_key, "audio-123.mp3");
    assert_eq!(valid.media_type, "audio/mpeg");
    assert_eq!(valid.byte_size, 1024);

    // Empty title
    assert!(TrackPayload::new("", "audio-123.mp3", "audio/mpeg", 1024).is_err());
    assert!(TrackPayload::new("   ", "audio-123.mp3", "audio/mpeg", 1024).is_err());

    // Empty audio_key
    assert!(TrackPayload::new("Title", "", "audio/mpeg", 1024).is_err());
    assert!(TrackPayload::new("Title", "   ", "audio/mpeg", 1024).is_err());

    // Empty media_type
    assert!(TrackPayload::new("Title", "audio-123.mp3", "", 1024).is_err());

    // File exceeding 100 MiB limit (104,857,600 bytes)
    assert!(TrackPayload::new("Title", "audio-123.mp3", "audio/mpeg", 104_857_601).is_err());
    // Exactly 100 MiB is allowed
    assert!(TrackPayload::new("Title", "audio-123.mp3", "audio/mpeg", 104_857_600).is_ok());
    // Negative size
    assert!(TrackPayload::new("Title", "audio-123.mp3", "audio/mpeg", -1).is_err());
}

#[test]
fn test_show_item_constructors_and_invariants() {
    let pres_id = "00000000-0000-0000-0000-000000000001";

    let lyrics_payload = ShowItemPayload::Lyrics(LyricsPayload::new("Verse 1").unwrap());
    let item1 = ShowItem::new(pres_id, 0, lyrics_payload);
    assert_eq!(item1.presentation_id, pres_id);
    assert_eq!(item1.position, 0);
    assert_eq!(item1.kind(), ShowItemKind::Lyrics);

    let note_payload = ShowItemPayload::Note(NotePayload::new("Key of G").unwrap());
    let item2 = ShowItem::new(pres_id, 1, note_payload);
    assert_eq!(item2.position, 1);
    assert_eq!(item2.kind(), ShowItemKind::Note);

    let track_payload = ShowItemPayload::Track(
        TrackPayload::new("Solo", "track-1.mp3", "audio/mpeg", 5000).unwrap()
    );
    let item3 = ShowItem::new(pres_id, 2, track_payload);
    assert_eq!(item3.position, 2);
    assert_eq!(item3.kind(), ShowItemKind::Track);

    // Serialization roundtrip
    let serialized = serde_json::to_string(&item3).unwrap();
    let deserialized: ShowItem = serde_json::from_str(&serialized).unwrap();
    assert_eq!(item3, deserialized);
}

#[test]
fn test_show_item_from_db_mappings_and_corruption() {
    // Valid DB row for track
    let track_item = ShowItem::from_db(
        "id-1".into(),
        "p-1".into(),
        0,
        "track".into(),
        Some("My Track".into()),
        None,
        Some("key.mp3".into()),
        Some("audio/mpeg".into()),
        Some(2048),
        "2026-03-30T00:00:00Z".into(),
    ).unwrap();
    assert_eq!(track_item.kind(), ShowItemKind::Track);

    // Corrupted DB row: track without audio_key
    let err_track = ShowItem::from_db(
        "id-2".into(),
        "p-1".into(),
        0,
        "track".into(),
        Some("My Track".into()),
        None,
        None,
        Some("audio/mpeg".into()),
        Some(2048),
        "2026-03-30T00:00:00Z".into(),
    );
    assert!(err_track.is_err());

    // Corrupted DB row: lyrics without body
    let err_lyrics = ShowItem::from_db(
        "id-3".into(),
        "p-1".into(),
        1,
        "lyrics".into(),
        None,
        None,
        None,
        None,
        None,
        "2026-03-30T00:00:00Z".into(),
    );
    assert!(err_lyrics.is_err());
}
