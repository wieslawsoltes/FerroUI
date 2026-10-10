//! The calls into Xlib and its extension libraries (the port of `XLib.cs` and
//! `XLib.Helpers.cs`).
//!
//! The reference declares every function it calls as a platform invoke of a
//! library that is loaded when the function is first called. Here the
//! libraries are opened at run time through `x11-dl` (nothing is linked, so
//! the crate builds on a machine without the X libraries), and this module
//! is the one place that calls into them: every other module of the crate
//! calls the safe functions below.
//!
//! # Safety of the wrappers
//!
//! The wrappers that take only a [`XDisplay`] and plain values (identifiers,
//! numbers) are safe to call for these reasons, which the comment
//! `// SAFETY: plain call` refers to:
//!
//! - a [`XDisplay`] is only made by [`x_open_display`] from a connection
//!   that Xlib opened, and the backend never closes a connection, so the
//!   pointer is valid for the life of the process;
//! - [`x_init_threads`] is called before the first connection is opened
//!   (the platform does it), so Xlib serialises the calls of several
//!   threads on one connection itself;
//! - an identifier (window, atom, cursor, ...) that does not name an object
//!   of the server is answered by the server with an error event, which the
//!   error handler of the backend records: it is never dereferenced on this
//!   side.
//!
//! Wrappers that pass memory state their own argument.

#![allow(clippy::too_many_arguments)]

use std::ffi::{c_char, c_int, c_long, c_uchar, c_uint, c_ulong, c_void, CStr, CString};
use std::ptr;
use std::sync::OnceLock;
use x11_dl::xlib as xl;

pub use x11_dl::xinput2 as xi2;
pub use x11_dl::xlib::{
    Visual, XClientMessageEvent, XColor, XConfigureEvent, XErrorEvent, XEvent, XGenericEventCookie, XImage, XKeyEvent,
    XPoint, XRectangle, XSetWindowAttributes, XSizeHints, XVisualInfo, XWMHints, XWindowAttributes, XWindowChanges,
    XIC, XIM,
};
pub use x11_dl::xrandr as xrr;
pub use x11_dl::sync::XSyncValue;

/// An identifier of a server object (`XID`): a window, a pixmap, a cursor.
pub type XID = c_ulong;
/// An atom of the server.
pub type Atom = c_ulong;
/// A server timestamp in milliseconds.
pub type Time = c_ulong;
/// A graphics context of Xlib.
pub type GC = xl::GC;
/// A font set of Xlib.
pub type XFontSet = xl::XFontSet;

/// `AnyPropertyType`.
pub const ANY_PROPERTY_TYPE: Atom = 0;
/// `None`: the null identifier.
pub const NONE: XID = 0;

/// A connection to the X server (`Display*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct XDisplay(*mut xl::Display);

// SAFETY: the connection is used from the UI thread and from the thread
// that renders (the reference does the same with its two connections).
// Xlib locks a connection around every call once `XInitThreads` was
// called, which the platform does before it opens one.
unsafe impl Send for XDisplay {}
// SAFETY: see `Send`.
unsafe impl Sync for XDisplay {}

impl XDisplay {
    /// The connection as the pointer other libraries take (EGL, GLX,
    /// Vulkan).
    pub fn as_ptr(self) -> *mut c_void {
        self.0.cast()
    }

    fn raw(self) -> *mut xl::Display {
        self.0
    }
}

/// The libraries of the X Window System the backend calls.
pub struct Libraries {
    /// `libX11`: the core protocol, Xkb, the input method functions.
    pub xlib: xl::Xlib,
    /// `libXi`: the X Input extension, version 2.
    pub xinput2: Option<x11_dl::xinput2::XInput2>,
    /// `libXrandr`: outputs and monitors.
    pub xrandr: Option<x11_dl::xrandr::Xrandr>,
    /// `libXcursor`: themed and image cursors.
    pub xcursor: Option<x11_dl::xcursor::Xcursor>,
    /// `libXfixes`: regions and input shapes.
    pub xfixes: Option<x11_dl::xfixes::Xlib>,
    /// `libXext`: the synchronisation extension.
    pub xsync: Option<x11_dl::sync::Xext>,
    /// `libXext`: the shared memory extension.
    pub xshm: Option<x11_dl::xshm::Xext>,
}

static LIBRARIES: OnceLock<Result<Libraries, String>> = OnceLock::new();

/// Opens the libraries, once. Fails with the message of the loader when
/// `libX11` cannot be opened; the extension libraries are optional, as in
/// the reference, which treats a library that is not found as an extension
/// that is not supported.
pub fn try_libraries() -> Result<&'static Libraries, &'static str> {
    LIBRARIES
        .get_or_init(|| {
            let xlib = xl::Xlib::open().map_err(|e| format!("Unable to load libX11: {e}"))?;
            Ok(Libraries {
                xlib,
                xinput2: x11_dl::xinput2::XInput2::open().ok(),
                xrandr: x11_dl::xrandr::Xrandr::open().ok(),
                xcursor: x11_dl::xcursor::Xcursor::open().ok(),
                xfixes: x11_dl::xfixes::Xlib::open().ok(),
                xsync: x11_dl::sync::Xext::open().ok(),
                xshm: x11_dl::xshm::Xext::open().ok(),
            })
        })
        .as_ref()
        .map_err(String::as_str)
}

/// The libraries.
///
/// # Panics
/// Panics when `libX11` cannot be loaded (the reference fails with a
/// library-not-found exception at its first call).
pub fn libraries() -> &'static Libraries {
    match try_libraries() {
        Ok(libraries) => libraries,
        Err(message) => panic!("{message}"),
    }
}

fn x() -> &'static xl::Xlib {
    &libraries().xlib
}

fn c_string(text: &str) -> CString {
    // A name with an interior NUL ends there, as it does for the C string
    // the reference marshals.
    let end = text.find('\0').unwrap_or(text.len());
    CString::new(&text[..end]).expect("no interior NUL")
}

