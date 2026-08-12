## try5: Real Geany Plugin (Rust FFI)

### Binary paths
```
cargo: /usr/bin/cargo-1.91
rustc: /usr/bin/rustc-1.91
```

### System info
- **Geany version:** 2.0 (GTK 3.24.33)
- **API version:** 247
- **ABI version:** 73 << 8 (18688)
- **Plugin directory (system):** `/usr/lib/x86_64-linux-gnu/geany/`
- **Plugin directory (user):** `~/.config/geany/plugins/`
- **Headers:** `/usr/include/geany/`

### geany-rs crate: NOT AVAILABLE on crates.io
`geany-rs` is not on crates.io or GitHub → cannot use procedural macros.
This plugin uses manual Rust FFI bindings to the Geany C plugin API instead.

### Plugin API: Proxy registration API
The plugin exports `geany_load_module(*GeanyPlugin)`. It fills in `PluginInfo` and
`GeanyPluginFuncs`, then registers against Geany API 247 and ABI 18688 with
`geany_plugin_register`.

### Build
```bash
/usr/bin/cargo-1.91 build --release
```

### Install
```bash
# User-local (no sudo):
mkdir -p ~/.config/geany/plugins
install -m 755 target/release/libcopilot_plugin.so ~/.config/geany/plugins/copilot_plugin.so

# System-wide (requires sudo):
sudo install -m 755 target/release/libcopilot_plugin.so /usr/lib/x86_64-linux-gnu/geany/copilot_plugin.so
```

### Enable
1. Restart Geany (or reload plugins)
2. **Tools → Plugin Manager**
3. Check **"Geany Copilot"** in the list
4. An **"Ask Copilot"** button appears in the toolbar
5. Open a document and click it to send the surrounding text to the configured backend
