/// The rotation of a surface relative to its natural orientation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SurfaceOrientation {
    #[default]
    Rotation0 = 0,
    Rotation90 = 1,
    Rotation180 = 2,
    Rotation270 = 3,
}
