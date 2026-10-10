// Permission is hereby granted, free of charge, to any person obtaining
// a copy of this software and associated documentation files (the
// "Software"), to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to
// permit persons to whom the Software is furnished to do so, subject to
// the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
// LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
// WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//
// Copyright (c) 2004 Novell, Inc.
//
// Authors:
//	Peter Bartok	pbartok@novell.com
//

//! The enumerations and the hint structures of `X11Structs.cs`.
//!
//! The event and request structures of that file are the ones of the Xlib bindings
//! (`crate::xlib`), which declare them with the layout of the C headers; what is ported
//! here is what the bindings do not have in a typed form: the enumerations, and the
//! Motif window manager hints.
//!
//! The file this is ported from carries the licence of the project it came from,
//! reproduced in the `NOTICE.md` of the crate.

/// `XWindowClass`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum XWindowClass {
    InputOutput = 1,
    InputOnly = 2,
}

impl XWindowClass {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<XWindowClass> {
        match value {
            1 => Some(XWindowClass::InputOutput),
            2 => Some(XWindowClass::InputOnly),
            _ => None,
        }
    }
}

/// `XEventName`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum XEventName {
    KeyPress = 2,
    KeyRelease = 3,
    ButtonPress = 4,
    ButtonRelease = 5,
    MotionNotify = 6,
    EnterNotify = 7,
    LeaveNotify = 8,
    FocusIn = 9,
    FocusOut = 10,
    KeymapNotify = 11,
    Expose = 12,
    GraphicsExpose = 13,
    NoExpose = 14,
    VisibilityNotify = 15,
    CreateNotify = 16,
    DestroyNotify = 17,
    UnmapNotify = 18,
    MapNotify = 19,
    MapRequest = 20,
    ReparentNotify = 21,
    ConfigureNotify = 22,
    ConfigureRequest = 23,
    GravityNotify = 24,
    ResizeRequest = 25,
    CirculateNotify = 26,
    CirculateRequest = 27,
    PropertyNotify = 28,
    SelectionClear = 29,
    SelectionRequest = 30,
    SelectionNotify = 31,
    ColormapNotify = 32,
    ClientMessage = 33,
    MappingNotify = 34,
    GenericEvent = 35,
    LASTEvent = 36,
}

impl XEventName {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<XEventName> {
        match value {
            2 => Some(XEventName::KeyPress),
            3 => Some(XEventName::KeyRelease),
            4 => Some(XEventName::ButtonPress),
            5 => Some(XEventName::ButtonRelease),
            6 => Some(XEventName::MotionNotify),
            7 => Some(XEventName::EnterNotify),
            8 => Some(XEventName::LeaveNotify),
            9 => Some(XEventName::FocusIn),
            10 => Some(XEventName::FocusOut),
            11 => Some(XEventName::KeymapNotify),
            12 => Some(XEventName::Expose),
            13 => Some(XEventName::GraphicsExpose),
            14 => Some(XEventName::NoExpose),
            15 => Some(XEventName::VisibilityNotify),
            16 => Some(XEventName::CreateNotify),
            17 => Some(XEventName::DestroyNotify),
            18 => Some(XEventName::UnmapNotify),
            19 => Some(XEventName::MapNotify),
            20 => Some(XEventName::MapRequest),
            21 => Some(XEventName::ReparentNotify),
            22 => Some(XEventName::ConfigureNotify),
            23 => Some(XEventName::ConfigureRequest),
            24 => Some(XEventName::GravityNotify),
            25 => Some(XEventName::ResizeRequest),
            26 => Some(XEventName::CirculateNotify),
            27 => Some(XEventName::CirculateRequest),
            28 => Some(XEventName::PropertyNotify),
            29 => Some(XEventName::SelectionClear),
            30 => Some(XEventName::SelectionRequest),
            31 => Some(XEventName::SelectionNotify),
            32 => Some(XEventName::ColormapNotify),
            33 => Some(XEventName::ClientMessage),
            34 => Some(XEventName::MappingNotify),
            35 => Some(XEventName::GenericEvent),
            36 => Some(XEventName::LASTEvent),
            _ => None,
        }
    }
}

