use shirley_show::domain::youtube_url::{YouTubeUrl, YouTubeUrlError};

#[test]
fn test_valid_youtube_watch_urls() {
    let urls = [
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
        "http://www.youtube.com/watch?v=dQw4w9WgXcQ",
        "https://youtube.com/watch?v=dQw4w9WgXcQ",
        "https://m.youtube.com/watch?v=dQw4w9WgXcQ",
    ];

    for raw in urls {
        let parsed = YouTubeUrl::parse(raw).expect("should parse valid watch URL");
        assert_eq!(parsed.video_id(), "dQw4w9WgXcQ");
        assert_eq!(
            parsed.canonical_url(),
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        );
    }
}

#[test]
fn test_valid_youtu_be_urls() {
    let parsed = YouTubeUrl::parse("https://youtu.be/dQw4w9WgXcQ")
        .expect("should parse valid youtu.be URL");
    assert_eq!(parsed.video_id(), "dQw4w9WgXcQ");
    assert_eq!(
        parsed.canonical_url(),
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
    );

    let parsed_http = YouTubeUrl::parse("http://youtu.be/dQw4w9WgXcQ")
        .expect("should parse http youtu.be URL");
    assert_eq!(parsed_http.video_id(), "dQw4w9WgXcQ");
    assert_eq!(
        parsed_http.canonical_url(),
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
    );
}

#[test]
fn test_valid_youtube_shorts_urls() {
    let urls = [
        "https://www.youtube.com/shorts/dQw4w9WgXcQ",
        "https://youtube.com/shorts/dQw4w9WgXcQ",
        "http://www.youtube.com/shorts/dQw4w9WgXcQ",
    ];

    for raw in urls {
        let parsed = YouTubeUrl::parse(raw).expect("should parse valid shorts URL");
        assert_eq!(parsed.video_id(), "dQw4w9WgXcQ");
        assert_eq!(
            parsed.canonical_url(),
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        );
    }
}

#[test]
fn test_normalization_ignores_query_params_and_fragments() {
    let urls = [
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PLrEnWoR732-DNX90-Y_wB9B",
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=42s&index=3&feature=share#timestamp",
        "https://youtu.be/dQw4w9WgXcQ?t=10&feature=shared",
        "https://www.youtube.com/shorts/dQw4w9WgXcQ?feature=share",
    ];

    for raw in urls {
        let parsed = YouTubeUrl::parse(raw).expect("should parse and normalize URL");
        assert_eq!(parsed.video_id(), "dQw4w9WgXcQ");
        assert_eq!(
            parsed.canonical_url(),
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        );
    }
}

#[test]
fn test_rejects_external_hosts() {
    let invalid_hosts = [
        "https://vimeo.com/12345678901",
        "https://soundcloud.com/artist/track-name",
        "https://notyoutube.com/watch?v=dQw4w9WgXcQ",
        "https://attacker.com/youtube.com/watch?v=dQw4w9WgXcQ",
        "https://localhost/watch?v=dQw4w9WgXcQ",
        "https://127.0.0.1/watch?v=dQw4w9WgXcQ",
        "https://evil-youtube.com/watch?v=dQw4w9WgXcQ",
    ];

    for raw in invalid_hosts {
        let err = YouTubeUrl::parse(raw).expect_err("should reject external host");
        assert!(
            matches!(err, YouTubeUrlError::UnsupportedHost { .. }),
            "expected UnsupportedHost for {raw}, got {err:?}"
        );
        assert_eq!(err.code(), "unsupported_url");
    }
}

#[test]
fn test_rejects_invalid_video_ids() {
    let invalid_ids = [
        "https://www.youtube.com/watch?v=short",
        "https://www.youtube.com/watch?v=toolongvideoid123",
        "https://www.youtube.com/watch?v=dQw4w9WgXc!",
        "https://www.youtube.com/watch?v=dQw4w9WgXc ",
        "https://www.youtube.com/watch",
        "https://youtu.be/",
        "https://www.youtube.com/shorts/",
    ];

    for raw in invalid_ids {
        let err = YouTubeUrl::parse(raw).expect_err("should reject invalid video ID");
        assert!(
            matches!(err, YouTubeUrlError::InvalidVideoId { .. }),
            "expected InvalidVideoId for {raw}, got {err:?}"
        );
        assert_eq!(err.code(), "invalid_video_id");
    }
}

#[test]
fn test_rejects_unsupported_youtube_paths() {
    let unsupported_paths = [
        "https://www.youtube.com/feed/trending",
        "https://www.youtube.com/channel/UC1234567890",
        "https://www.youtube.com/user/rickastley",
        "https://www.youtube.com/live/dQw4w9WgXcQ",
        "https://www.youtube.com/embed/dQw4w9WgXcQ",
    ];

    for raw in unsupported_paths {
        let err = YouTubeUrl::parse(raw).expect_err("should reject unsupported path");
        assert_eq!(err.code(), "unsupported_url");
    }
}

#[test]
fn test_rejects_shell_metacharacters() {
    let injection_attempts = [
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ;rm -rf /",
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ|whoami",
        "https://www.youtube.com/watch?v=$(whoami)",
        "https://www.youtube.com/watch?v=`whoami`",
        "https://www.youtube.com/watch?v=\"dQw4w9WgXcQ\"",
        "https://www.youtube.com/watch?v='dQw4w9WgXcQ'",
        "https://www.youtube.com/watch?v=dQw4w9WgXc\\Q",
    ];

    for raw in injection_attempts {
        let err = YouTubeUrl::parse(raw).expect_err("should reject shell metacharacters");
        assert!(
            matches!(err, YouTubeUrlError::InvalidCharacters { .. }),
            "expected InvalidCharacters for {raw}, got {err:?}"
        );
        assert_eq!(err.code(), "invalid_characters");
    }
}

#[test]
fn test_rejects_invalid_protocols_credentials_and_ports() {
    let invalid_urls = [
        "ftp://www.youtube.com/watch?v=dQw4w9WgXcQ",
        "javascript:alert(1)",
        "https://admin:secret@www.youtube.com/watch?v=dQw4w9WgXcQ",
        "https://www.youtube.com:8080/watch?v=dQw4w9WgXcQ",
        "",
        "not a url",
    ];

    for raw in invalid_urls {
        assert!(YouTubeUrl::parse(raw).is_err(), "should reject: {raw}");
    }
}
