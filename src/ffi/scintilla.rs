use crate::ffi::glib::{GBoolean, GChar};
use std::os::raw::{c_uint, c_void};

pub type ScintillaObject = c_void;

extern "C" {
    pub fn scintilla_send_message(
        sci: *mut ScintillaObject,
        message: c_uint,
        w_param: usize,
        l_param: isize,
    ) -> isize;
    pub fn sci_has_selection(sci: *mut ScintillaObject) -> GBoolean;
    pub fn sci_get_current_position(sci: *mut ScintillaObject) -> i32;
    pub fn sci_get_length(sci: *mut ScintillaObject) -> i32;
    pub fn sci_get_selection_contents(sci: *mut ScintillaObject) -> *mut GChar;
    pub fn sci_get_contents_range(sci: *mut ScintillaObject, start: i32, end: i32) -> *mut GChar;
    pub fn sci_get_selection_start(sci: *mut ScintillaObject) -> i32;
    pub fn sci_get_selection_end(sci: *mut ScintillaObject) -> i32;
    pub fn sci_set_selection_start(sci: *mut ScintillaObject, position: i32);
    pub fn sci_set_selection_end(sci: *mut ScintillaObject, position: i32);
    pub fn sci_replace_sel(sci: *mut ScintillaObject, text: *const GChar);
    pub fn sci_insert_text(sci: *mut ScintillaObject, pos: i32, text: *const GChar);
}
