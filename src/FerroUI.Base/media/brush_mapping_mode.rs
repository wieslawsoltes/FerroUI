/// Specifies the coordinate system used by a brush.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum BrushMappingMode {
    #[default]
    Absolute = 0,
    RelativeToBoundingBox = 1,
}
