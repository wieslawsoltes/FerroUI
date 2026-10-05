//! Port of `Pages/Transitions/CardStackPageTransition.cs`.

use ferroui_base::animation::{
    Animation, AnimationTask, Cue, IPageTransition, IProgressPageTransition, KeyFrame, PageSlide, PageTransitionItem,
    SlideAxis, TimeSpan,
};
use ferroui_base::media::{ITransform, RotateTransform, ScaleTransform, TransformGroup, TranslateTransform};
use ferroui_base::styling::Setter;
use ferroui_base::threading::{CancellationToken, DispatcherTask};
use ferroui_base::{Ref, RelativePoint, RelativeUnit, Visual};
use mini_mvvm::start_async;
use std::cell::Cell;
use std::rc::Rc;

const VIEWPORT_LIFT_SCALE: f64 = 0.03;
const VIEWPORT_PROMOTION_SCALE: f64 = 0.02;
const VIEWPORT_DEPTH_OPACITY_FALLOFF: f64 = 0.08;
const SIDE_PEEK_ANGLE: f64 = 4.0;
const FAR_PEEK_ANGLE: f64 = 7.0;

/// Transitions between two pages with a card-stack effect: the top page moves and rotates
/// away while the next page scales up underneath.
///
/// The class of the original derives from `PageSlide`; here the slide is the field `base`,
/// which [`page_slide`](Self::page_slide) gives access to.
pub struct CardStackPageTransition {
    base: PageSlide,
    max_swipe_angle: Cell<f64>,
    back_card_scale: Cell<f64>,
    back_card_offset: Cell<f64>,
}

impl Default for CardStackPageTransition {
    fn default() -> Self {
        Self::new()
    }
}

/// The transforms of a transform group that is the render transform of `visual`, when the
/// group has `count` children.
fn group_children(visual: &Ref<Visual>, count: usize) -> Option<Ref<TransformGroup>> {
    visual
        .render_transform()
        .and_then(|transform| transform.as_object().and_then(|object| object.to_ref().cast::<TransformGroup>()))
        .filter(|group| group.children().len() == count)
}

fn set_center_origin(visual: &Ref<Visual>) {
    visual.set_render_transform_origin(RelativePoint::new(0.5, 0.5, RelativeUnit::Relative));
}

fn set_group(visual: &Ref<Visual>, group: Ref<TransformGroup>) {
    let transform: Rc<dyn ITransform> = group.into();
    visual.set_render_transform(Some(transform));
}

/// `await Task.WhenAll(tasks)`: a failure to run one of the animations fails the transition.
async fn when_all(tasks: Vec<AnimationTask>) {
    if let Err(error) = AnimationTask::when_all(tasks).await {
        panic!("{error}");
    }
}

impl CardStackPageTransition {
    /// `new CardStackPageTransition()`.
    pub fn new() -> Self {
        Self::from_slide(PageSlide::new())
    }

    /// `new CardStackPageTransition(duration, orientation)`: `duration` is the duration of
    /// the animation and `orientation` the axis on which the animation occurs.
    pub fn with_duration(duration: TimeSpan, orientation: SlideAxis) -> Self {
        Self::from_slide(PageSlide::with_duration(duration, orientation))
    }

    fn from_slide(base: PageSlide) -> Self {
        Self { base, max_swipe_angle: Cell::new(15.0), back_card_scale: Cell::new(0.05), back_card_offset: Cell::new(0.0) }
    }

    /// The slide this transition derives from: its duration, orientation, easings and fill
    /// mode.
    pub fn page_slide(&self) -> &PageSlide {
        &self.base
    }

    /// The maximum rotation angle (degrees) applied to the top card.
    pub fn max_swipe_angle(&self) -> f64 {
        self.max_swipe_angle.get()
    }

    pub fn set_max_swipe_angle(&self, value: f64) {
        self.max_swipe_angle.set(value)
    }

