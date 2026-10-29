//! Ported from `Composition/SelectionIndicatorAnimation.cs`.

use super::composition_animation_helpers::to_duration;
use crate::argument_out_of_range::throw_if_less_than;
use ferroui_base::animation::easings::{IEasing, SpringEasing};
use ferroui_base::animation::TimeSpan;
use ferroui_base::numerics::Vector3;
use ferroui_base::rendering::composition::animations::{ICompositionAnimation, ICompositionAnimationBase};
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::Visual;
use ferroui_controls::primitives::TemplatedControl;
use std::rc::Rc;

/// Creates and applies the composition animation used to move a selection indicator between
/// item containers.
pub struct SelectionIndicatorAnimation;

impl SelectionIndicatorAnimation {
    const INDICATOR_PART_NAME: &'static str = "PART_SelectedPipe";

    /// Gets the default selection indicator animation duration.
    pub fn default_duration() -> TimeSpan {
        TimeSpan::from_milliseconds(250.0)
    }

    /// Finds the standard selection indicator parts in the newly and the previously selected
    /// item containers and starts their movement animation over `duration`. Returns `true` when
    /// composition visuals were available and the animation was installed; otherwise, `false`.
    ///
    /// # Panics
    ///
    /// When `duration` is negative (the `ArgumentOutOfRangeException` of the reference).
    #[track_caller]
    pub fn try_start(
        new_selection: Option<&TemplatedControl>,
        old_selection: Option<&TemplatedControl>,
        duration: TimeSpan,
    ) -> bool {
        throw_if_less_than(duration, TimeSpan::ZERO, "duration");

        let (Some(new_selection), Some(old_selection)) = (new_selection, old_selection) else {
            return false;
        };

        let find_indicator = |selection: &TemplatedControl| {
            selection
                .get_visual_descendants()
                .find(|visual| visual.name().as_deref() == Some(Self::INDICATOR_PART_NAME))
        };
        let new_indicator = find_indicator(new_selection);
        let old_indicator = find_indicator(old_selection);

        let new_selection: &Visual = new_selection;
        let old_selection: &Visual = old_selection;
        Self::try_start_with_visuals(
            new_indicator.as_deref(),
            old_indicator.as_deref(),
            Some(new_selection),
            Some(old_selection),
            duration,
        )
    }

    /// Starts a selection indicator movement animation using explicitly supplied visuals: the
    /// indicator visuals in the newly and the previously selected containers, and the container
    /// visuals themselves. Returns `true` when composition visuals were available and the
    /// animation was installed; otherwise, `false`.
    ///
    /// Deviation (DEVIATIONS.md, Behaviors): the `TryStart(Visual, Visual, Visual, Visual,
    /// TimeSpan)` overload of the reference; Rust has no overloads.
    ///
    /// # Panics
    ///
    /// When `duration` is negative (the `ArgumentOutOfRangeException` of the reference).
    #[track_caller]
    pub fn try_start_with_visuals(
        new_indicator: Option<&Visual>,
        old_indicator: Option<&Visual>,
        new_selection: Option<&Visual>,
        old_selection: Option<&Visual>,
        duration: TimeSpan,
    ) -> bool {
        throw_if_less_than(duration, TimeSpan::ZERO, "duration");

        if duration == TimeSpan::ZERO {
            return false;
        }
        let (Some(new_indicator), Some(old_indicator), Some(new_selection), Some(old_selection)) =
            (new_indicator, old_indicator, new_selection, old_selection)
        else {
            return false;
        };

        if let Some(implicit_animations) =
            ElementComposition::get_element_visual(old_indicator).and_then(|visual| visual.implicit_animations())
        {
            implicit_animations.clear();
        }

        let (Some(indicator_visual), Some(new_selection_visual), Some(old_selection_visual)) = (
            ElementComposition::get_element_visual(new_indicator),
            ElementComposition::get_element_visual(new_selection),
            ElementComposition::get_element_visual(old_selection),
        ) else {
            return false;
        };

        let selection_offset = old_selection_visual.offset() - new_selection_visual.offset();
        let is_vertical_offset = selection_offset.y != 0.0;
        let offset = if is_vertical_offset { selection_offset.y } else { selection_offset.x };
        let compositor = indicator_visual.compositor();
        let spring_easing: Rc<dyn IEasing> = Rc::new(SpringEasing::new());

        let offset_animation = compositor.create_vector3_key_frame_animation();
        offset_animation.set_target(Some("Offset".to_string()));
        // `Math.Abs(offset)` formatted as the reference formats a double: the shortest text that
        // reads back as the same value.
        let expression = format!("{}{}", if offset > 0.0 { "+" } else { "-" }, offset.abs());
        offset_animation.insert_expression_key_frame(
            0.0,
            &if is_vertical_offset {
                format!("Vector3(this.FinalValue.X, this.FinalValue.Y{expression}, 0)")
            } else {
                format!("Vector3(this.FinalValue.X{expression}, this.FinalValue.Y, 0)")
            },
            None,
        );
        offset_animation.insert_expression_key_frame(1.0, "this.FinalValue", None);
        offset_animation.set_duration(to_duration(duration));

        let scale_animation = compositor.create_vector3_key_frame_animation();
        scale_animation.set_target(Some("Scale".to_string()));
        scale_animation.insert_key_frame_with_easing(0.0, Vector3::ONE, spring_easing.clone());
        scale_animation.insert_key_frame_with_easing(
            0.5,
            Vector3::new(
                1.0 + if !is_vertical_offset { 0.75 } else { 0.0 },
                1.0 + if is_vertical_offset { 0.75 } else { 0.0 },
                1.0,
            ),
            spring_easing.clone(),
        );
        scale_animation.insert_key_frame_with_easing(1.0, Vector3::ONE, spring_easing);
        scale_animation.set_duration(to_duration(duration));

        let animation_group = compositor.create_animation_group();
        animation_group.add(offset_animation as Rc<dyn ICompositionAnimation>);
        animation_group.add(scale_animation as Rc<dyn ICompositionAnimation>);

        let implicit_animations = compositor.create_implicit_animation_collection();
        let current_offset =
            if is_vertical_offset { indicator_visual.offset().y } else { indicator_visual.offset().x };
        implicit_animations.set(
            if current_offset == 0.0 { "Offset" } else { "Visible" },
            animation_group as Rc<dyn ICompositionAnimationBase>,
        );
        indicator_visual.set_implicit_animations(Some(implicit_animations));
        true
    }
}
