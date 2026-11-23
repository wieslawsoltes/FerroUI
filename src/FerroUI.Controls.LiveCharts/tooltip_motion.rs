//! The show and hide animation of the default tooltip (`Show`, `Hide` and `Initialize` of the
//! pinned `SKDefaultTooltip`): its position, opacity and scale ease out over 150 ms. The tooltip
//! is compiled once with its corner at the origin and each frame only moves it, so a frame of the
//! animation makes no scene.

use crate::renderer::NativeScene;
use crate::TooltipPosition;
use ferroui_base::media::Color;
use ferroui_base::{Matrix, Point, Size};
use ferroui_livecharts::easing_functions::EasingFunctions;
use std::rc::Rc;

/// `AnimationsSpeed` of `SKDefaultTooltip`.
pub(crate) const TOOLTIP_ANIMATION_MS: f64 = 150.0;
/// The scale `Hide` sets.
const HIDDEN_SCALE: f64 = 0.85;

/// Where a tooltip is drawn in one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TooltipPose {
    pub(crate) origin: Point,
    pub(crate) opacity: f64,
    pub(crate) scale: f64,
}

impl TooltipPose {
    pub(crate) fn shown(origin: Point) -> Self {
        Self {
            origin,
            opacity: 1.0,
            scale: 1.0,
        }
    }

    pub(crate) fn hidden(origin: Point) -> Self {
        Self {
            origin,
            opacity: 0.0,
            scale: HIDDEN_SCALE,
        }
    }

    fn between(from: Self, to: Self, t: f64) -> Self {
        let mix = |a: f64, b: f64| a + (b - a) * t;
        Self {
            origin: Point::new(
                mix(from.origin.x, to.origin.x),
                mix(from.origin.y, to.origin.y),
            ),
            opacity: mix(from.opacity, to.opacity),
            scale: mix(from.scale, to.scale),
        }
    }

    /// The transform of a tooltip of `size` compiled at the origin: scaled about its centre
    /// (`TransformOrigin` of the source) and moved to its place.
    pub(crate) fn matrix(self, size: Size) -> Matrix {
        let (cx, cy) = (size.width * 0.5, size.height * 0.5);
        Matrix::create_translation(-cx, -cy)
            * Matrix::create_scale(self.scale, self.scale)
            * Matrix::create_translation(self.origin.x + cx, self.origin.y + cy)
    }
}

/// What the compiled tooltip depends on. While it stays the same, a frame reuses the tooltip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TooltipKey {
    pub(crate) hover: u64,
    pub(crate) measurement: Option<u64>,
    pub(crate) control: Size,
    pub(crate) text: Color,
    pub(crate) position: TooltipPosition,
}

/// The compiled tooltip and its motion.
pub(crate) struct TooltipMotion {
    pub(crate) key: TooltipKey,
    pub(crate) frame: Rc<NativeScene>,
    pub(crate) size: Size,
    from: TooltipPose,
    pub(crate) to: TooltipPose,
    start_ms: f64,
    duration_ms: f64,
}

impl TooltipMotion {
    /// A tooltip that appears at its place.
    pub(crate) fn appear(
        key: TooltipKey,
        frame: Rc<NativeScene>,
        size: Size,
        origin: Point,
        now_ms: f64,
        duration_ms: f64,
    ) -> Self {
        Self {
            key,
            frame,
            size,
            from: TooltipPose::hidden(origin),
            to: TooltipPose::shown(origin),
            start_ms: now_ms,
            duration_ms,
        }
    }

    /// The tooltip of another hover, moving on from where it is drawn now.
    pub(crate) fn show(
        &mut self,
        key: TooltipKey,
        frame: Rc<NativeScene>,
        size: Size,
        origin: Point,
        now_ms: f64,
        duration_ms: f64,
    ) {
        self.key = key;
        self.frame = frame;
        self.size = size;
        self.retarget(TooltipPose::shown(origin), now_ms, duration_ms);
    }

    /// Eases to `to` from the pose at `now_ms`. The same target keeps the motion it is in.
    pub(crate) fn retarget(&mut self, to: TooltipPose, now_ms: f64, duration_ms: f64) {
        if to == self.to {
            return;
        }
        self.from = self.pose(now_ms);
        self.to = to;
        self.start_ms = now_ms;
        self.duration_ms = duration_ms;
    }

