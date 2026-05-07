use serde_json::{Number, Value};

#[derive(Debug, Clone, Default)]
pub struct MiniMaxProxyAdapter;

impl MiniMaxProxyAdapter {
    pub fn prepare_chat_request(&mut self, chat_request: &mut Value) {
        let Some(object) = chat_request.as_object_mut() else {
            return;
        };

        normalize_system_messages(object);
        clamp_f64_setting(object, "temperature", 1.0);
        clamp_f64_setting(object, "top_p", 0.95);
        clamp_u64_setting(object, "max_completion_tokens", 2048);
    }
}

fn normalize_system_messages(object: &mut serde_json::Map<String, Value>) {
    let Some(messages) = object.get_mut("messages").and_then(Value::as_array_mut) else {
        return;
    };

    let mut system_parts = Vec::new();
    let mut rest = Vec::with_capacity(messages.len());

    for message in std::mem::take(messages) {
        if message.get("role").and_then(Value::as_str) == Some("system") {
            if let Some(content) = message.get("content").and_then(content_to_text) {
                if !content.is_empty() {
                    system_parts.push(content);
                }
            }
        } else {
            rest.push(message);
        }
    }

    if !system_parts.is_empty() {
        rest.insert(
            0,
            serde_json::json!({
                "role": "system",
                "content": system_parts.join("\n\n")
            }),
        );
    }

    *messages = rest;
}

fn content_to_text(content: &Value) -> Option<String> {
    match content {
        Value::String(text) => Some(text.trim().to_string()),
        Value::Array(parts) => {
            let text = parts
                .iter()
                .filter_map(|part| {
                    part.get("text")
                        .or_else(|| part.get("input_text"))
                        .and_then(Value::as_str)
                })
                .filter(|text| !text.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");
            Some(text)
        }
        _ => None,
    }
}

fn clamp_f64_setting(object: &mut serde_json::Map<String, Value>, key: &str, fallback: f64) {
    let Some(value) = object.get(key) else {
        return;
    };
    let next = value
        .as_f64()
        .filter(|value| *value > 0.0 && *value <= 1.0)
        .unwrap_or(fallback);
    if let Some(number) = Number::from_f64(next) {
        object.insert(key.to_string(), Value::Number(number));
    } else {
        object.remove(key);
    }
}

fn clamp_u64_setting(object: &mut serde_json::Map<String, Value>, key: &str, max: u64) {
    let Some(value) = object.get(key) else {
        return;
    };
    let next = value
        .as_u64()
        .filter(|value| *value >= 1)
        .unwrap_or(max)
        .min(max);
    object.insert(key.to_string(), Value::Number(next.into()));
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn clamps_minimax_chat_settings_to_supported_ranges() {
        let mut adapter = MiniMaxProxyAdapter;
        let mut request = json!({
            "model": "MiniMax-M2.7",
            "messages": [],
            "temperature": 0,
            "top_p": 0,
            "max_completion_tokens": 8192
        });

        adapter.prepare_chat_request(&mut request);

        assert_eq!(request["temperature"], 1.0);
        assert_eq!(request["top_p"], 0.95);
        assert_eq!(request["max_completion_tokens"], 2048);
    }

    #[test]
    fn folds_system_messages_into_one_leading_message() {
        let mut adapter = MiniMaxProxyAdapter;
        let mut request = json!({
            "model": "MiniMax-M2.7",
            "messages": [
                { "role": "system", "content": "Global instructions." },
                { "role": "user", "content": "Hi" },
                { "role": "system", "content": "Developer instructions." },
                {
                    "role": "system",
                    "content": [{ "type": "text", "text": "Extra instructions." }]
                }
            ]
        });

        adapter.prepare_chat_request(&mut request);

        let messages = request["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(
            messages[0]["content"],
            "Global instructions.\n\nDeveloper instructions.\n\nExtra instructions."
        );
        assert_eq!(messages[1]["role"], "user");
    }

    #[test]
    fn leaves_valid_minimax_chat_settings_unchanged() {
        let mut adapter = MiniMaxProxyAdapter;
        let mut request = json!({
            "model": "MiniMax-M2.7",
            "messages": [],
            "temperature": 0.2,
            "top_p": 0.8,
            "max_completion_tokens": 1024
        });

        adapter.prepare_chat_request(&mut request);

        assert_eq!(request["temperature"], 0.2);
        assert_eq!(request["top_p"], 0.8);
        assert_eq!(request["max_completion_tokens"], 1024);
    }
}
