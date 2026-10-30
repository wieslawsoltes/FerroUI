// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
/// Original pointer event phases passed to the host/controller engine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(i32)]
pub(crate) enum EventType {
    #[default]
    Pressed = 0,
    Released = 1,
    Moved = 2,
    Enter = 3,
    Leave = 4,
    CaptureLost = 5,
    WheelChanged = 6,
}
ferroui_base::ferro_markup_enum!(EventType {
    Pressed,
    Released,
    Moved,
    Enter,
    Leave,
    CaptureLost,
    WheelChanged
});
