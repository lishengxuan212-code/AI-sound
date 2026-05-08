use serde_json::Value;

pub fn redact_text(input: &str) -> String {
    let mut output = input.to_string();
    for marker in ["Authorization", "Bearer", "DASHSCOPE_API_KEY", "GAME_TTS_API_KEY"] {
        if output.to_lowercase().contains(&marker.to_lowercase()) {
            output = "[REDACTED]".to_string();
        }
    }
    output
}

pub fn redact_json(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, item)| {
                    let lower = key.to_lowercase();
                    if lower.contains("key")
                        || lower.contains("token")
                        || lower.contains("secret")
                        || lower.contains("password")
                        || lower == "authorization"
                    {
                        (key, Value::String("[REDACTED]".to_string()))
                    } else {
                        (key, redact_json(item))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(redact_json).collect()),
        Value::String(text) => Value::String(redact_text(&text)),
        other => other,
    }
}
