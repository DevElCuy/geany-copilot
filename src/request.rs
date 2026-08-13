use crate::backend::{
    build_backend_url, build_request_payload, estimate_token_count, parse_api_error,
    parse_complete_response, parse_ollama_response, parse_openai_compatible_response,
    parse_temperature, BackendType, InsertMode,
};
use crate::ffi::geany::{document_find_by_id, document_get_current, GeanyPlugin};
use crate::ffi::glib::*;
use crate::ffi::scintilla::*;
use crate::globals::{
    with_global_state, CURL_TIMEOUT_INDEX, MAX_TOKENS, NEXT_REQUEST_ID, THINKING_LOG_ENABLED,
};
use std::ffi::{CStr, CString};
use std::ptr;
use std::sync::{Arc, atomic::{AtomicI32, Ordering}};

pub static mut ACTIVE_REQUEST: *mut RequestData = ptr::null_mut();

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StreamTarget {
    Response,
    Thinking,
}

pub struct StreamUpdate {
    pub request_id: u64,
    pub target: StreamTarget,
    pub delta: String,
    pub estimated_tokens: usize,
    pub tokens_per_second: f64,
}

struct WorkerInput {
    id: u64,
    backend: BackendType,
    url: String,
    payload: String,
    bearer_token: String,
    cancel_requested: Arc<AtomicI32>,
    stop_requested: Arc<AtomicI32>,
    started_at_us: i64,
}

struct StreamAccumulator {
    raw_response: String,
    stream_buffer: String,
    response_text: String,
    in_thinking: bool,
    estimated_tokens: usize,
    error_message: String,
}

struct RequestResult {
    request_id: u64,
    response_text: String,
    raw_response: String,
    error_message: String,
    http_status: i64,
    cancelled: bool,
}

struct CompletionEvent {
    request_addr: usize,
    result: RequestResult,
}

pub struct RequestData {
    pub id: u64,
    pub document_id: u32,
    pub insert_pos: i32,
    pub selection_start: i32,
    pub selection_end: i32,
    pub insert_mode: InsertMode,
    pub response_text: String,
    pub raw_response: String,
    pub error_message: String,
    pub http_status: i64,
    pub cancel_requested: Arc<AtomicI32>,
    pub stop_requested: Arc<AtomicI32>,
    pub abandon_result: Arc<AtomicI32>,
    pub completed: AtomicI32,
}

impl RequestData {
    fn new(
        document_id: u32,
        insert_pos: i32,
        selection_start: i32,
        selection_end: i32,
        insert_mode: InsertMode,
    ) -> Self {
        Self {
            id: NEXT_REQUEST_ID.fetch_add(1, Ordering::SeqCst),
            document_id,
            insert_pos,
            selection_start,
            selection_end,
            insert_mode,
            response_text: String::new(),
            raw_response: String::new(),
            error_message: String::new(),
            http_status: 0,
            cancel_requested: Arc::new(AtomicI32::new(0)),
            stop_requested: Arc::new(AtomicI32::new(0)),
            abandon_result: Arc::new(AtomicI32::new(0)),
            completed: AtomicI32::new(0),
        }
    }
}

pub unsafe fn build_context_text(sci: *mut ScintillaObject) -> String {
    if sci.is_null() {
        return String::new();
    }

    if sci_has_selection(sci) != 0 {
        let text = sci_get_selection_contents(sci);
        if text.is_null() {
            return String::new();
        }
        let result = CStr::from_ptr(text).to_string_lossy().into_owned();
        g_free(text as GPointer);
        if !result.is_empty() { return result; }
    }

    let current_pos = sci_get_current_position(sci);
    let doc_len = sci_get_length(sci);
    let start_pos = (current_pos - 100).max(0);
    let end_pos = (current_pos + 100).min(doc_len);
    if end_pos <= start_pos {
        return String::new();
    }

    let text = sci_get_contents_range(sci, start_pos, end_pos);
    if text.is_null() {
        return String::new();
    }
    let result = CStr::from_ptr(text).to_string_lossy().into_owned();
    g_free(text as GPointer);
    result
}