    /// The scale reduction applied to the back card (0.05 = 5%).
    pub fn back_card_scale(&self) -> f64 {
        self.back_card_scale.get()
    }

    pub fn set_back_card_scale(&self, value: f64) {
        self.back_card_scale.set(value)
    }

    /// The vertical offset (pixels) applied to the back card.
    pub fn back_card_offset(&self) -> f64 {
        self.back_card_offset.get()
    }

    pub fn set_back_card_offset(&self, value: f64) {
        self.back_card_offset.set(value)
    }

    fn update_visible_items(
        &self,
        progress: f64,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        page_length: f64,
        visible_items: &[PageTransitionItem],
    ) {
        let is_horizontal = self.base.orientation() == SlideAxis::Horizontal;
        let rotation_target =
            if is_horizontal { if forward { -self.max_swipe_angle() } else { self.max_swipe_angle() } } else { 0.0 };
        let stack_offset = self.get_viewport_stack_offset(page_length);
        let lift = (progress.clamp(0.0, 1.0) * std::f64::consts::PI).sin();

        for item in visible_items {
            let visual = &item.visual;
            let (rotate, scale, translate) = Self::ensure_viewport_transforms(visual);
            let depth = Self::get_viewport_depth(item.viewport_center_offset);
            let scale_value = 0.84_f64.max(1.0 - (self.back_card_scale() * depth));
            let mut stack_value = stack_offset * depth;
            let mut base_opacity = 0.8_f64.max(1.0 - (VIEWPORT_DEPTH_OPACITY_FALLOFF * depth));
            let resting_angle =
                if is_horizontal { Self::get_viewport_resting_angle(item.viewport_center_offset) } else { 0.0 };

            rotate.set_angle(resting_angle);
            scale.set_scale_x(scale_value);
            scale.set_scale_y(scale_value);
            translate.set_x(0.0);
            translate.set_y(0.0);

            if from.is_some_and(|from| from == visual) {
                rotate.set_angle(resting_angle + (rotation_target * progress));
                stack_value -= stack_offset * 0.2 * lift;
                base_opacity = 1.0_f64.min(base_opacity + 0.08);
            }

            if to.is_some_and(|to| to == visual) {
                let promoted_scale =
                    1.0_f64.min(scale_value + (VIEWPORT_LIFT_SCALE * lift) + (VIEWPORT_PROMOTION_SCALE * progress));

                scale.set_scale_x(promoted_scale);
                scale.set_scale_y(promoted_scale);
                rotate.set_angle(resting_angle * (1.0 - progress));
                stack_value = 0.0_f64.max(stack_value - (stack_offset * (0.45 + (0.2 * lift)) * progress));
                base_opacity = 1.0_f64.min(base_opacity + (0.12 * lift));
            }

            if is_horizontal {
                translate.set_y(stack_value);
            } else {
                translate.set_x(stack_value);
            }

            visual.set_is_visible(true);
            visual.set_opacity(base_opacity);
            visual.set_z_index(Self::get_viewport_z_index(item.viewport_center_offset, visual, from, to));
        }
    }

    fn ensure_top_transforms(visual: &Ref<Visual>) -> (Ref<RotateTransform>, Ref<TranslateTransform>) {
        if let Some(group) = group_children(visual, 2) {
            let children = group.children();
            if let (Some(rotate), Some(translate)) =
                (children.get(0).cast::<RotateTransform>(), children.get(1).cast::<TranslateTransform>())
            {
                set_center_origin(visual);
                return (rotate, translate);
            }
        }

        let rotate = RotateTransform::new();
        let translate = TranslateTransform::new();
        let group = TransformGroup::new();
        group.children().add(rotate.clone().upcast());
        group.children().add(translate.clone().upcast());
        set_group(visual, group);
        set_center_origin(visual);
        (rotate, translate)
    }