bitflags::bitflags! {
    /// `SetWindowValuemask`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct SetWindowValuemask: i32 {
        const NOTHING = 0;
        const BACK_PIXMAP = 1;
        const BACK_PIXEL = 2;
        const BORDER_PIXMAP = 4;
        const BORDER_PIXEL = 8;
        const BIT_GRAVITY = 0x10;
        const WIN_GRAVITY = 0x20;
        const BACKING_STORE = 0x40;
        const BACKING_PLANES = 0x80;
        const BACKING_PIXEL = 0x100;
        const OVERRIDE_REDIRECT = 0x200;
        const SAVE_UNDER = 0x400;
        const EVENT_MASK = 0x800;
        const DONT_PROPAGATE = 0x1000;
        const COLOR_MAP = 0x2000;
        const CURSOR = 0x4000;
    }
}

/// `CreateWindowArgs`. Some of its members share a value, so it is a set of constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CreateWindowArgs(pub i32);

#[allow(non_upper_case_globals)]
impl CreateWindowArgs {
    pub const CopyFromParent: CreateWindowArgs = CreateWindowArgs(0);
    pub const ParentRelative: CreateWindowArgs = CreateWindowArgs(1);
    pub const InputOutput: CreateWindowArgs = CreateWindowArgs(1);
    pub const InputOnly: CreateWindowArgs = CreateWindowArgs(2);
}

/// `Gravity`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Gravity {
    ForgetGravity = 0,
    NorthWestGravity = 1,
    NorthGravity = 2,
    NorthEastGravity = 3,
    WestGravity = 4,
    CenterGravity = 5,
    EastGravity = 6,
    SouthWestGravity = 7,
    SouthGravity = 8,
    SouthEastGravity = 9,
    StaticGravity = 10,
}

impl Gravity {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<Gravity> {
        match value {
            0 => Some(Gravity::ForgetGravity),
            1 => Some(Gravity::NorthWestGravity),
            2 => Some(Gravity::NorthGravity),
            3 => Some(Gravity::NorthEastGravity),
            4 => Some(Gravity::WestGravity),
            5 => Some(Gravity::CenterGravity),
            6 => Some(Gravity::EastGravity),
            7 => Some(Gravity::SouthWestGravity),
            8 => Some(Gravity::SouthGravity),
            9 => Some(Gravity::SouthEastGravity),
            10 => Some(Gravity::StaticGravity),
            _ => None,
        }
    }
}

bitflags::bitflags! {
    /// `EventMask`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct EventMask: i32 {
        const NO_EVENT_MASK = 0;
        const KEY_PRESS_MASK = 1;
        const KEY_RELEASE_MASK = 2;
        const BUTTON_PRESS_MASK = 4;
        const BUTTON_RELEASE_MASK = 8;
        const ENTER_WINDOW_MASK = 0x10;
        const LEAVE_WINDOW_MASK = 0x20;
        const POINTER_MOTION_MASK = 0x40;
        const POINTER_MOTION_HINT_MASK = 0x80;
        const BUTTON1_MOTION_MASK = 0x100;
        const BUTTON2_MOTION_MASK = 0x200;
        const BUTTON3_MOTION_MASK = 0x400;
        const BUTTON4_MOTION_MASK = 0x800;
        const BUTTON5_MOTION_MASK = 0x1000;
        const BUTTON_MOTION_MASK = 0x2000;
        const KEYMAP_STATE_MASK = 0x4000;
        const EXPOSURE_MASK = 0x8000;
        const VISIBILITY_CHANGE_MASK = 0x10000;
        const STRUCTURE_NOTIFY_MASK = 0x20000;
        const RESIZE_REDIRECT_MASK = 0x40000;
        const SUBSTRUCTURE_NOTIFY_MASK = 0x80000;
        const SUBSTRUCTURE_REDIRECT_MASK = 0x100000;
        const FOCUS_CHANGE_MASK = 0x200000;
        const PROPERTY_CHANGE_MASK = 0x400000;
        const COLORMAP_CHANGE_MASK = 0x800000;
        const OWNER_GRAB_BUTTON_MASK = 0x1000000;
    }
}

bitflags::bitflags! {
    /// `RandrEventMask`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct RandrEventMask: i32 {
        const RR_SCREEN_CHANGE_NOTIFY = 1;
        const RR_CRTC_CHANGE_NOTIFY_MASK = 2;
        const RR_OUTPUT_CHANGE_NOTIFY_MASK = 4;
        const RR_OUTPUT_PROPERTY_NOTIFY_MASK = 8;
        const RR_PROVIDER_CHANGE_NOTIFY_MASK = 0x10;
        const RR_PROVIDER_PROPERTY_NOTIFY_MASK = 0x20;
        const RR_RESOURCE_CHANGE_NOTIFY_MASK = 0x40;
        const RR_LEASE_NOTIFY_MASK = 0x80;
    }
}