/// Generates a wrapper for a call that passes the connection and plain
/// values only.
macro_rules! plain {
    ($(#[$meta:meta])* $name:ident => $function:ident($($arg:ident: $ty:ty),*) -> $ret:ty) => {
        $(#[$meta])*
        pub fn $name(display: XDisplay $(, $arg: $ty)*) -> $ret {
            // SAFETY: plain call (see the module documentation).
            unsafe { (x().$function)(display.raw() $(, $arg)*) }
        }
    };
}

/// `XInitThreads`.
pub fn x_init_threads() -> c_int {
    // SAFETY: takes no argument; it has to run before any other Xlib call,
    // which the platform ensures by calling it first.
    unsafe { (x().XInitThreads)() }
}

/// `XOpenDisplay(NULL)`: connects to the server named by `DISPLAY`.
pub fn x_open_display() -> Option<XDisplay> {
    // SAFETY: a null name selects the display of the environment.
    let display = unsafe { (x().XOpenDisplay)(ptr::null()) };
    (!display.is_null()).then_some(XDisplay(display))
}

plain!(
    /// `XConnectionNumber`: the file descriptor of the connection.
    x_connection_number => XConnectionNumber() -> c_int);
plain!(
    /// `XDefaultScreen`.
    x_default_screen => XDefaultScreen() -> c_int);
plain!(
    /// `XBlackPixel`.
    x_black_pixel => XBlackPixel(screen: c_int) -> c_ulong);
plain!(
    /// `XRootWindow`.
    x_root_window => XRootWindow(screen: c_int) -> XID);
plain!(
    /// `XDefaultRootWindow`.
    x_default_root_window => XDefaultRootWindow() -> XID);
plain!(
    /// `XCreateFontCursor`.
    x_create_font_cursor => XCreateFontCursor(shape: c_uint) -> XID);
plain!(
    /// `XFreeCursor`.
    x_free_cursor => XFreeCursor(cursor: XID) -> c_int);
plain!(
    /// `XDefineCursor`.
    x_define_cursor => XDefineCursor(window: XID, cursor: XID) -> c_int);
plain!(
    /// `XCreateSimpleWindow`.
    x_create_simple_window => XCreateSimpleWindow(parent: XID, x: c_int, y: c_int, width: c_uint, height: c_uint,
        border_width: c_uint, border: c_ulong, background: c_ulong) -> XID);
plain!(
    /// `XDestroyWindow`.
    x_destroy_window => XDestroyWindow(window: XID) -> c_int);
plain!(
    /// `XMapWindow`.
    x_map_window => XMapWindow(window: XID) -> c_int);
plain!(
    /// `XUnmapWindow`.
    x_unmap_window => XUnmapWindow(window: XID) -> c_int);
plain!(
    /// `XRaiseWindow`.
    x_raise_window => XRaiseWindow(window: XID) -> c_int);
plain!(
    /// `XResizeWindow`.
    x_resize_window => XResizeWindow(window: XID, width: c_uint, height: c_uint) -> c_int);
plain!(
    /// `XIconifyWindow`.
    x_iconify_window => XIconifyWindow(window: XID, screen: c_int) -> c_int);
plain!(
    /// `XSelectInput`.
    x_select_input => XSelectInput(window: XID, mask: c_long) -> c_int);
plain!(
    /// `XDeleteProperty`.
    x_delete_property => XDeleteProperty(window: XID, property: Atom) -> c_int);
plain!(
    /// `XSetTransientForHint`.
    x_set_transient_for_hint => XSetTransientForHint(window: XID, parent: XID) -> c_int);
plain!(
    /// `XSetInputFocus`.
    x_set_input_focus => XSetInputFocus(focus: XID, revert_to: c_int, time: Time) -> c_int);
plain!(
    /// `XUngrabPointer`.
    x_ungrab_pointer => XUngrabPointer(time: Time) -> c_int);
plain!(
    /// `XGetSelectionOwner`.
    x_get_selection_owner => XGetSelectionOwner(selection: Atom) -> XID);
plain!(
    /// `XSetSelectionOwner`.
    x_set_selection_owner => XSetSelectionOwner(selection: Atom, owner: XID, time: Time) -> c_int);
plain!(
    /// `XConvertSelection`.
    x_convert_selection => XConvertSelection(selection: Atom, target: Atom, property: Atom, requestor: XID,
        time: Time) -> c_int);
plain!(
    /// `XFlush`.
    x_flush => XFlush() -> c_int);
plain!(
    /// `XPending`: the number of events that were received and not yet
    /// taken from the queue.
    x_pending => XPending() -> c_int);
plain!(
    /// `XLockDisplay`.
    x_lock_display => XLockDisplay() -> ());
plain!(
    /// `XUnlockDisplay`.
    x_unlock_display => XUnlockDisplay() -> ());
plain!(
    /// `XMaxRequestSize`, in units of four bytes.
    x_max_request_size => XMaxRequestSize() -> c_long);
plain!(
    /// `XExtendedMaxRequestSize`, in units of four bytes; zero without the
    /// big requests extension.
    x_extended_max_request_size => XExtendedMaxRequestSize() -> c_long);
plain!(
    /// `XCreateColormap` with a visual that came from the server
    /// (`XMatchVisualInfo`, a GLX or EGL configuration).
    x_create_colormap_raw => XCreateColormap(window: XID, visual: *mut Visual, alloc: c_int) -> XID);

/// `XSync`.
pub fn x_sync(display: XDisplay, discard: bool) -> c_int {
    // SAFETY: plain call.
    unsafe { (x().XSync)(display.raw(), c_int::from(discard)) }
}

/// A visual of the server with its depth: what `XMatchVisualInfo` and the
/// configurations of GLX and EGL give.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisualInfo {
    /// The visual. It belongs to the connection and lives as long as it.
    pub visual: *mut Visual,
    /// The identifier of the visual.
    pub visual_id: c_ulong,
    /// The depth in bits.
    pub depth: c_int,
}

/// `XMatchVisualInfo`: the visual of the screen with the depth and class,
/// when there is one.
pub fn x_match_visual_info(display: XDisplay, screen: c_int, depth: c_int, class: c_int) -> Option<VisualInfo> {
    // SAFETY: `info` is a valid place for the result, which Xlib fills
    // only when it returns non-zero.
    let mut info: XVisualInfo = unsafe { std::mem::zeroed() };
    let found = unsafe { (x().XMatchVisualInfo)(display.raw(), screen, depth, class, &mut info) };
    (found != 0).then_some(VisualInfo { visual: info.visual, visual_id: info.visualid, depth: info.depth })
}

/// `XCreateColormap` for a visual of the connection.
pub fn x_create_colormap(display: XDisplay, window: XID, visual: &VisualInfo, alloc: c_int) -> XID {
    x_create_colormap_raw(display, window, visual.visual, alloc)
}

/// `XCreateWindow`. `visual` is null for `CopyFromParent` or a visual of
/// the connection.
pub fn x_create_window(
    display: XDisplay,
    parent: XID,
    x_: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
    border_width: c_int,
    depth: c_int,
    class: c_int,
    visual: *mut Visual,
    value_mask: c_ulong,
    attributes: &mut XSetWindowAttributes,
) -> XID {
    // SAFETY: `attributes` is a valid structure for the call; the visual
    // is null or one the server gave for this connection.
    unsafe {
        (x().XCreateWindow)(
            display.raw(),
            parent,
            x_,
            y,
            width as c_uint,
            height as c_uint,
            border_width as c_uint,
            depth,
            class as c_uint,
            visual,
            value_mask,
            attributes,
        )
    }
}

/// `XInternAtom`.
pub fn x_intern_atom(display: XDisplay, name: &str, only_if_exists: bool) -> Atom {
    let name = c_string(name);
    // SAFETY: `name` is a NUL-terminated string that outlives the call.
    unsafe { (x().XInternAtom)(display.raw(), name.as_ptr(), c_int::from(only_if_exists)) }
}

/// `XInternAtoms`: the atoms of `names`, in one round trip.
pub fn x_intern_atoms(display: XDisplay, names: &[&str], only_if_exists: bool) -> Vec<Atom> {
    let owned: Vec<CString> = names.iter().map(|name| c_string(name)).collect();
    let mut pointers: Vec<*mut c_char> = owned.iter().map(|name| name.as_ptr().cast_mut()).collect();
    let mut atoms = vec![0 as Atom; names.len()];
    // SAFETY: `pointers` holds `names.len()` NUL-terminated strings that
    // Xlib only reads, and `atoms` has room for as many results.
    unsafe {
        (x().XInternAtoms)(
            display.raw(),
            pointers.as_mut_ptr(),
            names.len() as c_int,
            c_int::from(only_if_exists),
            atoms.as_mut_ptr(),
        );
    }
    atoms
}

/// `XGetAtomName` as a string (`XLib.GetAtomName`).
pub fn get_atom_name(display: XDisplay, atom: Atom) -> Option<String> {
    // SAFETY: the result is null or a NUL-terminated string that Xlib
    // allocated; it is copied and freed here.
    unsafe {
        let name = (x().XGetAtomName)(display.raw(), atom);
        if name.is_null() {
            return None;
        }
        let text = CStr::from_ptr(name).to_string_lossy().into_owned();
        (x().XFree)(name.cast());
        Some(text)
    }
}

/// How `XChangeProperty` combines the data with the property.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum PropertyMode {
    Replace = 0,
    Prepend = 1,
    Append = 2,
}

/// `XChangeProperty` with data of format 8.
pub fn x_change_property_bytes(
    display: XDisplay,
    window: XID,
    property: Atom,
    type_: Atom,
    mode: PropertyMode,
    data: &[u8],
) -> c_int {
    // SAFETY: `data` is `data.len()` bytes that Xlib reads.
    unsafe {
        (x().XChangeProperty)(display.raw(), window, property, type_, 8, mode as c_int, data.as_ptr(), data.len() as c_int)
    }
}

/// `XChangeProperty` with data of format 32. Xlib takes items of format 32
/// as an array of C `long`, whatever the size of a `long` is.
pub fn x_change_property_longs(
    display: XDisplay,
    window: XID,
    property: Atom,
    type_: Atom,
    mode: PropertyMode,
    data: &[c_ulong],
) -> c_int {
    // SAFETY: `data` is `data.len()` items of the size Xlib expects for
    // format 32.
    unsafe {
        (x().XChangeProperty)(
            display.raw(),
            window,
            property,
            type_,
            32,
            mode as c_int,
            data.as_ptr().cast(),
            data.len() as c_int,
        )
    }
}

/// The value of a window property as `XGetWindowProperty` returns it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WindowProperty {
    /// The status the call returned (`Success` is zero).
    pub status: c_int,
    /// The type of the property, or zero when it does not exist.
    pub actual_type: Atom,
    /// 8, 16 or 32; zero when the property does not exist.
    pub actual_format: c_int,
    /// The number of items in `data`.
    pub nitems: c_ulong,
    /// The bytes of the property that were not returned.
    pub bytes_after: c_ulong,
    /// The items as Xlib returns them: bytes for format 8, C `short` for
    /// format 16 and C `long` for format 32, in the byte order of the
    /// machine.
    pub data: Vec<u8>,
}

impl WindowProperty {
    /// The items of a property of format 32.
    pub fn longs(&self) -> Vec<c_ulong> {
        decode_longs(&self.data)
    }
}

/// The items of the data of a property of format 32, as Xlib returns them
/// (an array of C `long`).
pub fn decode_longs(data: &[u8]) -> Vec<c_ulong> {
    data.chunks_exact(std::mem::size_of::<c_ulong>())
        .map(|chunk| c_ulong::from_ne_bytes(chunk.try_into().expect("a chunk of the size of a long")))
        .collect()
}

/// `XGetWindowProperty`. `long_offset` and `long_length` are in units of
/// four bytes, as in the protocol.
pub fn x_get_window_property(
    display: XDisplay,
    window: XID,
    property: Atom,
    long_offset: c_long,
    long_length: c_long,
    delete: bool,
    req_type: Atom,
) -> WindowProperty {
    let mut result = WindowProperty::default();
    let mut prop: *mut c_uchar = ptr::null_mut();
    // SAFETY: every out pointer is a valid place for its result; the data
    // Xlib allocates is copied (by the size of an item of its format) and
    // freed before returning.
    unsafe {
        result.status = (x().XGetWindowProperty)(
            display.raw(),
            window,
            property,
            long_offset,
            long_length,
            c_int::from(delete),
            req_type,
            &mut result.actual_type,
            &mut result.actual_format,
            &mut result.nitems,
            &mut result.bytes_after,
            &mut prop,
        );
        if !prop.is_null() {
            if result.status == 0 {
                let item_size = match result.actual_format {
                    8 => 1,
                    16 => std::mem::size_of::<std::ffi::c_short>(),
                    32 => std::mem::size_of::<c_long>(),
                    _ => 0,
                };
                let length = item_size * result.nitems as usize;
                result.data = std::slice::from_raw_parts(prop, length).to_vec();
            }
            (x().XFree)(prop.cast());
        }
    }
    result
}

/// The items of a property of format 32 and the type `req_type`
/// (`XLib.XGetWindowPropertyAsIntPtrArray`): `None` when the property does
/// not exist, has another type or format, or is empty.
pub fn x_get_window_property_as_int_ptr_array(
    display: XDisplay,
    window: XID,
    atom: Atom,
    req_type: Atom,
) -> Option<Vec<c_ulong>> {
    let property = x_get_window_property(display, window, atom, 0, 0x7fffffff, false, req_type);
    if property.status != 0 {
        return None;
    }
    if property.actual_type != req_type || property.actual_format != 32 || property.nitems == 0 {
        return None;
    }
    Some(property.longs())
}

/// The one item of a property of format 32 and the type `req_type`
/// (`XLib.XGetWindowPropertyAsIntPtr`).
pub fn x_get_window_property_as_int_ptr(display: XDisplay, window: XID, atom: Atom, req_type: Atom) -> Option<c_ulong> {
    let property = x_get_window_property(display, window, atom, 0, 1, false, req_type);
    if property.status != 0 {
        return None;
    }
    if property.actual_type != req_type || property.actual_format != 32 || property.nitems != 1 {
        return None;
    }
    property.longs().first().copied()
}

