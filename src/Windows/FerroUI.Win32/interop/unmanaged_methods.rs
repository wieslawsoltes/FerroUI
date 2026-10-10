//! The declarations of the Windows API the backend uses.
//!
//! The first part of the file holds what is data: the constants, the flag
//! sets and the plain structures. It is compiled on every host, because the
//! logic that reads messages and computes styles is tested on every host.
//!
//! The second part, [`native`], exists on Windows only. It holds the system
//! calls, each behind a function that is safe to call: the `unsafe` block of
//! a call is in its wrapper, with the reason it is sound, and the rest of the
//! backend is written without `unsafe`. The declarations of the functions
//! themselves come from the `windows-sys` crate
//! (docs/porting/win32-platform.md, section 2).
//!
//! Handles (`HWND`, `HMONITOR`, `HCURSOR`, ...) are `isize` values everywhere
//! in the backend: they are opaque numbers the system hands out, and a
//! number can be kept in a cell, compared, and given to the thread that
//! renders, which a pointer type cannot without a claim about what it points
//! at.

#![allow(non_camel_case_types, non_snake_case, clippy::upper_case_acronyms)]

/// `CW_USEDEFAULT`: the system chooses the position or the size.
pub const CW_USEDEFAULT: i32 = 0x8000_0000_u32 as i32;

/// The DPI awareness contexts of `SetProcessDpiAwarenessContext`.
pub const DPI_AWARENESS_CONTEXT_UNAWARE: isize = -1;
#[allow(missing_docs)]
pub const DPI_AWARENESS_CONTEXT_SYSTEM_AWARE: isize = -2;
#[allow(missing_docs)]
pub const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE: isize = -3;
#[allow(missing_docs)]
pub const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;

/// `SC_MOUSEMOVE`: the system command that starts moving a window with the
/// mouse.
pub const SC_MOUSEMOVE: i32 = 0xf012;

/// The flags of `TrackMouseEvent`.
pub const TME_HOVER: u32 = 0x0000_0001;
#[allow(missing_docs)]
pub const TME_LEAVE: u32 = 0x0000_0002;
#[allow(missing_docs)]
pub const TME_NONCLIENT: u32 = 0x0000_0010;

/// `ISC_SHOWUICOMPOSITIONWINDOW` of `WM_IME_SETCONTEXT`.
pub const ISC_SHOWUICOMPOSITIONWINDOW: i64 = 0x8000_0000;

/// The values of the insert-after argument of `SetWindowPos`.
pub struct WindowPosZOrder;

#[allow(missing_docs)]
impl WindowPosZOrder {
    pub const HWND_BOTTOM: isize = 1;
    pub const HWND_TOP: isize = 0;
    pub const HWND_TOPMOST: isize = -1;
    pub const HWND_NOTOPMOST: isize = -2;
}

/// A point in device pixels.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct POINT {
    #[allow(missing_docs)]
    pub x: i32,
    #[allow(missing_docs)]
    pub y: i32,
}

/// A rectangle in device pixels, by its edges.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RECT {
    #[allow(missing_docs)]
    pub left: i32,
    #[allow(missing_docs)]
    pub top: i32,
    #[allow(missing_docs)]
    pub right: i32,
    #[allow(missing_docs)]
    pub bottom: i32,
}

impl RECT {
    /// Creates the rectangle of a pixel rectangle.
    pub fn from_pixel_rect(rect: ferroui_base::PixelRect) -> RECT {
        RECT { left: rect.x, top: rect.y, right: rect.x + rect.width, bottom: rect.y + rect.height }
    }

    /// The width of the rectangle.
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    /// The height of the rectangle.
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }

    /// Moves the rectangle by the given offsets.
    pub fn offset(&mut self, x: i32, y: i32) {
        self.left += x;
        self.right += x;
        self.top += y;
        self.bottom += y;
    }
}

/// The margins of `DwmExtendFrameIntoClientArea`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MARGINS {
    #[allow(missing_docs)]
    pub cx_left_width: i32,
    #[allow(missing_docs)]
    pub cx_right_width: i32,
    #[allow(missing_docs)]
    pub cy_top_height: i32,
    #[allow(missing_docs)]
    pub cy_bottom_height: i32,
}

/// The sizes and positions of `WM_GETMINMAXINFO`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MINMAXINFO {
    #[allow(missing_docs)]
    pub pt_reserved: POINT,
    #[allow(missing_docs)]
    pub pt_max_size: POINT,
    #[allow(missing_docs)]
    pub pt_max_position: POINT,
    #[allow(missing_docs)]
    pub pt_min_track_size: POINT,
    #[allow(missing_docs)]
    pub pt_max_track_size: POINT,
}

/// The position of a window of `WM_WINDOWPOSCHANGING` and
/// `WM_WINDOWPOSCHANGED`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WINDOWPOS {
    #[allow(missing_docs)]
    pub hwnd: isize,
    #[allow(missing_docs)]
    pub hwnd_insert_after: isize,
    #[allow(missing_docs)]
    pub x: i32,
    #[allow(missing_docs)]
    pub y: i32,
    #[allow(missing_docs)]
    pub cx: i32,
    #[allow(missing_docs)]
    pub cy: i32,
    #[allow(missing_docs)]
    pub flags: u32,
}

/// The placement of a window: its show state and its restored, minimized
/// and maximized positions.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WINDOWPLACEMENT {
    /// The size of the structure, in bytes.
    pub length: u32,
    #[allow(missing_docs)]
    pub flags: u32,
    /// A [`ShowWindowCommand`].
    pub show_cmd: i32,
    #[allow(missing_docs)]
    pub min_position: POINT,
    #[allow(missing_docs)]
    pub max_position: POINT,
    #[allow(missing_docs)]
    pub normal_position: RECT,
}

/// A point of the history of the mouse (`GetMouseMovePointsEx`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MOUSEMOVEPOINT {
    #[allow(missing_docs)]
    pub x: i32,
    #[allow(missing_docs)]
    pub y: i32,
    #[allow(missing_docs)]
    pub time: i32,
    #[allow(missing_docs)]
    pub dw_extra_info: isize,
}

/// What the system knows of a pointer (`GetPointerInfo`): the members of
/// the structure of the system that the backend reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct POINTER_INFO {
    /// A [`PointerInputType`].
    pub pointer_type: u32,
    #[allow(missing_docs)]
    pub pointer_id: u32,
    #[allow(missing_docs)]
    pub pointer_flags: u32,
    #[allow(missing_docs)]
    pub source_device: isize,
    #[allow(missing_docs)]
    pub pt_pixel_location_x: i32,
    #[allow(missing_docs)]
    pub pt_pixel_location_y: i32,
    #[allow(missing_docs)]
    pub pt_himetric_location_raw_x: i32,
    #[allow(missing_docs)]
    pub pt_himetric_location_raw_y: i32,
    #[allow(missing_docs)]
    pub dw_time: u32,
    #[allow(missing_docs)]
    pub history_count: u32,
    /// A [`PointerButtonChangeType`].
    pub button_change_type: u32,
}

/// What the system knows of a touch contact (`GetPointerTouchInfo`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct POINTER_TOUCH_INFO {
    #[allow(missing_docs)]
    pub pointer_info: POINTER_INFO,
    /// A set of [`TouchMask`].
    pub touch_mask: u32,
    #[allow(missing_docs)]
    pub rc_contact_left: i32,
    #[allow(missing_docs)]
    pub rc_contact_top: i32,
    #[allow(missing_docs)]
    pub rc_contact_right: i32,
    #[allow(missing_docs)]
    pub rc_contact_bottom: i32,
    #[allow(missing_docs)]
    pub pressure: u32,
}

/// What the system knows of a pen (`GetPointerPenInfo`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct POINTER_PEN_INFO {
    #[allow(missing_docs)]
    pub pointer_info: POINTER_INFO,
    /// A set of [`PenFlags`].
    pub pen_flags: u32,
    #[allow(missing_docs)]
    pub pressure: u32,
    #[allow(missing_docs)]
    pub rotation: u32,
    #[allow(missing_docs)]
    pub tilt_x: i32,
    #[allow(missing_docs)]
    pub tilt_y: i32,
}

/// One contact of a `WM_TOUCH` message (`GetTouchInputInfo`): the members
/// the backend reads. Positions and sizes are hundredths of a pixel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TOUCHINPUT {
    #[allow(missing_docs)]
    pub x: i32,
    #[allow(missing_docs)]
    pub y: i32,
    #[allow(missing_docs)]
    pub id: u32,
    /// A set of [`TouchInputFlags`].
    pub flags: u32,
    #[allow(missing_docs)]
    pub mask: u32,
    #[allow(missing_docs)]
    pub time: u32,
    #[allow(missing_docs)]
    pub cx_contact: u32,
    #[allow(missing_docs)]
    pub cy_contact: u32,
}

/// The flags of `EnableMenuItem`.
pub const MF_BYCOMMAND: u32 = 0x0000_0000;
#[allow(missing_docs)]
pub const MF_ENABLED: u32 = 0x0000_0000;
#[allow(missing_docs)]
pub const MF_GRAYED: u32 = 0x0000_0001;
#[allow(missing_docs)]
pub const MF_DISABLED: u32 = 0x0000_0002;

bitflags::bitflags! {
    /// `TrackPopupMenuFlags` of the interop declarations.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct TrackPopupMenuFlags: u32 {
        const TPM_LEFTBUTTON = 0x0000;
        const TPM_RIGHTBUTTON = 0x0002;
        const TPM_NONOTIFY = 0x0080;
        const TPM_RETURNCMD = 0x0100;
    }
}

/// The header of a device independent bitmap.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BITMAPINFOHEADER {
    #[allow(missing_docs)]
    pub bi_size: u32,
    #[allow(missing_docs)]
    pub bi_width: i32,
    #[allow(missing_docs)]
    pub bi_height: i32,
    #[allow(missing_docs)]
    pub bi_planes: u16,
    #[allow(missing_docs)]
    pub bi_bit_count: u16,
    #[allow(missing_docs)]
    pub bi_compression: u32,
    #[allow(missing_docs)]
    pub bi_size_image: u32,
    #[allow(missing_docs)]
    pub bi_x_pels_per_meter: i32,
    #[allow(missing_docs)]
    pub bi_y_pels_per_meter: i32,
    #[allow(missing_docs)]
    pub bi_clr_used: u32,
    #[allow(missing_docs)]
    pub bi_clr_important: u32,
}

/// The size of a [`BITMAPINFOHEADER`], in bytes.
pub const SIZE_OF_BITMAPINFOHEADER: u32 = 40;

impl BITMAPINFOHEADER {
    /// Sets the size member to the size of the structure.
    pub fn init(&mut self) {
        self.bi_size = SIZE_OF_BITMAPINFOHEADER;
    }
}

/// What `GetMonitorInfo` reports about a monitor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MONITORINFOEX {
    /// The rectangle of the monitor, in the coordinates of the virtual
    /// screen.
    pub rc_monitor: RECT,
    /// The work area of the monitor: without the taskbar and the
    /// application bars.
    pub rc_work: RECT,
    /// 1 for the primary monitor.
    pub dw_flags: u32,
    /// The device name of the monitor (`\\.\DISPLAY1`).
    pub sz_device: String,
}

/// `Cursor` of the interop declarations: the values the system uses, as constants.
pub struct Cursor;

#[allow(missing_docs)]
impl Cursor {
    pub const IDC_ARROW: i32 = 32512;
    pub const IDC_IBEAM: i32 = 32513;
    pub const IDC_WAIT: i32 = 32514;
    pub const IDC_CROSS: i32 = 32515;
    pub const IDC_UPARROW: i32 = 32516;
    pub const IDC_SIZE: i32 = 32640;
    pub const IDC_ICON: i32 = 32641;
    pub const IDC_SIZENWSE: i32 = 32642;
    pub const IDC_SIZENESW: i32 = 32643;
    pub const IDC_SIZEWE: i32 = 32644;
    pub const IDC_SIZENS: i32 = 32645;
    pub const IDC_SIZEALL: i32 = 32646;
    pub const IDC_NO: i32 = 32648;
    pub const IDC_HAND: i32 = 32649;
    pub const IDC_APPSTARTING: i32 = 32650;
    pub const IDC_HELP: i32 = 32651;
}

/// `MouseActivate` of the interop declarations: the values the system uses, as constants.
pub struct MouseActivate;

#[allow(missing_docs)]
impl MouseActivate {
    pub const MA_ACTIVATE: i32 = 1;
    pub const MA_ACTIVATEANDEAT: i32 = 2;
    pub const MA_NOACTIVATE: i32 = 3;
    pub const MA_NOACTIVATEANDEAT: i32 = 4;
}

bitflags::bitflags! {
    /// `SetWindowPosFlags` of the interop declarations.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct SetWindowPosFlags: u32 {
        const SWP_ASYNCWINDOWPOS = 0x4000;
        const SWP_DEFERERASE = 0x2000;
        const SWP_DRAWFRAME = 0x0020;
        const SWP_FRAMECHANGED = 0x0020;
        const SWP_HIDEWINDOW = 0x0080;
        const SWP_NOACTIVATE = 0x0010;
        const SWP_NOCOPYBITS = 0x0100;
        const SWP_NOMOVE = 0x0002;
        const SWP_NOOWNERZORDER = 0x0200;
        const SWP_NOREDRAW = 0x0008;
        const SWP_NOREPOSITION = 0x0200;
        const SWP_NOSENDCHANGING = 0x0400;
        const SWP_NOSIZE = 0x0001;
        const SWP_NOZORDER = 0x0004;
        const SWP_SHOWWINDOW = 0x0040;
        const SWP_RESIZE = Self::SWP_NOACTIVATE.bits() | Self::SWP_NOMOVE.bits() | Self::SWP_NOZORDER.bits();
    }
}

/// `SizeCommand` of the interop declarations: the values the system uses, as constants.
pub struct SizeCommand;

#[allow(missing_docs)]
impl SizeCommand {
    pub const RESTORED: i32 = 0;
    pub const MINIMIZED: i32 = Self::RESTORED + 1;
    pub const MAXIMIZED: i32 = Self::MINIMIZED + 1;
    pub const MAX_SHOW: i32 = Self::MAXIMIZED + 1;
    pub const MAX_HIDE: i32 = Self::MAX_SHOW + 1;
}

/// `ShowWindowCommand` of the interop declarations: the values the system uses, as constants.
pub struct ShowWindowCommand;

#[allow(missing_docs)]
impl ShowWindowCommand {
    pub const HIDE: i32 = 0;
    pub const NORMAL: i32 = 1;
    pub const SHOW_MINIMIZED: i32 = 2;
    pub const MAXIMIZE: i32 = 3;
    pub const SHOW_MAXIMIZED: i32 = Self::MAXIMIZE;
    pub const SHOW_NO_ACTIVATE: i32 = 4;
    pub const SHOW: i32 = 5;
    pub const MINIMIZE: i32 = 6;
    pub const SHOW_MIN_NO_ACTIVE: i32 = 7;
    pub const SHOW_NA: i32 = 8;
    pub const RESTORE: i32 = 9;
    pub const SHOW_DEFAULT: i32 = 10;
    pub const FORCE_MINIMIZE: i32 = 11;
}

/// `SystemMetric` of the interop declarations: the values the system uses, as constants.
pub struct SystemMetric;

#[allow(missing_docs)]
impl SystemMetric {
    pub const SM_CXSCREEN: i32 = 0;
    pub const SM_CYSCREEN: i32 = 1;
    pub const SM_CXVSCROLL: i32 = 2;
    pub const SM_CYHSCROLL: i32 = 3;
    pub const SM_CYCAPTION: i32 = 4;
    pub const SM_CXBORDER: i32 = 5;
    pub const SM_CYBORDER: i32 = 6;
    pub const SM_CXDLGFRAME: i32 = 7;
    pub const SM_CXFIXEDFRAME: i32 = 7;
    pub const SM_CYDLGFRAME: i32 = 8;
    pub const SM_CYFIXEDFRAME: i32 = 8;
    pub const SM_CYVTHUMB: i32 = 9;
    pub const SM_CXHTHUMB: i32 = 10;
    pub const SM_CXICON: i32 = 11;
    pub const SM_CYICON: i32 = 12;
    pub const SM_CXCURSOR: i32 = 13;
    pub const SM_CYCURSOR: i32 = 14;
    pub const SM_CYMENU: i32 = 15;
    pub const SM_CXFULLSCREEN: i32 = 16;
    pub const SM_CYFULLSCREEN: i32 = 17;
    pub const SM_CYKANJIWINDOW: i32 = 18;
    pub const SM_MOUSEPRESENT: i32 = 19;
    pub const SM_CYVSCROLL: i32 = 20;
    pub const SM_CXHSCROLL: i32 = 21;
    pub const SM_DEBUG: i32 = 22;
    pub const SM_SWAPBUTTON: i32 = 23;
    pub const SM_CXMIN: i32 = 28;
    pub const SM_CYMIN: i32 = 29;
    pub const SM_CXSIZE: i32 = 30;
    pub const SM_CYSIZE: i32 = 31;
    pub const SM_CXSIZEFRAME: i32 = 32;
    pub const SM_CXFRAME: i32 = 32;
    pub const SM_CYSIZEFRAME: i32 = 33;
    pub const SM_CYFRAME: i32 = 33;
    pub const SM_CXMINTRACK: i32 = 34;
    pub const SM_CYMINTRACK: i32 = 35;
    pub const SM_CXDOUBLECLK: i32 = 36;
    pub const SM_CYDOUBLECLK: i32 = 37;
    pub const SM_CXICONSPACING: i32 = 38;
    pub const SM_CYICONSPACING: i32 = 39;
    pub const SM_MENUDROPALIGNMENT: i32 = 40;
    pub const SM_PENWINDOWS: i32 = 41;
    pub const SM_DBCSENABLED: i32 = 42;
    pub const SM_CMOUSEBUTTONS: i32 = 43;
    pub const SM_SECURE: i32 = 44;
    pub const SM_CXEDGE: i32 = 45;
    pub const SM_CYEDGE: i32 = 46;
    pub const SM_CXMINSPACING: i32 = 47;
    pub const SM_CYMINSPACING: i32 = 48;
    pub const SM_CXSMICON: i32 = 49;
    pub const SM_CYSMICON: i32 = 50;
    pub const SM_CYSMCAPTION: i32 = 51;
    pub const SM_CXSMSIZE: i32 = 52;
    pub const SM_CYSMSIZE: i32 = 53;
    pub const SM_CXMENUSIZE: i32 = 54;
    pub const SM_CYMENUSIZE: i32 = 55;
    pub const SM_ARRANGE: i32 = 56;
    pub const SM_CXMINIMIZED: i32 = 57;
    pub const SM_CYMINIMIZED: i32 = 58;
    pub const SM_CXMAXTRACK: i32 = 59;
    pub const SM_CYMAXTRACK: i32 = 60;
    pub const SM_CXMAXIMIZED: i32 = 61;
    pub const SM_CYMAXIMIZED: i32 = 62;
    pub const SM_NETWORK: i32 = 63;
    pub const SM_CLEANBOOT: i32 = 67;
    pub const SM_CXDRAG: i32 = 68;
    pub const SM_CYDRAG: i32 = 69;
    pub const SM_SHOWSOUNDS: i32 = 70;
    pub const SM_CXMENUCHECK: i32 = 71;
    pub const SM_CYMENUCHECK: i32 = 72;
    pub const SM_SLOWMACHINE: i32 = 73;
    pub const SM_MIDEASTENABLED: i32 = 74;
    pub const SM_MOUSEWHEELPRESENT: i32 = 75;
    pub const SM_XVIRTUALSCREEN: i32 = 76;
    pub const SM_YVIRTUALSCREEN: i32 = 77;
    pub const SM_CXVIRTUALSCREEN: i32 = 78;
    pub const SM_CYVIRTUALSCREEN: i32 = 79;
    pub const SM_CMONITORS: i32 = 80;
    pub const SM_SAMEDISPLAYFORMAT: i32 = 81;
    pub const SM_IMMENABLED: i32 = 82;
    pub const SM_CXFOCUSBORDER: i32 = 83;
    pub const SM_CYFOCUSBORDER: i32 = 84;
    pub const SM_TABLETPC: i32 = 86;
    pub const SM_MEDIACENTER: i32 = 87;
    pub const SM_STARTER: i32 = 88;
    pub const SM_SERVERR2: i32 = 89;
    pub const SM_MOUSEHORIZONTALWHEELPRESENT: i32 = 91;
    pub const SM_CXPADDEDBORDER: i32 = 92;
    pub const SM_DIGITIZER: i32 = 94;
    pub const SM_MAXIMUMTOUCHES: i32 = 95;
    pub const SM_REMOTESESSION: i32 = 0x1000;
    pub const SM_SHUTTINGDOWN: i32 = 0x2000;
    pub const SM_REMOTECONTROL: i32 = 0x2001;
    pub const SM_CONVERTABLESLATEMODE: i32 = 0x2003;
    pub const SM_SYSTEMDOCKED: i32 = 0x2004;
}

bitflags::bitflags! {
    /// `ModifierKeys` of the interop declarations.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ModifierKeys: i32 {
        const MK_NONE = 0x0000;
        const MK_LBUTTON = 0x0001;
        const MK_RBUTTON = 0x0002;
        const MK_SHIFT = 0x0004;
        const MK_CONTROL = 0x0008;
        const MK_MBUTTON = 0x0010;
        const MK_ALT = 0x0020;
        const MK_XBUTTON1 = 0x0020;
        const MK_XBUTTON2 = 0x0040;
    }
}

/// `VirtualKeyStates` of the interop declarations: the values the system uses, as constants.
pub struct VirtualKeyStates;