unsafe fn document_language_hint(doc: *mut crate::ffi::geany::GeanyDocument) -> Option<&'static str> {
    if doc.is_null() || (*doc).file_name.is_null() {
        return None;
    }
    let file_name = CStr::from_ptr((*doc).file_name).to_string_lossy();
    let extension = file_name.rsplit('.').next()?.to_ascii_lowercase();
    match extension.as_str() {
        "rs" => Some("Rust"),
        "py" | "pyw" => Some("Python"),
        "c" | "h" => Some("C"),
        "cc" | "cpp" | "cxx" | "hpp" | "hh" => Some("C++"),
        "js" | "mjs" | "cjs" => Some("JavaScript"),
        "ts" | "tsx" => Some("TypeScript"),
        "jsx" => Some("JavaScript (JSX)"),
        "java" => Some("Java"),
        "go" => Some("Go"),
        "rb" => Some("Ruby"),
        "php" => Some("PHP"),
        "sh" | "bash" | "zsh" => Some("Shell"),
        "json" => Some("JSON"),
        "toml" => Some("TOML"),
        "yaml" | "yml" => Some("YAML"),
        "md" | "markdown" => Some("Markdown"),
        "html" | "htm" => Some("HTML"),
        "css" => Some("CSS"),
        "sql" => Some("SQL"),
        "tex" => Some("LaTeX"),
        _ => None,
    }
}

unsafe fn set_request_cancelled(req: &mut RequestData, stop: bool) {
    if stop {
        req.stop_requested.store(1, Ordering::SeqCst);
    }
    req.cancel_requested.store(1, Ordering::SeqCst);
    crate::ui::set_copilot_panel_cancelling(stop);
}

pub unsafe fn stop_active_request() {
    if !ACTIVE_REQUEST.is_null() {
        set_request_cancelled(&mut *ACTIVE_REQUEST, true);
    }
}

pub unsafe fn cancel_active_request() {
    if !ACTIVE_REQUEST.is_null() {
        set_request_cancelled(&mut *ACTIVE_REQUEST, false);
    }
}

unsafe fn queue_stream_update(
    id: u64,
    target: StreamTarget,
    delta: String,
    estimated_tokens: usize,
    started_at_us: i64,
) {
    if delta.is_empty() { return; }
    let elapsed = (g_get_monotonic_time() - started_at_us).max(1) as f64 / 1_000_000.0;
    let update = Box::new(StreamUpdate {
        request_id: id,
        target,
        delta,
        estimated_tokens,
        tokens_per_second: estimated_tokens as f64 / elapsed,
    });
    g_idle_add(Some(on_stream_update), Box::into_raw(update) as GPointer);
}