    fn ensure_back_transforms(visual: &Ref<Visual>) -> (Ref<ScaleTransform>, Ref<TranslateTransform>) {
        if let Some(group) = group_children(visual, 2) {
            let children = group.children();
            if let (Some(scale), Some(translate)) =
                (children.get(0).cast::<ScaleTransform>(), children.get(1).cast::<TranslateTransform>())
            {
                set_center_origin(visual);
                return (scale, translate);
            }
        }

        let scale = ScaleTransform::new();
        let translate = TranslateTransform::new();
        let group = TransformGroup::new();
        group.children().add(scale.clone().upcast());
        group.children().add(translate.clone().upcast());
        set_group(visual, group);
        set_center_origin(visual);
        (scale, translate)
    }

    fn ensure_viewport_transforms(
        visual: &Ref<Visual>,
    ) -> (Ref<RotateTransform>, Ref<ScaleTransform>, Ref<TranslateTransform>) {
        if let Some(group) = group_children(visual, 3) {
            let children = group.children();
            if let (Some(rotate), Some(scale), Some(translate)) = (
                children.get(0).cast::<RotateTransform>(),
                children.get(1).cast::<ScaleTransform>(),
                children.get(2).cast::<TranslateTransform>(),
            ) {
                set_center_origin(visual);
                return (rotate, scale, translate);
            }
        }

        let rotate = RotateTransform::new();
        let scale = ScaleTransform::with_scale(1.0, 1.0);
        let translate = TranslateTransform::new();
        let group = TransformGroup::new();
        group.children().add(rotate.clone().upcast());
        group.children().add(scale.clone().upcast());
        group.children().add(translate.clone().upcast());
        set_group(visual, group);
        set_center_origin(visual);
        (rotate, scale, translate)
    }

    fn get_viewport_stack_offset(&self, page_length: f64) -> f64 {
        if self.back_card_offset() > 0.0 {
            return self.back_card_offset();
        }

        (page_length * 0.045).clamp(10.0, 18.0)
    }

    fn get_viewport_depth(offset_from_center: f64) -> f64 {
        let distance = offset_from_center.abs();

        if distance <= 1.0 {
            return distance;
        }

        if distance <= 2.0 {
            return 1.0 + ((distance - 1.0) * 0.8);
        }

        1.8
    }

    fn get_viewport_resting_angle(offset_from_center: f64) -> f64 {
        // `Math.Sign`: zero for zero.
        if offset_from_center == 0.0 {
            return 0.0;
        }
        let sign = if offset_from_center < 0.0 { -1.0 } else { 1.0 };

        let distance = offset_from_center.abs();
        if distance <= 1.0 {
            return sign * Self::lerp(0.0, SIDE_PEEK_ANGLE, distance);
        }

        if distance <= 2.0 {
            return sign * Self::lerp(SIDE_PEEK_ANGLE, FAR_PEEK_ANGLE, distance - 1.0);
        }

        sign * FAR_PEEK_ANGLE
    }

    /// `double.Lerp(from, to, Math.Clamp(t, 0, 1))`.
    fn lerp(from: f64, to: f64, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        (from * (1.0 - t)) + (to * t)
    }

    fn get_viewport_z_index(
        offset_from_center: f64,
        visual: &Ref<Visual>,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
    ) -> i32 {
        if from.is_some_and(|from| from == visual) {
            return 5;
        }

        if to.is_some_and(|to| to == visual) {
            return 4;
        }

        let distance = offset_from_center.abs();
        if distance < 0.5 {
            return 4;
        }
        if distance < 1.5 {
            return 3;
        }
        2
    }
}

