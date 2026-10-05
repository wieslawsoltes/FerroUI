use std::rc::Rc;

/// A callback that customizes the window styles: receives the style and
/// the extended style and returns the pair to use.
pub type CustomWindowStylesCallback = Rc<dyn Fn(u32, u32) -> (u32, u32)>;

/// A callback that hooks the window procedure: receives the window handle,
/// the message and its two parameters, sets the flag when it handled the
/// message and returns the result of the message.
pub type CustomWndProcHookCallback = Rc<dyn Fn(isize, u32, isize, isize, &mut bool) -> isize>;

/// Represents a hit test value for a visual.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Win32HitTestValue {
    #[default]
    Nowhere = 0,
    Client = 1,
    Caption = 2,
    MinButton = 8,
    MaxButton = 9,
    Left = 10,
    Right = 11,
    Top = 12,
    TopLeft = 13,
    TopRight = 14,
    Bottom = 15,
    BottomLeft = 16,
    BottomRight = 17,
    Close = 20,
}

/// Represents the rounded corner preference for a window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum WindowCornerPreference {
    /// Let the system decide when to round window corners.
    #[default]
    Default = 0,

    /// Never round window corners.
    DoNotRound = 1,

    /// Round the corners, if appropriate.
    Round = 2,

    /// Round the corners if appropriate, with a small radius.
    RoundSmall = 3,
}