#[allow(missing_docs)]
impl VirtualKeyStates {
    pub const VK_LBUTTON: i32 = 0x01;
    pub const VK_RBUTTON: i32 = 0x02;
    pub const VK_CANCEL: i32 = 0x03;
    pub const VK_MBUTTON: i32 = 0x04;
    pub const VK_XBUTTON1: i32 = 0x05;
    pub const VK_XBUTTON2: i32 = 0x06;
    pub const VK_BACK: i32 = 0x08;
    pub const VK_TAB: i32 = 0x09;
    pub const VK_CLEAR: i32 = 0x0C;
    pub const VK_RETURN: i32 = 0x0D;
    pub const VK_SHIFT: i32 = 0x10;
    pub const VK_CONTROL: i32 = 0x11;
    pub const VK_MENU: i32 = 0x12;
    pub const VK_PAUSE: i32 = 0x13;
    pub const VK_CAPITAL: i32 = 0x14;
    pub const VK_KANA: i32 = 0x15;
    pub const VK_HANGEUL: i32 = 0x15;
    pub const VK_HANGUL: i32 = 0x15;
    pub const VK_JUNJA: i32 = 0x17;
    pub const VK_FINAL: i32 = 0x18;
    pub const VK_HANJA: i32 = 0x19;
    pub const VK_KANJI: i32 = 0x19;
    pub const VK_ESCAPE: i32 = 0x1B;
    pub const VK_CONVERT: i32 = 0x1C;
    pub const VK_NONCONVERT: i32 = 0x1D;
    pub const VK_ACCEPT: i32 = 0x1E;
    pub const VK_MODECHANGE: i32 = 0x1F;
    pub const VK_SPACE: i32 = 0x20;
    pub const VK_PRIOR: i32 = 0x21;
    pub const VK_NEXT: i32 = 0x22;
    pub const VK_END: i32 = 0x23;
    pub const VK_HOME: i32 = 0x24;
    pub const VK_LEFT: i32 = 0x25;
    pub const VK_UP: i32 = 0x26;
    pub const VK_RIGHT: i32 = 0x27;
    pub const VK_DOWN: i32 = 0x28;
    pub const VK_SELECT: i32 = 0x29;
    pub const VK_PRINT: i32 = 0x2A;
    pub const VK_EXECUTE: i32 = 0x2B;
    pub const VK_SNAPSHOT: i32 = 0x2C;
    pub const VK_INSERT: i32 = 0x2D;
    pub const VK_DELETE: i32 = 0x2E;
    pub const VK_HELP: i32 = 0x2F;
    pub const VK_LWIN: i32 = 0x5B;
    pub const VK_RWIN: i32 = 0x5C;
    pub const VK_APPS: i32 = 0x5D;
    pub const VK_SLEEP: i32 = 0x5F;
    pub const VK_NUMPAD0: i32 = 0x60;
    pub const VK_NUMPAD1: i32 = 0x61;
    pub const VK_NUMPAD2: i32 = 0x62;
    pub const VK_NUMPAD3: i32 = 0x63;
    pub const VK_NUMPAD4: i32 = 0x64;
    pub const VK_NUMPAD5: i32 = 0x65;
    pub const VK_NUMPAD6: i32 = 0x66;
    pub const VK_NUMPAD7: i32 = 0x67;
    pub const VK_NUMPAD8: i32 = 0x68;
    pub const VK_NUMPAD9: i32 = 0x69;
    pub const VK_MULTIPLY: i32 = 0x6A;
    pub const VK_ADD: i32 = 0x6B;
    pub const VK_SEPARATOR: i32 = 0x6C;
    pub const VK_SUBTRACT: i32 = 0x6D;
    pub const VK_DECIMAL: i32 = 0x6E;
    pub const VK_DIVIDE: i32 = 0x6F;
    pub const VK_F1: i32 = 0x70;
    pub const VK_F2: i32 = 0x71;
    pub const VK_F3: i32 = 0x72;
    pub const VK_F4: i32 = 0x73;
    pub const VK_F5: i32 = 0x74;
    pub const VK_F6: i32 = 0x75;
    pub const VK_F7: i32 = 0x76;
    pub const VK_F8: i32 = 0x77;
    pub const VK_F9: i32 = 0x78;
    pub const VK_F10: i32 = 0x79;
    pub const VK_F11: i32 = 0x7A;
    pub const VK_F12: i32 = 0x7B;
    pub const VK_F13: i32 = 0x7C;
    pub const VK_F14: i32 = 0x7D;
    pub const VK_F15: i32 = 0x7E;
    pub const VK_F16: i32 = 0x7F;
    pub const VK_F17: i32 = 0x80;
    pub const VK_F18: i32 = 0x81;
    pub const VK_F19: i32 = 0x82;
    pub const VK_F20: i32 = 0x83;
    pub const VK_F21: i32 = 0x84;
    pub const VK_F22: i32 = 0x85;
    pub const VK_F23: i32 = 0x86;
    pub const VK_F24: i32 = 0x87;
    pub const VK_NUMLOCK: i32 = 0x90;
    pub const VK_SCROLL: i32 = 0x91;
    pub const VK_OEM_NEC_EQUAL: i32 = 0x92;
    pub const VK_OEM_FJ_JISHO: i32 = 0x92;
    pub const VK_OEM_FJ_MASSHOU: i32 = 0x93;
    pub const VK_OEM_FJ_TOUROKU: i32 = 0x94;
    pub const VK_OEM_FJ_LOYA: i32 = 0x95;
    pub const VK_OEM_FJ_ROYA: i32 = 0x96;
    pub const VK_LSHIFT: i32 = 0xA0;
    pub const VK_RSHIFT: i32 = 0xA1;
    pub const VK_LCONTROL: i32 = 0xA2;
    pub const VK_RCONTROL: i32 = 0xA3;
    pub const VK_LMENU: i32 = 0xA4;
    pub const VK_RMENU: i32 = 0xA5;
    pub const VK_BROWSER_BACK: i32 = 0xA6;
    pub const VK_BROWSER_FORWARD: i32 = 0xA7;
    pub const VK_BROWSER_REFRESH: i32 = 0xA8;
    pub const VK_BROWSER_STOP: i32 = 0xA9;
    pub const VK_BROWSER_SEARCH: i32 = 0xAA;
    pub const VK_BROWSER_FAVORITES: i32 = 0xAB;
    pub const VK_BROWSER_HOME: i32 = 0xAC;
    pub const VK_VOLUME_MUTE: i32 = 0xAD;
    pub const VK_VOLUME_DOWN: i32 = 0xAE;
    pub const VK_VOLUME_UP: i32 = 0xAF;
    pub const VK_MEDIA_NEXT_TRACK: i32 = 0xB0;
    pub const VK_MEDIA_PREV_TRACK: i32 = 0xB1;
    pub const VK_MEDIA_STOP: i32 = 0xB2;
    pub const VK_MEDIA_PLAY_PAUSE: i32 = 0xB3;
    pub const VK_LAUNCH_MAIL: i32 = 0xB4;
    pub const VK_LAUNCH_MEDIA_SELECT: i32 = 0xB5;
    pub const VK_LAUNCH_APP1: i32 = 0xB6;
    pub const VK_LAUNCH_APP2: i32 = 0xB7;
    pub const VK_OEM_1: i32 = 0xBA;
    pub const VK_OEM_PLUS: i32 = 0xBB;
    pub const VK_OEM_COMMA: i32 = 0xBC;
    pub const VK_OEM_MINUS: i32 = 0xBD;
    pub const VK_OEM_PERIOD: i32 = 0xBE;
    pub const VK_OEM_2: i32 = 0xBF;
    pub const VK_OEM_3: i32 = 0xC0;
    pub const VK_ABNT_C1: i32 = 0xC1;
    pub const VK_ABNT_C2: i32 = 0xC2;
    pub const VK_OEM_4: i32 = 0xDB;
    pub const VK_OEM_5: i32 = 0xDC;
    pub const VK_OEM_6: i32 = 0xDD;
    pub const VK_OEM_7: i32 = 0xDE;
    pub const VK_OEM_8: i32 = 0xDF;
    pub const VK_OEM_AX: i32 = 0xE1;
    pub const VK_OEM_102: i32 = 0xE2;
    pub const VK_ICO_HELP: i32 = 0xE3;
    pub const VK_ICO_00: i32 = 0xE4;
    pub const VK_PROCESSKEY: i32 = 0xE5;
    pub const VK_ICO_CLEAR: i32 = 0xE6;
    pub const VK_PACKET: i32 = 0xE7;
    pub const VK_OEM_RESET: i32 = 0xE9;
    pub const VK_OEM_JUMP: i32 = 0xEA;
    pub const VK_OEM_PA1: i32 = 0xEB;
    pub const VK_OEM_PA2: i32 = 0xEC;
    pub const VK_OEM_PA3: i32 = 0xED;
    pub const VK_OEM_WSCTRL: i32 = 0xEE;
    pub const VK_OEM_CUSEL: i32 = 0xEF;
    pub const VK_OEM_ATTN: i32 = 0xF0;
    pub const VK_OEM_FINISH: i32 = 0xF1;
    pub const VK_OEM_COPY: i32 = 0xF2;
    pub const VK_OEM_AUTO: i32 = 0xF3;
    pub const VK_OEM_ENLW: i32 = 0xF4;
    pub const VK_OEM_BACKTAB: i32 = 0xF5;
    pub const VK_ATTN: i32 = 0xF6;
    pub const VK_CRSEL: i32 = 0xF7;
    pub const VK_EXSEL: i32 = 0xF8;
    pub const VK_EREOF: i32 = 0xF9;
    pub const VK_PLAY: i32 = 0xFA;
    pub const VK_ZOOM: i32 = 0xFB;
    pub const VK_NONAME: i32 = 0xFC;
    pub const VK_PA1: i32 = 0xFD;
    pub const VK_OEM_CLEAR: i32 = 0xFE;
}

/// `WindowActivate` of the interop declarations: the values the system uses, as constants.
pub struct WindowActivate;

#[allow(missing_docs)]
impl WindowActivate {
    pub const WA_INACTIVE: i32 = 0;
    pub const WA_ACTIVE: i32 = Self::WA_INACTIVE + 1;
    pub const WA_CLICKACTIVE: i32 = Self::WA_ACTIVE + 1;
}

/// `HitTestValues` of the interop declarations: the values the system uses, as constants.
pub struct HitTestValues;

#[allow(missing_docs)]
impl HitTestValues {
    pub const HTERROR: i32 = -2;
    pub const HTTRANSPARENT: i32 = -1;
    pub const HTNOWHERE: i32 = 0;
    pub const HTCLIENT: i32 = 1;
    pub const HTCAPTION: i32 = 2;
    pub const HTSYSMENU: i32 = 3;
    pub const HTGROWBOX: i32 = 4;
    pub const HTMENU: i32 = 5;
    pub const HTHSCROLL: i32 = 6;
    pub const HTVSCROLL: i32 = 7;
    pub const HTMINBUTTON: i32 = 8;
    pub const HTMAXBUTTON: i32 = 9;
    pub const HTLEFT: i32 = 10;
    pub const HTRIGHT: i32 = 11;
    pub const HTTOP: i32 = 12;
    pub const HTTOPLEFT: i32 = 13;
    pub const HTTOPRIGHT: i32 = 14;
    pub const HTBOTTOM: i32 = 15;
    pub const HTBOTTOMLEFT: i32 = 16;
    pub const HTBOTTOMRIGHT: i32 = 17;
    pub const HTBORDER: i32 = 18;
    pub const HTOBJECT: i32 = 19;
    pub const HTCLOSE: i32 = 20;
    pub const HTHELP: i32 = 21;
}

bitflags::bitflags! {
    /// `WindowStyles` of the interop declarations.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct WindowStyles: u32 {
        const WS_BORDER = 0x800000;
        const WS_CAPTION = 0xc00000;
        const WS_CHILD = 0x40000000;
        const WS_CLIPCHILDREN = 0x2000000;
        const WS_CLIPSIBLINGS = 0x4000000;
        const WS_DISABLED = 0x8000000;
        const WS_DLGFRAME = 0x400000;
        const WS_GROUP = 0x20000;
        const WS_HSCROLL = 0x100000;
        const WS_MAXIMIZE = 0x1000000;
        const WS_MAXIMIZEBOX = 0x10000;
        const WS_MINIMIZE = 0x20000000;
        const WS_MINIMIZEBOX = 0x20000;
        const WS_OVERLAPPED = 0x0;
        const WS_OVERLAPPEDWINDOW = Self::WS_OVERLAPPED.bits() | Self::WS_CAPTION.bits() | Self::WS_SYSMENU.bits() | Self::WS_THICKFRAME.bits() | Self::WS_MINIMIZEBOX.bits() | Self::WS_MAXIMIZEBOX.bits();
        const WS_POPUP = 0x80000000;
        const WS_POPUPWINDOW = Self::WS_POPUP.bits() | Self::WS_BORDER.bits() | Self::WS_SYSMENU.bits();
        const WS_SYSMENU = 0x80000;
        const WS_TABSTOP = 0x10000;
        const WS_THICKFRAME = 0x40000;
        const WS_VISIBLE = 0x10000000;
        const WS_VSCROLL = 0x200000;
        const WS_EX_DLGMODALFRAME = 0x00000001;
        const WS_EX_NOPARENTNOTIFY = 0x00000004;
        const WS_EX_NOREDIRECTIONBITMAP = 0x00200000;
        const WS_EX_TOPMOST = 0x00000008;
        const WS_EX_ACCEPTFILES = 0x00000010;
        const WS_EX_TRANSPARENT = 0x00000020;
        const WS_EX_MDICHILD = 0x00000040;
        const WS_EX_TOOLWINDOW = 0x00000080;
        const WS_EX_WINDOWEDGE = 0x00000100;
        const WS_EX_CLIENTEDGE = 0x00000200;
        const WS_EX_CONTEXTHELP = 0x00000400;
        const WS_EX_RIGHT = 0x00001000;
        const WS_EX_LEFT = 0x00000000;
        const WS_EX_RTLREADING = 0x00002000;
        const WS_EX_LTRREADING = 0x00000000;
        const WS_EX_LEFTSCROLLBAR = 0x00004000;
        const WS_EX_RIGHTSCROLLBAR = 0x00000000;
        const WS_EX_CONTROLPARENT = 0x00010000;
        const WS_EX_STATICEDGE = 0x00020000;
        const WS_EX_APPWINDOW = 0x00040000;
        const WS_EX_OVERLAPPEDWINDOW = Self::WS_EX_WINDOWEDGE.bits() | Self::WS_EX_CLIENTEDGE.bits();
        const WS_EX_PALETTEWINDOW = Self::WS_EX_WINDOWEDGE.bits() | Self::WS_EX_TOOLWINDOW.bits() | Self::WS_EX_TOPMOST.bits();
        const WS_EX_LAYERED = 0x00080000;
        const WS_EX_NOINHERITLAYOUT = 0x00100000;
        const WS_EX_LAYOUTRTL = 0x00400000;
        const WS_EX_COMPOSITED = 0x02000000;
        const WS_EX_NOACTIVATE = 0x08000000;
    }
}

bitflags::bitflags! {
    /// `ClassStyles` of the interop declarations.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ClassStyles: u32 {
        const CS_VREDRAW = 0x0001;
        const CS_HREDRAW = 0x0002;
        const CS_DBLCLKS = 0x0008;
        const CS_OWNDC = 0x0020;
        const CS_CLASSDC = 0x0040;
        const CS_PARENTDC = 0x0080;
        const CS_NOCLOSE = 0x0200;
        const CS_SAVEBITS = 0x0800;
        const CS_BYTEALIGNCLIENT = 0x1000;
        const CS_BYTEALIGNWINDOW = 0x2000;
        const CS_GLOBALCLASS = 0x4000;
        const CS_IME = 0x00010000;
        const CS_DROPSHADOW = 0x00020000;
    }
}

/// `PointerInputType` of the interop declarations: the values the system uses, as constants.
pub struct PointerInputType;

#[allow(missing_docs)]
impl PointerInputType {
    pub const PT_NONE: u32 = 0x00000000;
    pub const PT_POINTER: u32 = 0x00000001;
    pub const PT_TOUCH: u32 = 0x00000002;
    pub const PT_PEN: u32 = 0x00000003;
    pub const PT_MOUSE: u32 = 0x00000004;
    pub const PT_TOUCHPAD: u32 = 0x00000005;
}

/// `WindowsMessage` of the interop declarations: the values the system uses, as constants.
pub struct WindowsMessage;