/// `RandrEvent`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum RandrEvent {
    RRScreenChangeNotify = 0,
    RRNotify = 1,
}

impl RandrEvent {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<RandrEvent> {
        match value {
            0 => Some(RandrEvent::RRScreenChangeNotify),
            1 => Some(RandrEvent::RRNotify),
            _ => None,
        }
    }
}

/// `RandrRotate`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum RandrRotate {
    RR_Rotate_0 = 1,
    RR_Rotate_90 = 2,
    RR_Rotate_180 = 4,
    RR_Rotate_270 = 8,
}

impl RandrRotate {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<RandrRotate> {
        match value {
            1 => Some(RandrRotate::RR_Rotate_0),
            2 => Some(RandrRotate::RR_Rotate_90),
            4 => Some(RandrRotate::RR_Rotate_180),
            8 => Some(RandrRotate::RR_Rotate_270),
            _ => None,
        }
    }
}

/// `Atom`. Some of its members share a value, so it is a set of constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Atom(pub i32);

#[allow(non_upper_case_globals)]
impl Atom {
    pub const AnyPropertyType: Atom = Atom(0);
    pub const XA_PRIMARY: Atom = Atom(1);
    pub const XA_SECONDARY: Atom = Atom(2);
    pub const XA_ARC: Atom = Atom(3);
    pub const XA_ATOM: Atom = Atom(4);
    pub const XA_BITMAP: Atom = Atom(5);
    pub const XA_CARDINAL: Atom = Atom(6);
    pub const XA_COLORMAP: Atom = Atom(7);
    pub const XA_CURSOR: Atom = Atom(8);
    pub const XA_CUT_BUFFER0: Atom = Atom(9);
    pub const XA_CUT_BUFFER1: Atom = Atom(10);
    pub const XA_CUT_BUFFER2: Atom = Atom(11);
    pub const XA_CUT_BUFFER3: Atom = Atom(12);
    pub const XA_CUT_BUFFER4: Atom = Atom(13);
    pub const XA_CUT_BUFFER5: Atom = Atom(14);
    pub const XA_CUT_BUFFER6: Atom = Atom(15);
    pub const XA_CUT_BUFFER7: Atom = Atom(16);
    pub const XA_DRAWABLE: Atom = Atom(17);
    pub const XA_FONT: Atom = Atom(18);
    pub const XA_INTEGER: Atom = Atom(19);
    pub const XA_PIXMAP: Atom = Atom(20);
    pub const XA_POINT: Atom = Atom(21);
    pub const XA_RECTANGLE: Atom = Atom(22);
    pub const XA_RESOURCE_MANAGER: Atom = Atom(23);
    pub const XA_RGB_COLOR_MAP: Atom = Atom(24);
    pub const XA_RGB_BEST_MAP: Atom = Atom(25);
    pub const XA_RGB_BLUE_MAP: Atom = Atom(26);
    pub const XA_RGB_DEFAULT_MAP: Atom = Atom(27);
    pub const XA_RGB_GRAY_MAP: Atom = Atom(28);
    pub const XA_RGB_GREEN_MAP: Atom = Atom(29);
    pub const XA_RGB_RED_MAP: Atom = Atom(30);
    pub const XA_STRING: Atom = Atom(31);
    pub const XA_VISUALID: Atom = Atom(32);
    pub const XA_WINDOW: Atom = Atom(33);
    pub const XA_WM_COMMAND: Atom = Atom(34);
    pub const XA_WM_HINTS: Atom = Atom(35);
    pub const XA_WM_CLIENT_MACHINE: Atom = Atom(36);
    pub const XA_WM_ICON_NAME: Atom = Atom(37);
    pub const XA_WM_ICON_SIZE: Atom = Atom(38);
    pub const XA_WM_NAME: Atom = Atom(39);
    pub const XA_WM_NORMAL_HINTS: Atom = Atom(40);
    pub const XA_WM_SIZE_HINTS: Atom = Atom(41);
    pub const XA_WM_ZOOM_HINTS: Atom = Atom(42);
    pub const XA_MIN_SPACE: Atom = Atom(43);
    pub const XA_NORM_SPACE: Atom = Atom(44);
    pub const XA_MAX_SPACE: Atom = Atom(45);
    pub const XA_END_SPACE: Atom = Atom(46);
    pub const XA_SUPERSCRIPT_X: Atom = Atom(47);
    pub const XA_SUPERSCRIPT_Y: Atom = Atom(48);
    pub const XA_SUBSCRIPT_X: Atom = Atom(49);
    pub const XA_SUBSCRIPT_Y: Atom = Atom(50);
    pub const XA_UNDERLINE_POSITION: Atom = Atom(51);
    pub const XA_UNDERLINE_THICKNESS: Atom = Atom(52);
    pub const XA_STRIKEOUT_ASCENT: Atom = Atom(53);
    pub const XA_STRIKEOUT_DESCENT: Atom = Atom(54);
    pub const XA_ITALIC_ANGLE: Atom = Atom(55);
    pub const XA_X_HEIGHT: Atom = Atom(56);
    pub const XA_QUAD_WIDTH: Atom = Atom(57);
    pub const XA_WEIGHT: Atom = Atom(58);
    pub const XA_POINT_SIZE: Atom = Atom(59);
    pub const XA_RESOLUTION: Atom = Atom(60);
    pub const XA_COPYRIGHT: Atom = Atom(61);
    pub const XA_NOTICE: Atom = Atom(62);
    pub const XA_FONT_NAME: Atom = Atom(63);
    pub const XA_FAMILY_NAME: Atom = Atom(64);
    pub const XA_FULL_NAME: Atom = Atom(65);
    pub const XA_CAP_HEIGHT: Atom = Atom(66);
    pub const XA_WM_CLASS: Atom = Atom(67);
    pub const XA_WM_TRANSIENT_FOR: Atom = Atom(68);
    pub const XA_LAST_PREDEFINED: Atom = Atom(68);
}

