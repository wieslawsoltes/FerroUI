/// The level of transparency a composition target is rendered with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CompositionTransparencyLevel {
    #[default]
    None = 0,
    Transparent = 1,
    Blur = 2,
    AcrylicBlur = 3,
    Mica = 4,
}