    pub(crate) fn pose(&self, now_ms: f64) -> TooltipPose {
        if self.is_completed(now_ms) {
            return self.to;
        }
        let t = ((now_ms - self.start_ms) / self.duration_ms).clamp(0.0, 1.0);
        TooltipPose::between(self.from, self.to, EasingFunctions::ease_out(t))
    }

    pub(crate) fn is_completed(&self, now_ms: f64) -> bool {
        self.duration_ms <= 0.0 || now_ms - self.start_ms >= self.duration_ms
    }

    /// Whether the motion ends with the tooltip gone.
    pub(crate) fn is_hiding(&self) -> bool {
        self.to.opacity <= 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::Rect;
    use ferroui_livecharts::Scene;

    fn key(hover: u64) -> TooltipKey {
        TooltipKey {
            hover,
            measurement: None,
            control: Size::new(100.0, 100.0),
            text: Color::from_rgb(0, 0, 0),
            position: TooltipPosition::Auto,
        }
    }

    fn frame() -> Rc<NativeScene> {
        let size = Size::new(100.0, 100.0);
        Rc::new(NativeScene::compile(&Scene::new(
            size,
            Rect::from_size(size),
        )))
    }

    #[test]
    fn a_tooltip_appears_at_its_place_and_ends_shown() {
        let at = Point::new(10.0, 20.0);
        let motion =
            TooltipMotion::appear(key(1), frame(), Size::new(40.0, 20.0), at, 100.0, 150.0);
        assert_eq!(motion.pose(100.0), TooltipPose::hidden(at));
        let middle = motion.pose(175.0);
        assert_eq!(middle.origin, at);
        assert!(
            middle.opacity > 0.5 && middle.opacity < 1.0,
            "ease out is past half way"
        );
        assert!(middle.scale > 0.925 && middle.scale < 1.0);
        assert!(!motion.is_completed(249.0));
        assert!(motion.is_completed(250.0));
        assert_eq!(motion.pose(250.0), TooltipPose::shown(at));
    }

    #[test]
    fn a_new_target_moves_on_from_the_pose_drawn_now() {
        let (a, b) = (Point::new(0.0, 0.0), Point::new(100.0, 0.0));
        let mut motion =
            TooltipMotion::appear(key(1), frame(), Size::new(40.0, 20.0), a, 0.0, 150.0);
        let now = motion.pose(75.0);
        motion.show(key(2), frame(), Size::new(40.0, 20.0), b, 75.0, 150.0);
        assert_eq!(motion.pose(75.0), now);
        assert_eq!(motion.pose(225.0), TooltipPose::shown(b));
    }

    #[test]
    fn the_same_target_keeps_the_motion_it_is_in() {
        let at = Point::new(5.0, 5.0);
        let mut motion =
            TooltipMotion::appear(key(1), frame(), Size::new(40.0, 20.0), at, 0.0, 150.0);
        motion.show(key(2), frame(), Size::new(40.0, 20.0), at, 100.0, 150.0);
        assert!(motion.is_completed(150.0));
    }

    #[test]
    fn no_duration_is_at_its_target_at_once() {
        let at = Point::new(5.0, 5.0);
        let mut motion =
            TooltipMotion::appear(key(1), frame(), Size::new(40.0, 20.0), at, 0.0, 0.0);
        assert_eq!(motion.pose(0.0), TooltipPose::shown(at));
        motion.retarget(TooltipPose::hidden(at), 1.0, 0.0);
        assert!(motion.is_hiding() && motion.is_completed(1.0));
        assert_eq!(motion.pose(1.0).opacity, 0.0);
    }

    #[test]
    fn the_transform_scales_about_the_centre_and_moves_to_the_origin() {
        let size = Size::new(40.0, 20.0);
        let pose = TooltipPose {
            origin: Point::new(100.0, 50.0),
            opacity: 1.0,
            scale: 0.5,
        };
        let matrix = pose.matrix(size);
        let close = |a: Point, b: Point| (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9;
        assert!(close(
            matrix.transform(Point::new(20.0, 10.0)),
            Point::new(120.0, 60.0)
        ));
        assert!(close(
            matrix.transform(Point::new(0.0, 0.0)),
            Point::new(110.0, 55.0)
        ));
        assert!(close(
            TooltipPose::shown(pose.origin)
                .matrix(size)
                .transform(Point::new(0.0, 0.0)),
            pose.origin
        ));
    }
}