bitflags::bitflags! {
    /// `ChangeWindowFlags`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ChangeWindowFlags: i32 {
        const CWX = 1;
        const CWY = 2;
        const CW_WIDTH = 4;
        const CW_HEIGHT = 8;
        const CW_BORDER_WIDTH = 0x10;
        const CW_SIBLING = 0x20;
        const CW_STACK_MODE = 0x40;
    }
}

/// `StackMode`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum StackMode {
    Above = 0,
    Below = 1,
    TopIf = 2,
    BottomIf = 3,
    Opposite = 4,
}

impl StackMode {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<StackMode> {
        match value {
            0 => Some(StackMode::Above),
            1 => Some(StackMode::Below),
            2 => Some(StackMode::TopIf),
            3 => Some(StackMode::BottomIf),
            4 => Some(StackMode::Opposite),
            _ => None,
        }
    }
}

/// `NotifyMode`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NotifyMode {
    NotifyNormal = 0,
    NotifyGrab = 1,
    NotifyUngrab = 2,
}

impl NotifyMode {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<NotifyMode> {
        match value {
            0 => Some(NotifyMode::NotifyNormal),
            1 => Some(NotifyMode::NotifyGrab),
            2 => Some(NotifyMode::NotifyUngrab),
            _ => None,
        }
    }
}

/// `NotifyDetail`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NotifyDetail {
    NotifyAncestor = 0,
    NotifyVirtual = 1,
    NotifyInferior = 2,
    NotifyNonlinear = 3,
    NotifyNonlinearVirtual = 4,
    NotifyPointer = 5,
    NotifyPointerRoot = 6,
    NotifyDetailNone = 7,
}

impl NotifyDetail {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<NotifyDetail> {
        match value {
            0 => Some(NotifyDetail::NotifyAncestor),
            1 => Some(NotifyDetail::NotifyVirtual),
            2 => Some(NotifyDetail::NotifyInferior),
            3 => Some(NotifyDetail::NotifyNonlinear),
            4 => Some(NotifyDetail::NotifyNonlinearVirtual),
            5 => Some(NotifyDetail::NotifyPointer),
            6 => Some(NotifyDetail::NotifyPointerRoot),
            7 => Some(NotifyDetail::NotifyDetailNone),
            _ => None,
        }
    }
}

bitflags::bitflags! {
    /// `MotifFlags`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct MotifFlags: i32 {
        const FUNCTIONS = 1;
        const DECORATIONS = 2;
        const INPUT_MODE = 4;
        const STATUS = 8;
    }
}

