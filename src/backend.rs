use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_OLLAMA_URI: &str = "http://localhost:11434";
pub const DEFAULT_OPENAI_COMPATIBLE_URI: &str = "http://localhost:11434/v1";
pub const BACKEND_OLLAMA_ID: &str = "ollama";
pub const BACKEND_OPENAI_COMPATIBLE_ID: &str = "openai-compatible";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendType {
    Ollama,
    OpenAICompatible,
}

impl BackendType {
    pub fn from_id(id: &str) -> Self {
        if id == BACKEND_OPENAI_COMPATIBLE_ID {
            BackendType::OpenAICompatible
        } else {
            BackendType::Ollama
        }
    }

    pub fn id(&self) -> &'static str {
        match self {
            BackendType::Ollama => BACKEND_OLLAMA_ID,
            BackendType::OpenAICompatible => BACKEND_OPENAI_COMPATIBLE_ID,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            BackendType::Ollama => "Ollama",
            BackendType::OpenAICompatible => "OpenAI-compatible",
        }
    }

    pub fn default_uri(&self) -> &'static str {
        match self {
            BackendType::Ollama => DEFAULT_OLLAMA_URI,
            BackendType::OpenAICompatible => DEFAULT_OPENAI_COMPATIBLE_URI,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendPreset {
    pub name: String,
    pub backend_type: BackendType,
    pub uri: String,
    pub model: String,
    #[serde(default)]
    pub system_prompt: String,
    /// Optional bearer token for servers that require authentication.  This is
    /// intentionally kept out of the request-payload audit because it is sent
    /// as an HTTP header rather than JSON.
    #[serde(default)]
    pub api_key: String,
    /// An empty value means "let the server decide".
    #[serde(default)]
    pub temperature: String,
    #[serde(default = "default_language_hint")]
    pub include_language_hint: bool,
    #[serde(default)]
    pub insert_mode: InsertMode,
}

fn default_language_hint() -> bool { true }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsertMode {
    Cursor,
    ReplaceSelection,
    AppendAfterSelection,
}

impl Default for InsertMode {
    fn default() -> Self { Self::Cursor }
}

impl InsertMode {
    pub fn from_id(id: &str) -> Self {
        match id {
            "replace-selection" => Self::ReplaceSelection,
            "append-after-selection" => Self::AppendAfterSelection,
            _ => Self::Cursor,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Cursor => "cursor",
            Self::ReplaceSelection => "replace-selection",
            Self::AppendAfterSelection => "append-after-selection",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CurlTimeoutOption {
    pub seconds: i64,
    pub label: &'static str,
}

pub static CURL_TIMEOUT_OPTIONS: &[CurlTimeoutOption] = &[
    CurlTimeoutOption { seconds: 30, label: "30s" },
    CurlTimeoutOption { seconds: 60, label: "1 min" },
    CurlTimeoutOption { seconds: 120, label: "2 min" },
    CurlTimeoutOption { seconds: 180, label: "3 min" },
    CurlTimeoutOption { seconds: 300, label: "5 min" },
    CurlTimeoutOption { seconds: 420, label: "7 min" },
    CurlTimeoutOption { seconds: 600, label: "10 min" },
];

pub const DEFAULT_CURL_TIMEOUT_INDEX: usize = 1; // 1 min

pub fn active_curl_timeout_seconds(index: usize) -> i64 {
    if index < CURL_TIMEOUT_OPTIONS.len() {
        CURL_TIMEOUT_OPTIONS[index].seconds
    } else {
        CURL_TIMEOUT_OPTIONS[DEFAULT_CURL_TIMEOUT_INDEX].seconds
    }
}

pub fn curl_timeout_index_for_seconds(seconds: i64) -> usize {
    for (i, opt) in CURL_TIMEOUT_OPTIONS.iter().enumerate() {
        if opt.seconds == seconds {
            return i;
        }
    }
    DEFAULT_CURL_TIMEOUT_INDEX
}

pub fn build_backend_url(backend_type: BackendType, base_uri: &str) -> String {
    let uri = if base_uri.trim().is_empty() {
        backend_type.default_uri()
    } else {
        base_uri.trim()
    }
    .trim_end_matches('/');
    match backend_type {
        BackendType::Ollama => {
            if uri.ends_with("/api/generate") { uri.to_string() } else { format!("{}/api/generate", uri) }
        }
        BackendType::OpenAICompatible => {
            if uri.ends_with("/chat/completions") {
                uri.to_string()
            } else if uri.ends_with("/v1") {
                format!("{}/chat/completions", uri)
            } else {
                format!("{}/v1/chat/completions", uri)
            }
        }
    }
}

pub fn build_model_list_url(backend_type: BackendType, base_uri: &str) -> String {
    let uri = if base_uri.trim().is_empty() {
        backend_type.default_uri()
    } else {
        base_uri.trim()
    }
    .trim_end_matches('/');
    match backend_type {
        BackendType::Ollama => {
            if uri.ends_with("/api/tags") { uri.to_string() }
            else if uri.ends_with("/api/generate") { format!("{}/api/tags", &uri[..uri.len() - "/api/generate".len()]) }
            else { format!("{}/api/tags", uri) }
        }
        BackendType::OpenAICompatible => {
            if uri.ends_with("/models") {
                uri.to_string()
            } else if uri.ends_with("/chat/completions") {
                format!("{}/models", &uri[..uri.len() - "/chat/completions".len()])
            } else if uri.ends_with("/v1") {
                format!("{}/models", uri)
            } else {
                format!("{}/v1/models", uri)
            }
        }
    }
}

pub fn build_ollama_payload(
    context_text: &str,
    model: &str,
    system_prompt: &str,
    max_tokens: i32,
    temperature: Option<f64>,
) -> String {
    let mut payload = serde_json::Map::new();
    if !model.is_empty() {
        payload.insert("model".to_string(), Value::String(model.to_string()));
    }
    if !system_prompt.trim().is_empty() {
        payload.insert("system".to_string(), Value::String(system_prompt.to_string()));
    }
    let mut options = serde_json::Map::new();
    if max_tokens > 0 {
        options.insert("num_predict".to_string(), serde_json::json!(max_tokens));
    }
    if let Some(temperature) = temperature {
        options.insert("temperature".to_string(), serde_json::json!(temperature));
    }
    if !options.is_empty() {
        payload.insert("options".to_string(), Value::Object(options));
    }
    payload.insert("prompt".to_string(), Value::String(context_text.to_string()));
    payload.insert("stream".to_string(), Value::Bool(true));
    Value::Object(payload).to_string()
}

pub fn build_openai_compatible_payload(
    context_text: &str,
    model: &str,
    system_prompt: &str,
    max_tokens: i32,
    temperature: Option<f64>,
) -> String {
    let mut payload = serde_json::Map::new();
    if !model.is_empty() {
        payload.insert("model".to_string(), Value::String(model.to_string()));
    }
    let mut messages = Vec::new();
    if !system_prompt.trim().is_empty() {
        messages.push(serde_json::json!({"role": "system", "content": system_prompt}));
    }
    messages.push(serde_json::json!({"role": "user", "content": context_text}));
    payload.insert(
        "messages".to_string(),
        Value::Array(messages),
    );
    if max_tokens > 0 {
        payload.insert("max_tokens".to_string(), serde_json::json!(max_tokens));
    }
    if let Some(temperature) = temperature {
        payload.insert("temperature".to_string(), serde_json::json!(temperature));
    }
    payload.insert("stream".to_string(), Value::Bool(true));
    Value::Object(payload).to_string()
}

pub fn build_request_payload(
    context_text: &str,
    backend_type: BackendType,
    model: &str,
    system_prompt: &str,
    max_tokens: i32,
    temperature: Option<f64>,
) -> String {
    match backend_type {
        BackendType::Ollama => build_ollama_payload(context_text, model, system_prompt, max_tokens, temperature),
        BackendType::OpenAICompatible => {
            build_openai_compatible_payload(context_text, model, system_prompt, max_tokens, temperature)
        }
    }
}

pub fn parse_temperature(value: &str) -> Option<f64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let parsed = value.parse::<f64>().ok()?;
    if parsed.is_finite() && (0.0..=2.0).contains(&parsed) {
        Some(parsed)
    } else {
        None
    }
}

pub fn parse_ollama_response(val: &Value) -> Option<String> {
    val.get("response").and_then(|v| v.as_str()).map(|s| s.to_string())
}

pub fn parse_openai_compatible_response(val: &Value) -> Option<String> {
    if let Some(choices) = val.get("choices").and_then(|v| v.as_array()) {
        if let Some(first) = choices.first() {
            if let Some(delta) = first.get("delta").and_then(|v| v.get("content")).and_then(|v| v.as_str()) {
            return Some(delta.to_string());
            }
            if let Some(msg) = first.get("message").and_then(|v| v.get("content")).and_then(|v| v.as_str()) {
                return Some(msg.to_string());
            }
            if let Some(text) = first.get("text").and_then(|v| v.as_str()) {
                return Some(text.to_string());
            }
        }
    }
    None
}

pub fn parse_api_error(val: &Value) -> Option<String> {
    if let Some(err) = val.get("error") {
        if let Some(msg) = err.get("message").and_then(|v| v.as_str()) {
            return Some(format!("API error: {}", msg));
        }
        if let Some(msg) = err.as_str() {
            return Some(format!("API error: {}", msg));
        }
    }
    None
}

pub fn parse_model_list_response(body: &str, backend_type: BackendType) -> Result<Vec<String>, String> {
    let val: Value = serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {}", e))?;

    let mut models = Vec::new();
    // JSON strings may contain NUL bytes, but these names cross into GTK as
    // C strings — strip NUL here rather than panic at the FFI boundary.
    match backend_type {
        BackendType::Ollama => {
            if let Some(arr) = val.get("models").and_then(|v| v.as_array()) {
                for item in arr {
                    if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                        models.push(name.replace('\0', ""));
                    }
                }
            }
        }
        BackendType::OpenAICompatible => {
            if let Some(arr) = val.get("data").and_then(|v| v.as_array()) {
                for item in arr {
                    if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                        models.push(id.replace('\0', ""));
                    }
                }
            }
        }
    }

    if models.is_empty() {
        if let Some(err_msg) = parse_api_error(&val) {
            Err(err_msg)
        } else {
            Err("No models found in server response.".to_string())
        }
    } else {
        Ok(models)
    }
}

pub fn estimate_token_count(text: &str) -> usize {
    // Rough estimate: ~4 chars per token for English / code.
    if text.is_empty() { 0 } else { ((text.len() + 3) / 4).max(1) }
}

pub fn parse_complete_response(body: &str, backend_type: BackendType) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    match backend_type {
        BackendType::Ollama => parse_ollama_response(&value),
        BackendType::OpenAICompatible => parse_openai_compatible_response(&value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_normalization_matches_try4() {
        assert_eq!(build_backend_url(BackendType::Ollama, ""), "http://localhost:11434/api/generate");
        assert_eq!(build_backend_url(BackendType::Ollama, "http://x/api/generate"), "http://x/api/generate");
        assert_eq!(build_backend_url(BackendType::OpenAICompatible, "http://x"), "http://x/v1/chat/completions");
        assert_eq!(build_backend_url(BackendType::OpenAICompatible, "http://x/v1/"), "http://x/v1/chat/completions");
        assert_eq!(build_model_list_url(BackendType::OpenAICompatible, "http://x/v1"), "http://x/v1/models");
    }

    #[test]
    fn empty_model_is_omitted() {
        let payload: Value = serde_json::from_str(&build_ollama_payload("ctx", "", "", 0, None)).unwrap();
        assert!(payload.get("model").is_none());
    }

    #[test]
    fn empty_system_prompt_is_omitted() {
        let ollama: Value =
            serde_json::from_str(&build_ollama_payload("ctx", "model", " \n\t ", 0, None)).unwrap();
        assert!(ollama.get("system").is_none());

        let openai: Value = serde_json::from_str(&build_openai_compatible_payload(
            "ctx", "model", " \n\t ", 0, None,
        ))
        .unwrap();
        let messages = openai["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "user");
    }

    #[test]
    fn system_prompt_is_sent_in_backend_native_form() {
        let system_prompt = "Answer with code only.";
        let ollama: Value =
            serde_json::from_str(&build_ollama_payload("ctx", "model", system_prompt, 0, None)).unwrap();
        assert_eq!(ollama["system"], system_prompt);

        let openai: Value = serde_json::from_str(&build_openai_compatible_payload(
            "ctx",
            "model",
            system_prompt,
            0,
            None,
        ))
        .unwrap();
        let messages = openai["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], system_prompt);
        assert_eq!(messages[1]["role"], "user");
    }

    #[test]
    fn max_tokens_is_omitted_when_zero_and_sent_when_positive() {
        let openai: Value = serde_json::from_str(&build_openai_compatible_payload(
            "ctx", "model", "", 512, None,
        ))
        .unwrap();
        assert_eq!(openai["max_tokens"], 512);

        let ollama: Value =
            serde_json::from_str(&build_ollama_payload("ctx", "model", "", 512, None)).unwrap();
        assert_eq!(ollama["options"]["num_predict"], 512);

        let openai_default: Value = serde_json::from_str(&build_openai_compatible_payload(
            "ctx", "model", "", 0, None,
        ))
        .unwrap();
        assert!(openai_default.get("max_tokens").is_none());
    }

    #[test]
    fn model_list_strips_nul_bytes() {
        let ollama = parse_model_list_response(
            "{\"models\":[{\"name\":\"bad\\u0000name\"}]}",
            BackendType::Ollama,
        )
        .unwrap();
        assert_eq!(ollama, vec!["badname".to_string()]);

        let openai = parse_model_list_response(
            "{\"data\":[{\"id\":\"m\\u0000odel\"}]}",
            BackendType::OpenAICompatible,
        )
        .unwrap();
        assert_eq!(openai, vec!["model".to_string()]);
    }

    #[test]
    fn temperature_is_sent_only_when_configured() {
        let ollama: Value = serde_json::from_str(&build_ollama_payload(
            "ctx", "model", "", 0, Some(0.3),
        )).unwrap();
        assert_eq!(ollama["options"]["temperature"], 0.3);

        let openai: Value = serde_json::from_str(&build_openai_compatible_payload(
            "ctx", "model", "", 0, Some(0.7),
        )).unwrap();
        assert_eq!(openai["temperature"], 0.7);
        assert_eq!(parse_temperature(" 1.5 "), Some(1.5));
        assert_eq!(parse_temperature(""), None);
        assert_eq!(parse_temperature("2.1"), None);
    }
}