fn process_stream_line(
    input: &WorkerInput,
    line: &str,
    response_text: &mut String,
    in_thinking: &mut bool,
    estimated_tokens: &mut usize,
    error_message: &mut String,
) {
    let mut line = line.trim().trim_end_matches('\r').trim().to_string();
    if line.is_empty() || line.starts_with("event:") || line.starts_with(':') { return; }
    if input.backend == BackendType::OpenAICompatible {
        if let Some(data) = line.strip_prefix("data:") { line = data.trim().to_string(); }
        if line.is_empty() || line == "[DONE]" { return; }
    }

    let value: serde_json::Value = match serde_json::from_str(&line) {
        Ok(value) => value,
        Err(error) => { *error_message = format!("Failed to parse streaming JSON: {}", error); return; }
    };
    if let Some(error) = parse_api_error(&value) { *error_message = error; return; }

    let mut deltas: Vec<(String, bool)> = Vec::new();
    match input.backend {
        BackendType::Ollama => {
            if let Some(delta) = parse_ollama_response(&value) { deltas.push((delta, false)); }
        }
        BackendType::OpenAICompatible => {
            if let Some(choices) = value.get("choices").and_then(|v| v.as_array()) {
                if let Some(choice) = choices.first() {
                    if let Some(delta) = choice.get("delta") {
                        if let Some(reasoning) = delta.get("reasoning_content").and_then(|v| v.as_str()) {
                            deltas.push((reasoning.to_string(), true));
                        }
                        if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                            deltas.push((content.to_string(), false));
                        }
                    } else if let Some(text) = choice.get("text").and_then(|v| v.as_str()) {
                        deltas.push((text.to_string(), false));
                    }
                }
            }
        }
    }

    for (delta, thinking) in deltas {
        if delta.is_empty() { continue; }
        if thinking {
            if THINKING_LOG_ENABLED.load(Ordering::SeqCst) == 0 {
                // A disabled log keeps no decoded reasoning state.  If the
                // setting changes mid-request, discard all later reasoning.
                *in_thinking = false;
                continue;
            }
            if !*in_thinking {
                *in_thinking = true;
                unsafe {
                    queue_stream_update(
                        input.id,
                        StreamTarget::Thinking,
                        "<think>\n".to_string(),
                        0,
                        input.started_at_us,
                    );
                }
            }
            unsafe {
                queue_stream_update(
                    input.id,
                    StreamTarget::Thinking,
                    delta,
                    0,
                    input.started_at_us,
                );
            }
        } else {
            if *in_thinking && THINKING_LOG_ENABLED.load(Ordering::SeqCst) != 0 {
                *in_thinking = false;
                unsafe {
                    queue_stream_update(
                        input.id,
                        StreamTarget::Thinking,
                        "\n</think>\n\n".to_string(),
                        0,
                        input.started_at_us,
                    );
                }
            } else if *in_thinking {
                *in_thinking = false;
            }
            response_text.push_str(&delta);
            *estimated_tokens = estimate_token_count(response_text);
            unsafe {
                queue_stream_update(
                    input.id,
                    StreamTarget::Response,
                    delta,
                    *estimated_tokens,
                    input.started_at_us,
                );
            }
        }
    }
}

fn fallback_response(raw: &str, backend: BackendType) -> Option<String> {
    if let Some(value) = parse_complete_response(raw, backend) { return Some(value); }
    for line in raw.lines() {
        let line = line.trim().strip_prefix("data:").map(str::trim).unwrap_or(line.trim());
        if line == "[DONE]" || line.is_empty() { continue; }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            let parsed = match backend {
                BackendType::Ollama => parse_ollama_response(&value),
                BackendType::OpenAICompatible => parse_openai_compatible_response(&value),
            };
            if parsed.is_some() { return parsed; }
        }
    }
    None
}