/// The geometry of a drawable (`XGetGeometry`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct XGeometry {
    pub root: XID,
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
    pub border_width: c_int,
    pub depth: c_int,
}

/// `XGetGeometry`: `None` when the call fails (the drawable is gone).
pub fn x_get_geometry(display: XDisplay, drawable: XID) -> Option<XGeometry> {
    let mut root: XID = 0;
    let (mut x_, mut y, mut width, mut height, mut border_width, mut depth) = (0, 0, 0u32, 0u32, 0u32, 0u32);
    // SAFETY: every out pointer is a valid place for its result.
    let status = unsafe {
        (x().XGetGeometry)(
            display.raw(),
            drawable,
            &mut root,
            &mut x_,
            &mut y,
            &mut width,
            &mut height,
            &mut border_width,
            &mut depth,
        )
    };
    (status != 0).then_some(XGeometry {
        root,
        x: x_,
        y,
        width: width as c_int,
        height: height as c_int,
        border_width: border_width as c_int,
        depth: depth as c_int,
    })
}

/// `XTranslateCoordinates`: the point in the coordinates of `dest` and the
/// child of `dest` that contains it; `None` when the windows are on
/// different screens.
pub fn x_translate_coordinates(
    display: XDisplay,
    src: XID,
    dest: XID,
    src_x: c_int,
    src_y: c_int,
) -> Option<(c_int, c_int, XID)> {
    let (mut dest_x, mut dest_y, mut child) = (0, 0, 0);
    // SAFETY: every out pointer is a valid place for its result.
    let same_screen =
        unsafe { (x().XTranslateCoordinates)(display.raw(), src, dest, src_x, src_y, &mut dest_x, &mut dest_y, &mut child) };
    (same_screen != 0).then_some((dest_x, dest_y, child))
}

/// The answer of `XQueryPointer`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct XPointerInfo {
    pub same_screen: bool,
    pub root: XID,
    pub child: XID,
    pub root_x: c_int,
    pub root_y: c_int,
    pub win_x: c_int,
    pub win_y: c_int,
    pub mask: c_uint,
}

/// `XQueryPointer`.
pub fn x_query_pointer(display: XDisplay, window: XID) -> XPointerInfo {
    let mut info = XPointerInfo::default();
    // SAFETY: every out pointer is a valid place for its result.
    let same_screen = unsafe {
        (x().XQueryPointer)(
            display.raw(),
            window,
            &mut info.root,
            &mut info.child,
            &mut info.root_x,
            &mut info.root_y,
            &mut info.win_x,
            &mut info.win_y,
            &mut info.mask,
        )
    };
    info.same_screen = same_screen != 0;
    info
}

/// The parent and the children of a window, bottom to top (`XQueryTree`).
pub fn x_query_tree(display: XDisplay, window: XID) -> Option<(XID, XID, Vec<XID>)> {
    let (mut root, mut parent) = (0, 0);
    let mut children: *mut XID = ptr::null_mut();
    let mut count: c_uint = 0;
    // SAFETY: every out pointer is a valid place for its result; the list
    // Xlib allocates is copied and freed.
    unsafe {
        if (x().XQueryTree)(display.raw(), window, &mut root, &mut parent, &mut children, &mut count) == 0 {
            return None;
        }
        let list = if children.is_null() {
            Vec::new()
        } else {
            let list = std::slice::from_raw_parts(children, count as usize).to_vec();
            (x().XFree)(children.cast());
            list
        };
        Some((root, parent, list))
    }
}

/// `XConfigureWindow`.
pub fn x_configure_window(display: XDisplay, window: XID, value_mask: c_uint, changes: &mut XWindowChanges) -> c_int {
    // SAFETY: `changes` is a valid structure for the call.
    unsafe { (x().XConfigureWindow)(display.raw(), window, value_mask, changes) }
}

/// Resizes a window with `XConfigureWindow` (`XLib.XConfigureResizeWindow`).
pub fn x_configure_resize_window(display: XDisplay, window: XID, width: c_int, height: c_int) -> c_int {
    // SAFETY: an all-zero `XWindowChanges` is a valid value.
    let mut changes: XWindowChanges = unsafe { std::mem::zeroed() };
    changes.width = width;
    changes.height = height;
    x_configure_window(display, window, (xl::CWWidth | xl::CWHeight) as c_uint, &mut changes)
}

/// `XSetWMProtocols`.
pub fn x_set_wm_protocols(display: XDisplay, window: XID, protocols: &[Atom]) -> c_int {
    let mut protocols = protocols.to_vec();
    // SAFETY: `protocols` holds as many atoms as the count says; Xlib
    // reads them.
    unsafe { (x().XSetWMProtocols)(display.raw(), window, protocols.as_mut_ptr(), protocols.len() as c_int) }
}

/// `XSetWMNormalHints`.
pub fn x_set_wm_normal_hints(display: XDisplay, window: XID, hints: &mut XSizeHints) {
    // SAFETY: `hints` is a valid structure for the call.
    unsafe { (x().XSetWMNormalHints)(display.raw(), window, hints) }
}

/// The size hints of a window (`XGetWMNormalHints`), when it has them.
pub fn x_get_wm_normal_hints(display: XDisplay, window: XID) -> Option<XSizeHints> {
    // SAFETY: an all-zero `XSizeHints` is a valid value, and both out
    // pointers are valid places for the results.
    unsafe {
        let mut hints: XSizeHints = std::mem::zeroed();
        let mut supplied: c_long = 0;
        ((x().XGetWMNormalHints)(display.raw(), window, &mut hints, &mut supplied) != 0).then_some(hints)
    }
}

/// The window manager hints of a window (`XGetWMHints`), when it has them.
pub fn x_get_wm_hints(display: XDisplay, window: XID) -> Option<XWMHints> {
    // SAFETY: the result is null or a structure Xlib allocated; it is
    // copied and freed here.
    unsafe {
        let hints = (x().XGetWMHints)(display.raw(), window);
        if hints.is_null() {
            return None;
        }
        let value = *hints;
        (x().XFree)(hints.cast());
        Some(value)
    }
}

/// `XSetWMHints`.
pub fn x_set_wm_hints(display: XDisplay, window: XID, hints: &mut XWMHints) -> c_int {
    // SAFETY: `hints` is a valid structure for the call.
    unsafe { (x().XSetWMHints)(display.raw(), window, hints) }
}

/// Sets `WM_CLASS` (`XAllocClassHint`, `XSetClassHint`).
pub fn x_set_class_hint(display: XDisplay, window: XID, res_name: &[u8], res_class: &[u8]) {
    let name = CString::new(res_name.iter().copied().take_while(|b| *b != 0).collect::<Vec<u8>>()).expect("no NUL");
    let class = CString::new(res_class.iter().copied().take_while(|b| *b != 0).collect::<Vec<u8>>()).expect("no NUL");
    // SAFETY: the hint is allocated by Xlib, points at two NUL-terminated
    // strings that outlive the call (Xlib copies them into the property)
    // and is freed here.
    unsafe {
        let hint = (x().XAllocClassHint)();
        if hint.is_null() {
            return;
        }
        (*hint).res_name = name.as_ptr().cast_mut();
        (*hint).res_class = class.as_ptr().cast_mut();
        (x().XSetClassHint)(display.raw(), window, hint);
        (x().XFree)(hint.cast());
    }
}

/// `XStoreName`.
pub fn x_store_name(display: XDisplay, window: XID, name: &str) -> c_int {
    let name = c_string(name);
    // SAFETY: `name` is a NUL-terminated string that outlives the call.
    unsafe { (x().XStoreName)(display.raw(), window, name.as_ptr()) }
}

/// The name of a window (`XFetchName`).
pub fn x_fetch_name(display: XDisplay, window: XID) -> Option<String> {
    let mut name: *mut c_char = ptr::null_mut();
    // SAFETY: `name` receives null or a string Xlib allocated, which is
    // copied and freed here.
    unsafe {
        if (x().XFetchName)(display.raw(), window, &mut name) == 0 || name.is_null() {
            return None;
        }
        let text = CStr::from_ptr(name).to_string_lossy().into_owned();
        (x().XFree)(name.cast());
        Some(text)
    }
}

/// The attributes of a window (`XGetWindowAttributes`).
pub fn x_get_window_attributes(display: XDisplay, window: XID) -> Option<XWindowAttributes> {
    // SAFETY: an all-zero `XWindowAttributes` is a valid value and a valid
    // place for the result.
    unsafe {
        let mut attributes: XWindowAttributes = std::mem::zeroed();
        ((x().XGetWindowAttributes)(display.raw(), window, &mut attributes) != 0).then_some(attributes)
    }
}

/// `XSendEvent`.
pub fn x_send_event(display: XDisplay, window: XID, propagate: bool, event_mask: c_long, event: &mut XEvent) -> c_int {
    // SAFETY: `event` is a valid event that Xlib reads.
    unsafe { (x().XSendEvent)(display.raw(), window, c_int::from(propagate), event_mask, event) }
}

/// A zeroed event, to be filled in before it is sent.
pub fn new_event() -> XEvent {
    // SAFETY: an event is plain data; every member is valid when zero.
    unsafe { std::mem::zeroed() }
}

/// Zeroed window attributes, to be filled in before a window is created.
pub fn new_set_window_attributes() -> XSetWindowAttributes {
    // SAFETY: the structure is plain data; every member is valid when zero.
    unsafe { std::mem::zeroed() }
}

/// Zeroed size hints.
pub fn new_size_hints() -> XSizeHints {
    // SAFETY: the structure is plain data; every member is valid when zero.
    unsafe { std::mem::zeroed() }
}

/// Zeroed window manager hints.
pub fn new_wm_hints() -> XWMHints {
    // SAFETY: the structure is plain data; every member is valid when zero.
    unsafe { std::mem::zeroed() }
}

/// Zeroed window changes.
pub fn new_window_changes() -> XWindowChanges {
    // SAFETY: the structure is plain data; every member is valid when zero.
    unsafe { std::mem::zeroed() }
}

plain!(
    /// `XGrabServer`.
    x_grab_server => XGrabServer() -> c_int);
plain!(
    /// `XUngrabServer`.
    x_ungrab_server => XUngrabServer() -> c_int);

