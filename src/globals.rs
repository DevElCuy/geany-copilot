use crate::backend::{BackendPreset, BackendType, InsertMode, DEFAULT_CURL_TIMEOUT_INDEX, DEFAULT_OLLAMA_URI};
use std::sync::atomic::{AtomicI32, AtomicU64};
use std::sync::Mutex;

pub static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);
pub static CURL_TIMEOUT_INDEX: AtomicI32 = AtomicI32::new(DEFAULT_CURL_TIMEOUT_INDEX as i32);
pub static MAX_TOKENS: AtomicI32 = AtomicI32::new(0);
/// Whether decoded reasoning is retained and displayed in the sidebar.
/// Keeping this off avoids allocating a second copy of streamed reasoning.
pub static THINKING_LOG_ENABLED: AtomicI32 = AtomicI32::new(1);

pub static ACTIVE_PRESET_INDEX: AtomicI32 = AtomicI32::new(0);

// Thread-safe wrapper for global settings shared across main thread calls
pub struct GlobalState {
    pub presets: Vec<BackendPreset>,
    pub backend_type: BackendType,
    pub upstream_uri: String,
    pub model_name: String,
    pub system_prompt: String,
    pub api_key: String,
    pub temperature: String,
    pub include_language_hint: bool,
    pub insert_mode: InsertMode,
}

static GLOBAL_STATE: Mutex<Option<GlobalState>> = Mutex::new(None);

pub fn init_global_state() {
    let mut guard = GLOBAL_STATE.lock().unwrap();
    if guard.is_none() {
        *guard = Some(GlobalState {
            presets: Vec::new(),
            backend_type: BackendType::Ollama,
            upstream_uri: DEFAULT_OLLAMA_URI.to_string(),
            model_name: String::new(),
            system_prompt: String::new(),
            api_key: String::new(),
            temperature: String::new(),
            include_language_hint: true,
            insert_mode: InsertMode::Cursor,
        });
    }
}

pub fn with_global_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut GlobalState) -> R,
{
    init_global_state();
    let mut guard = GLOBAL_STATE.lock().unwrap();
    f(guard.as_mut().unwrap())
}
