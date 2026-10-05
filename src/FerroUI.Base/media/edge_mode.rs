/// Specifies how the edges of non-text primitives are rendered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EdgeMode {
    #[default]
    Unspecified = 0,
    Antialias = 1,
    Aliased = 2,
}
