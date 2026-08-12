# Geany Copilot

A native [Geany](https://www.geany.org/) plugin that sends the surrounding
text of your document to an LLM backend and inserts the response back into
the editor — a lightweight, editor-native AI assistant.

This is `v0.2`, a full rewrite of the plugin in Rust using hand-written FFI
bindings to the Geany/GTK3/GLib C APIs (no external `geany-rs` crate exists,
so the bindings in `src/ffi/` are maintained directly against the Geany 2.0
headers). It replaces the earlier Lua-script-based version of the project.

## Features

- Toolbar button ("Ask Copilot") and status bar backend/preset selectors
- Streaming responses (SSE) via `curl`, rendered incrementally into the buffer
- Two backend types:
  - **Ollama** (local models)
  - **OpenAI-compatible** APIs (OpenAI, DeepSeek, and other compatible providers)
- Multiple named presets, each with its own backend, model, URL, API key, and
  optional system prompt
- Config stored in `GKeyFile` format, compatible with prior plugin versions,
  at `~/.config/geany/plugins/geany-copilot/geany-copilot.conf`

## Building

Requires the Geany 2.0 development headers and `libcurl`.

```bash
cargo build --release
```

or, using the provided Makefile:

```bash
make build
```

## Installing

```bash
make install
```

This builds the plugin and installs it to
`~/.config/geany/plugins/copilot_plugin.so`. This is the only supported
install location — avoid keeping additional copies elsewhere, since Geany
tracks active plugins by absolute path and a stray duplicate can cause it to
load a stale build.

To remove it:

```bash
make uninstall
```

## Enabling in Geany

1. Restart Geany (or reload plugins)
2. **Tools → Plugin Manager**
3. Check **"Geany Copilot"** in the list
4. An **"Ask Copilot"** button appears in the toolbar
5. Open a document and click it to send the surrounding text to the
   configured backend

Configure backends, models, and API keys via **Tools → Preferences →
Geany Copilot**, or the plugin's own preset editor.

## Project layout

```
src/
├── lib.rs          # Entry point: geany_load_module, init, cleanup
├── ffi/
│   ├── mod.rs       # FFI exports
│   ├── geany.rs     # Geany plugin & document API
│   ├── gtk.rs        # GTK3 widgets, dialogs, signals
│   ├── glib.rs       # GLib memory, idle callbacks, GKeyFile, paths
│   └── scintilla.rs  # Scintilla editor message constants & FFI
├── backend.rs       # BackendType enum, BackendPreset, URL & payload generators
├── config.rs        # GKeyFile config loader/saver & preset initialization
├── request.rs       # Request lifecycle & streaming worker thread
├── ui.rs            # Toolbar button & status bar combo boxes
└── configure.rs      # Preferences / Configure dialog logic
```

See `docs/migration.md` for background on the rewrite from the earlier
C++/Lua versions.

## License

MIT — see [LICENSE](LICENSE).
