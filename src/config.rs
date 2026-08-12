use crate::backend::{
    curl_timeout_index_for_seconds, BackendPreset, BackendType, InsertMode, DEFAULT_CURL_TIMEOUT_INDEX,
    DEFAULT_OLLAMA_URI,
};
use crate::ffi::glib::*;
use crate::ffi::geany::GeanyPlugin;
use crate::globals::{
    with_global_state, ACTIVE_PRESET_INDEX, CURL_TIMEOUT_INDEX, MAX_TOKENS,
    THINKING_LOG_ENABLED,
};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::sync::atomic::Ordering;

pub const CONFIG_DIR_NAME: &str = "geany-copilot";
pub const CONFIG_FILE_NAME: &str = "geany-copilot.conf";
pub const CONFIG_GROUP_SETTINGS: &str = "settings";
pub const CONFIG_GROUP_PRESET_PREFIX: &str = "preset";

pub unsafe fn key_file_get_string_default(
    key_file: *mut GKeyFile,
    group_name: &str,
    key: &str,
    fallback: &str,
) -> String {
    let c_group = CString::new(group_name).unwrap();
    let c_key = CString::new(key).unwrap();
    let mut error: *mut GError = ptr::null_mut();

    let val_ptr = g_key_file_get_string(key_file, c_group.as_ptr(), c_key.as_ptr(), &mut error);
    if !val_ptr.is_null() {
        let str_val = CStr::from_ptr(val_ptr).to_string_lossy().into_owned();
        g_free(val_ptr as GPointer);
        str_val
    } else {
        if !error.is_null() {
            g_error_free(error);
        }
        fallback.to_string()
    }
}

pub unsafe fn key_file_get_integer_default(
    key_file: *mut GKeyFile,
    group_name: &str,
    key: &str,
    fallback: i32,
) -> i32 {
    let c_group = CString::new(group_name).unwrap();
    let c_key = CString::new(key).unwrap();
    let mut error: *mut GError = ptr::null_mut();

    if g_key_file_has_key(key_file, c_group.as_ptr(), c_key.as_ptr(), ptr::null_mut()) != 0 {
        let val = g_key_file_get_integer(key_file, c_group.as_ptr(), c_key.as_ptr(), &mut error);
        if error.is_null() {
            return val;
        }
        g_error_free(error);
    }
    fallback
}

pub unsafe fn build_config_dir(plugin: *mut GeanyPlugin) -> String {
    let config_root = if !plugin.is_null()
        && !(*plugin).geany_data.is_null()
        && !(*(*plugin).geany_data).app.is_null()
        && !(*(*(*plugin).geany_data).app).configdir.is_null()
    {
        (*(*(*plugin).geany_data).app).configdir as *const GChar
    } else {
        g_get_user_config_dir()
    };
    let c_plugins = CString::new("plugins").unwrap();
    let c_dir_name = CString::new(CONFIG_DIR_NAME).unwrap();
    let path_ptr = g_build_filename(
        config_root,
        c_plugins.as_ptr(),
        c_dir_name.as_ptr(),
        ptr::null_mut::<c_char>(),
    );
    let res = CStr::from_ptr(path_ptr).to_string_lossy().into_owned();
    g_free(path_ptr as GPointer);
    res
}

pub unsafe fn build_config_file_path(plugin: *mut GeanyPlugin) -> String {
    let config_dir = build_config_dir(plugin);
    let c_config_dir = CString::new(config_dir).unwrap();
    let c_file_name = CString::new(CONFIG_FILE_NAME).unwrap();
    let path_ptr = g_build_filename(
        c_config_dir.as_ptr(),
        c_file_name.as_ptr(),
        ptr::null_mut::<c_char>(),
    );
    let res = CStr::from_ptr(path_ptr).to_string_lossy().into_owned();
    g_free(path_ptr as GPointer);
    res
}

pub fn make_default_preset() -> BackendPreset {
    BackendPreset {
        name: "Local Ollama".to_string(),
        backend_type: BackendType::Ollama,
        uri: DEFAULT_OLLAMA_URI.to_string(),
        model: String::new(),
        system_prompt: String::new(),
        api_key: String::new(),
        temperature: String::new(),
        include_language_hint: true,
        insert_mode: InsertMode::Cursor,
    }
}