fn copilot_request_worker(input: WorkerInput) -> RequestResult {
    let accumulator = Arc::new(std::sync::Mutex::new(StreamAccumulator {
        raw_response: String::new(),
        stream_buffer: String::new(),
        response_text: String::new(),
        in_thinking: false,
        estimated_tokens: 0,
        error_message: String::new(),
    }));
    let mut error_message = String::new();
    let mut http_status = 0i64;

    let mut easy = curl::easy::Easy::new();
    if let Err(error) = easy.url(&input.url).and_then(|_| easy.post(true)).and_then(|_| easy.post_fields_copy(input.payload.as_bytes())) {
        error_message = format!("curl setup error: {}", error);
    } else {
        let mut headers = curl::easy::List::new();
        if let Err(error) = headers.append("Content-Type: application/json") {
            error_message = format!("curl setup error: {}", error);
        } else if !input.bearer_token.is_empty()
            && headers
                .append(&format!("Authorization: Bearer {}", input.bearer_token))
                .is_err()
        {
            error_message = "curl setup error: invalid authorization header".to_string();
        } else if let Err(error) = easy.http_headers(headers) {
            error_message = format!("curl setup error: {}", error);
        } else {
            let timeout = crate::backend::active_curl_timeout_seconds(CURL_TIMEOUT_INDEX.load(Ordering::SeqCst) as usize);
            let _ = easy.timeout(std::time::Duration::from_secs(timeout as u64));
            let cancel_for_write = Arc::clone(&input.cancel_requested);
            let id = input.id;
            let backend = input.backend;
            let started = input.started_at_us;
            let stop_for_write = Arc::clone(&input.stop_requested);
            let accumulator_for_write = Arc::clone(&accumulator);
            let write_result = easy.write_function(move |data| {
                let chunk = String::from_utf8_lossy(data);
                let mut accumulator = accumulator_for_write.lock().unwrap();
                accumulator.raw_response.push_str(&chunk);
                accumulator.stream_buffer.push_str(&chunk);
                while let Some(pos) = accumulator.stream_buffer.find('\n') {
                    let line = accumulator.stream_buffer[..pos].to_string();
                    accumulator.stream_buffer.drain(..=pos);
                    let worker_view = WorkerInput { id, backend, url: String::new(), payload: String::new(), bearer_token: String::new(), cancel_requested: Arc::clone(&cancel_for_write), stop_requested: Arc::clone(&stop_for_write), started_at_us: started };
                    let StreamAccumulator { response_text, in_thinking, estimated_tokens, error_message, .. } = &mut *accumulator;
                    process_stream_line(&worker_view, &line, response_text, in_thinking, estimated_tokens, error_message);
                }
                Ok(data.len())
            });
            if write_result.is_err() {
                error_message = "Unable to configure curl response handler.".to_string();
            } else {
                let cancel_for_progress = Arc::clone(&input.cancel_requested);
                let _ = easy.progress(true);
                let _ = easy.progress_function(move |_, _, _, _| cancel_for_progress.load(Ordering::SeqCst) == 0);
                match easy.perform() {
                    Ok(()) => {}
                    Err(error) => {
                        if input.cancel_requested.load(Ordering::SeqCst) == 0 {
                            error_message = format!("curl error: {}", error);
                        }
                    }
                }
                if let Ok(status) = easy.response_code() { http_status = status as i64; }
            }
        }
    }

    drop(easy);
    let (raw_response, stream_buffer, mut response_text, mut in_thinking, mut estimated_tokens, stream_error) = {
        let accumulator = accumulator.lock().unwrap();
        (accumulator.raw_response.clone(), accumulator.stream_buffer.clone(), accumulator.response_text.clone(), accumulator.in_thinking, accumulator.estimated_tokens, accumulator.error_message.clone())
    };
    if error_message.is_empty() { error_message = stream_error; }

    if !stream_buffer.trim().is_empty() {
        process_stream_line(&input, &stream_buffer, &mut response_text, &mut in_thinking, &mut estimated_tokens, &mut error_message);
    }
    if THINKING_LOG_ENABLED.load(Ordering::SeqCst) != 0 && in_thinking {
        unsafe {
            queue_stream_update(
                input.id,
                StreamTarget::Thinking,
                "\n</think>\n".to_string(),
                0,
                input.started_at_us,
            );
        }
    }
    if !response_text.is_empty() && error_message.starts_with("curl error:") {
        error_message.clear();
    }
    let cancelled = input.cancel_requested.load(Ordering::SeqCst) != 0 && input.stop_requested.load(Ordering::SeqCst) == 0;
    if cancelled {
        error_message.clear();
    } else if error_message.is_empty() && response_text.is_empty() {
        if let Some(response) = fallback_response(&raw_response, input.backend) {
            response_text = response;
        }
    }
    if error_message.is_empty() && http_status >= 400 {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw_response) {
            error_message = parse_api_error(&value).unwrap_or_else(|| format!("HTTP error {}", http_status));
        } else {
            error_message = format!("HTTP error {}", http_status);
        }
    }

    RequestResult { request_id: input.id, response_text, raw_response, error_message, http_status, cancelled }
}