bitflags::bitflags! {
    /// `MotifFunctions`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct MotifFunctions: i32 {
        const ALL = 1;
        const RESIZE = 2;
        const MOVE = 4;
        const MINIMIZE = 8;
        const MAXIMIZE = 0x10;
        const CLOSE = 0x20;
    }
}

bitflags::bitflags! {
    /// `MotifDecorations`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct MotifDecorations: i32 {
        const ALL = 1;
        const BORDER = 2;
        const RESIZE_H = 4;
        const TITLE = 8;
        const MENU = 0x10;
        const MINIMIZE = 0x20;
        const MAXIMIZE = 0x40;
    }
}

bitflags::bitflags! {
    /// `MotifInputMode`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct MotifInputMode: i32 {
        const MODELESS = 0;
        const APPLICATION_MODAL = 1;
        const SYSTEM_MODAL = 2;
        const FULL_APPLICATION_MODAL = 3;
    }
}

/// `NetWindowManagerState`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NetWindowManagerState {
    Remove = 0,
    Add = 1,
    Toggle = 2,
}

impl NetWindowManagerState {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<NetWindowManagerState> {
        match value {
            0 => Some(NetWindowManagerState::Remove),
            1 => Some(NetWindowManagerState::Add),
            2 => Some(NetWindowManagerState::Toggle),
            _ => None,
        }
    }
}

/// `RevertTo`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum RevertTo {
    None = 0,
    PointerRoot = 1,
    Parent = 2,
}

impl RevertTo {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<RevertTo> {
        match value {
            0 => Some(RevertTo::None),
            1 => Some(RevertTo::PointerRoot),
            2 => Some(RevertTo::Parent),
            _ => None,
        }
    }
}

/// `MapState`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum MapState {
    IsUnmapped = 0,
    IsUnviewable = 1,
    IsViewable = 2,
}

impl MapState {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<MapState> {
        match value {
            0 => Some(MapState::IsUnmapped),
            1 => Some(MapState::IsUnviewable),
            2 => Some(MapState::IsViewable),
            _ => None,
        }
    }
}

/// `CursorFontShape`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CursorFontShape {
    XC_X_cursor = 0,
    XC_arrow = 2,
    XC_based_arrow_down = 4,
    XC_based_arrow_up = 6,
    XC_boat = 8,
    XC_bogosity = 10,
    XC_bottom_left_corner = 12,
    XC_bottom_right_corner = 14,
    XC_bottom_side = 16,
    XC_bottom_tee = 18,
    XC_box_spiral = 20,
    XC_center_ptr = 22,
    XC_circle = 24,
    XC_clock = 26,
    XC_coffee_mug = 28,
    XC_cross = 30,
    XC_cross_reverse = 32,
    XC_crosshair = 34,
    XC_diamond_cross = 36,
    XC_dot = 38,
    XC_dotbox = 40,
    XC_double_arrow = 42,
    XC_draft_large = 44,
    XC_draft_small = 46,
    XC_draped_box = 48,
    XC_exchange = 50,
    XC_fleur = 52,
    XC_gobbler = 54,
    XC_gumby = 56,
    XC_hand1 = 58,
    XC_hand2 = 60,
    XC_heart = 62,
    XC_icon = 64,
    XC_iron_cross = 66,
    XC_left_ptr = 68,
    XC_left_side = 70,
    XC_left_tee = 72,
    XC_left_button = 74,
    XC_ll_angle = 76,
    XC_lr_angle = 78,
    XC_man = 80,
    XC_middlebutton = 82,
    XC_mouse = 84,
    XC_pencil = 86,
    XC_pirate = 88,
    XC_plus = 90,
    XC_question_arrow = 92,
    XC_right_ptr = 94,
    XC_right_side = 96,
    XC_right_tee = 98,
    XC_rightbutton = 100,
    XC_rtl_logo = 102,
    XC_sailboat = 104,
    XC_sb_down_arrow = 106,
    XC_sb_h_double_arrow = 108,
    XC_sb_left_arrow = 110,
    XC_sb_right_arrow = 112,
    XC_sb_up_arrow = 114,
    XC_sb_v_double_arrow = 116,
    XC_sb_shuttle = 118,
    XC_sizing = 120,
    XC_spider = 122,
    XC_spraycan = 124,
    XC_star = 126,
    XC_target = 128,
    XC_tcross = 130,
    XC_top_left_arrow = 132,
    XC_top_left_corner = 134,
    XC_top_right_corner = 136,
    XC_top_side = 138,
    XC_top_tee = 140,
    XC_trek = 142,
    XC_ul_angle = 144,
    XC_umbrella = 146,
    XC_ur_angle = 148,
    XC_watch = 150,
    XC_xterm = 152,
    XC_num_glyphs = 154,
}

