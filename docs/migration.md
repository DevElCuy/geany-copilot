# Migration Plan: try4 (C++) → try5 (Rust FFI)

## Overview & Architecture Decisions

This project ports the `try4` Geany Copilot C++ plugin to Rust (`try5`) while preserving full feature parity and user configuration compatibility.

### Key Technology Choices
- **Geany / GTK3 / GLib API**: Hand-crafted FFI declarations (`extern "C"`) in modular `src/ffi/` submodules. (Avoids unmaintained, conflicting `gtk3-sys` crates).
- **HTTP & Streaming**: `curl` crate wrapping host `libcurl`, maintaining identical server-sent event (SSE) streaming semantics as C++.
- **JSON Processing**: `serde` / `serde_json` for safe, zero-C-overhead payload generation and parsing.
- **Config Storage**: `GKeyFile` FFI bindings to maintain exact compatibility with `~/.config/geany/plugins/geany-copilot/geany-copilot.conf`.
- **Threading Model**: `std::thread` worker thread communicating with the GTK main loop via `g_idle_add` and atomic control flags (`std::sync::atomic`).

---

## File Structure

```
src/
├── lib.rs              # Entry point: geany_load_module, init, cleanup
├── ffi/
│   ├── mod.rs          # FFI exports
│   ├── geany.rs        # Geany plugin & document API
│   ├── gtk.rs          # GTK3 widgets, dialogs, signals
│   ├── glib.rs         # GLib memory, idle callbacks, GKeyFile, paths
│   └── scintilla.rs    # Scintilla editor message constants & FFI
├── backend.rs          # BackendType enum, BackendPreset, URL & payload generators
├── config.rs           # GKeyFile config loader/saver & preset initialization
├── request.rs          # RequestData state, HTTP streaming worker thread, progress dialog
├── ui.rs               # Main window toolbar button & status bar combo boxes
└── configure.rs        # Geany Preferences / Configure dialog logic
```

---

## Execution Stages

| Stage | Description | Status |
|-------|-------------|--------|
| **Stage 1** | FFI foundation (`geany_load_module`, simple hello world) | ✅ Complete |
| **Stage 2** | Project structure, dependencies (`Cargo.toml`), FFI modules (`src/ffi/*`) | ✅ Complete |
| **Stage 3** | Backend model (`backend.rs`) & Config manager (`config.rs`) | ✅ Complete |
| **Stage 4** | Request lifecycle & streaming worker (`request.rs`) | ✅ Complete |
| **Stage 5** | UI integrations (`ui.rs`, `configure.rs`) | ✅ Complete |
| **Stage 6** | Main plugin wiring (`lib.rs`) & end-to-end verification | ✅ Complete |

The Rust FFI declarations mirror the Geany 2.0 headers installed on the target system,
including the `geany_load_module` registration API. Request workers own their response
state and send owned results to the GTK thread, so GTK widgets and request strings are
never accessed concurrently. Configuration remains compatible with try4 at:
`~/.config/geany/plugins/geany-copilot/geany-copilot.conf` (or Geany's configured
`configdir`). Each preset may also store an optional `system_prompt`; the settings
page opens it in a dedicated multiline editor, and an empty value is omitted from
backend requests.

## Installation note

The only supported installation location is
`~/.config/geany/plugins/copilot_plugin.so`, matching try4's filename in the user
plugin directory that Geany 2.0 scans.
Do not keep additional copies under `~/.local/lib/geany` or the system plugin
directory: Geany records active plugins by absolute path, so duplicate modules can
cause it to load an older build. Restart Geany after installing so the process cannot
retain a previously loaded shared object.
