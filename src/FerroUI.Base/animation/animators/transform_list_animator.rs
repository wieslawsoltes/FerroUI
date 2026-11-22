use crate::StyledProperty;
use crate::animation::Animatable;
use crate::animation::animators::{Animator, AnimatorBase};
use crate::media::{ITransform, transformation::TransformOperations};
use std::rc::Rc;

type TransformValue = Option<Rc<dyn ITransform>>;

/// Interpolates complete transform properties such as `Visual.RenderTransform`.
/// It uses the same transform-list interpolation as the transition animator.
#[derive(Default)]
pub struct TransformListAnimator {
    base: AnimatorBase,
}

impl TransformListAnimator {
    pub fn base_value(
        target: &Animatable,
        property: &'static StyledProperty<TransformValue>,
    ) -> TransformValue {
        target.values().unanimated_value(target, property)
    }

    pub fn interpolate_value(
        progress: f64,
        from: &TransformValue,
        to: &TransformValue,
    ) -> TransformValue {
        if progress == 0.0 || from == to {
            return from.clone();
        }
        if progress == 1.0 {
            return to.clone();
        }
        let interpolated = with_operations(from.as_deref(), |from| {
            with_operations(to.as_deref(), |to| {
                TransformOperations::try_interpolate(from, to, progress)
            })
        });
        interpolated
            .map(|value| Some(value as Rc<dyn ITransform>))
            .unwrap_or_else(|| {
                if progress < 0.5 {
                    from.clone()
                } else {
                    to.clone()
                }
            })
    }
}

fn with_operations<R>(
    transform: Option<&dyn ITransform>,
    use_list: impl FnOnce(&TransformOperations) -> R,
) -> R {
    let Some(transform) = transform else {
        return use_list(&TransformOperations::identity());
    };
    if let Some(operations) = transform.as_any().downcast_ref::<TransformOperations>() {
        return use_list(operations);
    }
    let mut list = TransformOperations::create_builder(1);
    list.append_matrix(transform.value());
    use_list(&list.build())
}

impl Animator for TransformListAnimator {
    type Value = TransformValue;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    fn neutral_value(&self, target: &Animatable) -> Self::Value {
        Self::base_value(target, self.base.styled_property())
    }

    fn interpolate(&self, progress: f64, from: &Self::Value, to: &Self::Value) -> Self::Value {
        Self::interpolate_value(progress, from, to)
    }
}
