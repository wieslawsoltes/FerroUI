//! The enumerations of the X protocol the backend names (the port of `X11Enums.cs`).

/// `Status`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Status {
    Success = 0,
    BadRequest = 1,
    BadValue = 2,
    BadWindow = 3,
    BadPixmap = 4,
    BadAtom = 5,
    BadCursor = 6,
    BadFont = 7,
    BadMatch = 8,
    BadDrawable = 9,
    BadAccess = 10,
    BadAlloc = 11,
    BadColor = 12,
    BadGC = 13,
    BadIDChoice = 14,
    BadName = 15,
    BadLength = 16,
    BadImplementation = 17,
    FirstExtensionError = 128,
    LastExtensionError = 255,
}

impl Status {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<Status> {
        match value {
            0 => Some(Status::Success),
            1 => Some(Status::BadRequest),
            2 => Some(Status::BadValue),
            3 => Some(Status::BadWindow),
            4 => Some(Status::BadPixmap),
            5 => Some(Status::BadAtom),
            6 => Some(Status::BadCursor),
            7 => Some(Status::BadFont),
            8 => Some(Status::BadMatch),
            9 => Some(Status::BadDrawable),
            10 => Some(Status::BadAccess),
            11 => Some(Status::BadAlloc),
            12 => Some(Status::BadColor),
            13 => Some(Status::BadGC),
            14 => Some(Status::BadIDChoice),
            15 => Some(Status::BadName),
            16 => Some(Status::BadLength),
            17 => Some(Status::BadImplementation),
            128 => Some(Status::FirstExtensionError),
            255 => Some(Status::LastExtensionError),
            _ => None,
        }
    }
}

bitflags::bitflags! {
    /// `XEventMask`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct XEventMask: i32 {
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
    /// `XModifierMask`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct XModifierMask: i32 {
        const SHIFT_MASK = 1;
        const LOCK_MASK = 2;
        const CONTROL_MASK = 4;
        const MOD1_MASK = 8;
        const MOD2_MASK = 0x10;
        const MOD3_MASK = 0x20;
        const MOD4_MASK = 0x40;
        const MOD5_MASK = 0x80;
        const BUTTON1_MASK = 0x100;
        const BUTTON2_MASK = 0x200;
        const BUTTON3_MASK = 0x400;
        const BUTTON4_MASK = 0x800;
        const BUTTON5_MASK = 0x1000;
        const ANY_MODIFIER = 0x8000;
    }
}

bitflags::bitflags! {
    /// `XCreateWindowFlags`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct XCreateWindowFlags: i32 {
        const CW_BACK_PIXMAP = 1;
        const CW_BACK_PIXEL = 2;
        const CW_BORDER_PIXMAP = 4;
        const CW_BORDER_PIXEL = 8;
        const CW_BIT_GRAVITY = 0x10;
        const CW_WIN_GRAVITY = 0x20;
        const CW_BACKING_STORE = 0x40;
        const CW_BACKING_PLANES = 0x80;
        const CW_BACKING_PIXEL = 0x100;
        const CW_OVERRIDE_REDIRECT = 0x200;
        const CW_SAVE_UNDER = 0x400;
        const CW_EVENT_MASK = 0x800;
        const CW_DONT_PROPAGATE = 0x1000;
        const CW_COLORMAP = 0x2000;
        const CW_CURSOR = 0x4000;
    }
}

/// `ShapeKind`.
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ShapeKind {
    ShapeBounding = 0,
    ShapeClip = 1,
    ShapeInput = 2,
}

impl ShapeKind {
    /// The member with the value, when there is one.
    pub fn from_value(value: i32) -> Option<ShapeKind> {
        match value {
            0 => Some(ShapeKind::ShapeBounding),
            1 => Some(ShapeKind::ShapeClip),
            2 => Some(ShapeKind::ShapeInput),
            _ => None,
        }
    }
}