/// The pointer as seen from a window, with the deepest window under it as
/// the child (`XLib.QueryPointer`): the tree is walked with the server
/// grabbed, so that it does not change meanwhile.
pub fn query_pointer(display: XDisplay, w: XID) -> XPointerInfo {
    x_grab_server(display);

    let mut info = x_query_pointer(display, w);
    let mut c = info.child;

    if info.root != w {
        c = info.root;
    }

    let mut child_last: XID = 0;
    while c != 0 {
        child_last = c;
        info = x_query_pointer(display, c);
        c = info.child;
    }
    x_ungrab_server(display);
    x_flush(display);

    info.child = child_last;
    info
}

/// The position of the pointer (`XLib.GetCursorPos`): in the coordinates
/// of `handle`, or of the root window without one.
pub fn get_cursor_pos(display: XDisplay, root_window: XID, handle: Option<XID>) -> (c_int, c_int) {
    let info = query_pointer(display, handle.unwrap_or(root_window));

    if handle.is_some() {
        (info.win_x, info.win_y)
    } else {
        (info.root_x, info.root_y)
    }
}

/// The keyboard group of a core event state (`XkbGetGroupForCoreState`).
pub fn xkb_get_group_for_core_state(state: c_int) -> c_int {
    (state >> 13) & 0x3
}

/// A core event state with another keyboard group
/// (`XkbSetGroupForCoreState`).
pub fn xkb_set_group_for_core_state(state: c_int, new_group: c_int) -> c_int {
    (state & !(0x3 << 13)) | ((new_group & 0x3) << 13)
}

/// `XNextEvent`: takes the next event from the queue, waiting for one.
pub fn x_next_event(display: XDisplay) -> XEvent {
    let mut event = new_event();
    // SAFETY: `event` is a valid place for the result.
    unsafe { (x().XNextEvent)(display.raw(), &mut event) };
    event
}

/// `XFilterEvent(event, None)`: whether an input method consumed the
/// event.
pub fn x_filter_event(event: &mut XEvent) -> bool {
    // SAFETY: `event` is a valid event read from a connection.
    unsafe { (x().XFilterEvent)(event, 0) != 0 }
}

/// `XGetEventData`: fetches the data of a generic event. Returns whether
/// the cookie now has data, which [`x_free_event_data`] must release.
pub fn x_get_event_data(display: XDisplay, event: &mut XEvent) -> bool {
    // SAFETY: the caller checked that the event is a generic event, so the
    // cookie member is the active one.
    unsafe { (x().XGetEventData)(display.raw(), &mut event.generic_event_cookie) != 0 }
}

/// `XFreeEventData`.
pub fn x_free_event_data(display: XDisplay, event: &mut XEvent) {
    // SAFETY: as for `x_get_event_data`.
    unsafe { (x().XFreeEventData)(display.raw(), &mut event.generic_event_cookie) }
}

/// The type of an event.
pub fn event_type(event: &XEvent) -> c_int {
    // SAFETY: every member of the union starts with the type.
    unsafe { event.type_ }
}

/// The window of an event (`xany.window`).
pub fn event_window(event: &XEvent) -> XID {
    // SAFETY: every core event starts with the members of `XAnyEvent`.
    unsafe { event.any.window }
}

