//! Exact pinned SDK enum values; unknown CLR values stay representable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ContentDialogButton(i32);
ferroui_base::styled_value!(ContentDialogButton);
#[allow(non_upper_case_globals)]
impl ContentDialogButton {
    pub const None: Self = Self(0);
    pub const Primary: Self = Self(1);
    pub const Secondary: Self = Self(2);
    pub const Close: Self = Self(3);
    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}
