//! What the backend knows about its connections and the server (the port
//! of `X11Info.cs`).

use crate::x11_atoms::X11Atoms;
use crate::x11_structs::CursorFontShape;
use crate::xlib::{self, Time, VisualInfo, XDisplay, XFontSet, XID, XIM};
use std::cell::Cell;

/// A version of an extension (`System.Version` with two components).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: i32,
    pub minor: i32,
}

impl Version {
    pub fn new(major: i32, minor: i32) -> Self {
        Self { major, minor }
    }
}

/// The connections, the default screen and the extensions of the server.
pub struct X11Info {
    display: XDisplay,
    deferred_display: XDisplay,
    default_screen: i32,
    black_pixel: u64,
    root_window: XID,
    default_root_window: XID,
    default_cursor: XID,
    atoms: X11Atoms,
    xim: XIM,

    randr_event_base: i32,
    randr_error_base: i32,

    randr_version: Option<Version>,

    x_input_opcode: i32,
    x_input_event_base: i32,
    x_input_error_base: i32,

    x_input_version: Option<Version>,

    last_activity_timestamp: Cell<Time>,
    transparent_visual_info: Option<VisualInfo>,
    has_xim: bool,
    has_x_sync: bool,
    has_x_shm: bool,

    has_x_fixes: bool,

    default_font_set: XFontSet,

    has_xkb: bool,
}

impl X11Info {
    pub fn new(display: XDisplay, deferred_display: XDisplay, use_xim: bool) -> Self {
        let default_screen = xlib::x_default_screen(display);
        let black_pixel = xlib::x_black_pixel(display, default_screen) as u64;
        let root_window = xlib::x_root_window(display, default_screen);
        let default_cursor = xlib::x_create_font_cursor(display, CursorFontShape::XC_left_ptr as u32);
        let default_root_window = xlib::x_default_root_window(display);
        let atoms = X11Atoms::new(display);

        let default_font_set = xlib::x_create_font_set(display, "-*-*-*-*-*-*-*-*-*-*-*-*-*-*");

        // We have problems with text input otherwise
        xlib::set_locale("");

        let mut xim: XIM = std::ptr::null_mut();
        let mut has_xim = false;
        if use_xim {
            xlib::x_set_locale_modifiers("");
            xim = xlib::x_open_im(display);
            if !xim.is_null() {
                has_xim = true;
            }
        }

        if xim.is_null() {
            if !xlib::x_set_locale_modifiers("@im=none") {
                xlib::set_locale("en_US.UTF-8");
                if !xlib::x_set_locale_modifiers("@im=none") {
                    xlib::set_locale("C.UTF-8");
                    xlib::x_set_locale_modifiers("@im=none");
                }
            }
            xim = xlib::x_open_im(display);
        }

        // TrueColor is class 4.
        let transparent_visual_info =
            xlib::x_match_visual_info(display, default_screen, 32, 4).filter(|visual| visual.depth == 32);

        // An extension whose library cannot be loaded is an extension
        // that is not supported: the wrappers answer `None` for it, where
        // the reference catches the failure of the call.
        let mut randr_event_base = 0;
        let mut randr_error_base = 0;
        let mut randr_version = None;
        if let Some((event_base, error_base)) = xlib::xrr_query_extension(display) {
            randr_event_base = event_base;
            randr_error_base = error_base;
            if let Some((major, minor)) = xlib::xrr_query_version(display) {
                randr_version = Some(Version::new(major, minor));
            }
        }

        let mut x_input_opcode = 0;
        let mut x_input_event_base = 0;
        let mut x_input_error_base = 0;
        let mut x_input_version = None;
        if let Some((xiopcode, xievent, xierror)) = xlib::x_query_extension(display, "XInputExtension") {
            if let Some((major, minor)) = xlib::xi_query_version(display) {
                x_input_version = Some(Version::new(major, minor));
                x_input_opcode = xiopcode;
                x_input_event_base = xievent;
                x_input_error_base = xierror;
            }
        }

        // As the reference: the extension is taken to be there when the
        // call does not answer zero (the library answers a non-zero
        // status for success).
        let has_x_sync = xlib::x_sync_initialize(display).is_some_and(|status| status != 0);

        // XShm rendering happens on the deferred display, so probe the extension there.
        let has_x_shm = xlib::x_shm_query(deferred_display);

        // Input shapes need XFixes 2.0 or newer.
        let has_x_fixes = xlib::x_fixes_query_major_version(display).is_some_and(|major| major >= 2);

        let has_xkb = xlib::xkb_library_version() && xlib::xkb_query_extension(display);

        Self {
            display,
            deferred_display,
            default_screen,
            black_pixel,
            root_window,
            default_root_window,
            default_cursor,
            atoms,
            xim,
            randr_event_base,
            randr_error_base,
            randr_version,
            x_input_opcode,
            x_input_event_base,
            x_input_error_base,
            x_input_version,
            last_activity_timestamp: Cell::new(0),
            transparent_visual_info,
            has_xim,
            has_x_sync,
            has_x_shm,
            has_x_fixes,
            default_font_set,
            has_xkb,
        }
    }

    /// The connection of the UI thread.
    pub fn display(&self) -> XDisplay {
        self.display
    }

    /// The second connection, which the thread that renders draws through.
    pub fn deferred_display(&self) -> XDisplay {
        self.deferred_display
    }

    pub fn default_screen(&self) -> i32 {
        self.default_screen
    }

    pub fn black_pixel(&self) -> u64 {
        self.black_pixel
    }

    pub fn root_window(&self) -> XID {
        self.root_window
    }

    pub fn default_root_window(&self) -> XID {
        self.default_root_window
    }

    pub fn default_cursor(&self) -> XID {
        self.default_cursor
    }

    pub fn atoms(&self) -> &X11Atoms {
        &self.atoms
    }

    pub fn xim(&self) -> XIM {
        self.xim
    }

    pub fn randr_event_base(&self) -> i32 {
        self.randr_event_base
    }

    pub fn randr_error_base(&self) -> i32 {
        self.randr_error_base
    }

    pub fn randr_version(&self) -> Option<Version> {
        self.randr_version
    }

    pub fn x_input_opcode(&self) -> i32 {
        self.x_input_opcode
    }

    pub fn x_input_event_base(&self) -> i32 {
        self.x_input_event_base
    }

    pub fn x_input_error_base(&self) -> i32 {
        self.x_input_error_base
    }

    pub fn x_input_version(&self) -> Option<Version> {
        self.x_input_version
    }

    /// The time of the last input event, which requests to the window
    /// manager carry.
    pub fn last_activity_timestamp(&self) -> Time {
        self.last_activity_timestamp.get()
    }

    pub fn set_last_activity_timestamp(&self, value: Time) {
        self.last_activity_timestamp.set(value);
    }

    /// The visual of depth 32 of the default screen, when it has one.
    pub fn transparent_visual_info(&self) -> Option<VisualInfo> {
        self.transparent_visual_info
    }

    pub fn has_xim(&self) -> bool {
        self.has_xim
    }

    pub fn has_x_sync(&self) -> bool {
        self.has_x_sync
    }

    pub fn has_x_shm(&self) -> bool {
        self.has_x_shm
    }

    pub fn has_x_fixes(&self) -> bool {
        self.has_x_fixes
    }

    pub fn default_font_set(&self) -> XFontSet {
        self.default_font_set
    }

    pub fn has_xkb(&self) -> bool {
        self.has_xkb
    }
}