pub unsafe extern "C" fn on_stream_update(user_data: GPointer) -> GBoolean {
    let update = Box::from_raw(user_data as *mut StreamUpdate);
    if ACTIVE_REQUEST.is_null() { return G_FALSE; }
    let req = &mut *ACTIVE_REQUEST;
    if req.id != update.request_id || req.completed.load(Ordering::SeqCst) != 0 { return G_FALSE; }
    if update.target == StreamTarget::Thinking {
        crate::ui::append_thinking_log(&update.delta);
        return G_FALSE;
    }
    crate::ui::update_copilot_panel_stats(update.estimated_tokens, update.tokens_per_second);
    G_FALSE
}

pub unsafe extern "C" fn on_request_finished(user_data: GPointer) -> GBoolean {
    let event = Box::from_raw(user_data as *mut CompletionEvent);
    let req_ptr = event.request_addr as *mut RequestData;
    if req_ptr.is_null() { return G_FALSE; }
    let result = event.result;
    if result.request_id == 0 { return G_FALSE; }
    let req = &mut *req_ptr;
    if req.id != result.request_id { return G_FALSE; }

    req.response_text = result.response_text;
    req.raw_response = result.raw_response;
    req.error_message = result.error_message;
    req.http_status = result.http_status;
    req.completed.store(1, Ordering::SeqCst);
    if ACTIVE_REQUEST == req_ptr { ACTIVE_REQUEST = ptr::null_mut(); }
    let abandoned = req.abandon_result.load(Ordering::SeqCst) != 0;
    let cancelled = result.cancelled;
    let stopped = req.stop_requested.load(Ordering::SeqCst) != 0;

    if abandoned || (cancelled && !stopped) {
        crate::ui::finish_copilot_request(if abandoned { "Request abandoned" } else { "Request cancelled" });
        drop(Box::from_raw(req_ptr));
        return G_FALSE;
    }
    if req.error_message.is_empty() && req.response_text.is_empty() {
        req.error_message = "No response text could be extracted from the upstream response.".to_string();
    }
    if !req.error_message.is_empty() {
        let status = format!("Error: {}", req.error_message);
        crate::ui::append_copilot_error(&status, &req.raw_response);
        crate::ui::finish_copilot_request(&status);
        drop(Box::from_raw(req_ptr));
        return G_FALSE;
    }

    let doc = document_find_by_id(req.document_id);
    if !doc.is_null() && (*doc).is_valid != 0 && !(*doc).editor.is_null() && !(*(*doc).editor).sci.is_null() {
        let sci = (*(*doc).editor).sci;
        let document_length = sci_get_length(sci).max(0);
        let start = req.selection_start.clamp(0, document_length);
        let end = req.selection_end.clamp(start, document_length);
        let response_text = match req.insert_mode {
            InsertMode::AppendAfterSelection if end > start && !req.response_text.starts_with('\n') => {
                format!("\n{}", req.response_text)
            }
            _ => req.response_text.clone(),
        };
        // The model's text may legally contain NUL bytes; strip them so one
        // NUL can't silently void the whole insert at the CString boundary.
        let response = CString::new(response_text.replace('\0', "")).unwrap_or_default();
        match req.insert_mode {
            InsertMode::ReplaceSelection if end > start => {
                sci_set_selection_start(sci, start);
                sci_set_selection_end(sci, end);
                sci_replace_sel(sci, response.as_ptr());
            }
            InsertMode::AppendAfterSelection if end > start => {
                sci_insert_text(sci, end, response.as_ptr());
            }
            _ => {
                let position = req.insert_pos.clamp(0, document_length);
                sci_insert_text(sci, position, response.as_ptr());
            }
        }
    }
    crate::ui::finish_copilot_request(if stopped {
        "Stopped — inserted partial response"
    } else {
        "Response inserted"
    });
    drop(Box::from_raw(req_ptr));
    G_FALSE
}