/// Generates the views of an event as one of its kinds.
///
/// An event is a C union of structures of integers and pointers, for which
/// every bit pattern is a valid value, and no view dereferences a pointer:
/// reading the event as any of its kinds is therefore sound whatever kind
/// it is (the values only mean something for the right kind, which the
/// caller selects by the type of the event).
macro_rules! event_views {
    ($($(#[$meta:meta])* $name:ident, $name_mut:ident => $member:ident: $ty:ty;)*) => {
        $(
            $(#[$meta])*
            pub fn $name(event: &XEvent) -> &$ty {
                // SAFETY: see `event_views`.
                unsafe { &event.$member }
            }

            $(#[$meta])*
            pub fn $name_mut(event: &mut XEvent) -> &mut $ty {
                // SAFETY: see `event_views`.
                unsafe { &mut event.$member }
            }
        )*
    };
}

event_views! {
    /// The event as a button event.
    button_event, button_event_mut => button: xl::XButtonEvent;
    /// The event as a motion event.
    motion_event, motion_event_mut => motion: xl::XMotionEvent;
    /// The event as an enter or leave event.
    crossing_event, crossing_event_mut => crossing: xl::XCrossingEvent;
    /// The event as a key event.
    key_event, key_event_mut => key: XKeyEvent;
    /// The event as a configure event.
    configure_event, configure_event_mut => configure: XConfigureEvent;
    /// The event as a property event.
    property_event, property_event_mut => property: xl::XPropertyEvent;
    /// The event as a client message.
    client_message_event, client_message_event_mut => client_message: XClientMessageEvent;
    /// The event as a focus event.
    focus_change_event, focus_change_event_mut => focus_change: xl::XFocusChangeEvent;
    /// The event as a visibility event.
    visibility_event, visibility_event_mut => visibility: xl::XVisibilityEvent;
    /// The event as a destroy event.
    destroy_window_event, destroy_window_event_mut => destroy_window: xl::XDestroyWindowEvent;
    /// The event as an unmap event.
    unmap_event, unmap_event_mut => unmap: xl::XUnmapEvent;
    /// The event as a selection notification.
    selection_event, selection_event_mut => selection: xl::XSelectionEvent;
    /// The event as a selection request.
    selection_request_event, selection_request_event_mut => selection_request: xl::XSelectionRequestEvent;
    /// The event as a selection clear event.
    selection_clear_event, selection_clear_event_mut => selection_clear: xl::XSelectionClearEvent;
    /// The event as the cookie of a generic event.
    generic_event_cookie, generic_event_cookie_mut => generic_event_cookie: XGenericEventCookie;
}

/// The handler Xlib calls for an error event of a connection.
pub type XErrorHandler = unsafe extern "C" fn(*mut xl::Display, *mut XErrorEvent) -> c_int;

/// `XSetErrorHandler`.
pub fn x_set_error_handler(handler: XErrorHandler) {
    // SAFETY: the handler is a function with the signature Xlib calls.
    unsafe {
        (x().XSetErrorHandler)(Some(handler));
    }
}

/// `XQueryExtension`: the major opcode, the first event and the first
/// error of an extension, when the server has it.
pub fn x_query_extension(display: XDisplay, name: &str) -> Option<(c_int, c_int, c_int)> {
    let name = c_string(name);
    let (mut opcode, mut event, mut error) = (0, 0, 0);
    // SAFETY: `name` is NUL-terminated and the out pointers are valid.
    let present = unsafe { (x().XQueryExtension)(display.raw(), name.as_ptr(), &mut opcode, &mut event, &mut error) };
    (present != 0).then_some((opcode, event, error))
}

/// `XCreateBitmapFromData`.
pub fn x_create_bitmap_from_data(display: XDisplay, drawable: XID, data: &[u8], width: c_uint, height: c_uint) -> XID {
    assert!(data.len() >= (width as usize).div_ceil(8) * height as usize, "the bitmap data is too short");
    // SAFETY: `data` holds the rows of a bitmap of the given size
    // (checked above), which Xlib reads.
    unsafe { (x().XCreateBitmapFromData)(display.raw(), drawable, data.as_ptr().cast(), width, height) }
}

/// `XCreatePixmapCursor` with black as both colours.
pub fn x_create_pixmap_cursor(display: XDisplay, source: XID, mask: XID, x_: c_uint, y: c_uint) -> XID {
    // SAFETY: an all-zero colour is valid; Xlib reads both.
    unsafe {
        let mut foreground: XColor = std::mem::zeroed();
        let mut background: XColor = std::mem::zeroed();
        (x().XCreatePixmapCursor)(display.raw(), source, mask, &mut foreground, &mut background, x_, y)
    }
}

/// Sends the pixels of a 32 bit image to a drawable with `XPutImage`
/// through a graphics context made for the call (`XCreateGC`, `XInitImage`,
/// `XPutImage`, `XFreeGC`).
///
/// # Safety
/// `data` must point at `bytes_per_line * height` readable bytes for the
/// duration of the call.
pub unsafe fn x_put_image_32(
    display: XDisplay,
    drawable: XID,
    depth: c_int,
    data: *mut u8,
    width: c_int,
    height: c_int,
    bytes_per_line: c_int,
) {
    let bits_per_pixel = 32;
    let mut image: XImage = std::mem::zeroed();
    image.width = width;
    image.height = height;
    image.format = xl::ZPixmap;
    image.data = data.cast();
    image.byte_order = xl::LSBFirst;
    image.bitmap_unit = bits_per_pixel;
    image.bitmap_bit_order = xl::LSBFirst;
    image.bitmap_pad = bits_per_pixel;
    image.depth = depth;
    image.bytes_per_line = bytes_per_line;
    image.bits_per_pixel = bits_per_pixel;
    (x().XInitImage)(&mut image);
    let gc = (x().XCreateGC)(display.raw(), drawable, 0, ptr::null_mut());
    (x().XPutImage)(display.raw(), drawable, gc, &mut image, 0, 0, 0, 0, width as c_uint, height as c_uint);
    (x().XFreeGC)(display.raw(), gc);
}

/// Reads one pixel of a drawable with `XGetImage` (`ZPixmap`, all planes):
/// the pixel value as the server stores it. `None` when the request fails
/// (the point is outside the drawable, or the window is not viewable).
pub fn x_get_pixel(display: XDisplay, drawable: XID, x_: c_int, y: c_int) -> Option<c_ulong> {
    // SAFETY: the image is null or allocated by Xlib with its accessor
    // functions set; it is read through them and destroyed here.
    unsafe {
        let image = (x().XGetImage)(display.raw(), drawable, x_, y, 1, 1, c_ulong::MAX, xl::ZPixmap);
        if image.is_null() {
            return None;
        }
        let pixel = (*image).funcs.get_pixel.map(|get_pixel| get_pixel(image, 0, 0));
        (x().XDestroyImage)(image);
        pixel
    }
}

/// `XSetLocaleModifiers`: whether the modifiers were accepted.
pub fn x_set_locale_modifiers(modifiers: &str) -> bool {
    let modifiers = c_string(modifiers);
    // SAFETY: `modifiers` is a NUL-terminated string that outlives the
    // call; the result is owned by Xlib and only tested for null.
    unsafe { !(x().XSetLocaleModifiers)(modifiers.as_ptr()).is_null() }
}

/// `XOpenIM(display, NULL, NULL, NULL)`: the input method of the locale
/// modifiers, or null.
pub fn x_open_im(display: XDisplay) -> XIM {
    // SAFETY: null selects the default resource database and names.
    unsafe { (x().XOpenIM)(display.raw(), ptr::null_mut(), ptr::null_mut(), ptr::null_mut()) }
}

/// `XCreateFontSet` for a base font name list; the list of missing
/// character sets is freed. Returns null when no font set could be made.
pub fn x_create_font_set(display: XDisplay, base_font_name_list: &str) -> XFontSet {
    let names = c_string(base_font_name_list);
    let mut missing: *mut *mut c_char = ptr::null_mut();
    let mut missing_count: c_int = 0;
    // SAFETY: the out pointers are valid; the default string out pointer
    // may be null. The list of missing character sets is freed with the
    // function Xlib provides for it.
    unsafe {
        let font_set =
            (x().XCreateFontSet)(display.raw(), names.as_ptr(), &mut missing, &mut missing_count, ptr::null_mut());
        if !missing.is_null() {
            (x().XFreeStringList)(missing);
        }
        font_set
    }
}

/// `XIMStyles` of Xlib: the list `XGetIMValues` returns for
/// `XNQueryInputStyle`.
#[repr(C)]
struct XIMStyles {
    count_styles: std::ffi::c_ushort,
    supported_styles: *mut c_ulong,
}

/// The input styles an input method supports (`XGetIMValues` with
/// `XNQueryInputStyle`).
pub fn x_get_im_supported_styles(xim: XIM) -> Vec<c_ulong> {
    if xim.is_null() {
        return Vec::new();
    }
    let mut styles: *mut XIMStyles = ptr::null_mut();
    // SAFETY: `xim` is an open input method; the argument list is the
    // name, the place for the result and the terminating null, as the
    // function documents. The result is copied and freed.
    unsafe {
        (x().XGetIMValues)(xim, xl::XNQueryInputStyle_0.as_ptr().cast::<c_char>(), &mut styles, ptr::null_mut::<c_void>());
        if styles.is_null() {
            return Vec::new();
        }
        let count = (*styles).count_styles as usize;
        let list = if (*styles).supported_styles.is_null() {
            Vec::new()
        } else {
            std::slice::from_raw_parts((*styles).supported_styles, count).to_vec()
        };
        (x().XFree)(styles.cast());
        list
    }
}

/// Creates an input context with the pre-edit position style: the spot
/// location and the font set as pre-edit attributes, the window as client
/// and focus window, and the resource name and class.
pub fn x_create_ic_preedit_position(
    xim: XIM,
    window: XID,
    style: c_ulong,
    resource_name: Option<&str>,
    font_set: XFontSet,
) -> XIC {
    let mut spot = XPoint { x: 0, y: 0 };
    let name = resource_name.map(c_string);
    let name_pointer = name.as_ref().map_or(ptr::null(), |name| name.as_ptr());
    // SAFETY: `xim` is an open input method. Both argument lists are
    // name/value pairs ended by null, with the value types the names
    // document (a pointer to a point, a font set, windows and the style
    // as `long`, strings); everything pointed at outlives the calls, and
    // the nested list is freed.
    unsafe {
        let list = (x().XVaCreateNestedList)(
            0,
            xl::XNSpotLocation_0.as_ptr().cast::<c_char>(),
            &mut spot as *mut XPoint,
            xl::XNFontSet_0.as_ptr().cast::<c_char>(),
            font_set,
            ptr::null_mut::<c_void>(),
        );
        let xic = (x().XCreateIC)(
            xim,
            xl::XNClientWindow_0.as_ptr().cast::<c_char>(),
            window,
            xl::XNFocusWindow_0.as_ptr().cast::<c_char>(),
            window,
            xl::XNInputStyle_0.as_ptr().cast::<c_char>(),
            style as c_long,
            xl::XNResourceName_0.as_ptr().cast::<c_char>(),
            name_pointer,
            xl::XNResourceClass_0.as_ptr().cast::<c_char>(),
            name_pointer,
            xl::XNPreeditAttributes_0.as_ptr().cast::<c_char>(),
            list,
            ptr::null_mut::<c_void>(),
        );
        (x().XFree)(list);
        xic
    }
}

/// Creates an input context with a style and the window as client and
/// focus window.
pub fn x_create_ic_simple(xim: XIM, window: XID, style: c_ulong) -> XIC {
    if xim.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: as for `x_create_ic_preedit_position`.
    unsafe {
        (x().XCreateIC)(
            xim,
            xl::XNInputStyle_0.as_ptr().cast::<c_char>(),
            style as c_long,
            xl::XNClientWindow_0.as_ptr().cast::<c_char>(),
            window,
            xl::XNFocusWindow_0.as_ptr().cast::<c_char>(),
            window,
            ptr::null_mut::<c_void>(),
        )
    }
}

/// Sets the spot location of the pre-edit of an input context
/// (`XSetICValues` with `XNPreeditAttributes`).
pub fn x_set_ic_spot_location(xic: XIC, x_: i16, y: i16) {
    let mut spot = XPoint { x: x_, y };
    // SAFETY: `xic` is an input context that was not destroyed; the lists
    // are name/value pairs ended by null and the point outlives the calls.
    unsafe {
        let list = (x().XVaCreateNestedList)(
            0,
            xl::XNSpotLocation_0.as_ptr().cast::<c_char>(),
            &mut spot as *mut XPoint,
            ptr::null_mut::<c_void>(),
        );
        (x().XSetICValues)(xic, xl::XNPreeditAttributes_0.as_ptr().cast::<c_char>(), list, ptr::null_mut::<c_void>());
        (x().XFree)(list);
    }
}

/// `XDestroyIC`.
pub fn x_destroy_ic(xic: XIC) {
    // SAFETY: the caller owns the context and forgets it after the call.
    unsafe { (x().XDestroyIC)(xic) }
}

/// `XSetICFocus`.
pub fn x_set_ic_focus(xic: XIC) {
    // SAFETY: `xic` is an input context that was not destroyed.
    unsafe { (x().XSetICFocus)(xic) }
}

/// `XUnsetICFocus`.
pub fn x_unset_ic_focus(xic: XIC) {
    // SAFETY: `xic` is an input context that was not destroyed.
    unsafe { (x().XUnsetICFocus)(xic) }
}

/// `XmbResetIC`; the pre-edit text it returns is freed.
pub fn xmb_reset_ic(xic: XIC) {
    // SAFETY: `xic` is an input context that was not destroyed; the
    // returned string is null or allocated by Xlib.
    unsafe {
        let data = (x().XmbResetIC)(xic);
        if !data.is_null() {
            (x().XFree)(data.cast());
        }
    }
}

/// `XkbLibraryVersion` for the version the backend was written against
/// (1.0).
pub fn xkb_library_version() -> bool {
    let (mut major, mut minor) = (1, 0);
    // SAFETY: both pointers are valid places for the versions.
    unsafe { (x().XkbLibraryVersion)(&mut major, &mut minor) != 0 }
}

/// `XkbQueryExtension` for version 1.0.
pub fn xkb_query_extension(display: XDisplay) -> bool {
    let (mut opcode, mut event, mut error, mut major, mut minor) = (0, 0, 0, 1, 0);
    // SAFETY: every pointer is a valid place for its result.
    unsafe { (x().XkbQueryExtension)(display.raw(), &mut opcode, &mut event, &mut error, &mut major, &mut minor) != 0 }
}

/// `XkbSetDetectableAutoRepeat`.
pub fn xkb_set_detectable_auto_repeat(display: XDisplay, detectable: bool) -> bool {
    let mut supported: c_int = 0;
    // SAFETY: `supported` is a valid place for the result.
    unsafe { (x().XkbSetDetectableAutoRepeat)(display.raw(), c_int::from(detectable), &mut supported) != 0 }
}

/// `XkbLookupKeySym`: the key symbol of a key code under a modifier
/// state, when the key has one.
pub fn xkb_lookup_key_sym(display: XDisplay, key_code: u8, state: c_uint) -> Option<c_ulong> {
    let mut modifiers: c_uint = 0;
    let mut key_sym: c_ulong = 0;
    // SAFETY: both out pointers are valid places for the results.
    let found = unsafe { (x().XkbLookupKeySym)(display.raw(), key_code, state, &mut modifiers, &mut key_sym) };
    (found != 0).then_some(key_sym)
}

/// `XkbTranslateKeySym`: the bytes a key symbol produces under no
/// modifiers.
pub fn xkb_translate_key_sym(display: XDisplay, key_sym: c_ulong) -> Vec<u8> {
    const BUFFER_SIZE: usize = 4;
    let mut sym = key_sym;
    let mut buffer = [0u8; BUFFER_SIZE];
    let mut extra_size: c_int = 0;
    // SAFETY: the buffer has the size that is passed; `sym` and
    // `extra_size` are valid places.
    let length = unsafe {
        (x().XkbTranslateKeySym)(
            display.raw(),
            &mut sym,
            0,
            buffer.as_mut_ptr().cast(),
            BUFFER_SIZE as c_int,
            &mut extra_size,
        )
    };
    if length <= 0 {
        return Vec::new();
    }
    if extra_size <= 0 {
        return buffer[..(length as usize).min(BUFFER_SIZE)].to_vec();
    }

    // A symbol should normally fit in 4 bytes, so this path isn't expected to be taken.
    let mut heap = vec![0u8; length as usize + extra_size as usize];
    let mut sym = key_sym;
    let mut ignored: c_int = 0;
    // SAFETY: as above, with the larger buffer.
    let length = unsafe {
        (x().XkbTranslateKeySym)(display.raw(), &mut sym, 0, heap.as_mut_ptr().cast(), heap.len() as c_int, &mut ignored)
    };
    heap.truncate(length.max(0) as usize);
    heap
}

/// `XLookupStatus`: what `Xutf8LookupString` reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct XLookupStatus(pub i32);

#[allow(non_upper_case_globals)]
impl XLookupStatus {
    pub const XBufferOverflow: XLookupStatus = XLookupStatus(-1);
    pub const XLookupNone: XLookupStatus = XLookupStatus(1);
    pub const XLookupChars: XLookupStatus = XLookupStatus(2);
    pub const XLookupKeySym: XLookupStatus = XLookupStatus(3);
    pub const XLookupBoth: XLookupStatus = XLookupStatus(4);
}

/// `XLookupString` without a compose status: the bytes and the key symbol
/// of a key event.
pub fn x_lookup_string(event: &mut XKeyEvent, buffer_size: usize) -> (Vec<u8>, c_ulong) {
    let mut buffer = vec![0u8; buffer_size];
    let mut key_sym: c_ulong = 0;
    // SAFETY: the buffer has the size that is passed; the event is a key
    // event read from a connection; the compose status may be null.
    let length = unsafe {
        (x().XLookupString)(event, buffer.as_mut_ptr().cast(), buffer_size as c_int, &mut key_sym, ptr::null_mut())
    };
    buffer.truncate((length.max(0) as usize).min(buffer_size));
    (buffer, key_sym)
}

/// `Xutf8LookupString`: the UTF-8 bytes an input context produces for a
/// key press, and the status of the lookup.
pub fn xutf8_lookup_string(xic: XIC, event: &mut XKeyEvent, buffer: &mut [u8]) -> (usize, c_int) {
    let mut key_sym: c_ulong = 0;
    let mut status: c_int = 0;
    // SAFETY: `xic` is an input context that was not destroyed; the
    // buffer has the size that is passed.
    let length = unsafe {
        (x().Xutf8LookupString)(
            xic,
            event,
            buffer.as_mut_ptr().cast(),
            buffer.len() as c_int,
            &mut key_sym,
            &mut status,
        )
    };
    ((length.max(0) as usize).min(buffer.len()), status)
}

/// `XKeysymToString`: the name of a key symbol.
pub fn x_keysym_to_string(key_sym: c_ulong) -> Option<Vec<u8>> {
    // SAFETY: the result is null or a static NUL-terminated string of
    // Xlib, which is copied.
    unsafe {
        let name = (x().XKeysymToString)(key_sym);
        if name.is_null() {
            return None;
        }
        Some(CStr::from_ptr(name).to_bytes().to_vec())
    }
}

// ---------------------------------------------------------------------------
// The C library.
// ---------------------------------------------------------------------------

/// `setlocale(LC_CTYPE as the reference passes it, locale)`.
pub fn set_locale(locale: &str) {
    let locale = c_string(locale);
    // SAFETY: `locale` is a NUL-terminated string; the category is the
    // value 0 the reference passes.
    unsafe {
        libc::setlocale(0, locale.as_ptr());
    }
}

/// `gethostname`: the bytes of the host name, without the terminator.
pub fn get_host_name() -> Option<Vec<u8>> {
    const MAX_LENGTH: usize = 1024;
    let mut name = [0u8; MAX_LENGTH];
    // SAFETY: the buffer has the length that is passed.
    if unsafe { libc::gethostname(name.as_mut_ptr().cast(), MAX_LENGTH) } != 0 {
        return None;
    }
    let length = name.iter().position(|b| *b == 0).unwrap_or(MAX_LENGTH);
    Some(name[..length].to_vec())
}

// ---------------------------------------------------------------------------
// Extensions.
// ---------------------------------------------------------------------------

/// `XRRQueryExtension`: the first event and the first error of RandR.
pub fn xrr_query_extension(display: XDisplay) -> Option<(c_int, c_int)> {
    let randr = libraries().xrandr.as_ref()?;
    let (mut event_base, mut error_base) = (0, 0);
    // SAFETY: both out pointers are valid places for the results.
    let present = unsafe { (randr.XRRQueryExtension)(display.raw(), &mut event_base, &mut error_base) };
    (present != 0).then_some((event_base, error_base))
}

/// `XRRQueryVersion`.
pub fn xrr_query_version(display: XDisplay) -> Option<(c_int, c_int)> {
    let randr = libraries().xrandr.as_ref()?;
    let (mut major, mut minor) = (0, 0);
    // SAFETY: both out pointers are valid places for the results.
    let status = unsafe { (randr.XRRQueryVersion)(display.raw(), &mut major, &mut minor) };
    (status != 0).then_some((major, minor))
}

/// `XRRSelectInput`.
pub fn xrr_select_input(display: XDisplay, window: XID, mask: c_int) {
    if let Some(randr) = libraries().xrandr.as_ref() {
        // SAFETY: plain call.
        unsafe { (randr.XRRSelectInput)(display.raw(), window, mask) }
    }
}

/// A monitor of RandR 1.5 (`XRRMonitorInfo`), copied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitorInfo {
    pub name: Atom,
    pub primary: bool,
    pub automatic: bool,
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
    pub mwidth: c_int,
    pub mheight: c_int,
    pub outputs: Vec<XID>,
}

/// `XRRGetMonitors(display, window, true)`: the active monitors.
pub fn xrr_get_monitors(display: XDisplay, window: XID) -> Vec<MonitorInfo> {
    let Some(randr) = libraries().xrandr.as_ref() else {
        return Vec::new();
    };
    let mut count: c_int = 0;
    // SAFETY: `count` is a valid place for the result; the list RandR
    // allocates (and the output lists inside it) is copied and freed with
    // the function RandR provides.
    unsafe {
        let monitors = (randr.XRRGetMonitors)(display.raw(), window, 1, &mut count);
        if monitors.is_null() {
            return Vec::new();
        }
        let list = std::slice::from_raw_parts(monitors, count.max(0) as usize)
            .iter()
            .map(|monitor| MonitorInfo {
                name: monitor.name,
                primary: monitor.primary != 0,
                automatic: monitor.automatic != 0,
                x: monitor.x,
                y: monitor.y,
                width: monitor.width,
                height: monitor.height,
                mwidth: monitor.mwidth,
                mheight: monitor.mheight,
                outputs: if monitor.outputs.is_null() {
                    Vec::new()
                } else {
                    std::slice::from_raw_parts(monitor.outputs, monitor.noutput.max(0) as usize).to_vec()
                },
            })
            .collect();
        (randr.XRRFreeMonitors)(monitors);
        list
    }
}

/// The value of an output property (`XRRGetOutputProperty` over the whole
/// property, not pending, any type), with its type and format.
pub fn xrr_get_output_property(display: XDisplay, output: XID, property: Atom) -> WindowProperty {
    let mut result = WindowProperty::default();
    let Some(randr) = libraries().xrandr.as_ref() else {
        result.status = 1;
        return result;
    };
    let mut prop: *mut c_uchar = ptr::null_mut();
    // SAFETY: every out pointer is a valid place for its result; the data
    // is copied by the size of an item of its format and freed.
    unsafe {
        result.status = (randr.XRRGetOutputProperty)(
            display.raw(),
            output,
            property,
            0,
            0x7fffffff,
            0,
            0,
            ANY_PROPERTY_TYPE,
            &mut result.actual_type,
            &mut result.actual_format,
            &mut result.nitems,
            &mut result.bytes_after,
            &mut prop,
        );
        if !prop.is_null() {
            if result.status == 0 {
                let item_size = match result.actual_format {
                    8 => 1,
                    16 => std::mem::size_of::<std::ffi::c_short>(),
                    32 => std::mem::size_of::<c_long>(),
                    _ => 0,
                };
                result.data = std::slice::from_raw_parts(prop, item_size * result.nitems as usize).to_vec();
            }
            (x().XFree)(prop.cast());
        }
    }
    result
}

/// The properties of an output (`XLib.XRRListOutputPropertiesAsArray`).
pub fn xrr_list_output_properties_as_array(display: XDisplay, output: XID) -> Vec<Atom> {
    let Some(randr) = libraries().xrandr.as_ref() else {
        return Vec::new();
    };
    let mut count: c_int = 0;
    // SAFETY: `count` is a valid place for the result; the list is copied
    // and freed.
    unsafe {
        let list = (randr.XRRListOutputProperties)(display.raw(), output, &mut count);
        if list.is_null() {
            return Vec::new();
        }
        let atoms = std::slice::from_raw_parts(list, count.max(0) as usize).to_vec();
        (x().XFree)(list.cast());
        atoms
    }
}

/// What the screen resources say about the mode of one output: the
/// vertical refresh rate of the mode its controller shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutputMode {
    pub output: XID,
    pub dot_clock: c_ulong,
    pub h_total: c_uint,
    pub v_total: c_uint,
    /// The flags of the mode (`XRRModeInfo.modeFlags`: interlace, double
    /// scan, ...).
    pub mode_flags: c_ulong,
}

/// For every output with a controller, the timing of the mode the
/// controller shows (`XRRGetScreenResources`, `XRRGetOutputInfo`,
/// `XRRGetCrtcInfo`, each freed).
pub fn xrr_get_output_modes(display: XDisplay, window: XID) -> Vec<OutputMode> {
    let Some(randr) = libraries().xrandr.as_ref() else {
        return Vec::new();
    };
    let mut result = Vec::new();
    // SAFETY: every structure RandR returns is checked for null, read
    // within the counts it states and freed with its own function.
    unsafe {
        let resources = (randr.XRRGetScreenResources)(display.raw(), window);
        if resources.is_null() {
            return result;
        }
        let outputs = if (*resources).outputs.is_null() {
            &[][..]
        } else {
            std::slice::from_raw_parts((*resources).outputs, (*resources).noutput.max(0) as usize)
        };
        let modes = if (*resources).modes.is_null() {
            &[][..]
        } else {
            std::slice::from_raw_parts((*resources).modes, (*resources).nmode.max(0) as usize)
        };
        for output in outputs {
            let info = (randr.XRRGetOutputInfo)(display.raw(), resources, *output);
            if info.is_null() {
                continue;
            }
            let crtc = (*info).crtc;
            (randr.XRRFreeOutputInfo)(info);
            if crtc == 0 {
                continue;
            }
            let crtc_info = (randr.XRRGetCrtcInfo)(display.raw(), resources, crtc);
            if crtc_info.is_null() {
                continue;
            }
            let mode_id = (*crtc_info).mode;
            (randr.XRRFreeCrtcInfo)(crtc_info);
            if let Some(mode) = modes.iter().find(|mode| mode.id == mode_id) {
                result.push(OutputMode {
                    output: *output,
                    dot_clock: mode.dotClock,
                    h_total: mode.hTotal,
                    v_total: mode.vTotal,
                    mode_flags: mode.modeFlags,
                });
            }
        }
        (randr.XRRFreeScreenResources)(resources);
    }
    result
}

/// `XIQueryVersion`: the version of the X Input extension the server and
/// the library agree on, asked for 2.2.
pub fn xi_query_version(display: XDisplay) -> Option<(c_int, c_int)> {
    let xi = libraries().xinput2.as_ref()?;
    let (mut major, mut minor) = (2, 2);
    // SAFETY: both pointers are valid places for the versions.
    let status = unsafe { (xi.XIQueryVersion)(display.raw(), &mut major, &mut minor) };
    (status == 0).then_some((major, minor))
}

/// A class of an input device, copied from `XIAnyClassInfo`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum XIClassInfo {
    /// `XIValuatorClassInfo`.
    Valuator(XIValuatorClassInfo),
    /// `XIScrollClassInfo`.
    Scroll(XIScrollClassInfo),
    /// A class the backend does not read.
    Other,
}

/// `XIValuatorClassInfo`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct XIValuatorClassInfo {
    pub source_id: c_int,
    pub number: c_int,
    pub label: Atom,
    pub min: f64,
    pub max: f64,
    pub value: f64,
    pub resolution: c_int,
    pub mode: c_int,
}

