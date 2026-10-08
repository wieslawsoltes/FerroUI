use super::SurfaceOrientation;

/// Implemented by a surface that reports its orientation.
pub trait ISurfaceOrientation {
    fn orientation(&self) -> SurfaceOrientation;
}
