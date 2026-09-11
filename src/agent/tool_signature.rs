use serde_json::Value;

pub fn canonical_tool_signature(tool_name: &str, raw_args: &str) -> String {
    let normalized_json = match serde_json::from_str::<Value>(raw_args) {
        Ok(val) => canonicalize_value(&val),
        Err(_) => raw_args.trim().to_string(),
    };
    format!("{}:{}", tool_name, normalized_json)
}

fn canonicalize_value(val: &Value) -> String {
    match val {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by_key(|(k, _)| *k);
            let parts: Vec<String> = entries
                .into_iter()
                .map(|(k, v)| format!("\"{}\":{}", k, canonicalize_value(v)))
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(canonicalize_value).collect();
            format!("[{}]", parts.join(","))
        }
        other => other.to_string(),
    }
}