/// `XIScrollClassInfo`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct XIScrollClassInfo {
    pub source_id: c_int,
    pub number: c_int,
    pub scroll_type: c_int,
    pub increment: f64,
    pub flags: c_int,
}

/// An input device, copied from `XIDeviceInfo`.
#[derive(Clone, Debug, PartialEq)]
pub struct XIDeviceInfo {
    pub device_id: c_int,
    pub name: String,
    pub use_: c_int,
    pub attachment: c_int,
    pub enabled: bool,
    pub classes: Vec<XIClassInfo>,
}

/// Copies the classes of a device.
///
/// # Safety
/// `classes` must be null or point at `count` pointers to class
/// structures, as the X Input library gives them in a device information
/// and in a device changed event.
pub unsafe fn copy_xi_classes(classes: *mut *mut xi2::XIAnyClassInfo, count: c_int) -> Vec<XIClassInfo> {
    if classes.is_null() {
        return Vec::new();
    }
    std::slice::from_raw_parts(classes, count.max(0) as usize)
        .iter()
        .map(|class| {
            if class.is_null() {
                return XIClassInfo::Other;
            }
            match (**class)._type {
                xi2::XIValuatorClass => {
                    let valuator = &*class.cast::<xi2::XIValuatorClassInfo>();
                    XIClassInfo::Valuator(XIValuatorClassInfo {
                        source_id: valuator.sourceid,
                        number: valuator.number,
                        label: valuator.label,
                        min: valuator.min,
                        max: valuator.max,
                        value: valuator.value,
                        resolution: valuator.resolution,
                        mode: valuator.mode,
                    })
                }
                xi2::XIScrollClass => {
                    let scroll = &*class.cast::<xi2::XIScrollClassInfo>();
                    XIClassInfo::Scroll(XIScrollClassInfo {
                        source_id: scroll.sourceid,
                        number: scroll.number,
                        scroll_type: scroll.scroll_type,
                        increment: scroll.increment,
                        flags: scroll.flags,
                    })
                }
                _ => XIClassInfo::Other,
            }
        })
        .collect()
}