pub unsafe fn abandon_active_request() {
    if ACTIVE_REQUEST.is_null() { return; }
    let req = &mut *ACTIVE_REQUEST;
    req.abandon_result.store(1, Ordering::SeqCst);
    req.cancel_requested.store(1, Ordering::SeqCst);
    crate::ui::finish_copilot_request("Request abandoned");
    ACTIVE_REQUEST = ptr::null_mut();
}

pub unsafe fn ask_copilot(_plugin: *mut GeanyPlugin) {
    if !ACTIVE_REQUEST.is_null() {
        return;
    }
    let doc = document_get_current();
    if doc.is_null() || (*doc).is_valid == 0 || (*doc).editor.is_null() || (*(*doc).editor).sci.is_null() {
        return;
    }
    let sci = (*(*doc).editor).sci;
    let current_pos = sci_get_current_position(sci);
    let selection_start = sci_get_selection_start(sci);
    let selection_end = sci_get_selection_end(sci);
    let mut context = build_context_text(sci);
    if context.is_empty() { return; }

    let (backend, uri, model, system_prompt, api_key, temperature, include_language_hint, insert_mode) = with_global_state(|state| {
        (
            state.backend_type,
            state.upstream_uri.clone(),
            state.model_name.clone(),
            state.system_prompt.clone(),
            state.api_key.clone(),
            state.temperature.clone(),
            state.include_language_hint,
            state.insert_mode,
        )
    });
    if include_language_hint {
        if let Some(language) = document_language_hint(doc) {
            context.push_str("\n\nLanguage: ");
            context.push_str(language);
        }
    }
    let max_tokens = MAX_TOKENS.load(Ordering::SeqCst).max(0);
    let url = build_backend_url(backend, &uri);
    let payload = build_request_payload(
        &context, backend, &model, &system_prompt, max_tokens, parse_temperature(&temperature),
    );
    let req = Box::new(RequestData::new(
        (*doc).id, current_pos, selection_start, selection_end, insert_mode,
    ));
    let input = WorkerInput { id: req.id, backend, url, payload, bearer_token: api_key, cancel_requested: Arc::clone(&req.cancel_requested), stop_requested: Arc::clone(&req.stop_requested), started_at_us: g_get_monotonic_time() };
    let req_ptr = Box::into_raw(req);
    ACTIVE_REQUEST = req_ptr;
    crate::ui::begin_copilot_request(
        &model,
        &input.url,
        &input.payload,
        !input.bearer_token.is_empty(),
    );
    let req_addr = req_ptr as usize;
    std::thread::spawn(move || {
        let result = copilot_request_worker(input);
        unsafe {
            let event = Box::new(CompletionEvent { request_addr: req_addr, result });
            g_idle_add(Some(on_request_finished), Box::into_raw(event) as GPointer);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_thinking_log_discards_decoded_reasoning() {
        let previous = THINKING_LOG_ENABLED.swap(0, Ordering::SeqCst);
        let input = WorkerInput {
            id: 1,
            backend: BackendType::OpenAICompatible,
            url: String::new(),
            payload: String::new(),
            bearer_token: String::new(),
            cancel_requested: Arc::new(AtomicI32::new(0)),
            stop_requested: Arc::new(AtomicI32::new(0)),
            started_at_us: 0,
        };
        let mut response = String::new();
        let mut in_thinking = false;
        let mut tokens = 0;
        let mut error = String::new();

        process_stream_line(
            &input,
            r#"data: {"choices":[{"delta":{"reasoning_content":"private reasoning"}}]}"#,
            &mut response,
            &mut in_thinking,
            &mut tokens,
            &mut error,
        );

        THINKING_LOG_ENABLED.store(previous, Ordering::SeqCst);
        assert!(response.is_empty());
        assert!(!in_thinking);
        assert!(error.is_empty());
    }
}