impl IPageTransition for CardStackPageTransition {
    fn start(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        cancellation_token: CancellationToken,
    ) -> DispatcherTask<()> {
        let orientation = self.base.orientation();
        let duration = self.base.duration();
        let fill_mode = self.base.fill_mode();
        let slide_out_easing = self.base.slide_out_easing();
        let slide_in_easing = self.base.slide_in_easing();
        let max_swipe_angle = self.max_swipe_angle();
        let back_card_scale = self.back_card_scale();
        let back_card_offset = self.back_card_offset();
        let from = from.cloned();
        let to = to.cloned();

        start_async(async move {
            if cancellation_token.is_cancellation_requested() {
                return;
            }

            let mut tasks = Vec::new();
            let parent = PageSlide::get_visual_parent(from.as_ref(), to.as_ref());
            let is_horizontal = orientation == SlideAxis::Horizontal;
            let distance = if is_horizontal { parent.bounds().width } else { parent.bounds().height };
            let translate_property =
                if is_horizontal { TranslateTransform::x_property() } else { TranslateTransform::y_property() };
            let rotation_target =
                if is_horizontal { if forward { -max_swipe_angle } else { max_swipe_angle } } else { 0.0 };
            let start_scale = 1.0 - back_card_scale;

            if let Some(from) = &from {
                let (rotate, translate) = Self::ensure_top_transforms(from);
                rotate.set_angle(0.0);
                translate.set_x(0.0);
                translate.set_y(0.0);
                from.set_opacity(1.0);
                from.set_z_index(1);

                let animation = Animation::new();
                animation.set_easing(slide_out_easing);
                animation.set_duration(duration);
                animation.set_fill_mode(fill_mode);
                animation.children().add(KeyFrame::with_cue(
                    Cue::new(0.0),
                    [
                        Setter::new(translate_property, 0.0) as _,
                        Setter::new(RotateTransform::angle_property(), 0.0) as _,
                    ],
                ));
                animation.children().add(KeyFrame::with_cue(
                    Cue::new(1.0),
                    [
                        Setter::new(translate_property, if forward { -distance } else { distance }) as _,
                        Setter::new(RotateTransform::angle_property(), rotation_target) as _,
                    ],
                ));
                tasks.push(animation.run_async(from, cancellation_token.clone()));
            }

            if let Some(to) = &to {
                let (scale, translate) = Self::ensure_back_transforms(to);
                scale.set_scale_x(start_scale);
                scale.set_scale_y(start_scale);
                translate.set_x(0.0);
                translate.set_y(back_card_offset);
                to.set_is_visible(true);
                to.set_opacity(1.0);
                to.set_z_index(0);

                let animation = Animation::new();
                animation.set_easing(slide_in_easing);
                animation.set_duration(duration);
                animation.set_fill_mode(fill_mode);
                animation.children().add(KeyFrame::with_cue(
                    Cue::new(0.0),
                    [
                        Setter::new(ScaleTransform::scale_x_property(), start_scale) as _,
                        Setter::new(ScaleTransform::scale_y_property(), start_scale) as _,
                        Setter::new(TranslateTransform::y_property(), back_card_offset) as _,
                    ],
                ));
                animation.children().add(KeyFrame::with_cue(
                    Cue::new(1.0),
                    [
                        Setter::new(ScaleTransform::scale_x_property(), 1.0) as _,
                        Setter::new(ScaleTransform::scale_y_property(), 1.0) as _,
                        Setter::new(TranslateTransform::y_property(), 0.0) as _,
                    ],
                ));

                tasks.push(animation.run_async(to, cancellation_token.clone()));
            }

            when_all(tasks).await;

            if let Some(from) = &from {
                if !cancellation_token.is_cancellation_requested() {
                    from.set_is_visible(false);
                }
            }

            if let Some(to) = &to {
                if !cancellation_token.is_cancellation_requested() {
                    let (scale, translate) = Self::ensure_back_transforms(to);
                    scale.set_scale_x(1.0);
                    scale.set_scale_y(1.0);
                    translate.set_x(0.0);
                    translate.set_y(0.0);
                }
            }
        })
    }

    fn as_progress_page_transition(&self) -> Option<&dyn IProgressPageTransition> {
        Some(self)
    }

    fn as_page_slide(&self) -> Option<&PageSlide> {
        Some(&self.base)
    }
}