/// `XIQueryDevice`: the devices `device_id` selects, copied
/// (`XIFreeDeviceInfo`).
pub fn xi_query_device(display: XDisplay, device_id: c_int) -> Vec<XIDeviceInfo> {
    let Some(xi) = libraries().xinput2.as_ref() else {
        return Vec::new();
    };
    let mut count: c_int = 0;
    // SAFETY: `count` is a valid place for the result; the list is read
    // within the count it states, copied and freed with the function of
    // the library.
    unsafe {
        let devices = (xi.XIQueryDevice)(display.raw(), device_id, &mut count);
        if devices.is_null() {
            return Vec::new();
        }
        let list = std::slice::from_raw_parts(devices, count.max(0) as usize)
            .iter()
            .map(|device| XIDeviceInfo {
                device_id: device.deviceid,
                name: if device.name.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(device.name).to_string_lossy().into_owned()
                },
                use_: device._use,
                attachment: device.attachment,
                enabled: device.enabled != 0,
                classes: copy_xi_classes(device.classes, device.num_classes),
            })
            .collect();
        (xi.XIFreeDeviceInfo)(devices);
        list
    }
}

/// The mask bytes that select a set of X Input event types
/// (`XISetMask` over the highest type).
pub fn xi_event_mask_bytes(event_types: &[c_int]) -> Vec<u8> {
    let max = event_types.iter().copied().max().unwrap_or(0).max(0);
    let mut mask = vec![0u8; (max as usize >> 3) + 1];
    for event_type in event_types {
        if *event_type >= 0 {
            mask[*event_type as usize >> 3] |= 1 << (event_type & 7);
        }
    }
    mask
}

/// Selects X Input events on a window, per device (`XLib.XiSelectEvents`).
/// Returns the status of `XISelectEvents` (`Success` is zero).
pub fn xi_select_events(display: XDisplay, window: XID, devices: &[(c_int, Vec<c_int>)]) -> c_int {
    let Some(xi) = libraries().xinput2.as_ref() else {
        return 1;
    };
    let mut masks: Vec<Vec<u8>> = devices.iter().map(|(_, types)| xi_event_mask_bytes(types)).collect();
    let mut event_masks: Vec<xi2::XIEventMask> = devices
        .iter()
        .zip(masks.iter_mut())
        .map(|((device, _), mask)| xi2::XIEventMask {
            deviceid: *device,
            mask_len: mask.len() as c_int,
            mask: mask.as_mut_ptr(),
        })
        .collect();
    // SAFETY: every mask structure points at a buffer of the length it
    // states, which outlives the call.
    unsafe { (xi.XISelectEvents)(display.raw(), window, event_masks.as_mut_ptr(), event_masks.len() as c_int) }
}

/// Whether a bit of a mask of the X Input extension is set (`XIMaskIsSet`).
/// A bit beyond the mask is not set.
pub fn xi_mask_is_set(mask: &[u8], bit: i32) -> bool {
    if bit < 0 {
        return false;
    }
    mask.get(bit as usize >> 3).is_some_and(|byte| byte & (1 << (bit & 7)) != 0)
}

/// The valuators of a device event: for every bit that is set in the
/// mask, in ascending order, the next of the values.
pub fn xi_valuators(mask: &[u8], values: &[f64]) -> Vec<(i32, f64)> {
    let mut values = values.iter();
    let mut result = Vec::new();
    for c in 0..(mask.len() * 8) as i32 {
        if xi_mask_is_set(mask, c) {
            match values.next() {
                Some(value) => result.push((c, *value)),
                None => break,
            }
        }
    }
    result
}

/// A device event of the X Input extension (`XIDeviceEvent`), copied out
/// of the data of its cookie.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct XIDeviceEventData {
    pub evtype: c_int,
    pub time: Time,
    pub device_id: c_int,
    pub source_id: c_int,
    pub detail: c_int,
    pub root: XID,
    pub event_window: XID,
    pub child: XID,
    pub root_x: f64,
    pub root_y: f64,
    pub event_x: f64,
    pub event_y: f64,
    pub flags: c_int,
    /// The mask of the buttons that are down.
    pub buttons: Vec<u8>,
    /// The valuators the event carries, by number.
    pub valuators: Vec<(i32, f64)>,
    /// The effective modifiers.
    pub mods_effective: c_int,
}

/// An enter or leave event of the X Input extension (`XIEnterEvent`),
/// copied.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct XIEnterLeaveEventData {
    pub evtype: c_int,
    pub time: Time,
    pub detail: c_int,
    pub event_window: XID,
    pub event_x: f64,
    pub event_y: f64,
    pub buttons: Vec<u8>,
    pub mods_effective: c_int,
}

/// A device changed event of the X Input extension
/// (`XIDeviceChangedEvent`), copied.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct XIDeviceChangedEventData {
    pub device_id: c_int,
    pub source_id: c_int,
    pub reason: c_int,
    pub classes: Vec<XIClassInfo>,
}

/// An event of the X Input extension, copied out of a generic event.
#[derive(Clone, Debug, PartialEq)]
pub enum XIEventData {
    DeviceChanged(XIDeviceChangedEventData),
    Device(XIDeviceEventData),
    EnterLeave(XIEnterLeaveEventData),
    /// An event the backend does not read.
    Other(c_int),
}

/// Copies a mask of the X Input extension.
///
/// # Safety
/// `mask` must be null or point at `len` readable bytes.
unsafe fn copy_xi_mask(mask: *const c_uchar, len: c_int) -> Vec<u8> {
    if mask.is_null() || len <= 0 {
        return Vec::new();
    }
    std::slice::from_raw_parts(mask, len as usize).to_vec()
}

