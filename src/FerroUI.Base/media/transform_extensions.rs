use crate::media::immutable::ImmutableTransform;
use crate::media::ITransform;

/// Extension methods for transform classes.
pub struct TransformExtensions;

impl TransformExtensions {
    /// Converts a transform to an immutable transform.
    pub fn to_immutable(transform: &dyn ITransform) -> ImmutableTransform {
        ImmutableTransform::new(transform.value())
    }
}