impl CursorFontShape {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<CursorFontShape> {
        match value {
            0 => Some(CursorFontShape::XC_X_cursor),
            2 => Some(CursorFontShape::XC_arrow),
            4 => Some(CursorFontShape::XC_based_arrow_down),
            6 => Some(CursorFontShape::XC_based_arrow_up),
            8 => Some(CursorFontShape::XC_boat),
            10 => Some(CursorFontShape::XC_bogosity),
            12 => Some(CursorFontShape::XC_bottom_left_corner),
            14 => Some(CursorFontShape::XC_bottom_right_corner),
            16 => Some(CursorFontShape::XC_bottom_side),
            18 => Some(CursorFontShape::XC_bottom_tee),
            20 => Some(CursorFontShape::XC_box_spiral),
            22 => Some(CursorFontShape::XC_center_ptr),
            24 => Some(CursorFontShape::XC_circle),
            26 => Some(CursorFontShape::XC_clock),
            28 => Some(CursorFontShape::XC_coffee_mug),
            30 => Some(CursorFontShape::XC_cross),
            32 => Some(CursorFontShape::XC_cross_reverse),
            34 => Some(CursorFontShape::XC_crosshair),
            36 => Some(CursorFontShape::XC_diamond_cross),
            38 => Some(CursorFontShape::XC_dot),
            40 => Some(CursorFontShape::XC_dotbox),
            42 => Some(CursorFontShape::XC_double_arrow),
            44 => Some(CursorFontShape::XC_draft_large),
            46 => Some(CursorFontShape::XC_draft_small),
            48 => Some(CursorFontShape::XC_draped_box),
            50 => Some(CursorFontShape::XC_exchange),
            52 => Some(CursorFontShape::XC_fleur),
            54 => Some(CursorFontShape::XC_gobbler),
            56 => Some(CursorFontShape::XC_gumby),
            58 => Some(CursorFontShape::XC_hand1),
            60 => Some(CursorFontShape::XC_hand2),
            62 => Some(CursorFontShape::XC_heart),
            64 => Some(CursorFontShape::XC_icon),
            66 => Some(CursorFontShape::XC_iron_cross),
            68 => Some(CursorFontShape::XC_left_ptr),
            70 => Some(CursorFontShape::XC_left_side),
            72 => Some(CursorFontShape::XC_left_tee),
            74 => Some(CursorFontShape::XC_left_button),
            76 => Some(CursorFontShape::XC_ll_angle),
            78 => Some(CursorFontShape::XC_lr_angle),
            80 => Some(CursorFontShape::XC_man),
            82 => Some(CursorFontShape::XC_middlebutton),
            84 => Some(CursorFontShape::XC_mouse),
            86 => Some(CursorFontShape::XC_pencil),
            88 => Some(CursorFontShape::XC_pirate),
            90 => Some(CursorFontShape::XC_plus),
            92 => Some(CursorFontShape::XC_question_arrow),
            94 => Some(CursorFontShape::XC_right_ptr),
            96 => Some(CursorFontShape::XC_right_side),
            98 => Some(CursorFontShape::XC_right_tee),
            100 => Some(CursorFontShape::XC_rightbutton),
            102 => Some(CursorFontShape::XC_rtl_logo),
            104 => Some(CursorFontShape::XC_sailboat),
            106 => Some(CursorFontShape::XC_sb_down_arrow),
            108 => Some(CursorFontShape::XC_sb_h_double_arrow),
            110 => Some(CursorFontShape::XC_sb_left_arrow),
            112 => Some(CursorFontShape::XC_sb_right_arrow),
            114 => Some(CursorFontShape::XC_sb_up_arrow),
            116 => Some(CursorFontShape::XC_sb_v_double_arrow),
            118 => Some(CursorFontShape::XC_sb_shuttle),
            120 => Some(CursorFontShape::XC_sizing),
            122 => Some(CursorFontShape::XC_spider),
            124 => Some(CursorFontShape::XC_spraycan),
            126 => Some(CursorFontShape::XC_star),
            128 => Some(CursorFontShape::XC_target),
            130 => Some(CursorFontShape::XC_tcross),
            132 => Some(CursorFontShape::XC_top_left_arrow),
            134 => Some(CursorFontShape::XC_top_left_corner),
            136 => Some(CursorFontShape::XC_top_right_corner),
            138 => Some(CursorFontShape::XC_top_side),
            140 => Some(CursorFontShape::XC_top_tee),
            142 => Some(CursorFontShape::XC_trek),
            144 => Some(CursorFontShape::XC_ul_angle),
            146 => Some(CursorFontShape::XC_umbrella),
            148 => Some(CursorFontShape::XC_ur_angle),
            150 => Some(CursorFontShape::XC_watch),
            152 => Some(CursorFontShape::XC_xterm),
            154 => Some(CursorFontShape::XC_num_glyphs),
            _ => None,
        }
    }
}