#[allow(missing_docs)]
impl WindowsMessage {
    pub const WM_NULL: u32 = 0x0000;
    pub const WM_CREATE: u32 = 0x0001;
    pub const WM_DESTROY: u32 = 0x0002;
    pub const WM_MOVE: u32 = 0x0003;
    pub const WM_SIZE: u32 = 0x0005;
    pub const WM_ACTIVATE: u32 = 0x0006;
    pub const WM_SETFOCUS: u32 = 0x0007;
    pub const WM_KILLFOCUS: u32 = 0x0008;
    pub const WM_ENABLE: u32 = 0x000A;
    pub const WM_SETREDRAW: u32 = 0x000B;
    pub const WM_SETTEXT: u32 = 0x000C;
    pub const WM_GETTEXT: u32 = 0x000D;
    pub const WM_GETTEXTLENGTH: u32 = 0x000E;
    pub const WM_PAINT: u32 = 0x000F;
    pub const WM_CLOSE: u32 = 0x0010;
    pub const WM_QUERYENDSESSION: u32 = 0x0011;
    pub const WM_QUERYOPEN: u32 = 0x0013;
    pub const WM_ENDSESSION: u32 = 0x0016;
    pub const WM_QUIT: u32 = 0x0012;
    pub const WM_ERASEBKGND: u32 = 0x0014;
    pub const WM_SYSCOLORCHANGE: u32 = 0x0015;
    pub const WM_SHOWWINDOW: u32 = 0x0018;
    pub const WM_WININICHANGE: u32 = 0x001A;
    pub const WM_SETTINGCHANGE: u32 = Self::WM_WININICHANGE;
    pub const WM_DEVMODECHANGE: u32 = 0x001B;
    pub const WM_ACTIVATEAPP: u32 = 0x001C;
    pub const WM_FONTCHANGE: u32 = 0x001D;
    pub const WM_TIMECHANGE: u32 = 0x001E;
    pub const WM_CANCELMODE: u32 = 0x001F;
    pub const WM_SETCURSOR: u32 = 0x0020;
    pub const WM_MOUSEACTIVATE: u32 = 0x0021;
    pub const WM_CHILDACTIVATE: u32 = 0x0022;
    pub const WM_QUEUESYNC: u32 = 0x0023;
    pub const WM_GETMINMAXINFO: u32 = 0x0024;
    pub const WM_PAINTICON: u32 = 0x0026;
    pub const WM_ICONERASEBKGND: u32 = 0x0027;
    pub const WM_NEXTDLGCTL: u32 = 0x0028;
    pub const WM_SPOOLERSTATUS: u32 = 0x002A;
    pub const WM_DRAWITEM: u32 = 0x002B;
    pub const WM_MEASUREITEM: u32 = 0x002C;
    pub const WM_DELETEITEM: u32 = 0x002D;
    pub const WM_VKEYTOITEM: u32 = 0x002E;
    pub const WM_CHARTOITEM: u32 = 0x002F;
    pub const WM_SETFONT: u32 = 0x0030;
    pub const WM_GETFONT: u32 = 0x0031;
    pub const WM_SETHOTKEY: u32 = 0x0032;
    pub const WM_GETHOTKEY: u32 = 0x0033;
    pub const WM_QUERYDRAGICON: u32 = 0x0037;
    pub const WM_COMPAREITEM: u32 = 0x0039;
    pub const WM_GETOBJECT: u32 = 0x003D;
    pub const WM_COMPACTING: u32 = 0x0041;
    pub const WM_WINDOWPOSCHANGING: u32 = 0x0046;
    pub const WM_WINDOWPOSCHANGED: u32 = 0x0047;
    pub const WM_COPYDATA: u32 = 0x004A;
    pub const WM_CANCELJOURNAL: u32 = 0x004B;
    pub const WM_NOTIFY: u32 = 0x004E;
    pub const WM_INPUTLANGCHANGEREQUEST: u32 = 0x0050;
    pub const WM_INPUTLANGCHANGE: u32 = 0x0051;
    pub const WM_TCARD: u32 = 0x0052;
    pub const WM_HELP: u32 = 0x0053;
    pub const WM_USERCHANGED: u32 = 0x0054;
    pub const WM_NOTIFYFORMAT: u32 = 0x0055;
    pub const WM_CONTEXTMENU: u32 = 0x007B;
    pub const WM_STYLECHANGING: u32 = 0x007C;
    pub const WM_STYLECHANGED: u32 = 0x007D;
    pub const WM_DISPLAYCHANGE: u32 = 0x007E;
    pub const WM_GETICON: u32 = 0x007F;
    pub const WM_SETICON: u32 = 0x0080;
    pub const WM_NCCREATE: u32 = 0x0081;
    pub const WM_NCDESTROY: u32 = 0x0082;
    pub const WM_NCCALCSIZE: u32 = 0x0083;
    pub const WM_NCHITTEST: u32 = 0x0084;
    pub const WM_NCPAINT: u32 = 0x0085;
    pub const WM_NCACTIVATE: u32 = 0x0086;
    pub const WM_GETDLGCODE: u32 = 0x0087;
    pub const WM_SYNCPAINT: u32 = 0x0088;
    pub const WM_NCMOUSEMOVE: u32 = 0x00A0;
    pub const WM_NCLBUTTONDOWN: u32 = 0x00A1;
    pub const WM_NCLBUTTONUP: u32 = 0x00A2;
    pub const WM_NCLBUTTONDBLCLK: u32 = 0x00A3;
    pub const WM_NCRBUTTONDOWN: u32 = 0x00A4;
    pub const WM_NCRBUTTONUP: u32 = 0x00A5;
    pub const WM_NCRBUTTONDBLCLK: u32 = 0x00A6;
    pub const WM_NCMBUTTONDOWN: u32 = 0x00A7;
    pub const WM_NCMBUTTONUP: u32 = 0x00A8;
    pub const WM_NCMBUTTONDBLCLK: u32 = 0x00A9;
    pub const WM_NCXBUTTONDOWN: u32 = 0x00AB;
    pub const WM_NCXBUTTONUP: u32 = 0x00AC;
    pub const WM_NCXBUTTONDBLCLK: u32 = 0x00AD;
    pub const WM_INPUT_DEVICE_CHANGE: u32 = 0x00FE;
    pub const WM_INPUT: u32 = 0x00FF;
    pub const WM_KEYFIRST: u32 = 0x0100;
    pub const WM_KEYDOWN: u32 = 0x0100;
    pub const WM_KEYUP: u32 = 0x0101;
    pub const WM_CHAR: u32 = 0x0102;
    pub const WM_DEADCHAR: u32 = 0x0103;
    pub const WM_SYSKEYDOWN: u32 = 0x0104;
    pub const WM_SYSKEYUP: u32 = 0x0105;
    pub const WM_SYSCHAR: u32 = 0x0106;
    pub const WM_SYSDEADCHAR: u32 = 0x0107;
    pub const WM_UNICHAR: u32 = 0x0109;
    pub const WM_KEYLAST: u32 = 0x0109;
    pub const WM_IME_STARTCOMPOSITION: u32 = 0x010D;
    pub const WM_IME_ENDCOMPOSITION: u32 = 0x010E;
    pub const WM_IME_COMPOSITION: u32 = 0x010F;
    pub const WM_IME_KEYLAST: u32 = 0x010F;
    pub const WM_INITDIALOG: u32 = 0x0110;
    pub const WM_COMMAND: u32 = 0x0111;
    pub const WM_SYSCOMMAND: u32 = 0x0112;
    pub const WM_TIMER: u32 = 0x0113;
    pub const WM_HSCROLL: u32 = 0x0114;
    pub const WM_VSCROLL: u32 = 0x0115;
    pub const WM_INITMENU: u32 = 0x0116;
    pub const WM_INITMENUPOPUP: u32 = 0x0117;
    pub const WM_MENUSELECT: u32 = 0x011F;
    pub const WM_MENUCHAR: u32 = 0x0120;
    pub const WM_ENTERIDLE: u32 = 0x0121;
    pub const WM_MENURBUTTONUP: u32 = 0x0122;
    pub const WM_MENUDRAG: u32 = 0x0123;
    pub const WM_MENUGETOBJECT: u32 = 0x0124;
    pub const WM_UNINITMENUPOPUP: u32 = 0x0125;
    pub const WM_MENUCOMMAND: u32 = 0x0126;
    pub const WM_CHANGEUISTATE: u32 = 0x0127;
    pub const WM_UPDATEUISTATE: u32 = 0x0128;
    pub const WM_QUERYUISTATE: u32 = 0x0129;
    pub const WM_CTLCOLORMSGBOX: u32 = 0x0132;
    pub const WM_CTLCOLOREDIT: u32 = 0x0133;
    pub const WM_CTLCOLORLISTBOX: u32 = 0x0134;
    pub const WM_CTLCOLORBTN: u32 = 0x0135;
    pub const WM_CTLCOLORDLG: u32 = 0x0136;
    pub const WM_CTLCOLORSCROLLBAR: u32 = 0x0137;
    pub const WM_CTLCOLORSTATIC: u32 = 0x0138;
    pub const WM_MOUSEFIRST: u32 = 0x0200;
    pub const WM_MOUSEMOVE: u32 = 0x0200;
    pub const WM_LBUTTONDOWN: u32 = 0x0201;
    pub const WM_LBUTTONUP: u32 = 0x0202;
    pub const WM_LBUTTONDBLCLK: u32 = 0x0203;
    pub const WM_RBUTTONDOWN: u32 = 0x0204;
    pub const WM_RBUTTONUP: u32 = 0x0205;
    pub const WM_RBUTTONDBLCLK: u32 = 0x0206;
    pub const WM_MBUTTONDOWN: u32 = 0x0207;
    pub const WM_MBUTTONUP: u32 = 0x0208;
    pub const WM_MBUTTONDBLCLK: u32 = 0x0209;
    pub const WM_MOUSEWHEEL: u32 = 0x020A;
    pub const WM_XBUTTONDOWN: u32 = 0x020B;
    pub const WM_XBUTTONUP: u32 = 0x020C;
    pub const WM_XBUTTONDBLCLK: u32 = 0x020D;
    pub const WM_MOUSEHWHEEL: u32 = 0x020E;
    pub const WM_MOUSELAST: u32 = 0x020E;
    pub const WM_PARENTNOTIFY: u32 = 0x0210;
    pub const WM_ENTERMENULOOP: u32 = 0x0211;
    pub const WM_EXITMENULOOP: u32 = 0x0212;
    pub const WM_NEXTMENU: u32 = 0x0213;
    pub const WM_SIZING: u32 = 0x0214;
    pub const WM_CAPTURECHANGED: u32 = 0x0215;
    pub const WM_MOVING: u32 = 0x0216;
    pub const WM_POWERBROADCAST: u32 = 0x0218;
    pub const WM_DEVICECHANGE: u32 = 0x0219;
    pub const WM_MDICREATE: u32 = 0x0220;
    pub const WM_MDIDESTROY: u32 = 0x0221;
    pub const WM_MDIACTIVATE: u32 = 0x0222;
    pub const WM_MDIRESTORE: u32 = 0x0223;
    pub const WM_MDINEXT: u32 = 0x0224;
    pub const WM_MDIMAXIMIZE: u32 = 0x0225;
    pub const WM_MDITILE: u32 = 0x0226;
    pub const WM_MDICASCADE: u32 = 0x0227;
    pub const WM_MDIICONARRANGE: u32 = 0x0228;
    pub const WM_MDIGETACTIVE: u32 = 0x0229;
    pub const WM_MDISETMENU: u32 = 0x0230;
    pub const WM_ENTERSIZEMOVE: u32 = 0x0231;
    pub const WM_EXITSIZEMOVE: u32 = 0x0232;
    pub const WM_DROPFILES: u32 = 0x0233;
    pub const WM_MDIREFRESHMENU: u32 = 0x0234;
    pub const WM_POINTERDEVICECHANGE: u32 = 0x0238;
    pub const WM_POINTERDEVICEINRANGE: u32 = 0x239;
    pub const WM_POINTERDEVICEOUTOFRANGE: u32 = 0x23A;
    pub const WM_NCPOINTERUPDATE: u32 = 0x0241;
    pub const WM_NCPOINTERDOWN: u32 = 0x0242;
    pub const WM_NCPOINTERUP: u32 = 0x0243;
    pub const WM_POINTERUPDATE: u32 = 0x0245;
    pub const WM_POINTERDOWN: u32 = 0x0246;
    pub const WM_POINTERUP: u32 = 0x0247;
    pub const WM_POINTERENTER: u32 = 0x0249;
    pub const WM_POINTERLEAVE: u32 = 0x024A;
    pub const WM_POINTERACTIVATE: u32 = 0x024B;
    pub const WM_POINTERCAPTURECHANGED: u32 = 0x024C;
    pub const WM_TOUCHHITTESTING: u32 = 0x024D;
    pub const WM_POINTERWHEEL: u32 = 0x024E;
    pub const WM_POINTERHWHEEL: u32 = 0x024F;
    pub const DM_POINTERHITTEST: u32 = 0x0250;
    pub const WM_IME_SETCONTEXT: u32 = 0x0281;
    pub const WM_IME_NOTIFY: u32 = 0x0282;
    pub const WM_IME_CONTROL: u32 = 0x0283;
    pub const WM_IME_COMPOSITIONFULL: u32 = 0x0284;
    pub const WM_IME_SELECT: u32 = 0x0285;
    pub const WM_IME_CHAR: u32 = 0x0286;
    pub const WM_IME_REQUEST: u32 = 0x0288;
    pub const WM_IME_KEYDOWN: u32 = 0x0290;
    pub const WM_IME_KEYUP: u32 = 0x0291;
    pub const WM_MOUSEHOVER: u32 = 0x02A1;
    pub const WM_MOUSELEAVE: u32 = 0x02A3;
    pub const WM_NCMOUSEHOVER: u32 = 0x02A0;
    pub const WM_NCMOUSELEAVE: u32 = 0x02A2;
    pub const WM_WTSSESSION_CHANGE: u32 = 0x02B1;
    pub const WM_TABLET_FIRST: u32 = 0x02c0;
    pub const WM_TABLET_LAST: u32 = 0x02df;
    pub const WM_DPICHANGED: u32 = 0x02E0;
    pub const WM_CUT: u32 = 0x0300;
    pub const WM_COPY: u32 = 0x0301;
    pub const WM_PASTE: u32 = 0x0302;
    pub const WM_CLEAR: u32 = 0x0303;
    pub const WM_UNDO: u32 = 0x0304;
    pub const WM_RENDERFORMAT: u32 = 0x0305;
    pub const WM_RENDERALLFORMATS: u32 = 0x0306;
    pub const WM_DESTROYCLIPBOARD: u32 = 0x0307;
    pub const WM_DRAWCLIPBOARD: u32 = 0x0308;
    pub const WM_PAINTCLIPBOARD: u32 = 0x0309;
    pub const WM_VSCROLLCLIPBOARD: u32 = 0x030A;
    pub const WM_SIZECLIPBOARD: u32 = 0x030B;
    pub const WM_ASKCBFORMATNAME: u32 = 0x030C;
    pub const WM_CHANGECBCHAIN: u32 = 0x030D;
    pub const WM_HSCROLLCLIPBOARD: u32 = 0x030E;
    pub const WM_QUERYNEWPALETTE: u32 = 0x030F;
    pub const WM_PALETTEISCHANGING: u32 = 0x0310;
    pub const WM_PALETTECHANGED: u32 = 0x0311;
    pub const WM_HOTKEY: u32 = 0x0312;
    pub const WM_PRINT: u32 = 0x0317;
    pub const WM_PRINTCLIENT: u32 = 0x0318;
    pub const WM_APPCOMMAND: u32 = 0x0319;
    pub const WM_THEMECHANGED: u32 = 0x031A;
    pub const WM_CLIPBOARDUPDATE: u32 = 0x031D;
    pub const WM_DWMCOMPOSITIONCHANGED: u32 = 0x031E;
    pub const WM_DWMNCRENDERINGCHANGED: u32 = 0x031F;
    pub const WM_DWMCOLORIZATIONCOLORCHANGED: u32 = 0x0320;
    pub const WM_DWMWINDOWMAXIMIZEDCHANGE: u32 = 0x0321;
    pub const WM_GETTITLEBARINFOEX: u32 = 0x033F;
    pub const WM_HANDHELDFIRST: u32 = 0x0358;
    pub const WM_HANDHELDLAST: u32 = 0x035F;
    pub const WM_AFXFIRST: u32 = 0x0360;
    pub const WM_AFXLAST: u32 = 0x037F;
    pub const WM_PENWINFIRST: u32 = 0x0380;
    pub const WM_PENWINLAST: u32 = 0x038F;
    pub const WM_TOUCH: u32 = 0x0240;
    pub const WM_APP: u32 = 0x8000;
    pub const WM_USER: u32 = 0x0400;
    pub const WM_DISPATCH_WORK_ITEM: u32 = Self::WM_USER;
}

/// `SystemParametersInfo` of the interop declarations: the values the system uses, as constants.
pub struct SystemParametersInfo;

#[allow(missing_docs)]
impl SystemParametersInfo {
    pub const SPI_SETWORKAREA: u32 = 0x002F;
}

/// `DwmWindowAttribute` of the interop declarations: the values the system uses, as constants.
pub struct DwmWindowAttribute;

#[allow(missing_docs)]
impl DwmWindowAttribute {
    pub const DWMWA_NCRENDERING_ENABLED: u32 = 1;
    pub const DWMWA_NCRENDERING_POLICY: u32 = Self::DWMWA_NCRENDERING_ENABLED + 1;
    pub const DWMWA_TRANSITIONS_FORCEDISABLED: u32 = Self::DWMWA_NCRENDERING_POLICY + 1;
    pub const DWMWA_ALLOW_NCPAINT: u32 = Self::DWMWA_TRANSITIONS_FORCEDISABLED + 1;
    pub const DWMWA_CAPTION_BUTTON_BOUNDS: u32 = Self::DWMWA_ALLOW_NCPAINT + 1;
    pub const DWMWA_NONCLIENT_RTL_LAYOUT: u32 = Self::DWMWA_CAPTION_BUTTON_BOUNDS + 1;
    pub const DWMWA_FORCE_ICONIC_REPRESENTATION: u32 = Self::DWMWA_NONCLIENT_RTL_LAYOUT + 1;
    pub const DWMWA_FLIP3D_POLICY: u32 = Self::DWMWA_FORCE_ICONIC_REPRESENTATION + 1;
    pub const DWMWA_EXTENDED_FRAME_BOUNDS: u32 = Self::DWMWA_FLIP3D_POLICY + 1;
    pub const DWMWA_HAS_ICONIC_BITMAP: u32 = Self::DWMWA_EXTENDED_FRAME_BOUNDS + 1;
    pub const DWMWA_DISALLOW_PEEK: u32 = Self::DWMWA_HAS_ICONIC_BITMAP + 1;
    pub const DWMWA_EXCLUDED_FROM_PEEK: u32 = Self::DWMWA_DISALLOW_PEEK + 1;
    pub const DWMWA_CLOAK: u32 = Self::DWMWA_EXCLUDED_FROM_PEEK + 1;
    pub const DWMWA_CLOAKED: u32 = Self::DWMWA_CLOAK + 1;
    pub const DWMWA_FREEZE_REPRESENTATION: u32 = Self::DWMWA_CLOAKED + 1;
    pub const DWMWA_PASSIVE_UPDATE_MODE: u32 = Self::DWMWA_FREEZE_REPRESENTATION + 1;
    pub const DWMWA_USE_HOSTBACKDROPBRUSH: u32 = Self::DWMWA_PASSIVE_UPDATE_MODE + 1;
    pub const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
    pub const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
    pub const DWMWA_BORDER_COLOR: u32 = Self::DWMWA_WINDOW_CORNER_PREFERENCE + 1;
    pub const DWMWA_CAPTION_COLOR: u32 = Self::DWMWA_BORDER_COLOR + 1;
    pub const DWMWA_TEXT_COLOR: u32 = Self::DWMWA_CAPTION_COLOR + 1;
    pub const DWMWA_VISIBLE_FRAME_BORDER_THICKNESS: u32 = Self::DWMWA_TEXT_COLOR + 1;
    pub const DWMWA_LAST: u32 = Self::DWMWA_VISIBLE_FRAME_BORDER_THICKNESS + 1;
}

/// `DwmWindowCornerPreference` of the interop declarations: the values the system uses, as constants.
pub struct DwmWindowCornerPreference;

#[allow(missing_docs)]
impl DwmWindowCornerPreference {
    pub const DWMWCP_DEFAULT: u32 = 0;
    pub const DWMWCP_DONOTROUND: u32 = Self::DWMWCP_DEFAULT + 1;
    pub const DWMWCP_ROUND: u32 = Self::DWMWCP_DONOTROUND + 1;
    pub const DWMWCP_ROUNDSMALL: u32 = Self::DWMWCP_ROUND + 1;
}

/// `DwmNCRenderingPolicy` of the interop declarations: the values the system uses, as constants.
pub struct DwmNCRenderingPolicy;

#[allow(missing_docs)]
impl DwmNCRenderingPolicy {
    pub const DWMNCRP_USEWINDOWSTYLE: u32 = 0;
    pub const DWMNCRP_DISABLED: u32 = Self::DWMNCRP_USEWINDOWSTYLE + 1;
    pub const DWMNCRP_ENABLED: u32 = Self::DWMNCRP_DISABLED + 1;
    pub const DWMNCRP_LAST: u32 = Self::DWMNCRP_ENABLED + 1;
}

/// `MapVirtualKeyMapTypes` of the interop declarations: the values the system uses, as constants.
pub struct MapVirtualKeyMapTypes;

#[allow(missing_docs)]
impl MapVirtualKeyMapTypes {
    pub const MAPVK_VK_TO_VSC: u32 = 0x00;
    pub const MAPVK_VSC_TO_VK: u32 = 0x01;
    pub const MAPVK_VK_TO_CHAR: u32 = 0x02;
    pub const MAPVK_VSC_TO_VK_EX: u32 = 0x03;
}

/// `WindowLongParam` of the interop declarations: the values the system uses, as constants.
pub struct WindowLongParam;

#[allow(missing_docs)]
impl WindowLongParam {
    pub const GWL_WNDPROC: i32 = -4;
    pub const GWL_HINSTANCE: i32 = -6;
    pub const GWL_HWNDPARENT: i32 = -8;
    pub const GWL_ID: i32 = -12;
    pub const GWL_STYLE: i32 = -16;
    pub const GWL_EXSTYLE: i32 = -20;
    pub const GWL_USERDATA: i32 = -21;
}

/// `MenuCharParam` of the interop declarations: the values the system uses, as constants.
pub struct MenuCharParam;

#[allow(missing_docs)]
impl MenuCharParam {
    pub const MNC_IGNORE: i32 = 0;
    pub const MNC_CLOSE: i32 = 1;
    pub const MNC_EXECUTE: i32 = 2;
    pub const MNC_SELECT: i32 = 3;
}

/// `SysCommands` of the interop declarations: the values the system uses, as constants.
pub struct SysCommands;

#[allow(missing_docs)]
impl SysCommands {
    pub const SC_SIZE: i32 = 0xF000;
    pub const SC_MOVE: i32 = 0xF010;
    pub const SC_MINIMIZE: i32 = 0xF020;
    pub const SC_MAXIMIZE: i32 = 0xF030;
    pub const SC_NEXTWINDOW: i32 = 0xF040;
    pub const SC_PREVWINDOW: i32 = 0xF050;
    pub const SC_CLOSE: i32 = 0xF060;
    pub const SC_VSCROLL: i32 = 0xF070;
    pub const SC_HSCROLL: i32 = 0xF080;
    pub const SC_MOUSEMENU: i32 = 0xF090;
    pub const SC_KEYMENU: i32 = 0xF100;
    pub const SC_ARRANGE: i32 = 0xF110;
    pub const SC_RESTORE: i32 = 0xF120;
    pub const SC_TASKLIST: i32 = 0xF130;
    pub const SC_SCREENSAVE: i32 = 0xF140;
    pub const SC_HOTKEY: i32 = 0xF150;
    pub const SC_DEFAULT: i32 = 0xF160;
    pub const SC_MONITORPOWER: i32 = 0xF170;
    pub const SC_CONTEXTHELP: i32 = 0xF180;
    pub const SC_SEPARATOR: i32 = 0xF00F;
    pub const SCF_ISSECURE: i32 = 0x00000001;
}

bitflags::bitflags! {
    /// `PointerFlags` of the interop declarations.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PointerFlags: u32 {
        const POINTER_FLAG_NONE = 0x00000000;
        const POINTER_FLAG_NEW = 0x00000001;
        const POINTER_FLAG_INRANGE = 0x00000002;
        const POINTER_FLAG_INCONTACT = 0x00000004;
        const POINTER_FLAG_FIRSTBUTTON = 0x00000010;
        const POINTER_FLAG_SECONDBUTTON = 0x00000020;
        const POINTER_FLAG_THIRDBUTTON = 0x00000040;
        const POINTER_FLAG_FOURTHBUTTON = 0x00000080;
        const POINTER_FLAG_FIFTHBUTTON = 0x00000100;
        const POINTER_FLAG_PRIMARY = 0x00002000;
        const POINTER_FLAG_CONFIDENCE = 0x00004000;
        const POINTER_FLAG_CANCELED = 0x00008000;
        const POINTER_FLAG_DOWN = 0x00010000;
        const POINTER_FLAG_UPDATE = 0x00020000;
        const POINTER_FLAG_UP = 0x00040000;
        const POINTER_FLAG_WHEEL = 0x00080000;
        const POINTER_FLAG_HWHEEL = 0x00100000;
        const POINTER_FLAG_CAPTURECHANGED = 0x00200000;
        const POINTER_FLAG_HASTRANSFORM = 0x00400000;
    }
}

/// `PointerButtonChangeType` of the interop declarations: the values the system uses, as constants.
pub struct PointerButtonChangeType;

#[allow(missing_docs)]
impl PointerButtonChangeType {
    pub const POINTER_CHANGE_NONE: u32 = 0;
    pub const POINTER_CHANGE_FIRSTBUTTON_DOWN: u32 = Self::POINTER_CHANGE_NONE + 1;
    pub const POINTER_CHANGE_FIRSTBUTTON_UP: u32 = Self::POINTER_CHANGE_FIRSTBUTTON_DOWN + 1;
    pub const POINTER_CHANGE_SECONDBUTTON_DOWN: u32 = Self::POINTER_CHANGE_FIRSTBUTTON_UP + 1;
    pub const POINTER_CHANGE_SECONDBUTTON_UP: u32 = Self::POINTER_CHANGE_SECONDBUTTON_DOWN + 1;
    pub const POINTER_CHANGE_THIRDBUTTON_DOWN: u32 = Self::POINTER_CHANGE_SECONDBUTTON_UP + 1;
    pub const POINTER_CHANGE_THIRDBUTTON_UP: u32 = Self::POINTER_CHANGE_THIRDBUTTON_DOWN + 1;
    pub const POINTER_CHANGE_FOURTHBUTTON_DOWN: u32 = Self::POINTER_CHANGE_THIRDBUTTON_UP + 1;
    pub const POINTER_CHANGE_FOURTHBUTTON_UP: u32 = Self::POINTER_CHANGE_FOURTHBUTTON_DOWN + 1;
    pub const POINTER_CHANGE_FIFTHBUTTON_DOWN: u32 = Self::POINTER_CHANGE_FOURTHBUTTON_UP + 1;
    pub const POINTER_CHANGE_FIFTHBUTTON_UP: u32 = Self::POINTER_CHANGE_FIFTHBUTTON_DOWN + 1;
}

bitflags::bitflags! {
    /// `PenFlags` of the interop declarations.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PenFlags: u32 {
        const PEN_FLAGS_NONE = 0x00000000;
        const PEN_FLAGS_BARREL = 0x00000001;
        const PEN_FLAGS_INVERTED = 0x00000002;
        const PEN_FLAGS_ERASER = 0x00000004;
    }
}

bitflags::bitflags! {
    /// `TouchMask` of the interop declarations.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct TouchMask: u32 {
        const TOUCH_MASK_NONE = 0x00000000;
        const TOUCH_MASK_CONTACTAREA = 0x00000001;
        const TOUCH_MASK_ORIENTATION = 0x00000002;
        const TOUCH_MASK_PRESSURE = 0x00000004;
    }
}

/// `ClassLongIndex` of the interop declarations: the values the system uses, as constants.
pub struct ClassLongIndex;

#[allow(missing_docs)]
impl ClassLongIndex {
    pub const GCLP_MENUNAME: i32 = -8;
    pub const GCLP_HBRBACKGROUND: i32 = -10;
    pub const GCLP_HCURSOR: i32 = -12;
    pub const GCLP_HICON: i32 = -14;
    pub const GCLP_HMODULE: i32 = -16;
    pub const GCL_CBWNDEXTRA: i32 = -18;
    pub const GCL_CBCLSEXTRA: i32 = -20;
    pub const GCLP_WNDPROC: i32 = -24;
    pub const GCL_STYLE: i32 = -26;
    pub const GCLP_HICONSM: i32 = -34;
    pub const GCW_ATOM: i32 = -32;
}

bitflags::bitflags! {
    /// `DWM_BB` of the interop declarations.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct DWM_BB: u32 {
        const ENABLE = 1;
        const BLUR_REGION = 2;
        const TRANSITION_MAXIMIZED = 4;
    }
}

bitflags::bitflags! {
    /// `QueueStatusFlags` of the interop declarations.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct QueueStatusFlags: u32 {
        const QS_KEY = 0x0001;
        const QS_MOUSEMOVE = 0x0002;
        const QS_MOUSEBUTTON = 0x0004;
        const QS_POSTMESSAGE = 0x0008;
        const QS_TIMER = 0x0010;
        const QS_PAINT = 0x0020;
        const QS_SENDMESSAGE = 0x0040;
        const QS_HOTKEY = 0x0080;
        const QS_ALLPOSTMESSAGE = 0x0100;
        const QS_EVENT = 0x02000;
        const QS_MOUSE = Self::QS_MOUSEMOVE.bits() | Self::QS_MOUSEBUTTON.bits();
        const QS_INPUT = Self::QS_MOUSE.bits() | Self::QS_KEY.bits();
        const QS_ALLEVENTS = Self::QS_INPUT.bits() | Self::QS_POSTMESSAGE.bits() | Self::QS_TIMER.bits() | Self::QS_PAINT.bits() | Self::QS_HOTKEY.bits();
        const QS_ALLINPUT = Self::QS_INPUT.bits() | Self::QS_POSTMESSAGE.bits() | Self::QS_TIMER.bits() | Self::QS_PAINT.bits() | Self::QS_HOTKEY.bits() | Self::QS_SENDMESSAGE.bits();
    }
}

bitflags::bitflags! {
    /// `MsgWaitForMultipleObjectsFlags` of the interop declarations.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct MsgWaitForMultipleObjectsFlags: u32 {
        const MWMO_WAITALL = 0x0001;
        const MWMO_ALERTABLE = 0x0002;
        const MWMO_INPUTAVAILABLE = 0x0004;
    }
}

