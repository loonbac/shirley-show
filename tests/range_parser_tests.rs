use shirley_show::handlers::audio::parse_range::{parse_range, RangeSpec};

#[test]
fn test_parse_range_none_or_empty() {
    assert_eq!(parse_range(None, 1000), RangeSpec::Full);
    assert_eq!(parse_range(Some(""), 1000), RangeSpec::Full);
    assert_eq!(parse_range(Some("   "), 1000), RangeSpec::Full);
}

#[test]
fn test_parse_range_valid_exact() {
    // bytes=0-499
    match parse_range(Some("bytes=0-499"), 1000) {
        RangeSpec::Satisfiable { start, end } => {
            assert_eq!(start, 0);
            assert_eq!(end, 499);
        }
        other => panic!("Expected Satisfiable, got {:?}", other),
    }

    // bytes=500-999
    match parse_range(Some("bytes=500-999"), 1000) {
        RangeSpec::Satisfiable { start, end } => {
            assert_eq!(start, 500);
            assert_eq!(end, 999);
        }
        other => panic!("Expected Satisfiable, got {:?}", other),
    }
}

#[test]
fn test_parse_range_open_ended() {
    // bytes=500-
    match parse_range(Some("bytes=500-"), 1000) {
        RangeSpec::Satisfiable { start, end } => {
            assert_eq!(start, 500);
            assert_eq!(end, 999);
        }
        other => panic!("Expected Satisfiable, got {:?}", other),
    }
}

#[test]
fn test_parse_range_suffix() {
    // bytes=-200 (last 200 bytes -> 800-999)
    match parse_range(Some("bytes=-200"), 1000) {
        RangeSpec::Satisfiable { start, end } => {
            assert_eq!(start, 800);
            assert_eq!(end, 999);
        }
        other => panic!("Expected Satisfiable, got {:?}", other),
    }

    // Suffix larger than total size clamps to entire file
    match parse_range(Some("bytes=-1500"), 1000) {
        RangeSpec::Satisfiable { start, end } => {
            assert_eq!(start, 0);
            assert_eq!(end, 999);
        }
        other => panic!("Expected Satisfiable, got {:?}", other),
    }
}

#[test]
fn test_parse_range_clamp_oversized_end() {
    // bytes=500-2000 on size 1000 clamps end to 999
    match parse_range(Some("bytes=500-2000"), 1000) {
        RangeSpec::Satisfiable { start, end } => {
            assert_eq!(start, 500);
            assert_eq!(end, 999);
        }
        other => panic!("Expected Satisfiable, got {:?}", other),
    }
}

#[test]
fn test_parse_range_unsatisfiable() {
    // start >= total_size
    assert_eq!(parse_range(Some("bytes=1000-1500"), 1000), RangeSpec::Unsatisfiable);
    assert_eq!(parse_range(Some("bytes=1500-"), 1000), RangeSpec::Unsatisfiable);

    // start > end
    assert_eq!(parse_range(Some("bytes=500-400"), 1000), RangeSpec::Unsatisfiable);
}

#[test]
fn test_parse_range_malformed_or_multi_falls_back_to_full() {
    // Malformed unit
    assert_eq!(parse_range(Some("items=0-10"), 1000), RangeSpec::Full);
    // Malformed numbers
    assert_eq!(parse_range(Some("bytes=abc-def"), 1000), RangeSpec::Full);
    assert_eq!(parse_range(Some("bytes=-"), 1000), RangeSpec::Full);
    // Multiple ranges (RFC 9110 allows servers to ignore and return 200 Full)
    assert_eq!(parse_range(Some("bytes=0-100, 200-300"), 1000), RangeSpec::Full);
}
