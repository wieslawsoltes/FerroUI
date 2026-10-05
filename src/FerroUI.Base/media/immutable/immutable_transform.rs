use crate::media::ITransform;
use crate::Matrix;
use std::any::Any;

/// Represents a transform on a visual or brush whose value cannot change.
#[derive(Clone, Copy, Debug)]
pub struct ImmutableTransform {
    value: Matrix,
}

impl ImmutableTransform {
    pub const fn new(matrix: Matrix) -> Self {
        Self { value: matrix }
    }

    /// The transform's matrix.
    #[inline]
    pub fn value(&self) -> Matrix {
        self.value
    }
}

impl ITransform for ImmutableTransform {
    #[inline]
    fn value(&self) -> Matrix {
        self.value
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
