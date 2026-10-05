use crate::animation::easings::Easing;
use crate::animation::{
    Animation, Cue, IAnimationSetter, IPageTransition, IProgressPageTransition, KeyFrame,
    PageSlide, PageTransitionItem, SlideAxis, TimeSpan,
};
use crate::media::{ITransform, Rotate3DTransform};
use crate::styling::Setter;
use crate::animation::i_page_transition::{start_async, when_all};
use crate::threading::{CancellationToken, DispatcherTask};
use crate::{Ref, Visual};
use std::cell::Cell;
use std::ops::Deref;
use std::rc::Rc;

const SIDE_PEEK_ANGLE: f64 = 24.0;
const FAR_PEEK_ANGLE: f64 = 38.0;

/// Transitions between two pages by rotating them around a shared axis in
/// 3D. It is a [`PageSlide`] whose members it derefs to.
pub struct Rotate3DTransition {
    base: PageSlide,
    depth: Cell<Option<f64>>,
}

impl Default for Rotate3DTransition {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for Rotate3DTransition {
    type Target = PageSlide;

    fn deref(&self) -> &PageSlide {
        &self.base
    }
}

impl Rotate3DTransition {
    /// Creates a horizontal rotation with a zero duration.
    pub fn new() -> Self {
        Self { base: PageSlide::new(), depth: Cell::new(None) }
    }

    /// Creates a rotation with the given duration and axis. `depth`
    /// defines the depth of the 3D effect; when `None` it is calculated
    /// automatically.
    pub fn with_duration(duration: TimeSpan, orientation: SlideAxis, depth: Option<f64>) -> Self {
        Self { base: PageSlide::with_duration(duration, orientation), depth: Cell::new(depth) }
    }

    /// Defines the depth of the 3D effect. If `None`, the depth is
    /// calculated automatically.
    pub fn depth(&self) -> Option<f64> {
        self.depth.get()
    }

    pub fn set_depth(&self, value: Option<f64>) {
        self.depth.set(value)
    }

    fn rotate_transform(visual: &Ref<Visual>) -> Ref<Rotate3DTransform> {
        let existing = visual
            .render_transform()
            .and_then(|transform| transform.as_object().and_then(|o| o.to_ref().cast::<Rotate3DTransform>()));
        match existing {
            Some(transform) => transform,
            None => {
                let transform = Rotate3DTransform::new();
                let handle: Rc<dyn ITransform> = (&transform).into();
                visual.set_render_transform(Some(handle));
                transform
            }
        }
    }

    fn update_visible_items(
        &self,
        progress: f64,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        page_length: f64,
        visible_items: &[PageTransitionItem],
    ) {
        let anchor = from.or(to).unwrap_or(&visible_items[0].visual);
        let Some(parent) = anchor.visual_parent() else { return };

        let horizontal = self.orientation() == SlideAxis::Horizontal;
        let center = if page_length > 0.0 {
            page_length
        } else if horizontal {
            parent.bounds().width
        } else {
            parent.bounds().height
        };
        let depth = self.depth.get().unwrap_or(center);
        let angle_strength = (progress.clamp(0.0, 1.0) * std::f64::consts::PI).sin();

        for item in visible_items {
            let visual = &item.visual;
            visual.set_is_visible(true);
            visual.set_z_index(Self::get_z_index(item.viewport_center_offset));

            let transform = Self::rotate_transform(visual);

            transform.set_depth(depth);
            transform.set_center_z(-center / 2.0);

            let angle = Self::get_angle_for_offset(item.viewport_center_offset) * angle_strength;
            if horizontal {
                transform.set_angle_y(angle);
                transform.set_angle_x(0.0);
            } else {
                transform.set_angle_x(angle);
                transform.set_angle_y(0.0);
            }
        }
    }

    fn get_angle_for_offset(offset_from_center: f64) -> f64 {
        if offset_from_center == 0.0 || offset_from_center.is_nan() {
            return 0.0;
        }
        let sign = offset_from_center.signum();

        let distance = offset_from_center.abs();
        if distance <= 1.0 {
            return sign * lerp(0.0, SIDE_PEEK_ANGLE, distance);
        }

        if distance <= 2.0 {
            return sign * lerp(SIDE_PEEK_ANGLE, FAR_PEEK_ANGLE, distance - 1.0);
        }

        sign * FAR_PEEK_ANGLE
    }