impl IProgressPageTransition for CardStackPageTransition {
    fn update(
        &self,
        progress: f64,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        page_length: f64,
        visible_items: &[PageTransitionItem],
    ) {
        if !visible_items.is_empty() {
            self.update_visible_items(progress, from, to, forward, page_length, visible_items);
            return;
        }

        if from.is_none() && to.is_none() {
            return;
        }

        let parent = PageSlide::get_visual_parent(from, to);
        let size = parent.bounds().size();
        let is_horizontal = self.base.orientation() == SlideAxis::Horizontal;
        let distance = if page_length > 0.0 {
            page_length
        } else if is_horizontal {
            size.width
        } else {
            size.height
        };
        let rotation_target =
            if is_horizontal { if forward { -self.max_swipe_angle() } else { self.max_swipe_angle() } } else { 0.0 };
        let start_scale = 1.0 - self.back_card_scale();

        if let Some(from) = from {
            let (rotate, translate) = Self::ensure_top_transforms(from);
            let offset = if forward { -distance * progress } else { distance * progress };
            if is_horizontal {
                translate.set_x(offset);
                translate.set_y(0.0);
            } else {
                translate.set_x(0.0);
                translate.set_y(offset);
            }

            rotate.set_angle(rotation_target * progress);
            from.set_is_visible(true);
            from.set_opacity(1.0);
            from.set_z_index(1);
        }

        if let Some(to) = to {
            let (scale, translate) = Self::ensure_back_transforms(to);
            let current_scale = start_scale + (1.0 - start_scale) * progress;
            let current_offset = self.back_card_offset() * (1.0 - progress);

            scale.set_scale_x(current_scale);
            scale.set_scale_y(current_scale);
            if is_horizontal {
                translate.set_x(0.0);
                translate.set_y(current_offset);
            } else {
                translate.set_x(current_offset);
                translate.set_y(0.0);
            }

            to.set_is_visible(true);
            to.set_opacity(1.0);
            to.set_z_index(0);
        }
    }

    fn reset(&self, visual: &Ref<Visual>) {
        visual.set_render_transform(None);
        visual.set_render_transform_origin(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative));
        visual.set_opacity(1.0);
        visual.set_z_index(0);
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn defaults_and_the_constructor_with_a_duration() {
        let transition = CardStackPageTransition::new();
        assert_eq!(15.0, transition.max_swipe_angle());
        assert_eq!(0.05, transition.back_card_scale());
        assert_eq!(0.0, transition.back_card_offset());
        assert_eq!(SlideAxis::Horizontal, transition.page_slide().orientation());

        let transition = CardStackPageTransition::with_duration(TimeSpan::from_seconds(0.5), SlideAxis::Vertical);
        assert_eq!(TimeSpan::from_seconds(0.5), transition.page_slide().duration());
        assert_eq!(SlideAxis::Vertical, transition.page_slide().orientation());
    }

    #[test]
    fn the_depth_and_the_resting_angle_of_the_viewport() {
        assert_eq!(0.5, CardStackPageTransition::get_viewport_depth(-0.5));
        assert_eq!(1.4, CardStackPageTransition::get_viewport_depth(1.5));
        assert_eq!(1.8, CardStackPageTransition::get_viewport_depth(3.0));
        assert_eq!(0.0, CardStackPageTransition::get_viewport_resting_angle(0.0));
        assert_eq!(-4.0, CardStackPageTransition::get_viewport_resting_angle(-1.0));
        assert_eq!(5.5, CardStackPageTransition::get_viewport_resting_angle(1.5));
        assert_eq!(7.0, CardStackPageTransition::get_viewport_resting_angle(4.0));
    }

    #[test]
    fn the_stack_offset_follows_the_page_length_unless_set() {
        let transition = CardStackPageTransition::new();
        assert_eq!(10.0, transition.get_viewport_stack_offset(100.0));
        assert_eq!(18.0, transition.get_viewport_stack_offset(1000.0));
        transition.set_back_card_offset(24.0);
        assert_eq!(24.0, transition.get_viewport_stack_offset(1000.0));
    }
}