/// Copies a device event.
///
/// # Safety
/// The masks of the event must be null or point at as many bytes as their
/// lengths say, and the values at one value for every bit that is set in
/// the valuator mask, as the X Input library fills the event in.
pub unsafe fn copy_xi_device_event(ev: &xi2::XIDeviceEvent) -> XIDeviceEventData {
    let valuator_mask = copy_xi_mask(ev.valuators.mask, ev.valuators.mask_len);
    let value_count = valuator_mask.iter().map(|byte| byte.count_ones() as usize).sum();
    let values = if ev.valuators.values.is_null() || ev.valuators.mask.is_null() {
        &[][..]
    } else {
        std::slice::from_raw_parts(ev.valuators.values, value_count)
    };
    XIDeviceEventData {
        evtype: ev.evtype,
        time: ev.time,
        device_id: ev.deviceid,
        source_id: ev.sourceid,
        detail: ev.detail,
        root: ev.root,
        event_window: ev.event,
        child: ev.child,
        root_x: ev.root_x,
        root_y: ev.root_y,
        event_x: ev.event_x,
        event_y: ev.event_y,
        flags: ev.flags,
        buttons: copy_xi_mask(ev.buttons.mask, ev.buttons.mask_len),
        valuators: xi_valuators(&valuator_mask, values),
        mods_effective: ev.mods.effective,
    }
}

/// Copies the event of the X Input extension a generic event carries.
/// `None` when the cookie has no data.
pub fn copy_xi_event(cookie: &XGenericEventCookie) -> Option<XIEventData> {
    if cookie.data.is_null() {
        return None;
    }
    // SAFETY: the data of a cookie of the X Input extension that
    // `XGetEventData` filled in is the event structure of the type the
    // cookie names; it stays valid until `XFreeEventData`, which the
    // dispatcher calls after the event was handled. Each structure is
    // read as the type its event type selects, and copied.
    unsafe {
        Some(match cookie.evtype {
            xi2::XI_DeviceChanged => {
                let ev = &*cookie.data.cast::<xi2::XIDeviceChangedEvent>();
                XIEventData::DeviceChanged(XIDeviceChangedEventData {
                    device_id: ev.deviceid,
                    source_id: ev.sourceid,
                    reason: ev.reason,
                    classes: copy_xi_classes(ev.classes, ev.num_classes),
                })
            }
            xi2::XI_ButtonPress..=xi2::XI_Motion | xi2::XI_TouchBegin..=xi2::XI_TouchEnd => {
                XIEventData::Device(copy_xi_device_event(&*cookie.data.cast::<xi2::XIDeviceEvent>()))
            }
            xi2::XI_Enter | xi2::XI_Leave => {
                let ev = &*cookie.data.cast::<xi2::XIEnterEvent>();
                XIEventData::EnterLeave(XIEnterLeaveEventData {
                    evtype: ev.evtype,
                    time: ev.time,
                    detail: ev.detail,
                    event_window: ev.event,
                    event_x: ev.event_x,
                    event_y: ev.event_y,
                    buttons: copy_xi_mask(ev.buttons.mask, ev.buttons.mask_len),
                    mods_effective: ev.mods.effective,
                })
            }
            other => XIEventData::Other(other),
        })
    }
}

/// `XSyncInitialize`: the status as the library returns it.
pub fn x_sync_initialize(display: XDisplay) -> Option<c_int> {
    let sync = libraries().xsync.as_ref()?;
    let (mut major, mut minor) = (0, 0);
    // SAFETY: both pointers are valid places for the versions.
    Some(unsafe { (sync.XSyncInitialize)(display.raw(), &mut major, &mut minor) })
}

/// `XSyncCreateCounter`.
pub fn x_sync_create_counter(display: XDisplay, initial_value: XSyncValue) -> XID {
    match libraries().xsync.as_ref() {
        // SAFETY: plain call.
        Some(sync) => unsafe { (sync.XSyncCreateCounter)(display.raw(), initial_value) },
        None => 0,
    }
}

/// `XSyncSetCounter`.
pub fn x_sync_set_counter(display: XDisplay, counter: XID, value: XSyncValue) {
    if let Some(sync) = libraries().xsync.as_ref() {
        // SAFETY: plain call.
        unsafe {
            (sync.XSyncSetCounter)(display.raw(), counter, value);
        }
    }
}

/// `XSyncDestroyCounter`.
pub fn x_sync_destroy_counter(display: XDisplay, counter: XID) {
    if let Some(sync) = libraries().xsync.as_ref() {
        // SAFETY: plain call.
        unsafe {
            (sync.XSyncDestroyCounter)(display.raw(), counter);
        }
    }
}

/// Whether the server has the shared memory extension
/// (`XShmQueryExtension` and `XShmQueryVersion`).
pub fn x_shm_query(display: XDisplay) -> bool {
    let Some(shm) = libraries().xshm.as_ref() else {
        return false;
    };
    let (mut major, mut minor, mut pixmaps) = (0, 0, 0);
    // SAFETY: the out pointers are valid places for the results.
    unsafe {
        (shm.XShmQueryExtension)(display.raw()) != 0
            && (shm.XShmQueryVersion)(display.raw(), &mut major, &mut minor, &mut pixmaps) != 0
    }
}

/// `XFixesQueryExtension` and `XFixesQueryVersion`: the major version of
/// the fixes extension.
pub fn x_fixes_query_major_version(display: XDisplay) -> Option<c_int> {
    let fixes = libraries().xfixes.as_ref()?;
    let (mut event_base, mut error_base, mut major, minor) = (0, 0, 0, 0);
    // SAFETY: the pointers are valid places for the results.
    unsafe {
        if (fixes.XFixesQueryExtension)(display.raw(), &mut event_base, &mut error_base) == 0 {
            return None;
        }
        if (fixes.XFixesQueryVersion)(display.raw(), &mut major, &minor) == 0 {
            return None;
        }
    }
    Some(major)
}

/// `ShapeInput`: the kind of the input shape of a window.
pub const SHAPE_INPUT: c_int = 2;

/// Sets the input shape of a window to nothing (`empty`) or back to the
/// whole window (`XFixesCreateRegion`, `XFixesSetWindowShapeRegion`,
/// `XFixesDestroyRegion`).
pub fn x_fixes_set_input_shape(display: XDisplay, windows: &[XID], empty: bool) {
    let Some(fixes) = libraries().xfixes.as_ref() else {
        return;
    };
    // SAFETY: a region of zero rectangles reads nothing through the
    // rectangle pointer; the other calls pass identifiers only.
    unsafe {
        // An empty input region makes the server route pointer input to whatever is behind
        // the window. None (0) restores the default input shape, i.e. the whole window.
        let mut region: XID = 0;
        if empty {
            let mut rect = XRectangle { x: 0, y: 0, width: 0, height: 0 };
            region = (fixes.XFixesCreateRegion)(display.raw(), &mut rect, 0);
        }
        for window in windows {
            (fixes.XFixesSetWindowShapeRegion)(display.raw(), *window, SHAPE_INPUT, 0, 0, region);
        }
        if region != 0 {
            (fixes.XFixesDestroyRegion)(display.raw(), region);
        }
    }
}

/// `XcursorLibraryLoadCursor`: a cursor of the cursor theme, or zero.
pub fn xcursor_library_load_cursor(display: XDisplay, name: &str) -> XID {
    let Some(xcursor) = libraries().xcursor.as_ref() else {
        return 0;
    };
    let name = c_string(name);
    // SAFETY: `name` is a NUL-terminated string that outlives the call.
    unsafe { (xcursor.XcursorLibraryLoadCursor)(display.raw(), name.as_ptr()) }
}

/// Makes a cursor of premultiplied BGRA pixels (`XcursorImageLoadCursor`
/// over an `XcursorImage` of version 1). Returns zero without the cursor
/// library.
pub fn xcursor_image_load_cursor(
    display: XDisplay,
    width: u32,
    height: u32,
    xhot: u32,
    yhot: u32,
    pixels: &mut [u32],
) -> XID {
    let Some(xcursor) = libraries().xcursor.as_ref() else {
        return 0;
    };
    assert!(pixels.len() >= width as usize * height as usize, "the cursor pixels are too short");
    let image = x11_dl::xcursor::XcursorImage {
        version: 1,
        size: std::mem::size_of::<x11_dl::xcursor::XcursorImage>() as u32,
        width,
        height,
        xhot,
        yhot,
        delay: 0,
        pixels: pixels.as_mut_ptr(),
    };
    // SAFETY: the image describes `pixels`, which holds width * height
    // pixels (checked above) and outlives the call; the library copies
    // them into the cursor.
    unsafe { (xcursor.XcursorImageLoadCursor)(display.raw(), &image) }
}

// ---------------------------------------------------------------------------
// Screens: further calls of the screen providers.
// ---------------------------------------------------------------------------

// (end of the screens section)

// ---------------------------------------------------------------------------
// Selections: further calls of the clipboard and the selection transfers.
// ---------------------------------------------------------------------------

// (end of the selections section)

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for its Xlib bindings.
    use super::*;

    #[test]
    fn longs_are_decoded_in_the_size_of_a_c_long() {
        let values: [c_ulong; 3] = [1, 0x1234_5678, c_ulong::MAX];
        let mut data = Vec::new();
        for value in values {
            data.extend_from_slice(&value.to_ne_bytes());
        }
        assert_eq!(decode_longs(&data), values);
        // A trailing partial item is not an item.
        data.push(7);
        assert_eq!(decode_longs(&data), values);
        assert!(decode_longs(&[]).is_empty());
    }

    #[test]
    fn the_event_mask_has_a_bit_per_event_type() {
        // XI_DeviceChanged is 1.
        assert_eq!(xi_event_mask_bytes(&[xi2::XI_DeviceChanged]), vec![0b0000_0010]);
        // Motion 6, button press 4, button release 5, leave 8, enter 7.
        assert_eq!(
            xi_event_mask_bytes(&[xi2::XI_Motion, xi2::XI_ButtonPress, xi2::XI_ButtonRelease, xi2::XI_Leave, xi2::XI_Enter]),
            vec![0b1111_0000, 0b0000_0001]
        );
        // The touch events are 18 to 20: a third byte.
        let mask = xi_event_mask_bytes(&[xi2::XI_TouchBegin, xi2::XI_TouchUpdate, xi2::XI_TouchEnd]);
        assert_eq!(mask, vec![0, 0, 0b0001_1100]);
        assert_eq!(xi_event_mask_bytes(&[]), vec![0]);
    }

    #[test]
    fn a_name_ends_at_its_first_nul() {
        assert_eq!(c_string("WM_NAME").as_bytes(), b"WM_NAME");
        assert_eq!(c_string("WM\0NAME").as_bytes(), b"WM");
    }
}
