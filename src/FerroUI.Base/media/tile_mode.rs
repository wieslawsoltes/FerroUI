/// Describes how a tile brush is tiled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TileMode {
    #[default]
    None = 0,
    FlipX = 1,
    FlipY = 2,
    FlipXY = 3,
    Tile = 4,
}