pub fn preset_group_name(index: usize) -> String {
    format!("{}{}", CONFIG_GROUP_PRESET_PREFIX, index)
}

pub unsafe fn load_config(plugin: *mut GeanyPlugin) {
    let config_path = build_config_file_path(plugin);
    let c_path = CString::new(config_path).unwrap();

    let key_file = g_key_file_new();
    let mut error: *mut GError = ptr::null_mut();

    let loaded = g_key_file_load_from_file(
        key_file,
        c_path.as_ptr(),
        G_KEY_FILE_NONE,
        &mut error,
    );

    let mut loaded_presets = Vec::new();
    let mut active_idx = 0i32;
    let mut timeout_idx = DEFAULT_CURL_TIMEOUT_INDEX as i32;
    let mut max_tokens = 0i32;
    let mut thinking_log_enabled = 1i32;

    if loaded != 0 && error.is_null() {
        active_idx = key_file_get_integer_default(key_file, CONFIG_GROUP_SETTINGS, "active_preset", 0);
        let timeout_sec = key_file_get_integer_default(
            key_file,
            CONFIG_GROUP_SETTINGS,
            "curl_timeout",
            60,
        );
        timeout_idx = curl_timeout_index_for_seconds(timeout_sec as i64) as i32;
        max_tokens = key_file_get_integer_default(key_file, CONFIG_GROUP_SETTINGS, "max_tokens", 0).max(0);
        thinking_log_enabled = key_file_get_integer_default(
            key_file,
            CONFIG_GROUP_SETTINGS,
            "thinking_log_enabled",
            1,
        );

        let preset_count = key_file_get_integer_default(key_file, CONFIG_GROUP_SETTINGS, "preset_count", 0);
        for i in 0..preset_count {
            let group = preset_group_name(i as usize);
            let name = key_file_get_string_default(key_file, &group, "name", &format!("Preset {}", i + 1));
            let type_id = key_file_get_string_default(key_file, &group, "type", "ollama");
            let backend_type = BackendType::from_id(&type_id);
            let uri = key_file_get_string_default(key_file, &group, "uri", "");
            let model = key_file_get_string_default(key_file, &group, "model", "");
            let system_prompt = key_file_get_string_default(key_file, &group, "system_prompt", "");
            let api_key = key_file_get_string_default(key_file, &group, "api_key", "");
            let temperature = key_file_get_string_default(key_file, &group, "temperature", "");
            let include_language_hint = key_file_get_integer_default(
                key_file, &group, "include_language_hint", 1,
            ) != 0;
            let insert_mode = InsertMode::from_id(&key_file_get_string_default(
                key_file, &group, "insert_mode", "cursor",
            ));

            loaded_presets.push(BackendPreset {
                name,
                backend_type,
                uri,
                model,
                system_prompt,
                api_key,
                temperature,
                include_language_hint,
                insert_mode,
            });
        }
    } else if !error.is_null() {
        g_error_free(error);
    }

    if loaded_presets.is_empty() {
        loaded_presets.push(make_default_preset());
    }

    if active_idx < 0 || (active_idx as usize) >= loaded_presets.len() {
        active_idx = 0;
    }

    g_key_file_free(key_file);

    ACTIVE_PRESET_INDEX.store(active_idx, Ordering::SeqCst);
    CURL_TIMEOUT_INDEX.store(timeout_idx, Ordering::SeqCst);
    MAX_TOKENS.store(max_tokens, Ordering::SeqCst);
    THINKING_LOG_ENABLED.store((thinking_log_enabled != 0) as i32, Ordering::SeqCst);

    with_global_state(|state| {
        let active_preset = &loaded_presets[active_idx as usize];
        state.backend_type = active_preset.backend_type;
        state.upstream_uri = active_preset.uri.clone();
        state.model_name = active_preset.model.clone();
        state.system_prompt = active_preset.system_prompt.clone();
        state.api_key = active_preset.api_key.clone();
        state.temperature = active_preset.temperature.clone();
        state.include_language_hint = active_preset.include_language_hint;
        state.insert_mode = active_preset.insert_mode;
        state.presets = loaded_presets;
    });
}