    fn get_z_index(offset_from_center: f64) -> i32 {
        let distance = offset_from_center.abs();

        if distance < 0.5 {
            return 3;
        }
        if distance < 1.5 {
            return 2;
        }
        1
    }
}

fn lerp(from: f64, to: f64, t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    from + (to - from) * t
}

impl IPageTransition for Rotate3DTransition {
    fn start(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        cancellation_token: CancellationToken,
    ) -> DispatcherTask<()> {
        let orientation = self.orientation();
        let depth = self.depth.get();
        let duration = self.duration();
        let fill_mode = self.fill_mode();
        let slide_in_easing = self.slide_in_easing();
        let slide_out_easing = self.slide_out_easing();
        let from = from.cloned();
        let to = to.cloned();

        start_async(async move {
            if cancellation_token.is_cancellation_requested() {
                return;
            }

            let mut tasks = Vec::with_capacity(if from.is_some() && to.is_some() { 2 } else { 1 });
            let parent = PageSlide::get_visual_parent(from.as_ref(), to.as_ref());
            let (rotate_property, center) = match orientation {
                SlideAxis::Vertical => (Rotate3DTransform::angle_x_property(), parent.bounds().height),
                SlideAxis::Horizontal => (Rotate3DTransform::angle_y_property(), parent.bounds().width),
            };

            let depth_setter: Rc<dyn IAnimationSetter> =
                Setter::new(Rotate3DTransform::depth_property(), depth.unwrap_or(center));
            let center_z_setter: Rc<dyn IAnimationSetter> =
                Setter::new(Rotate3DTransform::center_z_property(), -center / 2.0);

            let create_key_frame = |cue: f64, rotation: f64, z_index: i32, is_visible: bool| {
                let setters: [Rc<dyn IAnimationSetter>; 5] = [
                    Setter::new(rotate_property, rotation),
                    Setter::new(Visual::z_index_property(), z_index),
                    Setter::new(Visual::is_visible_property(), is_visible),
                    center_z_setter.clone(),
                    depth_setter.clone(),
                ];
                KeyFrame::with_cue(Cue::new(cue), setters)
            };

            let create_animation = |easing: Easing, key_frames: [Ref<KeyFrame>; 3]| {
                let animation = Animation::new();
                animation.set_easing(easing);
                animation.set_duration(duration);
                animation.set_fill_mode(fill_mode);
                animation.children().add_range(key_frames);
                animation
            };

            let direction = if forward { -1.0 } else { 1.0 };

            if let Some(from) = &from {
                let animation = create_animation(
                    slide_out_easing,
                    [
                        create_key_frame(0.0, 0.0, 2, true),
                        create_key_frame(0.5, 45.0 * direction, 1, true),
                        create_key_frame(1.0, 90.0 * direction, 1, false),
                    ],
                );

                tasks.push(animation.run_async_with_clock(from, None, cancellation_token.clone()));
            }

            if let Some(to) = &to {
                to.set_is_visible(true);
                let animation = create_animation(
                    slide_in_easing,
                    [
                        create_key_frame(0.0, 90.0 * -direction, 1, true),
                        create_key_frame(0.5, 45.0 * -direction, 1, true),
                        create_key_frame(1.0, 0.0, 2, true),
                    ],
                );

                tasks.push(animation.run_async_with_clock(to, None, cancellation_token.clone()));
            }

            when_all(tasks).await;

            if !cancellation_token.is_cancellation_requested() {
                if let Some(to) = &to {
                    to.set_z_index(2);
                }

                if let Some(from) = &from {
                    from.set_is_visible(false);
                    from.set_z_index(1);
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

impl IProgressPageTransition for Rotate3DTransition {
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
            self.update_visible_items(progress, from, to, page_length, visible_items);
            return;
        }

        if from.is_none() && to.is_none() {
            return;
        }

        let parent = PageSlide::get_visual_parent(from, to);
        let horizontal = self.orientation() == SlideAxis::Horizontal;
        let center = if page_length > 0.0 {
            page_length
        } else if horizontal {
            parent.bounds().width
        } else {
            parent.bounds().height
        };
        let depth = self.depth.get().unwrap_or(center);
        let sign = if forward { 1.0 } else { -1.0 };

        if let Some(from) = from {
            let ft = Self::rotate_transform(from);
            ft.set_depth(depth);
            ft.set_center_z(-center / 2.0);
            from.set_z_index(if progress < 0.5 { 2 } else { 1 });
            if horizontal {
                ft.set_angle_y(-sign * 90.0 * progress);
            } else {
                ft.set_angle_x(-sign * 90.0 * progress);
            }
        }

        if let Some(to) = to {
            to.set_is_visible(true);
            let tt = Self::rotate_transform(to);
            tt.set_depth(depth);
            tt.set_center_z(-center / 2.0);
            to.set_z_index(if progress < 0.5 { 1 } else { 2 });
            if horizontal {
                tt.set_angle_y(sign * 90.0 * (1.0 - progress));
            } else {
                tt.set_angle_x(sign * 90.0 * (1.0 - progress));
            }
        }
    }

    fn reset(&self, visual: &Ref<Visual>) {
        visual.set_render_transform(None);
        visual.set_z_index(0);
    }
}