/// `MONITOR` of the interop declarations: the values the system uses, as constants.
pub struct MONITOR;

#[allow(missing_docs)]
impl MONITOR {
    pub const MONITOR_DEFAULTTONULL: u32 = 0x00000000;
    pub const MONITOR_DEFAULTTOPRIMARY: u32 = 0x00000001;
    pub const MONITOR_DEFAULTTONEAREST: u32 = 0x00000002;
}

/// `DEVICECAP` of the interop declarations: the values the system uses, as constants.
pub struct DEVICECAP;

#[allow(missing_docs)]
impl DEVICECAP {
    pub const HORZRES: i32 = 8;
    pub const BITSPIXEL: i32 = 12;
    pub const PLANES: i32 = 14;
    pub const DESKTOPHORZRES: i32 = 118;
}

/// `PROCESS_DPI_AWARENESS` of the interop declarations: the values the system uses, as constants.
pub struct PROCESS_DPI_AWARENESS;

#[allow(missing_docs)]
impl PROCESS_DPI_AWARENESS {
    pub const PROCESS_DPI_UNAWARE: i32 = 0;
    pub const PROCESS_SYSTEM_DPI_AWARE: i32 = 1;
    pub const PROCESS_PER_MONITOR_DPI_AWARE: i32 = 2;
}

/// `MONITOR_DPI_TYPE` of the interop declarations: the values the system uses, as constants.
pub struct MONITOR_DPI_TYPE;

#[allow(missing_docs)]
impl MONITOR_DPI_TYPE {
    pub const MDT_EFFECTIVE_DPI: i32 = 0;
    pub const MDT_ANGULAR_DPI: i32 = 1;
    pub const MDT_RAW_DPI: i32 = 2;
    pub const MDT_DEFAULT: i32 = Self::MDT_EFFECTIVE_DPI;
}

/// `ClipboardFormat` of the interop declarations: the values the system uses, as constants.
pub struct ClipboardFormat;

#[allow(missing_docs)]
impl ClipboardFormat {
    pub const CF_BITMAP: u16 = 2;
    pub const CF_DIB: u16 = 8;
    pub const CF_UNICODETEXT: u16 = 13;
    pub const CF_HDROP: u16 = 15;
    pub const CF_DIBV5: u16 = 17;
}

bitflags::bitflags! {
    /// `WindowPlacementFlags` of the interop declarations.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct WindowPlacementFlags: u32 {
        const SET_MIN_POSITION = 0x0001;
        const RESTORE_TO_MAXIMIZED = 0x0002;
        const ASYNC_WINDOW_PLACEMENT = 0x0004;
    }
}

bitflags::bitflags! {
    /// `TouchInputFlags` of the interop declarations.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct TouchInputFlags: u32 {
        const TOUCHEVENTF_MOVE = 0x0001;
        const TOUCHEVENTF_DOWN = 0x0002;
        const TOUCHEVENTF_UP = 0x0004;
        const TOUCHEVENTF_INRANGE = 0x0008;
        const TOUCHEVENTF_PRIMARY = 0x0010;
        const TOUCHEVENTF_NOCOALESCE = 0x0020;
        const TOUCHEVENTF_PALM = 0x0080;
    }
}

/// `Icons` of the interop declarations: the values the system uses, as constants.
pub struct Icons;

#[allow(missing_docs)]
impl Icons {
    pub const ICON_SMALL: i32 = 0;
    pub const ICON_BIG: i32 = 1;
    pub const ICON_SMALL2: i32 = 2;
}

bitflags::bitflags! {
    /// `GlobalAllocFlags` of the interop declarations.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct GlobalAllocFlags: u32 {
        const GMEM_FIXED = 0x0000;
        const GMEM_MOVEABLE = 0x0002;
        const GMEM_ZEROINIT = 0x0040;
        const GPTR = Self::GMEM_FIXED.bits() | Self::GMEM_ZEROINIT.bits();
        const GHND = Self::GMEM_MOVEABLE.bits() | Self::GMEM_ZEROINIT.bits();
    }
}

/// A size in device pixels. The reference names its two numbers `X` and
/// `Y`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SIZE {
    #[allow(missing_docs)]
    pub x: i32,
    #[allow(missing_docs)]
    pub y: i32,
}

/// A size as two single-precision numbers.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SIZE_F {
    #[allow(missing_docs)]
    pub x: f32,
    #[allow(missing_docs)]
    pub y: f32,
}

/// The result codes the reference names (`HRESULT` of the interop
/// declarations).
pub struct HRESULT;

#[allow(missing_docs)]
impl HRESULT {
    pub const S_FALSE: u32 = 0x0001;
    pub const S_OK: u32 = 0x0000;
    pub const E_INVALIDARG: u32 = 0x8007_0057;
    pub const E_OUTOFMEMORY: u32 = 0x8007_000E;
    pub const E_NOTIMPL: u32 = 0x8000_4001;
    pub const E_UNEXPECTED: u32 = 0x8000_FFFF;
    pub const E_CANCELLED: u32 = 0x8007_04C7;
}

#[allow(missing_docs)]
pub const STG_E_MEDIUMFULL: u32 = 0x8003_0070;
#[allow(missing_docs)]
pub const DV_E_TYMED: u32 = 0x8004_0069;
#[allow(missing_docs)]
pub const DV_E_DVASPECT: u32 = 0x8004_006B;
#[allow(missing_docs)]
pub const DV_E_FORMATETC: u32 = 0x8004_0064;
#[allow(missing_docs)]
pub const OLE_E_ADVISENOTSUPPORTED: u32 = 0x8004_0003;
#[allow(missing_docs)]
pub const COR_E_OBJECTDISPOSED: u32 = 0x8013_1622;
#[allow(missing_docs)]
pub const STATFLAG_NONAME: i32 = 1;

bitflags::bitflags! {
    /// `TYMED`: the kinds of storage medium of a data transfer. The
    /// reference takes the type from the COM types of its runtime.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct TYMED: i32 {
        const TYMED_NULL = 0;
        const TYMED_HGLOBAL = 1;
        const TYMED_FILE = 2;
        const TYMED_ISTREAM = 4;
        const TYMED_ISTORAGE = 8;
        const TYMED_GDI = 16;
        const TYMED_MFPICT = 32;
        const TYMED_ENHMF = 64;
    }
}

bitflags::bitflags! {
    /// `DVASPECT`: the aspect of the data of a data transfer. The reference
    /// takes the type from the COM types of its runtime.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct DVASPECT: i32 {
        const DVASPECT_CONTENT = 1;
        const DVASPECT_THUMBNAIL = 2;
        const DVASPECT_ICON = 4;
        const DVASPECT_DOCPRINT = 8;
    }
}

/// `STGMEDIUM`: a storage medium of a data transfer: its kind, the handle
/// or the pointer of that kind, and the object that releases it.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct STGMEDIUM {
    #[allow(missing_docs)]
    pub tymed: TYMED,
    #[allow(missing_docs)]
    pub unionmember: isize,
    #[allow(missing_docs)]
    pub p_unk_for_release: isize,
}

/// `FORMATETC`: a format of a data transfer.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FORMATETC {
    #[allow(missing_docs)]
    pub cf_format: u16,
    #[allow(missing_docs)]
    pub ptd: isize,
    #[allow(missing_docs)]
    pub dw_aspect: DVASPECT,
    #[allow(missing_docs)]
    pub lindex: i32,
    #[allow(missing_docs)]
    pub tymed: TYMED,
}


/// Encodes a string as the null-terminated UTF-16 string the system takes.
pub fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Decodes a UTF-16 buffer up to its first null (or its end).
pub fn from_wide(value: &[u16]) -> String {
    let end = value.iter().position(|&c| c == 0).unwrap_or(value.len());
    String::from_utf16_lossy(&value[..end])
}

// --------------------------------------------------------------------
// Data transfer (the clipboard and drag and drop over OLE)
// --------------------------------------------------------------------

/// `DATADIR_GET`: the formats a data object can be asked for. The reference
/// takes the type from the COM types of its runtime.
pub const DATADIR_GET: i32 = 1;

/// `BitmapCompressionMode` of the interop declarations: the values the
/// system uses, as constants.
pub struct BitmapCompressionMode;

#[allow(missing_docs)]
impl BitmapCompressionMode {
    pub const BI_RGB: u32 = 0;
    pub const BI_RLE8: u32 = 1;
    pub const BI_RLE4: u32 = 2;
    pub const BI_BITFIELDS: u32 = 3;
    pub const BI_JPEG: u32 = 4;
    pub const BI_PNG: u32 = 5;
}

/// `BitmapColorSpace.LCS_sRGB` of the interop declarations.
pub const LCS_SRGB: u32 = 0x7352_4742;
/// `BitmapIntent.LCS_GM_ABS_COLORIMETRIC` of the interop declarations.
pub const LCS_GM_ABS_COLORIMETRIC: u32 = 8;

/// The size of a `BITMAPV5HEADER`, in bytes.
pub const SIZE_OF_BITMAPV5HEADER: u32 = 124;

/// `BITMAPV5HEADER`, with the members the reference sets; the others
/// (the resolution, the colour counts, the end points, the gamma values
/// and the profile) are zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct BITMAPV5HEADER {
    pub b_v5_width: i32,
    pub b_v5_height: i32,
    pub b_v5_planes: u16,
    pub b_v5_bit_count: u16,
    pub b_v5_compression: u32,
    pub b_v5_size_image: u32,
    pub b_v5_red_mask: u32,
    pub b_v5_green_mask: u32,
    pub b_v5_blue_mask: u32,
    pub b_v5_alpha_mask: u32,
    pub b_v5_cs_type: u32,
    pub b_v5_intent: u32,
}

impl BITMAPV5HEADER {
    /// The structure as the system lays it out, with its size as its first
    /// member (`Init` of the reference).
    pub fn to_bytes(&self) -> [u8; SIZE_OF_BITMAPV5HEADER as usize] {
        let mut bytes = [0u8; SIZE_OF_BITMAPV5HEADER as usize];
        let mut put = |offset: usize, value: &[u8]| bytes[offset..offset + value.len()].copy_from_slice(value);
        put(0, &SIZE_OF_BITMAPV5HEADER.to_le_bytes());
        put(4, &self.b_v5_width.to_le_bytes());
        put(8, &self.b_v5_height.to_le_bytes());
        put(12, &self.b_v5_planes.to_le_bytes());
        put(14, &self.b_v5_bit_count.to_le_bytes());
        put(16, &self.b_v5_compression.to_le_bytes());
        put(20, &self.b_v5_size_image.to_le_bytes());
        put(40, &self.b_v5_red_mask.to_le_bytes());
        put(44, &self.b_v5_green_mask.to_le_bytes());
        put(48, &self.b_v5_blue_mask.to_le_bytes());
        put(52, &self.b_v5_alpha_mask.to_le_bytes());
        put(56, &self.b_v5_cs_type.to_le_bytes());
        put(108, &self.b_v5_intent.to_le_bytes());
        bytes
    }
}

impl BITMAPINFOHEADER {
    /// The structure as the system lays it out.
    pub fn to_bytes(&self) -> [u8; SIZE_OF_BITMAPINFOHEADER as usize] {
        let mut bytes = [0u8; SIZE_OF_BITMAPINFOHEADER as usize];
        let mut put = |offset: usize, value: &[u8]| bytes[offset..offset + value.len()].copy_from_slice(value);
        put(0, &self.bi_size.to_le_bytes());
        put(4, &self.bi_width.to_le_bytes());
        put(8, &self.bi_height.to_le_bytes());
        put(12, &self.bi_planes.to_le_bytes());
        put(14, &self.bi_bit_count.to_le_bytes());
        put(16, &self.bi_compression.to_le_bytes());
        put(20, &self.bi_size_image.to_le_bytes());
        put(24, &self.bi_x_pels_per_meter.to_le_bytes());
        put(28, &self.bi_y_pels_per_meter.to_le_bytes());
        put(32, &self.bi_clr_used.to_le_bytes());
        put(36, &self.bi_clr_important.to_le_bytes());
        bytes
    }

    /// Reads the structure from the start of the bytes of a device
    /// independent bitmap; `None` when there are fewer bytes than it has.
    pub fn from_bytes(bytes: &[u8]) -> Option<BITMAPINFOHEADER> {
        if bytes.len() < SIZE_OF_BITMAPINFOHEADER as usize {
            return None;
        }
        let u32_at = |offset: usize| u32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]);
        let u16_at = |offset: usize| u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        Some(BITMAPINFOHEADER {
            bi_size: u32_at(0),
            bi_width: u32_at(4) as i32,
            bi_height: u32_at(8) as i32,
            bi_planes: u16_at(12),
            bi_bit_count: u16_at(14),
            bi_compression: u32_at(16),
            bi_size_image: u32_at(20),
            bi_x_pels_per_meter: u32_at(24) as i32,
            bi_y_pels_per_meter: u32_at(28) as i32,
            bi_clr_used: u32_at(32),
            bi_clr_important: u32_at(36),
        })
    }
}

/// The size of a `DROPFILES`, in bytes: the offset of the file names, a
/// point, and two flags of four bytes each.
pub const SIZE_OF_DROPFILES: u32 = 20;

/// `FILEDESCRIPTORW` of the interop declarations: its layout as constants.
pub struct FILEDESCRIPTORW;

impl FILEDESCRIPTORW {
    /// `FD_FILESIZE`: the size members are valid.
    pub const FD_FILESIZE: u32 = 0x0000_0040;
    /// The number of UTF-16 units of the name member (`MAX_PATH`).
    pub const FILE_NAME_LENGTH: usize = 260;
    /// The size of the structure, in bytes.
    pub const SIZE: usize = 592;
    /// The offset of `dwFlags`.
    pub const OFFSET_OF_FLAGS: usize = 0;
    /// The offset of `nFileSizeHigh`.
    pub const OFFSET_OF_FILE_SIZE_HIGH: usize = 64;
    /// The offset of `nFileSizeLow`.
    pub const OFFSET_OF_FILE_SIZE_LOW: usize = 68;
    /// The offset of `cFileName`.
    pub const OFFSET_OF_FILE_NAME: usize = 72;
}

/// `STATSTG`: what a stream reports of itself.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
#[allow(missing_docs)]
pub struct STATSTG {
    pub pwcs_name: isize,
    pub type_: u32,
    pub cb_size: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub atime: u64,
    pub grf_mode: u32,
    pub grf_locks_supported: u32,
    pub clsid: [u8; 16],
    pub grf_state_bits: u32,
    pub reserved: u32,
}

/// The pixels of a bitmap of the system, 32 bits each (blue first), rows
/// from the top.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BgraPixels {
    #[allow(missing_docs)]
    pub pixels: Vec<u8>,
    #[allow(missing_docs)]
    pub width: i32,
    #[allow(missing_docs)]
    pub height: i32,
}

/// `COMDLG_FILTERSPEC` of the interop declarations: a file type of a
/// file dialog, as two strings of UTF-16 code units with a terminator that
/// the caller keeps alive while the dialog reads them.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
#[allow(missing_docs)]
pub struct COMDLG_FILTERSPEC {
    pub psz_name: *const u16,
    pub psz_spec: *const u16,
}

/// `MessageFilterFlag` of the interop declarations: the values the system uses, as constants.
pub struct MessageFilterFlag;

#[allow(missing_docs)]
impl MessageFilterFlag {
    pub const MSGFLT_RESET: u32 = 0;
    pub const MSGFLT_ALLOW: u32 = 1;
    pub const MSGFLT_DISALLOW: u32 = 2;
}

/// `NIM` of the interop declarations: the values the system uses, as constants.
pub struct NIM;

#[allow(missing_docs)]
impl NIM {
    pub const ADD: u32 = 0x00000000;
    pub const MODIFY: u32 = 0x00000001;
    pub const DELETE: u32 = 0x00000002;
    pub const SETFOCUS: u32 = 0x00000003;
    pub const SETVERSION: u32 = 0x00000004;
}

/// `AppBarMessage` of the interop declarations: the values the system uses, as constants.
pub struct AppBarMessage;

#[allow(missing_docs)]
impl AppBarMessage {
    pub const ABM_GETSTATE: u32 = 0x00000004;
    pub const ABM_GETTASKBARPOS: u32 = 0x00000005;
}

bitflags::bitflags! {
    /// `NIF` of the interop declarations.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct NIF: u32 {
        const MESSAGE = 0x00000001;
        const ICON = 0x00000002;
        const TIP = 0x00000004;
        const STATE = 0x00000008;
        const INFO = 0x00000010;
        const GUID = 0x00000020;
        const REALTIME = 0x00000040;
        const SHOWTIP = 0x00000080;
    }
}

/// `APPBARDATA` of the interop declarations.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct APPBARDATA {
    pub cb_size: i32,
    pub h_wnd: isize,
    pub u_callback_message: u32,
    pub u_edge: u32,
    pub rc: RECT,
    pub l_param: i32,
}

impl Default for APPBARDATA {
    fn default() -> Self {
        Self {
            cb_size: std::mem::size_of::<APPBARDATA>() as i32,
            h_wnd: 0,
            u_callback_message: 0,
            u_edge: 0,
            rc: RECT::default(),
            l_param: 0,
        }
    }
}

/// `NOTIFYICONDATA` of the interop declarations: the structure of a
/// notification icon up to the flags of its balloon, which is the version
/// of the structure its size announces to the system. The three texts are
/// buffers of UTF-16 code units with a terminator.
#[repr(C)]
#[derive(Clone, Copy)]
#[allow(missing_docs)]
pub struct NOTIFYICONDATA {
    pub cb_size: i32,
    pub h_wnd: isize,
    pub u_id: i32,
    pub u_flags: u32,
    pub u_callback_message: i32,
    pub h_icon: isize,
    pub sz_tip: [u16; 128],
    pub dw_state: i32,
    pub dw_state_mask: i32,
    pub sz_info: [u16; 256],
    pub u_timeout_or_version: i32,
    pub sz_info_title: [u16; 64],
    pub dw_info_flags: u32,
}

impl Default for NOTIFYICONDATA {
    fn default() -> Self {
        Self {
            cb_size: std::mem::size_of::<NOTIFYICONDATA>() as i32,
            h_wnd: 0,
            u_id: 0,
            u_flags: 0,
            u_callback_message: 0,
            h_icon: 0,
            sz_tip: [0; 128],
            dw_state: 0,
            dw_state_mask: 0,
            sz_info: [0; 256],
            u_timeout_or_version: 0,
            sz_info_title: [0; 64],
            dw_info_flags: 0,
        }
    }
}

impl NOTIFYICONDATA {
    /// Sets the tip: as much of the text as fits the buffer with its
    /// terminator, cut between characters (the marshaling of the reference
    /// cuts between code units).
    pub fn set_tip(&mut self, text: &str) {
        self.sz_tip = [0; 128];
        let mut length = 0;
        let mut units = [0u16; 2];
        for character in text.chars() {
            let encoded = character.encode_utf16(&mut units);
            if length + encoded.len() > self.sz_tip.len() - 1 {
                break;
            }
            self.sz_tip[length..length + encoded.len()].copy_from_slice(encoded);
            length += encoded.len();
        }
    }
}

/// A path of the display configuration (`DISPLAYCONFIG_PATH_INFO`), as far
/// as the backend reads it: the adapter of its target (the two halves of
/// its identifier), and the identifiers of its source and of its target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayConfigPath {
    #[allow(missing_docs)]
    pub adapter_id: (u32, i32),
    #[allow(missing_docs)]
    pub source_id: u32,
    #[allow(missing_docs)]
    pub target_id: u32,
}

/// The class and interface identifiers of the shell the backend names.
#[allow(non_snake_case)]
pub mod ShellIds {
    use ferroui_microcom::Guid;

    #[allow(missing_docs)]
    pub const OPEN_FILE_DIALOG: Guid = Guid::from_u128(0xDC1C5A9C_E88A_4DDE_A5A1_60F82A20AEF7);
    #[allow(missing_docs)]
    pub const SAVE_FILE_DIALOG: Guid = Guid::from_u128(0xC0B4E2F3_BA21_4773_8DBA_335EC946EB8B);
    #[allow(missing_docs)]
    pub const I_FILE_DIALOG: Guid = Guid::from_u128(0x42F85136_DB7E_439C_85F1_E4075D135FC8);
    #[allow(missing_docs)]
    pub const I_SHELL_ITEM: Guid = Guid::from_u128(0x43826D1E_E718_42EE_BC55_A1E261C37BFE);
    #[allow(missing_docs)]
    pub const TASK_BAR_LIST: Guid = Guid::from_u128(0x56FDF344_FD6D_11D0_958A_006097C9A090);
    /// The identifier the reference asks the task bar list for under this
    /// name; it is the one of the third version of the interface.
    pub const I_TASK_BAR_LIST2: Guid = Guid::from_u128(0xea1afb91_9e28_4b86_90e9_9e9f8a5eefaf);
}

#[cfg(windows)]
pub use native::*;

/// The system calls. Every function here is safe to call: it takes and
/// returns values, owns the buffers the system writes to, and states why
/// its call is sound.
#[cfg(windows)]
mod native {
    use super::*;
    use std::ffi::c_void;
    use windows_sys::Win32::Devices::Display as display;
    use windows_sys::Win32::Foundation as wf;
    use windows_sys::Win32::Graphics::Dwm as dwm;
    use windows_sys::Win32::Graphics::Gdi as gdi;
    use windows_sys::Win32::Storage::FileSystem as fs;
    use windows_sys::Win32::System::DataExchange as dx;
    use windows_sys::Win32::System::Com as com;
    use windows_sys::Win32::System::Com::Marshal as marshal;
    use windows_sys::Win32::System::Com::StructuredStorage as storage;
    use windows_sys::Win32::System::LibraryLoader as ll;
    use windows_sys::Win32::System::Ole as ole;
    use windows_sys::Win32::System::Memory as mem;
    use windows_sys::Win32::System::SystemInformation as si;
    use windows_sys::Win32::UI::Controls as ctl;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse as kbm;
    use windows_sys::Win32::UI::Input::Pointer as ptr_input;
    use windows_sys::Win32::UI::Input::Touch as touch;
    use windows_sys::Win32::UI::Shell as shell;
    use windows_sys::Win32::UI::WindowsAndMessaging as wm;

    /// The signature of a window procedure.
    pub type WndProc = unsafe extern "system" fn(hwnd: wf::HWND, msg: u32, w_param: usize, l_param: isize) -> isize;

    fn h(handle: isize) -> *mut c_void {
        handle as *mut c_void
    }

    fn to_rect(rect: wf::RECT) -> RECT {
        RECT { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom }
    }

    fn from_rect(rect: RECT) -> wf::RECT {
        wf::RECT { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom }
    }

    /// The handle of a window as the number the backend keeps.
    pub fn hwnd_to_isize(hwnd: wf::HWND) -> isize {
        hwnd as isize
    }

    /// The error code of the last system call that failed on this thread.
    pub fn get_last_error() -> u32 {
        // SAFETY: reads the error slot of the calling thread.
        unsafe { wf::GetLastError() }
    }

