use crate::animation::AnimatorKeyFrame;
use crate::media::transformation::TransformOperations;
use crate::media::ITransform;
use crate::Ref;
use std::rc::Rc;

/// Interpolates transform operation lists.
///
/// This is the interpolation behind
/// [`TransformOperationsTransition`](crate::animation::TransformOperationsTransition).
/// It is not registered for any property type, so key frame animations do
/// not select it.
#[derive(Default)]
pub struct TransformOperationsAnimator;

impl TransformOperationsAnimator {
    pub fn new() -> Self {
        Self
    }

    /// Interpolates between two operation lists using the specified
    /// progress.
    pub fn interpolate(
        &self,
        progress: f64,
        old_value: &Rc<TransformOperations>,
        new_value: &Rc<TransformOperations>,
    ) -> Rc<TransformOperations> {
        TransformOperations::interpolate(old_value, new_value, progress)
    }

    /// The operation list of a transform; the identity when the transform
    /// is absent or not an operation list.
    pub(crate) fn ensure_operations(value: &Option<Rc<dyn ITransform>>) -> Rc<TransformOperations> {
        let Some(value) = value else { return TransformOperations::identity() };
        match value.as_any().downcast_ref::<TransformOperations>() {
            Some(operations) => {
                let mut builder = TransformOperations::create_builder(operations.operations().len());
                for operation in operations.operations() {
                    builder.append(operation.clone());
                }
                builder.build()
            }
            None => TransformOperations::identity(),
        }
    }

    /// Validates a key frame: its value must be an operation list.
    pub fn validate(&self, item: &Ref<AnimatorKeyFrame>) {
        let valid = item.with_value(|value| {
            value
                .and_then(|value| value.downcast_ref::<Option<Rc<dyn ITransform>>>())
                .and_then(|value| value.as_ref())
                .is_some_and(|value| value.as_any().is::<TransformOperations>())
        });
        if !valid {
            panic!("{} must have a value of type TransformOperations.", **item);
        }
    }
}
