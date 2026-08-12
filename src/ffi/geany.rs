use crate::ffi::glib::{GBoolean, GChar, GInt, GPointer, GUint};
use crate::ffi::gtk::{GtkDialog, GtkWidget};
use crate::ffi::scintilla::ScintillaObject;
use std::os::raw::c_uint;

pub const GEANY_API_VERSION: GInt = 247;
pub const GEANY_ABI_SHIFT: GInt = 8;
pub const GEANY_ABI_VERSION: GInt = 73 << GEANY_ABI_SHIFT;

#[repr(C)]
pub struct PluginInfo {
    pub name: *const GChar,
    pub description: *const GChar,
    pub version: *const GChar,
    pub author: *const GChar,
}

#[repr(C)]
pub struct PluginCallback {
    pub signal_name: *const GChar,
    pub callback: Option<unsafe extern "C" fn()>,
    pub after: GBoolean,
    pub user_data: GPointer,
}

#[repr(C)]
pub struct GeanyPluginFuncs {
    pub callbacks: *mut PluginCallback,
    pub init: Option<unsafe extern "C" fn(*mut GeanyPlugin, GPointer) -> GBoolean>,
    pub configure: Option<unsafe extern "C" fn(*mut GeanyPlugin, *mut GtkDialog, GPointer) -> *mut GtkWidget>,
    pub help: Option<unsafe extern "C" fn(*mut GeanyPlugin, GPointer)>,
    pub cleanup: Option<unsafe extern "C" fn(*mut GeanyPlugin, GPointer)>,
}

#[repr(C)]
pub struct GeanyApp {
    pub debug_mode: GBoolean,
    pub configdir: *mut GChar,
    pub datadir: *mut GChar,
    pub docdir: *mut GChar,
    pub tm_workspace: GPointer,
    pub project: GPointer,
}

#[repr(C)]
pub struct GeanyMainWidgets {
    pub window: *mut GtkWidget,
    pub toolbar: *mut GtkWidget,
    pub sidebar_notebook: *mut GtkWidget,
    pub notebook: *mut GtkWidget,
    pub editor_menu: *mut GtkWidget,
    pub tools_menu: *mut GtkWidget,
    pub progressbar: *mut GtkWidget,
    pub message_window_notebook: *mut GtkWidget,
    pub project_menu: *mut GtkWidget,
}

#[repr(C)]
pub struct GeanyData {
    pub app: *mut GeanyApp,
    pub main_widgets: *mut GeanyMainWidgets,
    pub documents_array: GPointer,
    pub filetypes_array: GPointer,
    pub prefs: GPointer,
    pub interface_prefs: GPointer,
    pub toolbar_prefs: GPointer,
    pub editor_prefs: GPointer,
    pub file_prefs: GPointer,
    pub search_prefs: GPointer,
    pub tool_prefs: GPointer,
    pub template_prefs: GPointer,
    pub compat: *mut GPointer,
    pub filetypes_by_title: GPointer,
    pub object: GPointer,
}

#[repr(C)]
pub struct GeanyProxyFuncs {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct GeanyPluginPrivate {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct GeanyPlugin {
    pub info: *mut PluginInfo,
    pub geany_data: *mut GeanyData,
    pub funcs: *mut GeanyPluginFuncs,
    pub proxy_funcs: *mut GeanyProxyFuncs,
    pub priv_: *mut GeanyPluginPrivate,
}

#[repr(C)]
pub struct GeanyEditor {
    pub document: *mut GeanyDocument,
    pub sci: *mut ScintillaObject,
    pub line_wrapping: GBoolean,
    pub auto_indent: GBoolean,
    pub scroll_percent: f32,
    pub indent_type: c_uint,
    pub line_breaking: GBoolean,
    pub indent_width: i32,
}

#[repr(C)]
pub struct GeanyDocument {
    pub is_valid: GBoolean,
    pub index: i32,
    pub has_tags: GBoolean,
    pub file_name: *mut GChar,
    pub encoding: *mut GChar,
    pub has_bom: GBoolean,
    pub editor: *mut GeanyEditor,
    pub file_type: GPointer,
    pub tm_file: GPointer,
    pub readonly: GBoolean,
    pub changed: GBoolean,
    pub real_path: *mut GChar,
    pub id: GUint,
    pub priv_: GPointer,
}

pub type GeanyKeyGroupFunc = Option<
    unsafe extern "C" fn(
        key_group: *mut GeanyKeyGroup,
        key_id: GUint,
        user_data: GPointer,
    ) -> GBoolean,
>;

pub type GeanyKeyCallback = Option<unsafe extern "C" fn(key_id: GUint)>;

#[repr(C)]
pub struct GeanyKeyGroup {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct GeanyKeyBinding {
    _opaque: [u8; 0],
}

extern "C" {
    pub fn geany_plugin_register(
        plugin: *mut GeanyPlugin,
        api_version: GInt,
        min_api_version: GInt,
        abi_version: GInt,
    ) -> i32;

    pub fn plugin_module_make_resident(plugin: *mut GeanyPlugin);

    pub fn document_get_current() -> *mut GeanyDocument;
    pub fn document_find_by_id(id: GUint) -> *mut GeanyDocument;

    pub fn plugin_set_key_group_full(
        plugin: *mut GeanyPlugin,
        section_name: *const GChar,
        count: usize,
        cb: GeanyKeyGroupFunc,
        pdata: GPointer,
        destroy_data: Option<unsafe extern "C" fn(GPointer)>,
    ) -> *mut GeanyKeyGroup;

    pub fn keybindings_set_item(
        key_group: *mut GeanyKeyGroup,
        key_id: usize,
        callback: GeanyKeyCallback,
        key: c_uint,
        mods: c_uint,
        name: *const GChar,
        label: *const GChar,
        menu_item: *mut GtkWidget,
    ) -> *mut GeanyKeyBinding;

    pub fn ui_lookup_widget(widget: *mut GtkWidget, widget_name: *const GChar) -> *mut GtkWidget;
}

#[cfg(all(test, target_pointer_width = "64"))]
mod layout_tests {
    use super::*;
    use std::mem::{offset_of, size_of};

    #[test]
    fn geany_2_struct_layouts_match_installed_x86_64_headers() {
        assert_eq!(size_of::<GeanyApp>(), 48);
        assert_eq!(offset_of!(GeanyApp, configdir), 8);

        assert_eq!(size_of::<GeanyMainWidgets>(), 72);
        assert_eq!(offset_of!(GeanyMainWidgets, toolbar), 8);

        assert_eq!(size_of::<GeanyData>(), 120);
        assert_eq!(offset_of!(GeanyData, main_widgets), 8);
        assert_eq!(offset_of!(GeanyData, template_prefs), 88);
        assert_eq!(offset_of!(GeanyData, object), 112);

        assert_eq!(size_of::<GeanyPlugin>(), 40);
        assert_eq!(size_of::<GeanyPluginFuncs>(), 40);

        assert_eq!(size_of::<GeanyEditor>(), 40);
        assert_eq!(offset_of!(GeanyEditor, sci), 8);

        assert_eq!(size_of::<GeanyDocument>(), 96);
        assert_eq!(offset_of!(GeanyDocument, editor), 40);
        assert_eq!(offset_of!(GeanyDocument, id), 80);
        assert_eq!(offset_of!(GeanyDocument, priv_), 88);
    }
}
