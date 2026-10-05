/// Defines constants that specify the orientation of a RefreshVisualizer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum RefreshVisualizerOrientation {
    #[default]
    Auto = 0,
    Normal = 1,
    Rotate90DegreesCounterclockwise = 2,
    Rotate270DegreesCounterclockwise = 3,
}
