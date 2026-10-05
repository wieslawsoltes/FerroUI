/// Defines how a gradient is extended beyond its start and end points.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum GradientSpreadMethod {
    #[default]
    Pad = 0,
    Reflect = 1,
    Repeat = 2,
}