/// `SystrayRequest`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SystrayRequest {
    SYSTEM_TRAY_REQUEST_DOCK = 0,
    SYSTEM_TRAY_BEGIN_MESSAGE = 1,
    SYSTEM_TRAY_CANCEL_MESSAGE = 2,
}

impl SystrayRequest {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<SystrayRequest> {
        match value {
            0 => Some(SystrayRequest::SYSTEM_TRAY_REQUEST_DOCK),
            1 => Some(SystrayRequest::SYSTEM_TRAY_BEGIN_MESSAGE),
            2 => Some(SystrayRequest::SYSTEM_TRAY_CANCEL_MESSAGE),
            _ => None,
        }
    }
}

/// `NetWmStateRequest`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NetWmStateRequest {
    _NET_WM_STATE_REMOVE = 0,
    _NET_WM_STATE_ADD = 1,
    _NET_WM_STATE_TOGGLE = 2,
}

impl NetWmStateRequest {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<NetWmStateRequest> {
        match value {
            0 => Some(NetWmStateRequest::_NET_WM_STATE_REMOVE),
            1 => Some(NetWmStateRequest::_NET_WM_STATE_ADD),
            2 => Some(NetWmStateRequest::_NET_WM_STATE_TOGGLE),
            _ => None,
        }
    }
}

/// `NetWmMoveResize`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NetWmMoveResize {
    _NET_WM_MOVERESIZE_SIZE_TOPLEFT = 0,
    _NET_WM_MOVERESIZE_SIZE_TOP = 1,
    _NET_WM_MOVERESIZE_SIZE_TOPRIGHT = 2,
    _NET_WM_MOVERESIZE_SIZE_RIGHT = 3,
    _NET_WM_MOVERESIZE_SIZE_BOTTOMRIGHT = 4,
    _NET_WM_MOVERESIZE_SIZE_BOTTOM = 5,
    _NET_WM_MOVERESIZE_SIZE_BOTTOMLEFT = 6,
    _NET_WM_MOVERESIZE_SIZE_LEFT = 7,
    _NET_WM_MOVERESIZE_MOVE = 8,
    _NET_WM_MOVERESIZE_SIZE_KEYBOARD = 9,
    _NET_WM_MOVERESIZE_MOVE_KEYBOARD = 10,
    _NET_WM_MOVERESIZE_CANCEL = 11,
}

impl NetWmMoveResize {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<NetWmMoveResize> {
        match value {
            0 => Some(NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_TOPLEFT),
            1 => Some(NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_TOP),
            2 => Some(NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_TOPRIGHT),
            3 => Some(NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_RIGHT),
            4 => Some(NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_BOTTOMRIGHT),
            5 => Some(NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_BOTTOM),
            6 => Some(NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_BOTTOMLEFT),
            7 => Some(NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_LEFT),
            8 => Some(NetWmMoveResize::_NET_WM_MOVERESIZE_MOVE),
            9 => Some(NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_KEYBOARD),
            10 => Some(NetWmMoveResize::_NET_WM_MOVERESIZE_MOVE_KEYBOARD),
            11 => Some(NetWmMoveResize::_NET_WM_MOVERESIZE_CANCEL),
            _ => None,
        }
    }
}

bitflags::bitflags! {
    /// `XSizeHintsFlags`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct XSizeHintsFlags: i32 {
        const US_POSITION = 1;
        const US_SIZE = 2;
        const P_POSITION = 4;
        const P_SIZE = 8;
        const P_MIN_SIZE = 0x10;
        const P_MAX_SIZE = 0x20;
        const P_RESIZE_INC = 0x40;
        const P_ASPECT = 0x80;
        const P_ALL_HINTS = 0xfc;
        const P_BASE_SIZE = 0x100;
        const P_WIN_GRAVITY = 0x200;
    }
}

