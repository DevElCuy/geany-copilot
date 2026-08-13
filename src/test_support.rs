//! Test-only helpers: a fake Geany plugin whose config dir points at a
//! caller-chosen path, so config I/O in tests never touches the real one.

use crate::ffi::geany::{GeanyApp, GeanyData, GeanyMainWidgets, GeanyPlugin};
use std::ffi::CString;
use std::path::{Path, PathBuf};

pub struct FakeGeany {
    _configdir: CString,
    _app: Box<GeanyApp>,
    _main_widgets: Box<GeanyMainWidgets>,
    _data: Box<GeanyData>,
    plugin: Box<GeanyPlugin>,
}

impl FakeGeany {
    pub fn ptr(&mut self) -> *mut GeanyPlugin {
        &mut *self.plugin
    }
}

pub fn fake_plugin(configdir: &Path) -> FakeGeany {
    let configdir = CString::new(configdir.to_str().unwrap()).unwrap();
    unsafe {
        // All-zero bit patterns are valid for these structs: every field is an
        // integer or a pointer, and the code under test null-checks pointers.
        let mut app: Box<GeanyApp> = Box::new(std::mem::MaybeUninit::zeroed().assume_init());
        app.configdir = configdir.as_ptr() as *mut _;
        let main_widgets: Box<GeanyMainWidgets> =
            Box::new(std::mem::MaybeUninit::zeroed().assume_init());
        let mut data: Box<GeanyData> = Box::new(std::mem::MaybeUninit::zeroed().assume_init());
        data.app = &mut *app;
        let mut plugin: Box<GeanyPlugin> = Box::new(std::mem::MaybeUninit::zeroed().assume_init());
        plugin.geany_data = &mut *data;
        FakeGeany {
            _configdir: configdir,
            _app: app,
            _main_widgets: main_widgets,
            _data: data,
            plugin,
        }
    }
}

/// Same fake, but with a (widget-less) `main_widgets` table wired in, for
/// code paths that dereference it.
pub fn fake_plugin_with_main_widgets(configdir: &Path) -> FakeGeany {
    let mut fake = fake_plugin(configdir);
    unsafe {
        (*fake.plugin.geany_data).main_widgets = &mut *fake._main_widgets;
    }
    fake
}

/// A per-process scratch directory under the system temp dir.
pub fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "geany-copilot-test-{}-{}",
        tag,
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&dir);
    dir
}