pub unsafe fn save_config(plugin: *mut GeanyPlugin) {
    let config_dir = build_config_dir(plugin);
    let c_config_dir = CString::new(config_dir).unwrap();
    g_mkdir_with_parents(c_config_dir.as_ptr(), 0o700);

    let config_path = build_config_file_path(plugin);
    let c_path = CString::new(config_path).unwrap();

    let key_file = g_key_file_new();
    let c_settings = CString::new(CONFIG_GROUP_SETTINGS).unwrap();

    let active_idx = ACTIVE_PRESET_INDEX.load(Ordering::SeqCst);
    let timeout_idx = CURL_TIMEOUT_INDEX.load(Ordering::SeqCst) as usize;
    let timeout_sec = crate::backend::active_curl_timeout_seconds(timeout_idx);
    let max_tokens = MAX_TOKENS.load(Ordering::SeqCst).max(0);
    let thinking_log_enabled = THINKING_LOG_ENABLED.load(Ordering::SeqCst);

    g_key_file_set_integer(key_file, c_settings.as_ptr(), CString::new("active_preset").unwrap().as_ptr(), active_idx);
    g_key_file_set_integer(key_file, c_settings.as_ptr(), CString::new("curl_timeout").unwrap().as_ptr(), timeout_sec as i32);
    g_key_file_set_integer(key_file, c_settings.as_ptr(), CString::new("max_tokens").unwrap().as_ptr(), max_tokens);
    g_key_file_set_integer(
        key_file,
        c_settings.as_ptr(),
        CString::new("thinking_log_enabled").unwrap().as_ptr(),
        (thinking_log_enabled != 0) as i32,
    );

    with_global_state(|state| {
        g_key_file_set_integer(
            key_file,
            c_settings.as_ptr(),
            CString::new("preset_count").unwrap().as_ptr(),
            state.presets.len() as i32,
        );

        for (i, preset) in state.presets.iter().enumerate() {
            let group = preset_group_name(i);
            let c_group = CString::new(group).unwrap();

            g_key_file_set_string(key_file, c_group.as_ptr(), CString::new("name").unwrap().as_ptr(), CString::new(preset.name.as_str()).unwrap().as_ptr());
            g_key_file_set_string(key_file, c_group.as_ptr(), CString::new("type").unwrap().as_ptr(), CString::new(preset.backend_type.id()).unwrap().as_ptr());
            g_key_file_set_string(key_file, c_group.as_ptr(), CString::new("uri").unwrap().as_ptr(), CString::new(preset.uri.as_str()).unwrap().as_ptr());
            g_key_file_set_string(key_file, c_group.as_ptr(), CString::new("model").unwrap().as_ptr(), CString::new(preset.model.as_str()).unwrap().as_ptr());
            g_key_file_set_string(key_file, c_group.as_ptr(), CString::new("system_prompt").unwrap().as_ptr(), CString::new(preset.system_prompt.as_str()).unwrap().as_ptr());
            g_key_file_set_string(key_file, c_group.as_ptr(), CString::new("api_key").unwrap().as_ptr(), CString::new(preset.api_key.as_str()).unwrap().as_ptr());
            g_key_file_set_string(key_file, c_group.as_ptr(), CString::new("temperature").unwrap().as_ptr(), CString::new(preset.temperature.as_str()).unwrap().as_ptr());
            g_key_file_set_integer(key_file, c_group.as_ptr(), CString::new("include_language_hint").unwrap().as_ptr(), preset.include_language_hint as i32);
            g_key_file_set_string(key_file, c_group.as_ptr(), CString::new("insert_mode").unwrap().as_ptr(), CString::new(preset.insert_mode.id()).unwrap().as_ptr());
        }
    });

    let mut error: *mut GError = ptr::null_mut();
    g_key_file_save_to_file(key_file, c_path.as_ptr(), &mut error);
    if !error.is_null() {
        g_error_free(error);
    }
    g_key_file_free(key_file);
}