    /// The identifier of the calling thread.
    pub fn get_current_thread_id() -> u32 {
        // SAFETY: no arguments, no preconditions.
        unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() }
    }

    /// The handle of the module the process was started from (`None`), or of
    /// a module that is loaded under `name`; 0 if it is not loaded.
    pub fn get_module_handle(name: Option<&str>) -> isize {
        let name = name.map(to_wide);
        // SAFETY: the name is null or a null-terminated string that lives
        // through the call.
        unsafe { ll::GetModuleHandleW(name.as_ref().map_or(std::ptr::null(), |n| n.as_ptr())) as isize }
    }

    /// Loads a library of the system by name; 0 if it cannot be loaded.
    pub fn load_library(name: &str) -> isize {
        let name = to_wide(name);
        // SAFETY: a null-terminated string that lives through the call.
        unsafe { ll::LoadLibraryW(name.as_ptr()) as isize }
    }

    /// The address of an export of a loaded module, if it has one.
    pub fn get_proc_address(module: isize, name: &std::ffi::CStr) -> Option<unsafe extern "system" fn() -> isize> {
        if module == 0 {
            return None;
        }
        // SAFETY: the module handle came from the loader and the name is a
        // null-terminated string.
        unsafe { ll::GetProcAddress(h(module), name.as_ptr().cast()) }
    }

    /// The version of the system as `RtlGetVersion` reports it: major,
    /// minor, build. Zeros if the call fails.
    pub fn rtl_get_version() -> (u32, u32, u32) {
        // On 32-bit x86 the export has no decoration, where the "system"
        // calling convention would add one to the name.
        #[cfg_attr(not(target_arch = "x86"), link(name = "ntdll", kind = "raw-dylib"))]
        #[cfg_attr(target_arch = "x86", link(name = "ntdll", kind = "raw-dylib", import_name_type = "undecorated"))]
        extern "system" {
            fn RtlGetVersion(info: *mut si::OSVERSIONINFOW) -> i32;
        }

        // SAFETY: the structure is zeroed plain data.
        let mut info: si::OSVERSIONINFOW = unsafe { std::mem::zeroed() };
        info.dwOSVersionInfoSize = std::mem::size_of::<si::OSVERSIONINFOW>() as u32;
        // SAFETY: a valid structure with its size set; the function fills it.
        let status = unsafe { RtlGetVersion(&mut info) };
        if status != 0 {
            return (0, 0, 0);
        }
        (info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber)
    }

    /// Whether the process is ending (`RtlDllShutdownInProgress`): the
    /// system has ended every thread but the calling one, wherever each
    /// was, and what is still destroyed must not wait for any of them.
    pub fn process_is_shutting_down() -> bool {
        #[cfg_attr(not(target_arch = "x86"), link(name = "ntdll", kind = "raw-dylib"))]
        #[cfg_attr(target_arch = "x86", link(name = "ntdll", kind = "raw-dylib", import_name_type = "undecorated"))]
        extern "system" {
            fn RtlDllShutdownInProgress() -> u8;
        }

        // SAFETY: takes nothing and reads a flag of the loader.
        unsafe { RtlDllShutdownInProgress() != 0 }
    }

    // ----------------------------------------------------------------
    // DPI awareness
    // ----------------------------------------------------------------

    /// Calls `SetProcessDpiAwarenessContext` if this version of the system
    /// has it; `None` if it does not.
    pub fn set_process_dpi_awareness_context(context: isize) -> Option<bool> {
        let function = get_proc_address(load_library("user32.dll"), c"SetProcessDpiAwarenessContext")?;
        // SAFETY: the export has this signature on every version of the
        // system that has it (Windows 10 1703 and later).
        let function: unsafe extern "system" fn(*mut c_void) -> i32 = unsafe { std::mem::transmute(function) };
        // SAFETY: the argument is one of the predefined context values.
        Some(unsafe { function(h(context)) } != 0)
    }

    /// Calls `SetProcessDpiAwareness` of shcore.dll if this version of the
    /// system has it; `None` if it does not.
    pub fn set_process_dpi_awareness(awareness: i32) -> Option<i32> {
        let function = get_proc_address(load_library("shcore.dll"), c"SetProcessDpiAwareness")?;
        // SAFETY: the export has this signature (Windows 8.1 and later).
        let function: unsafe extern "system" fn(i32) -> i32 = unsafe { std::mem::transmute(function) };
        // SAFETY: a plain value argument.
        Some(unsafe { function(awareness) })
    }

    /// `SetProcessDPIAware`.
    pub fn set_process_dpi_aware() -> bool {
        // SAFETY: no arguments, no preconditions.
        unsafe { wm::SetProcessDPIAware() != 0 }
    }

    /// Whether shcore.dll has `GetDpiForMonitor`.
    pub fn has_get_dpi_for_monitor() -> bool {
        get_proc_address(load_library("shcore.dll"), c"GetDpiForMonitor").is_some()
    }

    /// The DPI of a monitor (`GetDpiForMonitor`), horizontal and vertical;
    /// `None` if the system does not have the function or the call fails.
    pub fn get_dpi_for_monitor(monitor: isize, dpi_type: i32) -> Option<(u32, u32)> {
        let function = get_proc_address(load_library("shcore.dll"), c"GetDpiForMonitor")?;
        // SAFETY: the export has this signature (Windows 8.1 and later).
        let function: unsafe extern "system" fn(*mut c_void, i32, *mut u32, *mut u32) -> i32 =
            unsafe { std::mem::transmute(function) };
        let (mut x, mut y) = (0u32, 0u32);
        // SAFETY: both out pointers are valid for the call.
        let result = unsafe { function(h(monitor), dpi_type, &mut x, &mut y) };
        (result == 0).then_some((x, y))
    }

    /// `AdjustWindowRectEx`: grows a client rectangle to the window
    /// rectangle of the given styles.
    pub fn adjust_window_rect_ex(rect: &mut RECT, style: u32, menu: bool, ex_style: u32) -> bool {
        let mut native = from_rect(*rect);
        // SAFETY: a valid rectangle the function rewrites.
        let ok = unsafe { wm::AdjustWindowRectEx(&mut native, style, menu as i32, ex_style) } != 0;
        *rect = to_rect(native);
        ok
    }

    /// `AdjustWindowRectExForDpi` if this version of the system has it
    /// (Windows 10 1607 and later); `None` if it does not.
    pub fn adjust_window_rect_ex_for_dpi(rect: &mut RECT, style: u32, menu: bool, ex_style: u32, dpi: u32) -> Option<bool> {
        let function = get_proc_address(load_library("user32.dll"), c"AdjustWindowRectExForDpi")?;
        // SAFETY: the export has this signature.
        let function: unsafe extern "system" fn(*mut wf::RECT, u32, i32, u32, u32) -> i32 =
            unsafe { std::mem::transmute(function) };
        let mut native = from_rect(*rect);
        // SAFETY: a valid rectangle the function rewrites.
        let ok = unsafe { function(&mut native, style, menu as i32, ex_style, dpi) } != 0;
        *rect = to_rect(native);
        Some(ok)
    }

    // ----------------------------------------------------------------
    // Window classes and windows
    // ----------------------------------------------------------------

    /// Registers a window class; returns its atom, 0 on failure.
    pub fn register_class_ex(class_name: &str, style: u32, wnd_proc: WndProc, cursor: isize) -> u16 {
        let class_name = to_wide(class_name);
        let class = wm::WNDCLASSEXW {
            cbSize: std::mem::size_of::<wm::WNDCLASSEXW>() as u32,
            style,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: h(get_module_handle(None)),
            hIcon: std::ptr::null_mut(),
            hCursor: h(cursor),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        // SAFETY: the structure is complete and the name lives through the
        // call, which copies it. The window procedure is a function of this
        // crate that stays loaded as long as the process.
        unsafe { wm::RegisterClassExW(&class) }
    }

    /// Unregisters a window class of this module.
    pub fn unregister_class(class_name: &str) -> bool {
        let class_name = to_wide(class_name);
        // SAFETY: a null-terminated name that lives through the call.
        unsafe { wm::UnregisterClassW(class_name.as_ptr(), h(get_module_handle(None))) != 0 }
    }

    /// Creates a window of a registered class without a title; 0 on
    /// failure.
    #[allow(clippy::too_many_arguments)]
    pub fn create_window_ex(ex_style: u32, atom: u16, style: u32, x: i32, y: i32, width: i32, height: i32, parent: isize) -> isize {
        // SAFETY: the class is named by its atom (the documented form of a
        // class name with the atom in the low word); the other pointers are
        // null. The window procedure of the class is called during the
        // call, on this thread.
        unsafe {
            wm::CreateWindowExW(
                ex_style,
                atom as usize as *const u16,
                std::ptr::null(),
                style,
                x,
                y,
                width,
                height,
                h(parent),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            ) as isize
        }
    }

    /// Destroys a window of the calling thread.
    pub fn destroy_window(hwnd: isize) -> bool {
        // SAFETY: a stale handle makes the call fail; nothing is
        // dereferenced on this side.
        unsafe { wm::DestroyWindow(h(hwnd)) != 0 }
    }

    /// The default processing of a window message.
    pub fn def_window_proc(hwnd: isize, msg: u32, w_param: usize, l_param: isize) -> isize {
        // SAFETY: called with the arguments of a message of this window, or
        // with a message whose parameters carry no pointers.
        unsafe { wm::DefWindowProcW(h(hwnd), msg, w_param, l_param) }
    }

    /// A message taken from the queue of the thread.
    pub struct Msg(wm::MSG);

    /// Waits for a message of the calling thread. Greater than zero for a
    /// message, zero for `WM_QUIT`, negative for an error.
    pub fn get_message() -> (i32, Msg) {
        // SAFETY: zeroed plain data, filled by the call.
        let mut msg: wm::MSG = unsafe { std::mem::zeroed() };
        // SAFETY: a valid out structure; no window or range filter.
        let result = unsafe { wm::GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) };
        (result, Msg(msg))
    }

    /// Posts the character messages of a key message.
    pub fn translate_message(msg: &Msg) {
        // SAFETY: a message `get_message` filled.
        unsafe { wm::TranslateMessage(&msg.0) };
    }

    /// Calls the window procedure a message is for.
    pub fn dispatch_message(msg: &Msg) {
        // SAFETY: a message `get_message` filled.
        unsafe { wm::DispatchMessageW(&msg.0) };
    }

    /// Posts a message to the queue of the thread of a window. May be called
    /// from any thread.
    pub fn post_message(hwnd: isize, msg: u32, w_param: usize, l_param: isize) -> bool {
        // SAFETY: the parameters are plain numbers for the messages this
        // backend posts.
        unsafe { wm::PostMessageW(h(hwnd), msg, w_param, l_param) != 0 }
    }

    /// Sends a message to a window and waits for its result.
    pub fn send_message(hwnd: isize, msg: u32, w_param: usize, l_param: isize) -> isize {
        // SAFETY: the parameters are plain numbers or handles for the
        // messages this backend sends.
        unsafe { wm::SendMessageW(h(hwnd), msg, w_param, l_param) }
    }

    /// Starts (or restarts) the timer `id` of a window: `WM_TIMER` after
    /// `elapse` milliseconds, repeatedly.
    pub fn set_timer(hwnd: isize, id: usize, elapse: u32) -> usize {
        // SAFETY: no timer procedure: the timer posts `WM_TIMER`.
        unsafe { wm::SetTimer(h(hwnd), id, elapse, None) }
    }

    /// Stops the timer `id` of a window.
    pub fn kill_timer(hwnd: isize, id: usize) -> bool {
        // SAFETY: plain values.
        unsafe { wm::KillTimer(h(hwnd), id) != 0 }
    }

    /// `MsgWaitForMultipleObjectsEx` without handles: waits up to `timeout`
    /// milliseconds for input of the kinds of `wake_mask`. Returns the wait
    /// result (0 when input is available).
    pub fn msg_wait_for_multiple_objects_ex(timeout: u32, wake_mask: QueueStatusFlags, flags: MsgWaitForMultipleObjectsFlags) -> u32 {
        // SAFETY: a count of zero with a null array of handles.
        unsafe { wm::MsgWaitForMultipleObjectsEx(0, std::ptr::null(), timeout, wake_mask.bits(), flags.bits()) }
    }

    /// `ShowWindow` with a [`ShowWindowCommand`].
    pub fn show_window(hwnd: isize, command: i32) -> bool {
        // SAFETY: plain values.
        unsafe { wm::ShowWindow(h(hwnd), command) != 0 }
    }

    #[allow(missing_docs)]
    pub fn is_window_visible(hwnd: isize) -> bool {
        // SAFETY: plain values.
        unsafe { wm::IsWindowVisible(h(hwnd)) != 0 }
    }

    #[allow(missing_docs)]
    pub fn is_window(hwnd: isize) -> bool {
        // SAFETY: plain values.
        unsafe { wm::IsWindow(h(hwnd)) != 0 }
    }

    /// The client rectangle of a window; empty if the call fails.
    pub fn get_client_rect(hwnd: isize) -> RECT {
        let mut rect = wf::RECT { left: 0, top: 0, right: 0, bottom: 0 };
        // SAFETY: a valid out rectangle. May be called from any thread.
        unsafe { wm::GetClientRect(h(hwnd), &mut rect) };
        to_rect(rect)
    }

    /// The rectangle of a window on the screen; empty if the call fails.
    pub fn get_window_rect(hwnd: isize) -> RECT {
        let mut rect = wf::RECT { left: 0, top: 0, right: 0, bottom: 0 };
        // SAFETY: a valid out rectangle.
        unsafe { wm::GetWindowRect(h(hwnd), &mut rect) };
        to_rect(rect)
    }

    #[allow(missing_docs)]
    pub fn set_window_pos(hwnd: isize, insert_after: isize, x: i32, y: i32, cx: i32, cy: i32, flags: SetWindowPosFlags) -> bool {
        // SAFETY: plain values; the window procedure is called during the
        // call, on this thread.
        unsafe { wm::SetWindowPos(h(hwnd), h(insert_after), x, y, cx, cy, flags.bits()) != 0 }
    }

    /// The placement of a window; the default placement if the call fails.
    pub fn get_window_placement(hwnd: isize) -> WINDOWPLACEMENT {
        // SAFETY: zeroed plain data.
        let mut native: wm::WINDOWPLACEMENT = unsafe { std::mem::zeroed() };
        native.length = std::mem::size_of::<wm::WINDOWPLACEMENT>() as u32;
        // SAFETY: a valid out structure with its length set.
        unsafe { wm::GetWindowPlacement(h(hwnd), &mut native) };
        WINDOWPLACEMENT {
            length: native.length,
            flags: native.flags,
            show_cmd: native.showCmd as i32,
            min_position: POINT { x: native.ptMinPosition.x, y: native.ptMinPosition.y },
            max_position: POINT { x: native.ptMaxPosition.x, y: native.ptMaxPosition.y },
            normal_position: to_rect(native.rcNormalPosition),
        }
    }

    #[allow(missing_docs)]
    pub fn set_window_placement(hwnd: isize, placement: &WINDOWPLACEMENT) -> bool {
        let native = wm::WINDOWPLACEMENT {
            length: std::mem::size_of::<wm::WINDOWPLACEMENT>() as u32,
            flags: placement.flags,
            showCmd: placement.show_cmd as u32,
            ptMinPosition: wf::POINT { x: placement.min_position.x, y: placement.min_position.y },
            ptMaxPosition: wf::POINT { x: placement.max_position.x, y: placement.max_position.y },
            rcNormalPosition: from_rect(placement.normal_position),
        };
        // SAFETY: a complete structure that lives through the call.
        unsafe { wm::SetWindowPlacement(h(hwnd), &native) != 0 }
    }

    /// `GetWindowLong` with a [`WindowLongParam`].
    pub fn get_window_long(hwnd: isize, index: i32) -> u32 {
        // SAFETY: plain values.
        unsafe { wm::GetWindowLongW(h(hwnd), index) as u32 }
    }

    /// `SetWindowLong` with a [`WindowLongParam`]; returns the old value.
    pub fn set_window_long(hwnd: isize, index: i32, value: u32) -> u32 {
        // SAFETY: plain values: the indices this backend sets are the
        // styles, which are numbers.
        unsafe { wm::SetWindowLongW(h(hwnd), index, value as i32) as u32 }
    }

    /// `SetWindowLongPtr` with a [`WindowLongParam`]; returns the old value.
    pub fn set_window_long_ptr(hwnd: isize, index: i32, value: isize) -> isize {
        #[cfg(target_pointer_width = "64")]
        // SAFETY: the index this backend sets is the owner window, whose
        // value is a window handle.
        unsafe {
            wm::SetWindowLongPtrW(h(hwnd), index, value)
        }
        #[cfg(target_pointer_width = "32")]
        // SAFETY: see above; on a 32-bit system the function is
        // `SetWindowLong`.
        unsafe {
            wm::SetWindowLongW(h(hwnd), index, value as i32) as isize
        }
    }

    /// Sets the title of a window; `None` clears it.
    pub fn set_window_text(hwnd: isize, text: Option<&str>) -> bool {
        let text = text.map(to_wide);
        // SAFETY: null or a null-terminated string that lives through the
        // call.
        unsafe { wm::SetWindowTextW(h(hwnd), text.as_ref().map_or(std::ptr::null(), |t| t.as_ptr())) != 0 }
    }

    /// Loads a cursor of a module by its resource identifier; `module` 0
    /// names the cursors of the system.
    pub fn load_cursor(module: isize, id: i32) -> isize {
        // SAFETY: the identifier is passed as an integer resource (the
        // documented form: the value in the low word of the pointer).
        unsafe { wm::LoadCursorW(h(module), id as u16 as usize as *const u16) as isize }
    }

    /// Makes a cursor the cursor of the pointer; returns the previous one.
    pub fn set_cursor(cursor: isize) -> isize {
        // SAFETY: a cursor handle or 0.
        unsafe { wm::SetCursor(h(cursor)) as isize }
    }

    /// `SetClassLongPtr` with a [`ClassLongIndex`].
    pub fn set_class_long_ptr(hwnd: isize, index: i32, value: isize) -> usize {
        #[cfg(target_pointer_width = "64")]
        // SAFETY: the index this backend sets is the class cursor, whose
        // value is a cursor handle.
        unsafe {
            wm::SetClassLongPtrW(h(hwnd), index, value)
        }
        #[cfg(target_pointer_width = "32")]
        // SAFETY: see above; on a 32-bit system the function is
        // `SetClassLong`.
        unsafe {
            wm::SetClassLongW(h(hwnd), index, value as i32) as usize
        }
    }

    /// `GetClassLongPtr` with a [`ClassLongIndex`].
    pub fn get_class_long_ptr(hwnd: isize, index: i32) -> usize {
        #[cfg(target_pointer_width = "64")]
        // SAFETY: plain values.
        unsafe {
            wm::GetClassLongPtrW(h(hwnd), index)
        }
        #[cfg(target_pointer_width = "32")]
        // SAFETY: plain values; on a 32-bit system the function is
        // `GetClassLong`.
        unsafe {
            wm::GetClassLongW(h(hwnd), index) as usize
        }
    }

    /// The window of the calling thread that has the keyboard focus.
    pub fn get_focus() -> isize {
        // SAFETY: no arguments.
        unsafe { kbm::GetFocus() as isize }
    }

    /// `GetAncestor`: an ancestor of a window (`GA_PARENT` 1, `GA_ROOT` 2,
    /// `GA_ROOTOWNER` 3).
    pub fn get_ancestor(hwnd: isize, flags: u32) -> isize {
        // SAFETY: plain values.
        unsafe { wm::GetAncestor(h(hwnd), flags) as isize }
    }

    /// `GetSystemMetrics` with a [`SystemMetric`].
    pub fn get_system_metrics(index: i32) -> i32 {
        // SAFETY: plain values.
        unsafe { wm::GetSystemMetrics(index) }
    }

    #[allow(missing_docs)]
    pub fn set_foreground_window(hwnd: isize) -> bool {
        // SAFETY: plain values.
        unsafe { wm::SetForegroundWindow(h(hwnd)) != 0 }
    }

    /// Gives a window of the calling thread the keyboard focus.
    pub fn set_focus(hwnd: isize) -> isize {
        // SAFETY: plain values.
        unsafe { kbm::SetFocus(h(hwnd)) as isize }
    }

    /// The active window of the calling thread.
    pub fn get_active_window() -> isize {
        // SAFETY: no arguments.
        unsafe { kbm::GetActiveWindow() as isize }
    }

    #[allow(missing_docs)]
    pub fn set_active_window(hwnd: isize) -> isize {
        // SAFETY: plain values.
        unsafe { kbm::SetActiveWindow(h(hwnd)) as isize }
    }

    #[allow(missing_docs)]
    pub fn enable_window(hwnd: isize, enable: bool) -> bool {
        // SAFETY: plain values.
        unsafe { kbm::EnableWindow(h(hwnd), enable as i32) != 0 }
    }

    /// The time of the message the thread is processing, in milliseconds.
    pub fn get_message_time() -> i32 {
        // SAFETY: no arguments.
        unsafe { wm::GetMessageTime() }
    }

    /// The extra information of the message the thread is processing.
    pub fn get_message_extra_info() -> isize {
        // SAFETY: no arguments.
        unsafe { wm::GetMessageExtraInfo() }
    }

    /// Calls `callback` for every top-level window, top of the z-order
    /// first, until it returns `false`.
    pub fn enum_windows(callback: &mut dyn FnMut(isize) -> bool) {
        unsafe extern "system" fn thunk(hwnd: wf::HWND, l_param: isize) -> i32 {
            // SAFETY: `l_param` is the pointer to the callback reference
            // that `enum_windows` passes below and that outlives the
            // enumeration; the enumeration calls back on the same thread
            // before `EnumChildWindows` returns.
            let callback = unsafe { &mut *(l_param as *mut &mut dyn FnMut(isize) -> bool) };
            callback(hwnd as isize) as i32
        }

        let mut callback = callback;
        // SAFETY: the callback pointer is valid for the call (see the
        // thunk). A null parent enumerates the top-level windows.
        unsafe {
            wm::EnumChildWindows(std::ptr::null_mut(), Some(thunk), &mut callback as *mut &mut dyn FnMut(isize) -> bool as isize)
        };
    }

    /// `GetSystemMenu`: makes the window own a copy of the system menu
    /// (`revert` false) or go back to the default one.
    pub fn get_system_menu(hwnd: isize, revert: bool) -> isize {
        // SAFETY: plain values.
        unsafe { wm::GetSystemMenu(h(hwnd), revert as i32) as isize }
    }

    /// Whether mouse input arrives as pointer messages.
    pub fn is_mouse_in_pointer_enabled() -> bool {
        // SAFETY: no arguments.
        unsafe { ptr_input::IsMouseInPointerEnabled() != 0 }
    }

    /// Registers a window as one that takes touch input.
    pub fn register_touch_window(hwnd: isize) -> bool {
        // SAFETY: plain values.
        unsafe { touch::RegisterTouchWindow(h(hwnd), 0) != 0 }
    }

    // ----------------------------------------------------------------
    // Painting
    // ----------------------------------------------------------------

    /// Adds a rectangle of a window to what it has to paint.
    pub fn invalidate_rect(hwnd: isize, rect: &RECT, erase: bool) -> bool {
        let rect = from_rect(*rect);
        // SAFETY: a valid rectangle that lives through the call.
        unsafe { gdi::InvalidateRect(h(hwnd), &rect, erase as i32) != 0 }
    }

    /// A paint of a window between `BeginPaint` and `EndPaint`: the update
    /// region is validated when the value is dropped.
    pub struct PaintScope {
        hwnd: isize,
        paint: gdi::PAINTSTRUCT,
    }

    impl PaintScope {
        /// The rectangle that has to be painted.
        pub fn rc_paint(&self) -> RECT {
            to_rect(self.paint.rcPaint)
        }
    }

    impl Drop for PaintScope {
        fn drop(&mut self) {
            // SAFETY: the structure `BeginPaint` filled for this window.
            unsafe { gdi::EndPaint(h(self.hwnd), &self.paint) };
        }
    }

    /// `BeginPaint`; `None` if no device context is available.
    pub fn begin_paint(hwnd: isize) -> Option<PaintScope> {
        // SAFETY: zeroed plain data, filled by the call.
        let mut paint: gdi::PAINTSTRUCT = unsafe { std::mem::zeroed() };
        // SAFETY: a valid out structure; called by the window procedure of
        // the window for `WM_PAINT`.
        let dc = unsafe { gdi::BeginPaint(h(hwnd), &mut paint) };
        (!dc.is_null()).then_some(PaintScope { hwnd, paint })
    }

    /// Converts a point of the screen to the client coordinates of a
    /// window.
    pub fn screen_to_client(hwnd: isize, point: POINT) -> POINT {
        let mut native = wf::POINT { x: point.x, y: point.y };
        // SAFETY: a valid point the call rewrites.
        unsafe { gdi::ScreenToClient(h(hwnd), &mut native) };
        POINT { x: native.x, y: native.y }
    }

    /// Converts a point of the client area of a window to screen
    /// coordinates.
    pub fn client_to_screen(hwnd: isize, point: POINT) -> POINT {
        let mut native = wf::POINT { x: point.x, y: point.y };
        // SAFETY: a valid point the call rewrites.
        unsafe { gdi::ClientToScreen(h(hwnd), &mut native) };
        POINT { x: native.x, y: native.y }
    }

    /// Draws the rows of a top-down 32-bit bitmap to a window. `pixels`
    /// holds `width * height * 4` bytes. May be called from any thread.
    /// Returns whether a device context was available.
    pub fn draw_bitmap_to_window(hwnd: isize, pixels: &[u8], header: &BITMAPINFOHEADER) -> bool {
        let width = header.bi_width;
        let height = header.bi_height.abs();
        assert!(pixels.len() >= width.max(0) as usize * height as usize * 4, "the bitmap is smaller than its header says");
        if hwnd == 0 {
            return false;
        }

        // SAFETY: a window handle; a stale one makes the call fail.
        let dc = unsafe { gdi::GetDC(h(hwnd)) };
        if dc.is_null() {
            return false;
        }

        // SAFETY: zeroed plain data; the header is filled below.
        let mut info: gdi::BITMAPINFO = unsafe { std::mem::zeroed() };
        info.bmiHeader = gdi::BITMAPINFOHEADER {
            biSize: header.bi_size,
            biWidth: header.bi_width,
            biHeight: header.bi_height,
            biPlanes: header.bi_planes,
            biBitCount: header.bi_bit_count,
            biCompression: header.bi_compression,
            biSizeImage: header.bi_size_image,
            biXPelsPerMeter: header.bi_x_pels_per_meter,
            biYPelsPerMeter: header.bi_y_pels_per_meter,
            biClrUsed: header.bi_clr_used,
            biClrImportant: header.bi_clr_important,
        };

        // SAFETY: the pixel slice holds every row the header describes
        // (asserted above) and lives through the call, which copies it; the
        // device context is the one acquired above and released below.
        unsafe {
            gdi::SetDIBitsToDevice(
                dc,
                0,
                0,
                width as u32,
                height as u32,
                0,
                0,
                0,
                height as u32,
                pixels.as_ptr().cast(),
                &info,
                gdi::DIB_RGB_COLORS,
            );
            gdi::ReleaseDC(h(hwnd), dc);
        }
        true
    }

    // ----------------------------------------------------------------
    // Monitors
    // ----------------------------------------------------------------

    /// `MonitorFromWindow` with a [`MONITOR`] flag; 0 for none.
    pub fn monitor_from_window(hwnd: isize, flags: u32) -> isize {
        // SAFETY: plain values. May be called from any thread.
        unsafe { gdi::MonitorFromWindow(h(hwnd), flags) as isize }
    }

    /// `MonitorFromPoint` with a [`MONITOR`] flag; 0 for none.
    pub fn monitor_from_point(point: POINT, flags: u32) -> isize {
        // SAFETY: plain values.
        unsafe { gdi::MonitorFromPoint(wf::POINT { x: point.x, y: point.y }, flags) as isize }
    }

    /// `MonitorFromRect` with a [`MONITOR`] flag; 0 for none.
    pub fn monitor_from_rect(rect: &RECT, flags: u32) -> isize {
        let rect = from_rect(*rect);
        // SAFETY: a valid rectangle that lives through the call.
        unsafe { gdi::MonitorFromRect(&rect, flags) as isize }
    }

    /// The handles of the monitors of the desktop.
    pub fn enum_display_monitors() -> Vec<isize> {
        unsafe extern "system" fn thunk(monitor: gdi::HMONITOR, _dc: gdi::HDC, _rect: *mut wf::RECT, data: isize) -> i32 {
            // SAFETY: `data` is the pointer to the vector that
            // `enum_display_monitors` passes below; the enumeration calls
            // back on the same thread before it returns.
            let monitors = unsafe { &mut *(data as *mut Vec<isize>) };
            monitors.push(monitor as isize);
            1
        }

        let mut monitors: Vec<isize> = Vec::new();
        // SAFETY: the vector outlives the call (see the thunk); no device
        // context and no clip rectangle.
        unsafe {
            gdi::EnumDisplayMonitors(std::ptr::null_mut(), std::ptr::null(), Some(thunk), &mut monitors as *mut Vec<isize> as isize)
        };
        monitors
    }

    /// What the system knows about a monitor; `None` if the handle is
    /// stale.
    pub fn get_monitor_info(monitor: isize) -> Option<MONITORINFOEX> {
        // SAFETY: zeroed plain data.
        let mut info: gdi::MONITORINFOEXW = unsafe { std::mem::zeroed() };
        info.monitorInfo.cbSize = std::mem::size_of::<gdi::MONITORINFOEXW>() as u32;
        // SAFETY: the structure starts with a `MONITORINFO` whose size
        // member says that the extended structure follows.
        let ok = unsafe { gdi::GetMonitorInfoW(h(monitor), (&mut info as *mut gdi::MONITORINFOEXW).cast()) } != 0;
        ok.then(|| MONITORINFOEX {
            rc_monitor: to_rect(info.monitorInfo.rcMonitor),
            rc_work: to_rect(info.monitorInfo.rcWork),
            dw_flags: info.monitorInfo.dwFlags,
            sz_device: from_wide(&info.szDevice),
        })
    }

    /// The current refresh rate (hertz) and orientation (`DMDO_*`: 0, 1, 2,
    /// 3 for 0, 90, 180 and 270 degrees) of a display device.
    pub fn enum_current_display_settings(device: &str) -> Option<(u32, u32)> {
        let device = to_wide(device);
        // SAFETY: zeroed plain data.
        let mut mode: gdi::DEVMODEW = unsafe { std::mem::zeroed() };
        mode.dmSize = std::mem::size_of::<gdi::DEVMODEW>() as u16;
        mode.dmFields = gdi::DM_DISPLAYORIENTATION | gdi::DM_DISPLAYFREQUENCY;
        // SAFETY: a null-terminated name and a structure with its size set.
        let ok = unsafe { gdi::EnumDisplaySettingsW(device.as_ptr(), gdi::ENUM_CURRENT_SETTINGS, &mut mode) } != 0;
        // SAFETY: for a display device the union holds the display members.
        ok.then(|| (mode.dmDisplayFrequency, unsafe { mode.Anonymous1.Anonymous2.dmDisplayOrientation }))
    }

    /// `GetDisplayConfigBufferSizes` and `QueryDisplayConfig` for the
    /// active paths of the display configuration: the adapter, the source
    /// and the target of each path. `None` when either call fails.
    pub fn query_active_display_paths() -> Option<Vec<DisplayConfigPath>> {
        let (mut num_path_info, mut num_mode_info) = (0u32, 0u32);
        // SAFETY: two numbers of this frame the system writes to.
        if unsafe { display::GetDisplayConfigBufferSizes(display::QDC_ONLY_ACTIVE_PATHS, &mut num_path_info, &mut num_mode_info) } != 0 {
            return None;
        }

        let mut paths = vec![display::DISPLAYCONFIG_PATH_INFO::default(); num_path_info as usize];
        let mut modes = vec![display::DISPLAYCONFIG_MODE_INFO::default(); num_mode_info as usize];
        // SAFETY: the two buffers have the numbers of elements the two
        // counts say, which the system reads before it writes and updates
        // to what it wrote; the topology is not asked for with this flag.
        let result = unsafe {
            display::QueryDisplayConfig(
                display::QDC_ONLY_ACTIVE_PATHS,
                &mut num_path_info,
                paths.as_mut_ptr(),
                &mut num_mode_info,
                modes.as_mut_ptr(),
                std::ptr::null_mut(),
            )
        };
        if result != 0 {
            return None;
        }

        paths.truncate(num_path_info as usize);
        Some(
            paths
                .iter()
                .map(|path| DisplayConfigPath {
                    adapter_id: (path.targetInfo.adapterId.LowPart, path.targetInfo.adapterId.HighPart),
                    source_id: path.sourceInfo.id,
                    target_id: path.targetInfo.id,
                })
                .collect(),
        )
    }

    /// `DisplayConfigGetDeviceInfo` for the source of a path: the name of
    /// its GDI device (`\\.\DISPLAY1`). `None` when the call fails.
    pub fn display_config_source_name(path: &DisplayConfigPath) -> Option<String> {
        let mut source_name = display::DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
        source_name.header.r#type = display::DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
        source_name.header.size = std::mem::size_of::<display::DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
        source_name.header.adapterId = wf::LUID { LowPart: path.adapter_id.0, HighPart: path.adapter_id.1 };
        source_name.header.id = path.source_id;
        // SAFETY: the header is the start of a structure of the size it
        // announces, which the system fills.
        (unsafe { display::DisplayConfigGetDeviceInfo(&mut source_name.header) } == 0)
            .then(|| from_wide(&source_name.viewGdiDeviceName))
    }

    /// `DisplayConfigGetDeviceInfo` for the target of a path: the friendly
    /// name of its monitor. `None` when the call fails.
    pub fn display_config_target_name(path: &DisplayConfigPath) -> Option<String> {
        let mut target_name = display::DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
        target_name.header.r#type = display::DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
        target_name.header.size = std::mem::size_of::<display::DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
        target_name.header.adapterId = wf::LUID { LowPart: path.adapter_id.0, HighPart: path.adapter_id.1 };
        target_name.header.id = path.target_id;
        // SAFETY: as `display_config_source_name`.
        (unsafe { display::DisplayConfigGetDeviceInfo(&mut target_name.header) } == 0)
            .then(|| from_wide(&target_name.monitorFriendlyDeviceName))
    }

    /// The scaling of the primary display computed from the capabilities of
    /// the screen device context (the path for systems without
    /// `GetDpiForMonitor`): the DPI.
    pub fn screen_dpi_from_device_caps() -> f64 {
        // SAFETY: the device context of the screen, released below.
        unsafe {
            let dc = gdi::GetDC(std::ptr::null_mut());
            let virt_w = gdi::GetDeviceCaps(dc, DEVICECAP::HORZRES) as f64;
            let phys_w = gdi::GetDeviceCaps(dc, DEVICECAP::DESKTOPHORZRES) as f64;
            gdi::ReleaseDC(std::ptr::null_mut(), dc);
            96.0 * phys_w / virt_w
        }
    }

    /// The number of bits of a pixel of the screen: the bits of a plane
    /// times the planes.
    pub fn screen_bit_depth() -> u32 {
        // SAFETY: the device context of the screen, released below.
        unsafe {
            let dc = gdi::GetDC(std::ptr::null_mut());
            let mut bit_depth = gdi::GetDeviceCaps(dc, gdi::BITSPIXEL as i32);
            bit_depth *= gdi::GetDeviceCaps(dc, gdi::PLANES as i32);
            gdi::ReleaseDC(std::ptr::null_mut(), dc);
            bit_depth.max(0) as u32
        }
    }

    // ----------------------------------------------------------------
    // Icons
    // ----------------------------------------------------------------

    /// `CreateBitmap`: a bitmap of the system with the pixels given, rows
    /// of `(width * bits_per_pixel + 7) / 8` bytes. The caller deletes it;
    /// 0 when it cannot be created.
    pub fn create_bitmap(width: i32, height: i32, planes: u32, bits_per_pixel: u32, bits: &[u8]) -> isize {
        // The system reads rows that are a whole number of 16-bit words
        // long. The reference hands it rows of whole bytes, which the
        // system reads past for a width whose row has an odd number of
        // bytes: the bytes go into a block that is as long as what the
        // system reads, unchanged.
        let word_aligned_row = ((width.max(0) as usize * bits_per_pixel as usize * planes as usize + 15) / 16) * 2;
        let mut block = vec![0u8; (word_aligned_row * height.max(0) as usize).max(bits.len())];
        block[..bits.len()].copy_from_slice(bits);
        // SAFETY: the block is at least as long as the rows the system
        // reads for this size and depth; the system copies the pixels.
        unsafe { gdi::CreateBitmap(width, height, planes, bits_per_pixel, block.as_ptr().cast()) as isize }
    }

    /// `DeleteObject`: deletes a bitmap (or another GDI object) the caller
    /// owns; 0 is ignored.
    pub fn delete_object(object: isize) {
        if object != 0 {
            // SAFETY: an object of the caller that is not used again.
            unsafe {
                gdi::DeleteObject(h(object));
            }
        }
    }

    /// `CreateIconIndirect`: an icon, or a cursor with its hot spot, from a
    /// mask bitmap and a colour bitmap, which the system copies. The
    /// caller destroys it; 0 when it cannot be created.
    pub fn create_icon_indirect(is_icon: bool, x_hotspot: i32, y_hotspot: i32, mask_bitmap: isize, color_bitmap: isize) -> isize {
        let info = wm::ICONINFO {
            fIcon: i32::from(is_icon),
            xHotspot: x_hotspot as u32,
            yHotspot: y_hotspot as u32,
            hbmMask: h(mask_bitmap),
            hbmColor: h(color_bitmap),
        };
        // SAFETY: a structure of this frame with two bitmaps of the caller.
        unsafe { wm::CreateIconIndirect(&info) as isize }
    }

    /// `CreateIconFromResourceEx`: an icon from the bytes of one image of
    /// an icon file. The caller destroys it; 0 when the bytes are not an
    /// image the system takes.
    pub fn create_icon_from_resource_ex(res_bits: &[u8], icon: bool, version: u32, cx_desired: i32, cy_desired: i32, flags: u32) -> isize {
        // SAFETY: the bytes are valid for the length that is passed, and
        // the system copies what it needs.
        unsafe {
            wm::CreateIconFromResourceEx(
                res_bits.as_ptr(),
                res_bits.len() as u32,
                i32::from(icon),
                version,
                cx_desired,
                cy_desired,
                flags,
            ) as isize
        }
    }

    /// `DestroyIcon`: destroys an icon or a cursor the caller created.
    pub fn destroy_icon(icon: isize) -> bool {
        // SAFETY: an icon of the caller that is not used again; a handle
        // that is not an icon makes the call fail.
        unsafe { wm::DestroyIcon(h(icon)) != 0 }
    }

    // ----------------------------------------------------------------
    // Desktop window manager
    // ----------------------------------------------------------------

    #[allow(missing_docs)]
    pub fn dwm_extend_frame_into_client_area(hwnd: isize, margins: &MARGINS) -> i32 {
        let native = ctl::MARGINS {
            cxLeftWidth: margins.cx_left_width,
            cxRightWidth: margins.cx_right_width,
            cyTopHeight: margins.cy_top_height,
            cyBottomHeight: margins.cy_bottom_height,
        };
        // SAFETY: a complete structure that lives through the call.
        unsafe { dwm::DwmExtendFrameIntoClientArea(h(hwnd), &native) }
    }

    /// Whether the desktop is composed; `None` if the call fails.
    pub fn dwm_is_composition_enabled() -> Option<bool> {
        let mut enabled = 0i32;
        // SAFETY: a valid out value.
        let result = unsafe { dwm::DwmIsCompositionEnabled(&mut enabled) };
        (result >= 0).then_some(enabled != 0)
    }

    /// `DwmSetWindowAttribute` for an attribute whose value is a 32-bit
    /// integer; returns the result code.
    pub fn dwm_set_window_attribute(hwnd: isize, attribute: u32, value: i32) -> i32 {
        // SAFETY: the value lives through the call and its size is passed.
        unsafe { dwm::DwmSetWindowAttribute(h(hwnd), attribute, (&value as *const i32).cast(), std::mem::size_of::<i32>() as u32) }
    }

    /// `DwmFlush`: waits until the desktop window manager has composed
    /// its next frame.
    pub fn dwm_flush() {
        // SAFETY: no arguments.
        unsafe { dwm::DwmFlush() };
    }

    /// `DwmEnableBlurBehindWindow` over the whole window (a region that
    /// covers nothing visible, which the system reads as "transparent
    /// without a blur" since Windows 8). Returns whether the call succeeded.
    pub fn dwm_enable_blur_behind_window(hwnd: isize, enabled: bool) -> bool {
        // SAFETY: the region is created, used and deleted here.
        unsafe {
            let region = gdi::CreateRectRgn(0, 0, -1, -1);
            let blur = dwm::DWM_BLURBEHIND {
                dwFlags: DWM_BB::ENABLE.bits() | DWM_BB::BLUR_REGION.bits(),
                fEnable: enabled as i32,
                hRgnBlur: region,
                fTransitionOnMaximized: 0,
            };
            let result = dwm::DwmEnableBlurBehindWindow(h(hwnd), &blur);
            if !region.is_null() {
                gdi::DeleteObject(region);
            }
            result == 0
        }
    }

    // ----------------------------------------------------------------
    // Mouse and keyboard
    // ----------------------------------------------------------------

    /// Asks for `WM_MOUSELEAVE` when the pointer leaves the window.
    pub fn track_mouse_leave(hwnd: isize) -> bool {
        let mut event = kbm::TRACKMOUSEEVENT {
            cbSize: std::mem::size_of::<kbm::TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: h(hwnd),
            dwHoverTime: 0,
        };
        // SAFETY: a complete structure that lives through the call.
        unsafe { kbm::TrackMouseEvent(&mut event) != 0 }
    }

    /// `TrackMouseEvent` with the given flags (`TME_LEAVE`, with
    /// `TME_NONCLIENT` for the frame of the window).
    pub fn track_mouse_event(hwnd: isize, flags: u32) -> bool {
        let mut event = kbm::TRACKMOUSEEVENT {
            cbSize: std::mem::size_of::<kbm::TRACKMOUSEEVENT>() as u32,
            dwFlags: flags,
            hwndTrack: h(hwnd),
            dwHoverTime: 0,
        };
        // SAFETY: a complete structure that lives through the call.
        unsafe { kbm::TrackMouseEvent(&mut event) != 0 }
    }

    /// `GetMouseMovePointsEx` with `GMMP_USE_DISPLAY_POINTS`: up to
    /// `count` points of the history of the mouse that end at `point`,
    /// newest first; empty if the point is not in the history.
    pub fn get_mouse_move_points_ex(point: MOUSEMOVEPOINT, count: usize) -> Vec<MOUSEMOVEPOINT> {
        let native = kbm::MOUSEMOVEPOINT { x: point.x, y: point.y, time: point.time as u32, dwExtraInfo: 0 };
        let mut buffer = vec![kbm::MOUSEMOVEPOINT { x: 0, y: 0, time: 0, dwExtraInfo: 0 }; count];
        // SAFETY: the buffer has `count` elements, which is what the call
        // is told, and the point lives through the call.
        let written = unsafe {
            kbm::GetMouseMovePointsEx(
                std::mem::size_of::<kbm::MOUSEMOVEPOINT>() as u32,
                &native,
                buffer.as_mut_ptr(),
                count as i32,
                1,
            )
        };
        if written <= 0 {
            return Vec::new();
        }
        buffer
            .iter()
            .take(written as usize)
            .map(|p| MOUSEMOVEPOINT { x: p.x, y: p.y, time: p.time as i32, dw_extra_info: p.dwExtraInfo as isize })
            .collect()
    }

    // ----------------------------------------------------------------
    // Pointer and touch input
    // ----------------------------------------------------------------

    fn to_pointer_info(info: &ptr_input::POINTER_INFO) -> POINTER_INFO {
        POINTER_INFO {
            pointer_type: info.pointerType as u32,
            pointer_id: info.pointerId,
            pointer_flags: info.pointerFlags,
            source_device: info.sourceDevice as isize,
            pt_pixel_location_x: info.ptPixelLocation.x,
            pt_pixel_location_y: info.ptPixelLocation.y,
            pt_himetric_location_raw_x: info.ptHimetricLocationRaw.x,
            pt_himetric_location_raw_y: info.ptHimetricLocationRaw.y,
            dw_time: info.dwTime,
            history_count: info.historyCount,
            button_change_type: info.ButtonChangeType as u32,
        }
    }

    fn to_pointer_touch_info(info: &ptr_input::POINTER_TOUCH_INFO) -> POINTER_TOUCH_INFO {
        POINTER_TOUCH_INFO {
            pointer_info: to_pointer_info(&info.pointerInfo),
            touch_mask: info.touchMask,
            rc_contact_left: info.rcContact.left,
            rc_contact_top: info.rcContact.top,
            rc_contact_right: info.rcContact.right,
            rc_contact_bottom: info.rcContact.bottom,
            pressure: info.pressure,
        }
    }

    fn to_pointer_pen_info(info: &ptr_input::POINTER_PEN_INFO) -> POINTER_PEN_INFO {
        POINTER_PEN_INFO {
            pointer_info: to_pointer_info(&info.pointerInfo),
            pen_flags: info.penFlags,
            pressure: info.pressure,
            rotation: info.rotation,
            tilt_x: info.tiltX,
            tilt_y: info.tiltY,
        }
    }

    /// The kind of a pointer, a [`PointerInputType`]; `PT_NONE` if the
    /// system does not know the pointer.
    pub fn get_pointer_type(pointer_id: u32) -> u32 {
        let mut pointer_type = 0;
        // SAFETY: a valid out value.
        unsafe { ptr_input::GetPointerType(pointer_id, &mut pointer_type) };
        pointer_type as u32
    }

    /// `GetPointerInfo`; the zeroed structure if the call fails, which is
    /// what the reference reads then.
    pub fn get_pointer_info(pointer_id: u32) -> POINTER_INFO {
        // SAFETY: zeroed plain data the call fills.
        let mut info: ptr_input::POINTER_INFO = unsafe { std::mem::zeroed() };
        // SAFETY: a valid out structure.
        unsafe { ptr_input::GetPointerInfo(pointer_id, &mut info) };
        to_pointer_info(&info)
    }

    /// `GetPointerTouchInfo`; the zeroed structure if the call fails.
    pub fn get_pointer_touch_info(pointer_id: u32) -> POINTER_TOUCH_INFO {
        // SAFETY: zeroed plain data the call fills.
        let mut info: ptr_input::POINTER_TOUCH_INFO = unsafe { std::mem::zeroed() };
        // SAFETY: a valid out structure.
        unsafe { ptr_input::GetPointerTouchInfo(pointer_id, &mut info) };
        to_pointer_touch_info(&info)
    }

    /// `GetPointerPenInfo`; the zeroed structure if the call fails.
    pub fn get_pointer_pen_info(pointer_id: u32) -> POINTER_PEN_INFO {
        // SAFETY: zeroed plain data the call fills.
        let mut info: ptr_input::POINTER_PEN_INFO = unsafe { std::mem::zeroed() };
        // SAFETY: a valid out structure.
        unsafe { ptr_input::GetPointerPenInfo(pointer_id, &mut info) };
        to_pointer_pen_info(&info)
    }

    /// `GetPointerInfoHistory`: up to `count` entries of the history of a
    /// pointer, newest first; `None` if the call fails.
    pub fn get_pointer_info_history(pointer_id: u32, count: u32) -> Option<Vec<POINTER_INFO>> {
        // SAFETY: zeroed plain data the call fills.
        let mut buffer: Vec<ptr_input::POINTER_INFO> = vec![unsafe { std::mem::zeroed() }; count as usize];
        let mut entries = count;
        // SAFETY: the buffer has `entries` elements.
        let ok = unsafe { ptr_input::GetPointerInfoHistory(pointer_id, &mut entries, buffer.as_mut_ptr()) } != 0;
        ok.then(|| buffer.iter().take(entries.min(count) as usize).map(to_pointer_info).collect())
    }

    /// `GetPointerTouchInfoHistory`, as [`get_pointer_info_history`].
    pub fn get_pointer_touch_info_history(pointer_id: u32, count: u32) -> Option<Vec<POINTER_TOUCH_INFO>> {
        // SAFETY: zeroed plain data the call fills.
        let mut buffer: Vec<ptr_input::POINTER_TOUCH_INFO> = vec![unsafe { std::mem::zeroed() }; count as usize];
        let mut entries = count;
        // SAFETY: the buffer has `entries` elements.
        let ok = unsafe { ptr_input::GetPointerTouchInfoHistory(pointer_id, &mut entries, buffer.as_mut_ptr()) } != 0;
        ok.then(|| buffer.iter().take(entries.min(count) as usize).map(to_pointer_touch_info).collect())
    }

    /// `GetPointerPenInfoHistory`, as [`get_pointer_info_history`].
    pub fn get_pointer_pen_info_history(pointer_id: u32, count: u32) -> Option<Vec<POINTER_PEN_INFO>> {
        // SAFETY: zeroed plain data the call fills.
        let mut buffer: Vec<ptr_input::POINTER_PEN_INFO> = vec![unsafe { std::mem::zeroed() }; count as usize];
        let mut entries = count;
        // SAFETY: the buffer has `entries` elements.
        let ok = unsafe { ptr_input::GetPointerPenInfoHistory(pointer_id, &mut entries, buffer.as_mut_ptr()) } != 0;
        ok.then(|| buffer.iter().take(entries.min(count) as usize).map(to_pointer_pen_info).collect())
    }

    /// Whether this system exports `GetPointerDeviceRects` (Wine and
    /// Proton do not).
    pub fn has_get_pointer_device_rects() -> bool {
        get_proc_address(load_library("user32.dll"), c"GetPointerDeviceRects").is_some()
    }

    /// `GetPointerDeviceRects`: the rectangle of a pointer device in its
    /// own units and the rectangle of the display it is mapped to; `None`
    /// if the system does not have the function or the call fails.
    pub fn get_pointer_device_rects(device: isize) -> Option<(RECT, RECT)> {
        let function = get_proc_address(load_library("user32.dll"), c"GetPointerDeviceRects")?;
        // SAFETY: the export has this signature (Windows 8 and later).
        let function: unsafe extern "system" fn(*mut c_void, *mut wf::RECT, *mut wf::RECT) -> i32 =
            unsafe { std::mem::transmute(function) };
        let mut device_rect = wf::RECT { left: 0, top: 0, right: 0, bottom: 0 };
        let mut display_rect = wf::RECT { left: 0, top: 0, right: 0, bottom: 0 };
        // SAFETY: both out rectangles are valid for the call.
        let ok = unsafe { function(h(device), &mut device_rect, &mut display_rect) } != 0;
        ok.then(|| (to_rect(device_rect), to_rect(display_rect)))
    }

    /// `GetTouchInputInfo`: the contacts of a `WM_TOUCH` message, whose
    /// parameters are the count and the handle; `None` if the call fails.
    pub fn get_touch_input_info(touch_input: isize, count: u32) -> Option<Vec<TOUCHINPUT>> {
        // SAFETY: zeroed plain data the call fills.
        let mut buffer: Vec<touch::TOUCHINPUT> = vec![unsafe { std::mem::zeroed() }; count as usize];
        // SAFETY: the buffer has `count` elements of the size passed.
        let ok = unsafe {
            touch::GetTouchInputInfo(
                h(touch_input),
                count,
                buffer.as_mut_ptr(),
                std::mem::size_of::<touch::TOUCHINPUT>() as i32,
            )
        } != 0;
        ok.then(|| {
            buffer
                .iter()
                .map(|t| TOUCHINPUT {
                    x: t.x,
                    y: t.y,
                    id: t.dwID,
                    flags: t.dwFlags,
                    mask: t.dwMask,
                    time: t.dwTime,
                    cx_contact: t.cxContact,
                    cy_contact: t.cyContact,
                })
                .collect()
        })
    }

    #[allow(missing_docs)]
    pub fn close_touch_input_handle(touch_input: isize) -> bool {
        // SAFETY: plain values.
        unsafe { touch::CloseTouchInputHandle(h(touch_input)) != 0 }
    }

    // ----------------------------------------------------------------
    // The frame of a window and its system menu
    // ----------------------------------------------------------------

    /// `DwmDefWindowProc`: the answer of the desktop window manager to a
    /// message of the frame (the hit test of the caption buttons it
    /// draws), or `None` if it did not handle the message.
    pub fn dwm_def_window_proc(hwnd: isize, msg: u32, w_param: usize, l_param: isize) -> Option<isize> {
        let mut result = 0isize;
        // SAFETY: plain values and a valid out value.
        let handled = unsafe { dwm::DwmDefWindowProc(h(hwnd), msg, w_param, l_param, &mut result) } != 0;
        handled.then_some(result)
    }

    /// `TrackPopupMenu`: shows a menu at a point of the screen and runs
    /// its loop; with `TPM_RETURNCMD` the result is the chosen command.
    pub fn track_popup_menu(menu: isize, flags: TrackPopupMenuFlags, x: i32, y: i32, hwnd: isize) -> i32 {
        // SAFETY: plain values; the reserved rectangle is null. The window
        // procedure is called during the call, on this thread.
        unsafe { wm::TrackPopupMenu(h(menu), flags.bits(), x, y, 0, h(hwnd), std::ptr::null()) }
    }

    #[allow(missing_docs)]
    pub fn enable_menu_item(menu: isize, id: u32, flags: u32) -> i32 {
        // SAFETY: plain values.
        unsafe { wm::EnableMenuItem(h(menu), id, flags) }
    }

    #[allow(missing_docs)]
    pub fn set_menu_default_item(menu: isize, item: u32, by_position: u32) -> bool {
        // SAFETY: plain values.
        unsafe { wm::SetMenuDefaultItem(h(menu), item, by_position) != 0 }
    }

    /// Sends all mouse input to a window of the calling thread.
    pub fn set_capture(hwnd: isize) -> isize {
        // SAFETY: plain values.
        unsafe { kbm::SetCapture(h(hwnd)) as isize }
    }

    #[allow(missing_docs)]
    pub fn release_capture() -> bool {
        // SAFETY: no arguments.
        unsafe { kbm::ReleaseCapture() != 0 }
    }

    /// The state of the 256 virtual keys as the thread has seen it.
    pub fn get_keyboard_state() -> [u8; 256] {
        let mut state = [0u8; 256];
        // SAFETY: a buffer of the 256 bytes the function writes.
        unsafe { kbm::GetKeyboardState(state.as_mut_ptr()) };
        state
    }

    /// `MapVirtualKey` with a [`MapVirtualKeyMapTypes`].
    pub fn map_virtual_key(code: u32, map_type: u32) -> u32 {
        // SAFETY: plain values.
        unsafe { kbm::MapVirtualKeyW(code, map_type) }
    }

    /// The keyboard layout of a thread (0: the calling thread).
    pub fn get_keyboard_layout(thread: u32) -> isize {
        // SAFETY: plain values.
        unsafe { kbm::GetKeyboardLayout(thread) as isize }
    }

    /// `ToUnicodeEx`: translates a key with the given keyboard state. The
    /// result is the number of characters written to the buffer, negative
    /// for a dead key.
    pub fn to_unicode_ex(virtual_key: u32, scan_code: u32, key_state: &[u8; 256], flags: u32, layout: isize) -> (i32, [u16; 4]) {
        let mut buffer = [0u16; 4];
        // SAFETY: the state holds the 256 bytes the function reads and the
        // buffer the 4 characters it is told it may write.
        let length = unsafe {
            kbm::ToUnicodeEx(virtual_key, scan_code, key_state.as_ptr(), buffer.as_mut_ptr(), buffer.len() as i32, flags, h(layout))
        };
        (length, buffer)
    }

    /// The time within which a second click is a double click.
    pub fn get_double_click_time() -> u32 {
        // SAFETY: no arguments.
        unsafe { kbm::GetDoubleClickTime() }
    }

    // ----------------------------------------------------------------
    // Clipboard
    // ----------------------------------------------------------------

    /// Opens the clipboard for the calling thread. `owner` is the window
    /// that owns the clipboard once it is emptied, or 0: the system only
    /// accepts data from a thread that opened the clipboard with an owner.
    pub fn open_clipboard(owner: isize) -> bool {
        // SAFETY: a window of the calling thread, or no owner.
        unsafe { dx::OpenClipboard(h(owner)) != 0 }
    }

    #[allow(missing_docs)]
    pub fn close_clipboard() -> bool {
        // SAFETY: no arguments.
        unsafe { dx::CloseClipboard() != 0 }
    }

    /// Empties the clipboard, which the caller has open.
    pub fn empty_clipboard() -> bool {
        // SAFETY: no arguments.
        unsafe { dx::EmptyClipboard() != 0 }
    }

    /// The Unicode text on the clipboard, which the caller has open.
    pub fn get_clipboard_unicode_text() -> Option<String> {
        // SAFETY: the handle belongs to the clipboard and stays valid while
        // the clipboard is open; it is locked for the read and unlocked
        // after it, and the text is read up to its terminator or the size
        // of the block, whichever comes first.
        unsafe {
            let handle = dx::GetClipboardData(ClipboardFormat::CF_UNICODETEXT as u32);
            if handle.is_null() {
                return None;
            }
            let data = mem::GlobalLock(handle) as *const u16;
            if data.is_null() {
                return None;
            }
            let capacity = mem::GlobalSize(handle) / 2;
            let text = std::slice::from_raw_parts(data, capacity);
            let result = from_wide(text);
            mem::GlobalUnlock(handle);
            Some(result)
        }
    }

    /// Puts Unicode text on the clipboard, which the caller has open and
    /// has emptied.
    pub fn set_clipboard_unicode_text(text: &str) -> bool {
        let text = to_wide(text);
        // SAFETY: the block is allocated with room for the text and its
        // terminator, locked while the text is copied into it, and given to
        // the clipboard, which owns it from then on; it is freed here only
        // if the clipboard did not take it.
        unsafe {
            let handle = mem::GlobalAlloc(GlobalAllocFlags::GMEM_MOVEABLE.bits(), text.len() * 2);
            if handle.is_null() {
                return false;
            }
            let data = mem::GlobalLock(handle) as *mut u16;
            if data.is_null() {
                wf::GlobalFree(handle);
                return false;
            }
            std::ptr::copy_nonoverlapping(text.as_ptr(), data, text.len());
            mem::GlobalUnlock(handle);
            if dx::SetClipboardData(ClipboardFormat::CF_UNICODETEXT as u32, handle).is_null() {
                wf::GlobalFree(handle);
                return false;
            }
            true
        }
    }

    // ----------------------------------------------------------------
    // COM and OLE
    // ----------------------------------------------------------------

    /// `OleInitialize`: initialises OLE on the calling thread, which makes
    /// it a single-threaded apartment. Returns the result code.
    pub fn ole_initialize() -> u32 {
        // SAFETY: the reserved argument is null; the call has no other
        // precondition.
        unsafe { ole::OleInitialize(std::ptr::null()) as u32 }
    }

    /// `RegisterDragDrop`: registers a drop target for a window. Returns
    /// the result code.
    ///
    /// # Safety
    /// `target` must point at a live COM object that implements
    /// `IDropTarget`; the system takes a reference of its own.
    pub unsafe fn register_drag_drop(hwnd: isize, target: *mut c_void) -> u32 {
        // SAFETY: the contract of this function.
        unsafe { ole::RegisterDragDrop(h(hwnd), target) as u32 }
    }

    /// `RevokeDragDrop`: removes the drop target of a window. Returns the
    /// result code.
    pub fn revoke_drag_drop(hwnd: isize) -> u32 {
        // SAFETY: a window handle; a window without a target makes the
        // call fail.
        unsafe { ole::RevokeDragDrop(h(hwnd)) as u32 }
    }

    /// The kind of COM apartment of the calling thread (`APTTYPE`), or
    /// `None` when COM is not initialised on it.
    ///
    /// A thread that has not initialised COM is reported by the system as
    /// a thread of the multithreaded apartment as soon as any thread of
    /// the process has created that apartment (the implicit multithreaded
    /// apartment: the qualifier says so). Such a thread has no apartment
    /// of its own and can still enter a single-threaded one, so it is
    /// `None` here. The Windows.UI.Composition mode showed it: the
    /// libraries of the compositor create the multithreaded apartment on
    /// their threads before the UI thread initialises OLE (run
    /// 38077851321: the drag source was not registered and the clipboard
    /// was not opened in that mode).
    pub fn co_get_apartment_type() -> Option<i32> {
        let mut apartment_type = 0;
        let mut qualifier = 0;
        // SAFETY: two numbers of this frame the system writes to.
        let result = unsafe { com::CoGetApartmentType(&mut apartment_type, &mut qualifier) };
        apartment_of(result, apartment_type, qualifier)
    }

    /// `APTTYPEQUALIFIER_IMPLICIT_MTA`.
    const APTTYPEQUALIFIER_IMPLICIT_MTA: i32 = 1;

    /// The apartment of a thread from what `CoGetApartmentType` answered.
    pub(crate) fn apartment_of(result: i32, apartment_type: i32, qualifier: i32) -> Option<i32> {
        (result >= 0 && qualifier != APTTYPEQUALIFIER_IMPLICIT_MTA).then_some(apartment_type)
    }

    /// `APTTYPE_STA`: a single-threaded apartment.
    pub const APTTYPE_STA: i32 = com::APTTYPE_STA;
    /// `APTTYPE_MAINSTA`: the main single-threaded apartment.
    pub const APTTYPE_MAINSTA: i32 = com::APTTYPE_MAINSTA;

    /// `SetParent`: makes a window a child of another; the previous
    /// parent, 0 when the call fails.
    pub fn set_parent(child: isize, new_parent: isize) -> isize {
        // SAFETY: two handles; a handle that is not a window makes the
        // call fail.
        unsafe { wm::SetParent(h(child), h(new_parent)) as isize }
    }

    /// `MoveWindow`: the position and the size of a window in the
    /// coordinates of its parent.
    pub fn move_window(hwnd: isize, x: i32, y: i32, width: i32, height: i32, repaint: bool) -> bool {
        // SAFETY: plain values.
        unsafe { wm::MoveWindow(h(hwnd), x, y, width, height, repaint as i32) != 0 }
    }

    /// `SetLayeredWindowAttributes` with `LWA_ALPHA`: the opacity of a
    /// layered window, 255 for opaque.
    pub fn set_layered_window_alpha(hwnd: isize, alpha: u8) -> bool {
        // SAFETY: plain values; no colour key.
        unsafe { wm::SetLayeredWindowAttributes(h(hwnd), 0, alpha, wm::LWA_ALPHA) != 0 }
    }

    /// `InvalidateRect` without a rectangle: the whole client area of a
    /// window needs painting.
    pub fn invalidate_window(hwnd: isize, erase: bool) -> bool {
        // SAFETY: a null rectangle stands for the whole client area.
        unsafe { gdi::InvalidateRect(h(hwnd), std::ptr::null(), erase as i32) != 0 }
    }

    /// The window procedure of a window that does nothing of its own: the
    /// default processing of the system.
    ///
    /// # Safety
    /// Called by the system with the parameters of a message.
    pub unsafe extern "system" fn default_wnd_proc(hwnd: wf::HWND, msg: u32, w_param: usize, l_param: isize) -> isize {
        // SAFETY: the parameters of the message, passed on unchanged.
        unsafe { wm::DefWindowProcW(hwnd, msg, w_param, l_param) }
    }

    /// `GetLogicalDrives`: the drive letters in use, bit 0 for `A`.
    pub fn get_logical_drives() -> u32 {
        // SAFETY: the call takes nothing.
        unsafe { fs::GetLogicalDrives() }
    }

    /// `GetVolumeInformation` for the root directory of a drive (`C:\`):
    /// the label of its volume. `None` when the call fails (a drive
    /// without a medium).
    pub fn get_volume_label(root: &str) -> Option<String> {
        let root = to_wide(root);
        let mut label = [0u16; 261];
        // SAFETY: a null-terminated path and a buffer of the length given,
        // which the system writes a terminated string to; nothing else is
        // asked for.
        let ok = unsafe {
            fs::GetVolumeInformationW(
                root.as_ptr(),
                label.as_mut_ptr(),
                label.len() as u32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            )
        } != 0;
        ok.then(|| from_wide(&label))
    }

    /// `GetDiskFreeSpaceEx` for a directory: the size of its volume in
    /// bytes. `None` when the call fails.
    pub fn get_disk_total_size(root: &str) -> Option<u64> {
        let root = to_wide(root);
        let mut total = 0u64;
        // SAFETY: a null-terminated path and a number of this frame the
        // system writes to; the other two numbers are not asked for.
        let ok = unsafe { fs::GetDiskFreeSpaceExW(root.as_ptr(), std::ptr::null_mut(), &mut total, std::ptr::null_mut()) } != 0;
        ok.then_some(total)
    }

    /// `CoInitializeEx` for a single-threaded apartment: COM on the
    /// calling thread. The result code (`S_OK`, `S_FALSE` when the thread
    /// already has COM, or a failure).
    pub fn co_initialize_apartment_threaded() -> i32 {
        // SAFETY: the reserved argument is null; the flag is
        // `COINIT_APARTMENTTHREADED`.
        unsafe { com::CoInitializeEx(std::ptr::null(), 2) }
    }

    /// `CoUninitialize`: ends what a successful `CoInitializeEx` of the
    /// calling thread began.
    pub fn co_uninitialize() {
        // SAFETY: the caller pairs it with an initialisation of the thread
        // that succeeded; no interface pointer of the thread is used after.
        unsafe { com::CoUninitialize() }
    }

    /// `SHCreateItemFromParsingName` without a bind context: an interface
    /// pointer of the shell item of a path, which the caller owns, or the
    /// failure code.
    pub fn sh_create_item_from_parsing_name(path: &str, iid: &ferroui_microcom::Guid) -> Result<*mut c_void, i32> {
        let path = to_wide(path);
        let mut item = std::ptr::null_mut();
        // SAFETY: a null-terminated string and an identifier that live
        // through the call; the result is written to a pointer of this
        // frame.
        let result = unsafe {
            shell::SHCreateItemFromParsingName(
                path.as_ptr(),
                std::ptr::null_mut(),
                (iid as *const ferroui_microcom::Guid).cast(),
                &mut item,
            )
        };
        if result == 0 {
            Ok(item)
        } else {
            Err(result)
        }
    }

    /// `GetCursorPos`: the position of the cursor on the desktop, in
    /// pixels; the origin when the call fails.
    pub fn get_cursor_pos() -> POINT {
        let mut point = wf::POINT { x: 0, y: 0 };
        // SAFETY: a point of this frame the system writes to.
        unsafe { wm::GetCursorPos(&mut point) };
        POINT { x: point.x, y: point.y }
    }

    /// `Shell_NotifyIcon` with a [`NIM`] message: adds, changes or removes
    /// a notification icon. Whether the shell took it.
    pub fn shell_notify_icon(message: u32, data: &NOTIFYICONDATA) -> bool {
        // SAFETY: the structure starts with its size, which tells the
        // shell which version of `NOTIFYICONDATAW` it is: the members up
        // to the flags of the balloon, laid out as the system lays them
        // out. It lives through the call.
        unsafe { shell::Shell_NotifyIconW(message, (data as *const NOTIFYICONDATA).cast()) != 0 }
    }

    /// `SHAppBarMessage` with an [`AppBarMessage`]: asks the shell about
    /// the task bar; the answer is in the result and in the structure.
    pub fn sh_app_bar_message(message: u32, data: &mut APPBARDATA) -> usize {
        // SAFETY: a structure of the layout of the system with its size
        // set, which the shell reads and writes during the call.
        unsafe { shell::SHAppBarMessage(message, (data as *mut APPBARDATA).cast()) }
    }

    /// `RegisterWindowMessage`: the number of a message with a name, the
    /// same for every process of the session; 0 when the call fails.
    pub fn register_window_message(name: &str) -> u32 {
        let name = to_wide(name);
        // SAFETY: a null-terminated string that lives through the call.
        unsafe { wm::RegisterWindowMessageW(name.as_ptr()) }
    }

    /// `ChangeWindowMessageFilterEx` with a [`MessageFilterFlag`]: lets a
    /// message through to the window from processes of a lower integrity
    /// level. Whether the call succeeded.
    pub fn change_window_message_filter_ex(hwnd: isize, message: u32, action: u32) -> bool {
        // SAFETY: plain values; the structure of the result is not asked
        // for.
        unsafe { wm::ChangeWindowMessageFilterEx(h(hwnd), message, action, std::ptr::null_mut()) != 0 }
    }

    /// `CoCreateInstance` without an outer object: an interface pointer of
    /// a new object of the class, which the caller owns (one reference),
    /// or the failure code.
    pub fn co_create_instance(
        clsid: &ferroui_microcom::Guid,
        context: u32,
        iid: &ferroui_microcom::Guid,
    ) -> Result<*mut c_void, i32> {
        let mut instance = std::ptr::null_mut();
        // SAFETY: the two identifiers are structures of the layout of the
        // system that live through the call; the result is written to a
        // pointer of this frame.
        let result = unsafe {
            com::CoCreateInstance(
                (clsid as *const ferroui_microcom::Guid).cast(),
                std::ptr::null_mut(),
                context,
                (iid as *const ferroui_microcom::Guid).cast(),
                &mut instance,
            )
        };
        if result == 0 {
            Ok(instance)
        } else {
            Err(result)
        }
    }

    /// `CoTaskMemAlloc`: memory of the COM allocator, which the receiver of
    /// an out parameter frees. Null when there is none.
    pub fn co_task_mem_alloc(size: usize) -> *mut c_void {
        // SAFETY: allocates; the caller owns the block.
        unsafe { com::CoTaskMemAlloc(size) }
    }

    /// `CoTaskMemFree`.
    ///
    /// # Safety
    /// `block` must be null or a block of the COM allocator that is not
    /// used again.
    pub unsafe fn co_task_mem_free(block: *mut c_void) {
        // SAFETY: the contract of this function.
        unsafe { com::CoTaskMemFree(block) }
    }

    // ----------------------------------------------------------------
    // Data transfer: clipboard formats, global memory, OLE, GDI bitmaps
    // ----------------------------------------------------------------

    /// `GetClipboardFormatName`: the name of a registered clipboard format;
    /// `None` for a format without one (the formats of the system).
    pub fn get_clipboard_format_name(format: u16) -> Option<String> {
        let mut buffer = [0u16; 260];
        // SAFETY: a buffer of this frame and its length in characters.
        let length = unsafe { dx::GetClipboardFormatNameW(u32::from(format), buffer.as_mut_ptr(), buffer.len() as i32) };
        (length > 0).then(|| String::from_utf16_lossy(&buffer[..length as usize]))
    }

    /// `RegisterClipboardFormat`: the identifier of the format of a name,
    /// registered if it was not; 0 on failure.
    pub fn register_clipboard_format(name: &str) -> u32 {
        let name = to_wide(name);
        // SAFETY: a null-terminated string that outlives the call.
        unsafe { dx::RegisterClipboardFormatW(name.as_ptr()) }
    }

    /// `GlobalAlloc`: a block of global memory; 0 on failure.
    pub fn global_alloc(flags: GlobalAllocFlags, size: usize) -> isize {
        // SAFETY: allocates; the caller owns the block.
        unsafe { mem::GlobalAlloc(flags.bits(), size) as isize }
    }

    /// `GlobalSize`: the size of a block of global memory; 0 for a handle
    /// that is not one.
    pub fn global_size(h_global: isize) -> usize {
        // SAFETY: the system validates the handle.
        unsafe { mem::GlobalSize(h(h_global)) }
    }

    /// `GlobalFree`.
    ///
    /// # Safety
    /// `h_global` is a block of global memory the caller owns and does not
    /// use again.
    pub unsafe fn global_free(h_global: isize) {
        // SAFETY: the contract of this function.
        unsafe { wf::GlobalFree(h(h_global)) };
    }

    /// The bytes of a block of global memory (`GlobalLock`, a copy of
    /// `GlobalSize` bytes, `GlobalUnlock`); `None` when it cannot be
    /// locked.
    ///
    /// # Safety
    /// `h_global` is a live block of global memory.
    pub unsafe fn read_global(h_global: isize) -> Option<Vec<u8>> {
        // SAFETY: by the contract the block is live; it is locked while
        // its bytes, as many as the system says it has, are copied.
        unsafe {
            let source = mem::GlobalLock(h(h_global)) as *const u8;
            if source.is_null() {
                return None;
            }
            let size = mem::GlobalSize(h(h_global));
            let data = std::slice::from_raw_parts(source, size).to_vec();
            mem::GlobalUnlock(h(h_global));
            Some(data)
        }
    }

    /// Copies bytes to the start of a block of global memory; `false` when
    /// the block is smaller than the bytes or cannot be locked.
    ///
    /// # Safety
    /// `h_global` is a live block of global memory.
    pub unsafe fn write_global(h_global: isize, data: &[u8]) -> bool {
        // SAFETY: by the contract the block is live; the bytes are copied
        // only when the block has room for them, while it is locked.
        unsafe {
            if data.len() > mem::GlobalSize(h(h_global)) {
                return false;
            }
            let destination = mem::GlobalLock(h(h_global)) as *mut u8;
            if destination.is_null() {
                return false;
            }
            std::ptr::copy_nonoverlapping(data.as_ptr(), destination, data.len());
            mem::GlobalUnlock(h(h_global));
            true
        }
    }

    /// The file names of a `CF_HDROP` block (`DragQueryFile`).
    ///
    /// # Safety
    /// `h_global` is a live block of global memory.
    pub unsafe fn drag_query_file_names(h_global: isize) -> Vec<String> {
        // SAFETY: the system reads the block, which is live by the
        // contract, and writes at most the length given into the buffer.
        unsafe {
            let file_count = shell::DragQueryFileW(h(h_global), u32::MAX, std::ptr::null_mut(), 0);
            let mut files = Vec::with_capacity(file_count as usize);
            for i in 0..file_count {
                let path_length = shell::DragQueryFileW(h(h_global), i, std::ptr::null_mut(), 0);
                let mut buffer = vec![0u16; path_length as usize + 1];
                if shell::DragQueryFileW(h(h_global), i, buffer.as_mut_ptr(), buffer.len() as u32) == path_length {
                    files.push(String::from_utf16_lossy(&buffer[..path_length as usize]));
                }
            }
            files
        }
    }

    /// `ReleaseStgMedium`.
    ///
    /// # Safety
    /// `medium` was filled by a data object and is not used again.
    pub unsafe fn release_stg_medium(medium: &mut STGMEDIUM) {
        // SAFETY: the structure has the layout of the system's (tested in
        // `win32_com`); the rest is the contract of this function.
        unsafe { ole::ReleaseStgMedium((medium as *mut STGMEDIUM).cast()) }
    }

    /// `OleSetClipboard`. Returns the result code.
    ///
    /// # Safety
    /// `data_object` is null or a live COM object that implements
    /// `IDataObject`.
    pub unsafe fn ole_set_clipboard(data_object: *mut c_void) -> u32 {
        // SAFETY: the contract of this function.
        unsafe { ole::OleSetClipboard(data_object) as u32 }
    }

    /// `OleGetClipboard`: the result code and the data object of the
    /// clipboard, of which the caller owns a reference.
    pub fn ole_get_clipboard() -> (u32, *mut c_void) {
        let mut data_object = std::ptr::null_mut();
        // SAFETY: a pointer of this frame the system writes to.
        let result = unsafe { ole::OleGetClipboard(&mut data_object) };
        (result as u32, data_object)
    }

    /// `OleIsCurrentClipboard`. Returns the result code.
    ///
    /// # Safety
    /// `data_object` is a live COM object that implements `IDataObject`.
    pub unsafe fn ole_is_current_clipboard(data_object: *mut c_void) -> u32 {
        // SAFETY: the contract of this function.
        unsafe { ole::OleIsCurrentClipboard(data_object) as u32 }
    }

    /// `OleFlushClipboard`. Returns the result code.
    pub fn ole_flush_clipboard() -> u32 {
        // SAFETY: no arguments.
        unsafe { ole::OleFlushClipboard() as u32 }
    }

    /// `DoDragDrop`: runs a drag and drop operation, with a message loop of
    /// its own, until it ends. Returns the result code and the effect.
    ///
    /// # Safety
    /// `data_object` and `drop_source` are live COM objects that implement
    /// `IDataObject` and `IDropSource`.
    pub unsafe fn do_drag_drop(data_object: *mut c_void, drop_source: *mut c_void, ok_effects: i32) -> (u32, i32) {
        let mut effect = 0u32;
        // SAFETY: the contract of this function; the effect is a number of
        // this frame.
        let result = unsafe { ole::DoDragDrop(data_object, drop_source, ok_effects as u32, &mut effect) };
        (result as u32, effect as i32)
    }

    /// `CoMarshalInterThreadInterfaceInStream`: a stream that holds an
    /// interface pointer for another thread of the process, or the result
    /// code of the failure.
    ///
    /// # Safety
    /// `unknown` is a live COM object that implements the interface.
    pub unsafe fn co_marshal_inter_thread_interface_in_stream(
        iid: &ferroui_microcom::Guid,
        unknown: *mut c_void,
    ) -> Result<*mut c_void, u32> {
        let mut stream = std::ptr::null_mut();
        // SAFETY: the identifier has the layout of the system's; the rest
        // is the contract of this function.
        let result = unsafe {
            marshal::CoMarshalInterThreadInterfaceInStream((iid as *const ferroui_microcom::Guid).cast(), unknown, &mut stream)
        };
        if result < 0 || stream.is_null() {
            Err(result as u32)
        } else {
            Ok(stream)
        }
    }

    /// `CoGetInterfaceAndReleaseStream`: the interface pointer a stream of
    /// [`co_marshal_inter_thread_interface_in_stream`] holds, for the
    /// calling thread; the call releases a reference of the stream.
    ///
    /// # Safety
    /// `stream` is a live stream made by that function, of which the
    /// caller gives a reference away.
    pub unsafe fn co_get_interface_and_release_stream(
        stream: *mut c_void,
        iid: &ferroui_microcom::Guid,
    ) -> Result<*mut c_void, u32> {
        let mut pointer = std::ptr::null_mut();
        // SAFETY: as above.
        let result = unsafe {
            storage::CoGetInterfaceAndReleaseStream(stream, (iid as *const ferroui_microcom::Guid).cast(), &mut pointer)
        };
        if result < 0 || pointer.is_null() {
            Err(result as u32)
        } else {
            Ok(pointer)
        }
    }

    /// A device context of the screen with the memory contexts and the
    /// bitmaps made for a conversion, released when dropped.
    struct ScreenContexts {
        screen: gdi::HDC,
        memory: Vec<gdi::HDC>,
        objects: Vec<gdi::HGDIOBJ>,
    }

    impl ScreenContexts {
        fn new() -> Option<ScreenContexts> {
            // SAFETY: the context of the whole screen, released when the
            // value is dropped.
            let screen = unsafe { gdi::GetDC(std::ptr::null_mut()) };
            (!screen.is_null()).then(|| ScreenContexts { screen, memory: Vec::new(), objects: Vec::new() })
        }

        fn memory_context(&mut self) -> Option<gdi::HDC> {
            // SAFETY: a context compatible with a live one; deleted when
            // the value is dropped.
            let context = unsafe { gdi::CreateCompatibleDC(self.screen) };
            if context.is_null() {
                return None;
            }
            self.memory.push(context);
            Some(context)
        }

        /// A section selected into a memory context: the context and the
        /// memory of the pixels, which lives as long as this value.
        fn section(&mut self, header: &[u8]) -> Option<(gdi::HDC, *mut u8)> {
            let context = self.memory_context()?;
            let mut bits: *mut c_void = std::ptr::null_mut();
            // SAFETY: the header is a bitmap header of the size its first
            // member says, without a colour table (more than 8 bits a
            // pixel; the masks of a version 5 header are in the header).
            let section =
                unsafe { gdi::CreateDIBSection(context, header.as_ptr().cast(), 0, &mut bits, std::ptr::null_mut(), 0) };
            if section.is_null() || bits.is_null() {
                return None;
            }
            self.objects.push(section);
            // SAFETY: a live context and a live bitmap.
            unsafe { gdi::SelectObject(context, section) };
            Some((context, bits.cast()))
        }
    }

    impl Drop for ScreenContexts {
        fn drop(&mut self) {
            // SAFETY: the contexts and objects this value created. The
            // memory contexts are deleted before the bitmaps selected into
            // them. (The reference releases its memory contexts with
            // `ReleaseDC`, which does not free them.)
            unsafe {
                for &context in &self.memory {
                    gdi::DeleteDC(context);
                }
                for &object in &self.objects {
                    gdi::DeleteObject(object);
                }
                gdi::ReleaseDC(std::ptr::null_mut(), self.screen);
            }
        }
    }

    fn header_32(width: i32, height: i32) -> BITMAPINFOHEADER {
        let mut header = BITMAPINFOHEADER {
            bi_width: width,
            bi_height: height,
            bi_planes: 1,
            bi_bit_count: 32,
            bi_compression: BitmapCompressionMode::BI_RGB,
            bi_size_image: (width as u32).wrapping_mul(4).wrapping_mul(height.unsigned_abs()),
            ..Default::default()
        };
        header.init();
        header
    }

    /// The pixels of a bitmap handle (`CF_BITMAP`), drawn into a section
    /// of 32 bits a pixel (`ReadDataFromGdi` of the reference).
    ///
    /// # Safety
    /// `bitmap_handle` is a live bitmap that is selected into no context.
    pub unsafe fn hbitmap_to_bgra(bitmap_handle: isize) -> Option<BgraPixels> {
        let mut bitmap = gdi::BITMAP {
            bmType: 0,
            bmWidth: 0,
            bmHeight: 0,
            bmWidthBytes: 0,
            bmPlanes: 0,
            bmBitsPixel: 0,
            bmBits: std::ptr::null_mut(),
        };
        // SAFETY: a structure of this frame of the size given.
        let read = unsafe {
            gdi::GetObjectW(
                h(bitmap_handle),
                std::mem::size_of::<gdi::BITMAP>() as i32,
                (&mut bitmap as *mut gdi::BITMAP).cast(),
            )
        };
        if read == 0 || bitmap.bmWidth <= 0 || bitmap.bmHeight <= 0 {
            return None;
        }
        let (width, height) = (bitmap.bmWidth, bitmap.bmHeight);

        let mut contexts = ScreenContexts::new()?;
        // A negative height: the rows of the section run from the top, so
        // the bitmap is copied as it is. (The reference makes a section
        // whose rows run from the bottom and mirrors the copy.)
        let (destination, bits) = contexts.section(&header_32(width, -height).to_bytes())?;
        let source = contexts.memory_context()?;
        // SAFETY: live contexts; the bitmap is selected into the source
        // context for the copy and taken out of it again; the section has
        // `width * height * 4` bytes, read after the drawing is flushed.
        unsafe {
            let previous = gdi::SelectObject(source, h(bitmap_handle));
            let copied = gdi::BitBlt(destination, 0, 0, width, height, source, 0, 0, gdi::SRCCOPY) != 0;
            gdi::SelectObject(source, previous);
            if !copied {
                return None;
            }
            gdi::GdiFlush();
            let pixels = std::slice::from_raw_parts(bits, width as usize * height as usize * 4).to_vec();
            Some(BgraPixels { pixels, width, height })
        }
    }

    /// The pixels of a device independent bitmap (`CF_DIB`: a header, the
    /// colour table or masks `extra_header_size` bytes long, the pixels),
    /// drawn into a section of 32 bits a pixel (`StretchDIBits`).
    pub fn dib_to_bgra(data: &[u8], extra_header_size: usize) -> Option<BgraPixels> {
        let source_header = BITMAPINFOHEADER::from_bytes(data)?;
        let (width, height) = (source_header.bi_width, source_header.bi_height.checked_abs()?);
        let bits_offset = (source_header.bi_size as usize).checked_add(extra_header_size)?;
        if width <= 0 || height <= 0 || bits_offset > data.len() {
            return None;
        }
        // The bytes the pixels need, so that the system does not read
        // beyond the block: rows are padded to four bytes.
        if source_header.bi_compression == BitmapCompressionMode::BI_RGB
            || source_header.bi_compression == BitmapCompressionMode::BI_BITFIELDS
        {
            let row = (width as usize).checked_mul(usize::from(source_header.bi_bit_count))?.checked_add(31)? / 32 * 4;
            if row.checked_mul(height as usize)? > data.len() - bits_offset {
                return None;
            }
        } else if source_header.bi_size_image as usize > data.len() - bits_offset {
            return None;
        }

        let mut contexts = ScreenContexts::new()?;
        let (destination, bits) = contexts.section(&header_32(width, -height).to_bytes())?;
        // SAFETY: a live context; the header and the pixels are inside
        // `data` (checked above); the section has `width * height * 4`
        // bytes, read after the drawing is flushed.
        unsafe {
            let lines = gdi::StretchDIBits(
                destination,
                0,
                0,
                width,
                height,
                0,
                0,
                width,
                height,
                data.as_ptr().add(bits_offset).cast(),
                data.as_ptr().cast(),
                0,
                gdi::SRCCOPY,
            );
            if lines == 0 {
                return None;
            }
            gdi::GdiFlush();
            let pixels = std::slice::from_raw_parts(bits, width as usize * height as usize * 4).to_vec();
            Some(BgraPixels { pixels, width, height })
        }
    }

    /// A bitmap handle compatible with the screen that holds the pixels
    /// given, whose layout the version 5 header describes
    /// (`WriteDataToGdi` of the reference); 0 on failure. The caller owns
    /// the bitmap.
    pub fn pixels_to_hbitmap(pixels: &[u8], width: i32, height: i32, header: &BITMAPV5HEADER) -> isize {
        if width <= 0 || height <= 0 {
            return 0;
        }
        let Some(mut contexts) = ScreenContexts::new() else { return 0 };
        let Some((source, bits)) = contexts.section(&header.to_bytes()) else { return 0 };
        let Some(destination) = contexts.memory_context() else { return 0 };
        // The bytes of a row of a section: padded to four bytes.
        let row = (width as usize * usize::from(header.b_v5_bit_count)).div_ceil(32) * 4;
        let source_row = pixels.len() / height as usize;
        // SAFETY: the section has `row * height` bytes; every row of the
        // pixels is copied into its row of the section, no more of it than
        // either has. The bitmap is created for the screen, selected into
        // a context for the copy and taken out of it before it is handed
        // over.
        unsafe {
            for y in 0..height as usize {
                let count = row.min(source_row);
                std::ptr::copy_nonoverlapping(pixels.as_ptr().add(y * source_row), bits.add(y * row), count);
            }
            let bitmap = gdi::CreateCompatibleBitmap(contexts.screen, width, height);
            if bitmap.is_null() {
                return 0;
            }
            let previous = gdi::SelectObject(destination, bitmap);
            let copied = gdi::BitBlt(destination, 0, 0, width, height, source, 0, 0, gdi::SRCCOPY) != 0;
            gdi::SelectObject(destination, previous);
            gdi::GdiFlush();
            if !copied {
                gdi::DeleteObject(bitmap);
                return 0;
            }
            bitmap as isize
        }
    }

    // ----------------------------------------------------------------
    // Messages whose parameter points at a structure
    // ----------------------------------------------------------------

    /// Reads the structure of `WM_GETMINMAXINFO`.
    ///
    /// # Safety
    /// `l_param` is the parameter of a `WM_GETMINMAXINFO` message that is
    /// being processed.
    pub unsafe fn read_min_max_info(l_param: isize) -> MINMAXINFO {
        // SAFETY: by the contract of the function the parameter points at a
        // `MINMAXINFO`, whose layout the structure of this crate repeats.
        unsafe { *(l_param as *const MINMAXINFO) }
    }

    /// Writes the structure of `WM_GETMINMAXINFO`.
    ///
    /// # Safety
    /// As [`read_min_max_info`].
    pub unsafe fn write_min_max_info(l_param: isize, value: &MINMAXINFO) {
        // SAFETY: see `read_min_max_info`.
        unsafe { *(l_param as *mut MINMAXINFO) = *value };
    }

    /// Reads the structure of `WM_WINDOWPOSCHANGING` and
    /// `WM_WINDOWPOSCHANGED`.
    ///
    /// # Safety
    /// `l_param` is the parameter of one of the two messages, which is
    /// being processed.
    pub unsafe fn read_window_pos(l_param: isize) -> WINDOWPOS {
        // SAFETY: by the contract of the function the parameter points at a
        // `WINDOWPOS`.
        let native = unsafe { *(l_param as *const wm::WINDOWPOS) };
        WINDOWPOS {
            hwnd: native.hwnd as isize,
            hwnd_insert_after: native.hwndInsertAfter as isize,
            x: native.x,
            y: native.y,
            cx: native.cx,
            cy: native.cy,
            flags: native.flags,
        }
    }

    /// Writes the position and size of the structure of
    /// `WM_WINDOWPOSCHANGING`.
    ///
    /// # Safety
    /// As [`read_window_pos`], for `WM_WINDOWPOSCHANGING`.
    pub unsafe fn write_window_pos_bounds(l_param: isize, x: i32, y: i32, cx: i32, cy: i32) {
        // SAFETY: see `read_window_pos`.
        let native = unsafe { &mut *(l_param as *mut wm::WINDOWPOS) };
        native.x = x;
        native.y = y;
        native.cx = cx;
        native.cy = cy;
    }

    /// Reads the first rectangle of `WM_NCCALCSIZE` (with a true
    /// `wParam`): the proposed window rectangle.
    ///
    /// # Safety
    /// `l_param` is the parameter of a `WM_NCCALCSIZE` message whose
    /// `wParam` is 1 and that is being processed.
    pub unsafe fn read_nc_calc_size_rect(l_param: isize) -> RECT {
        // SAFETY: by the contract of the function the parameter points at
        // an `NCCALCSIZE_PARAMS`, which starts with three rectangles.
        to_rect(unsafe { (*(l_param as *const wm::NCCALCSIZE_PARAMS)).rgrc[0] })
    }

    /// Writes the first rectangle of `WM_NCCALCSIZE`: the client rectangle.
    ///
    /// # Safety
    /// As [`read_nc_calc_size_rect`].
    pub unsafe fn write_nc_calc_size_rect(l_param: isize, rect: RECT) {
        // SAFETY: see `read_nc_calc_size_rect`.
        unsafe { (*(l_param as *mut wm::NCCALCSIZE_PARAMS)).rgrc[0] = from_rect(rect) };
    }

    /// Reads the rectangle of `WM_DPICHANGED`: the suggested window
    /// rectangle at the new DPI.
    ///
    /// # Safety
    /// `l_param` is the parameter of a `WM_DPICHANGED` message that is
    /// being processed.
    pub unsafe fn read_rect(l_param: isize) -> RECT {
        // SAFETY: by the contract of the function the parameter points at a
        // `RECT`.
        to_rect(unsafe { *(l_param as *const wf::RECT) })
    }

    /// Reads the string of `WM_SETTINGCHANGE`: the name of the setting that
    /// changed, if the message carries one.
    ///
    /// # Safety
    /// `l_param` is the parameter of a `WM_SETTINGCHANGE` message that is
    /// being processed.
    pub unsafe fn read_setting_name(l_param: isize) -> Option<String> {
        if l_param == 0 {
            return None;
        }
        let start = l_param as *const u16;
        let mut length = 0usize;
        // SAFETY: by the contract of the function the parameter is null
        // (handled above) or a null-terminated string.
        unsafe {
            while *start.add(length) != 0 {
                length += 1;
            }
            Some(String::from_utf16_lossy(std::slice::from_raw_parts(start, length)))
        }
    }
}
