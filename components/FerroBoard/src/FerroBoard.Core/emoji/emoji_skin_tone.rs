// Generated from pinned upstream declarations by scripts/generate_settings.py.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EmojiSkinTone(pub i32);
#[allow(non_upper_case_globals)]
impl EmojiSkinTone {
    pub const Default: Self = Self(0);
    pub const Light: Self = Self(1);
    pub const MediumLight: Self = Self(2);
    pub const Medium: Self = Self(3);
    pub const MediumDark: Self = Self(4);
    pub const Dark: Self = Self(5);
}
