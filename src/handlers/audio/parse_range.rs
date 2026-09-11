#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeSpec {
    Full,
    Satisfiable { start: u64, end: u64 },
    Unsatisfiable,
}

pub fn parse_range(range_header: Option<&str>, total_size: u64) -> RangeSpec {
    let header = match range_header {
        Some(h) if !h.trim().is_empty() => h.trim(),
        _ => return RangeSpec::Full,
    };

    if !header.starts_with("bytes=") {
        return RangeSpec::Full;
    }

    let range_val = &header["bytes=".len()..];
    // If multiple ranges separated by comma, ignore and return Full per RFC 9110
    if range_val.contains(',') {
        return RangeSpec::Full;
    }

    let parts: Vec<&str> = range_val.split('-').collect();
    if parts.len() != 2 {
        return RangeSpec::Full;
    }

    let start_str = parts[0].trim();
    let end_str = parts[1].trim();

    if start_str.is_empty() && end_str.is_empty() {
        return RangeSpec::Full;
    }

    if start_str.is_empty() {
        // Suffix range: bytes=-N
        let suffix: u64 = match end_str.parse() {
            Ok(n) if n > 0 => n,
            _ => return RangeSpec::Full,
        };
        if suffix >= total_size {
            return RangeSpec::Satisfiable {
                start: 0,
                end: total_size.saturating_sub(1),
            };
        }
        let start = total_size - suffix;
        return RangeSpec::Satisfiable {
            start,
            end: total_size - 1,
        };
    }

    let start: u64 = match start_str.parse() {
        Ok(n) => n,
        _ => return RangeSpec::Full,
    };

    if start >= total_size {
        return RangeSpec::Unsatisfiable;
    }

    if end_str.is_empty() {
        // Open-ended range: bytes=N-
        return RangeSpec::Satisfiable {
            start,
            end: total_size - 1,
        };
    }

    let end: u64 = match end_str.parse() {
        Ok(n) => n,
        _ => return RangeSpec::Full,
    };

    if start > end {
        return RangeSpec::Unsatisfiable;
    }

    let clamped_end = if end >= total_size {
        total_size - 1
    } else {
        end
    };

    RangeSpec::Satisfiable {
        start,
        end: clamped_end,
    }
}