bitflags::bitflags! {
    /// `XWMHintsFlags`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct XWMHintsFlags: i32 {
        const INPUT_HINT = 1;
        const STATE_HINT = 2;
        const ICON_PIXMAP_HINT = 4;
        const ICON_WINDOW_HINT = 8;
        const ICON_POSITION_HINT = 0x10;
        const ICON_MASK_HINT = 0x20;
        const WINDOW_GROUP_HINT = 0x40;
        const ALL_HINTS = 0x7f;
    }
}

/// `XInitialState`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum XInitialState {
    DontCareState = 0,
    NormalState = 1,
    ZoomState = 2,
    IconicState = 3,
    InactiveState = 4,
}

impl XInitialState {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<XInitialState> {
        match value {
            0 => Some(XInitialState::DontCareState),
            1 => Some(XInitialState::NormalState),
            2 => Some(XInitialState::ZoomState),
            3 => Some(XInitialState::IconicState),
            4 => Some(XInitialState::InactiveState),
            _ => None,
        }
    }
}

bitflags::bitflags! {
    /// `XIMProperties`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct XIMProperties: i32 {
        const XIM_PREEDIT_AREA = 1;
        const XIM_PREEDIT_CALLBACKS = 2;
        const XIM_PREEDIT_POSITION = 4;
        const XIM_PREEDIT_NOTHING = 8;
        const XIM_PREEDIT_NONE = 0x10;
        const XIM_STATUS_AREA = 0x100;
        const XIM_STATUS_CALLBACKS = 0x200;
        const XIM_STATUS_NOTHING = 0x400;
        const XIM_STATUS_NONE = 0x800;
    }
}

bitflags::bitflags! {
    /// `WindowType`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct WindowType: i32 {
        const CLIENT = 1;
        const WHOLE = 2;
        const BOTH = 3;
    }
}

/// `XEmbedMessage`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum XEmbedMessage {
    EmbeddedNotify = 0,
    WindowActivate = 1,
    WindowDeactivate = 2,
    RequestFocus = 3,
    FocusIn = 4,
    FocusOut = 5,
    FocusNext = 6,
    FocusPrev = 7,
    ModalityOn = 10,
    ModalityOff = 11,
    RegisterAccelerator = 12,
    UnregisterAccelerator = 13,
    ActivateAccelerator = 14,
}

impl XEmbedMessage {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<XEmbedMessage> {
        match value {
            0 => Some(XEmbedMessage::EmbeddedNotify),
            1 => Some(XEmbedMessage::WindowActivate),
            2 => Some(XEmbedMessage::WindowDeactivate),
            3 => Some(XEmbedMessage::RequestFocus),
            4 => Some(XEmbedMessage::FocusIn),
            5 => Some(XEmbedMessage::FocusOut),
            6 => Some(XEmbedMessage::FocusNext),
            7 => Some(XEmbedMessage::FocusPrev),
            10 => Some(XEmbedMessage::ModalityOn),
            11 => Some(XEmbedMessage::ModalityOff),
            12 => Some(XEmbedMessage::RegisterAccelerator),
            13 => Some(XEmbedMessage::UnregisterAccelerator),
            14 => Some(XEmbedMessage::ActivateAccelerator),
            _ => None,
        }
    }
}

/// `MotifWmHints`: the value of the `_MOTIF_WM_HINTS` property, five items of format 32.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MotifWmHints {
    pub flags: std::ffi::c_ulong,
    pub functions: std::ffi::c_ulong,
    pub decorations: std::ffi::c_ulong,
    pub input_mode: std::ffi::c_ulong,
    pub status: std::ffi::c_ulong,
}

impl MotifWmHints {
    /// The items of the property, in the order of the structure.
    pub fn to_longs(self) -> [std::ffi::c_ulong; 5] {
        [self.flags, self.functions, self.decorations, self.input_mode, self.status]
    }
}

impl std::fmt::Display for MotifWmHints {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MotifWmHints <flags={:?}, functions={:?}, decorations={:?}, input_mode={:?}, status={}",
            MotifFlags::from_bits_retain(self.flags as i32),
            MotifFunctions::from_bits_retain(self.functions as i32),
            MotifDecorations::from_bits_retain(self.decorations as i32),
            MotifInputMode::from_bits_retain(self.input_mode as i32),
            self.status as i32
        )
    }
}
