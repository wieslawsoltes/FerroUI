//! Port of `Pages/Transitions/WaveRevealPageTransition.cs`.

use ferroui_base::animation::easings::{CubicEaseOut, Easing};
use ferroui_base::animation::{IPageTransition, IProgressPageTransition, PageSlide, PageTransitionItem, SlideAxis, TimeSpan};
use ferroui_base::media::{Geometry, RectangleGeometry, StreamGeometry};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::threading::{CancellationToken, DispatcherPriority, DispatcherTask, DispatcherTimer};
use ferroui_base::{Point, Rect, Ref, Size, Visual};
use mini_mvvm::start_async;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

#[derive(Default)]
struct DelayState {
    elapsed: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

/// `Task.Delay(duration)`: completes when a dispatcher timer of `duration` has ticked.
struct Delay(Rc<DelayState>);

impl Future for Delay {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0.elapsed.get() {
            Poll::Ready(())
        } else {
            *self.0.waker.borrow_mut() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

fn delay(duration: Duration) -> Delay {
    let state = Rc::new(DelayState::default());
    let timer_state = state.clone();
    DispatcherTimer::run_once(
        move || {
            timer_state.elapsed.set(true);
            let waker = timer_state.waker.borrow_mut().take();
            if let Some(waker) = waker {
                waker.wake();
            }
        },
        duration,
        DispatcherPriority::DEFAULT,
    );
    Delay(state)
}

/// Transitions between two pages using a wave clip that reveals the next page.
///
/// The class of the original derives from `PageSlide`; here the slide is the field `base`,
/// which [`page_slide`](Self::page_slide) gives access to.
pub struct WaveRevealPageTransition {
    base: PageSlide,
    max_bulge: Cell<f64>,
    bulge_factor: Cell<f64>,
    cross_bulge_factor: Cell<f64>,
    wave_center_offset: Cell<f64>,
    center_sensitivity: Cell<f64>,
    bulge_exponent: Cell<f64>,
    wave_easing: RefCell<Easing>,
}

impl Default for WaveRevealPageTransition {
    fn default() -> Self {
        Self::new()
    }
}

/// The values of a transition an update reads: what the running animation of
/// [`IPageTransition::start`] keeps of the transition.
#[derive(Clone)]
struct Wave {
    orientation: SlideAxis,
    max_bulge: f64,
    bulge_factor: f64,
    cross_bulge_factor: f64,
    wave_center_offset: f64,
    center_sensitivity: f64,
    bulge_exponent: f64,
    wave_easing: Easing,
}

impl Wave {
    fn create_wave_path(&self, progress: f64, size: Size, center_offset: f64, forward: bool) -> Ref<Geometry> {
        LiquidSwipeClipper::create_wave_path(
            progress,
            size,
            center_offset,
            forward,
            self.orientation == SlideAxis::Horizontal,
            self.max_bulge,
            self.bulge_factor,
            self.cross_bulge_factor,
            self.bulge_exponent,
        )
    }

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
            self.update_visible_items(from, to, forward, page_length, visible_items);
            return;
        }

        if from.is_none() && to.is_none() {
            return;
        }

        let parent = PageSlide::get_visual_parent(from, to);
        let size = parent.bounds().size();
        let center_offset = self.wave_center_offset * self.center_sensitivity;

        if let Some(to) = to {
            to.set_is_visible(progress > 0.0);
            to.set_z_index(1);
            to.set_opacity(1.0);

            if progress >= 1.0 {
                to.set_clip(None);
            } else {
                let wave_progress = self.wave_easing.ease(progress);
                to.set_clip(self.create_wave_path(wave_progress, size, center_offset, forward));
            }
        }

        if let Some(from) = from {
            from.set_is_visible(true);
            from.set_z_index(0);
            from.set_opacity(1.0);
        }
    }

    fn update_visible_items(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        page_length: f64,
        visible_items: &[PageTransitionItem],
    ) {
        if from.is_none() && to.is_none() {
            return;
        }

        let parent = PageSlide::get_visual_parent(from, to);
        let size = parent.bounds().size();
        let center_offset = self.wave_center_offset * self.center_sensitivity;
        let is_horizontal = self.orientation == SlideAxis::Horizontal;
        let resolved_page_length = if page_length > 0.0 {
            page_length
        } else if is_horizontal {
            size.width
        } else {
            size.height
        };

        for item in visible_items {
            let visual = &item.visual;
            let is_to = to.is_some_and(|to| to == visual);
            visual.set_is_visible(true);
            visual.set_opacity(1.0);
            visual.set_clip(None);
            visual.set_z_index(if is_to { 1 } else { 0 });

            if !is_to {
                continue;
            }

            let visible_fraction = WaveRevealPageTransition::get_visible_fraction(
                item.viewport_center_offset,
                size,
                resolved_page_length,
                is_horizontal,
            );
            if visible_fraction >= 1.0 {
                continue;
            }

            visual.set_clip(self.create_wave_path(visible_fraction, size, center_offset, forward));
        }
    }
}

impl WaveRevealPageTransition {
    /// `new WaveRevealPageTransition()`.
    pub fn new() -> Self {
        Self::from_slide(PageSlide::new())
    }

    /// `new WaveRevealPageTransition(duration, orientation)`: `duration` is the duration of
    /// the animation and `orientation` the axis on which the animation occurs.
    pub fn with_duration(duration: TimeSpan, orientation: SlideAxis) -> Self {
        Self::from_slide(PageSlide::with_duration(duration, orientation))
    }

    fn from_slide(base: PageSlide) -> Self {
        Self {
            base,
            max_bulge: Cell::new(120.0),
            bulge_factor: Cell::new(0.35),
            cross_bulge_factor: Cell::new(0.3),
            wave_center_offset: Cell::new(0.0),
            center_sensitivity: Cell::new(1.0),
            bulge_exponent: Cell::new(1.0),
            wave_easing: RefCell::new(CubicEaseOut.into()),
        }
    }

    /// The slide this transition derives from: its duration, orientation, easings and fill
    /// mode.
    pub fn page_slide(&self) -> &PageSlide {
        &self.base
    }

    /// The maximum wave bulge (pixels) along the movement axis.
    pub fn max_bulge(&self) -> f64 {
        self.max_bulge.get()
    }

    pub fn set_max_bulge(&self, value: f64) {
        self.max_bulge.set(value)
    }

    /// The bulge factor along the movement axis (0-1).
    pub fn bulge_factor(&self) -> f64 {
        self.bulge_factor.get()
    }

    pub fn set_bulge_factor(&self, value: f64) {
        self.bulge_factor.set(value)
    }

    /// The bulge factor along the cross axis (0-1).
    pub fn cross_bulge_factor(&self) -> f64 {
        self.cross_bulge_factor.get()
    }

    pub fn set_cross_bulge_factor(&self, value: f64) {
        self.cross_bulge_factor.set(value)
    }

    /// A cross-axis offset (pixels) to shift the wave center.
    pub fn wave_center_offset(&self) -> f64 {
        self.wave_center_offset.get()
    }

    pub fn set_wave_center_offset(&self, value: f64) {
        self.wave_center_offset.set(value)
    }

    /// How strongly the wave center follows the provided offset.
    pub fn center_sensitivity(&self) -> f64 {
        self.center_sensitivity.get()
    }

    pub fn set_center_sensitivity(&self, value: f64) {
        self.center_sensitivity.set(value)
    }

    /// The bulge exponent used to shape the wave (1.0 = linear). Higher values tighten the
    /// bulge near the center.
    pub fn bulge_exponent(&self) -> f64 {
        self.bulge_exponent.get()
    }

    pub fn set_bulge_exponent(&self, value: f64) {
        self.bulge_exponent.set(value)
    }

    /// The easing applied to the wave progress.
    pub fn wave_easing(&self) -> Easing {
        self.wave_easing.borrow().clone()
    }

    pub fn set_wave_easing(&self, value: impl Into<Easing>) {
        *self.wave_easing.borrow_mut() = value.into();
    }

    fn wave(&self) -> Wave {
        Wave {
            orientation: self.base.orientation(),
            max_bulge: self.max_bulge(),
            bulge_factor: self.bulge_factor(),
            cross_bulge_factor: self.cross_bulge_factor(),
            wave_center_offset: self.wave_center_offset(),
            center_sensitivity: self.center_sensitivity(),
            bulge_exponent: self.bulge_exponent(),
            wave_easing: self.wave_easing(),
        }
    }

    fn get_visible_fraction(offset_from_center: f64, viewport_size: Size, page_length: f64, is_horizontal: bool) -> f64 {
        if page_length <= 0.0 {
            return 1.0;
        }

        let viewport_length = if is_horizontal { viewport_size.width } else { viewport_size.height };
        if viewport_length <= 0.0 {
            return 0.0;
        }

        let viewport_units = viewport_length / page_length;
        let edge_peek = 0.0_f64.max((viewport_units - 1.0) / 2.0);
        (1.0 + edge_peek - offset_from_center.abs()).clamp(0.0, 1.0)
    }

    /// `AnimateProgress`: drives the updates of `wave` from `from` to `to` over the duration
    /// of the transition.
    #[allow(clippy::too_many_arguments)]
    async fn animate_progress(
        wave: Wave,
        duration: TimeSpan,
        slide_in_easing: Easing,
        from: f64,
        to: f64,
        from_visual: Option<Ref<Visual>>,
        to_visual: Option<Ref<Visual>>,
        forward: bool,
        cancellation_token: CancellationToken,
    ) {
        let parent = PageSlide::get_visual_parent(from_visual.as_ref(), to_visual.as_ref());
        let page_length =
            if wave.orientation == SlideAxis::Horizontal { parent.bounds().width } else { parent.bounds().height };
        let duration_ms = (duration.total_milliseconds() * (to - from).abs()).max(50.0);
        let start = Instant::now();

        while !cancellation_token.is_cancellation_requested() {
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
            let t = (elapsed_ms / duration_ms).clamp(0.0, 1.0);
            let eased = slide_in_easing.ease(t);
            let progress = from + (to - from) * eased;

            wave.update(progress, from_visual.as_ref(), to_visual.as_ref(), forward, page_length, &[]);

            if t >= 1.0 {
                break;
            }

            delay(Duration::from_millis(16)).await;
        }

        if !cancellation_token.is_cancellation_requested() {
            wave.update(to, from_visual.as_ref(), to_visual.as_ref(), forward, page_length, &[]);
        }
    }
}

impl IPageTransition for WaveRevealPageTransition {
    fn start(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        cancellation_token: CancellationToken,
    ) -> DispatcherTask<()> {
        let wave = self.wave();
        let duration = self.base.duration();
        let slide_in_easing = self.base.slide_in_easing();
        let from = from.cloned();
        let to = to.cloned();

        start_async(async move {
            if cancellation_token.is_cancellation_requested() {
                return;
            }

            if let Some(to) = &to {
                to.set_is_visible(true);
                to.set_z_index(1);
            }

            if let Some(from) = &from {
                from.set_z_index(0);
            }

            Self::animate_progress(
                wave,
                duration,
                slide_in_easing,
                0.0,
                1.0,
                from.clone(),
                to.clone(),
                forward,
                cancellation_token.clone(),
            )
            .await;

            if let Some(to) = &to {
                if !cancellation_token.is_cancellation_requested() {
                    to.set_clip(None);
                }
            }

            if let Some(from) = &from {
                if !cancellation_token.is_cancellation_requested() {
                    from.set_is_visible(false);
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

impl IProgressPageTransition for WaveRevealPageTransition {
    fn update(
        &self,
        progress: f64,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        page_length: f64,
        visible_items: &[PageTransitionItem],
    ) {
        self.wave().update(progress, from, to, forward, page_length, visible_items);
    }

    fn reset(&self, visual: &Ref<Visual>) {
        visual.set_clip(None);
        visual.set_z_index(0);
        visual.set_opacity(1.0);
    }
}

/// `WaveRevealPageTransition.LiquidSwipeClipper`.
struct LiquidSwipeClipper;

impl LiquidSwipeClipper {
    #[allow(clippy::too_many_arguments)]
    fn create_wave_path(
        progress: f64,
        size: Size,
        wave_center_offset: f64,
        forward: bool,
        is_horizontal: bool,
        max_bulge: f64,
        bulge_factor: f64,
        cross_bulge_factor: f64,
        bulge_exponent: f64,
    ) -> Ref<Geometry> {
        let width = size.width;
        let height = size.height;

        if progress <= 0.0 {
            return RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 0.0, 0.0)).upcast();
        }

        if progress >= 1.0 {
            return RectangleGeometry::with_rect(Rect::new(0.0, 0.0, width, height)).upcast();
        }

        if width <= 0.0 || height <= 0.0 {
            return RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 0.0, 0.0)).upcast();
        }

        let main_length = if is_horizontal { width } else { height };
        let cross_length = if is_horizontal { height } else { width };

        let wave_phase = (progress * std::f64::consts::PI).sin();
        let bulge_progress = if bulge_exponent == 1.0 { wave_phase } else { wave_phase.powf(bulge_exponent) };
        let revealed_length = main_length * progress;
        let bulge_main = (main_length * bulge_factor).min(max_bulge) * bulge_progress;
        let bulge_main = bulge_main.min(revealed_length * 0.45);
        let bulge_cross = cross_length * cross_bulge_factor;

        // `Math.Clamp(value, min, max)`: panics when the bounds cross (the argument exception
        // of the original).
        let wave_center = (cross_length / 2.0 + wave_center_offset).clamp(bulge_cross, cross_length - bulge_cross);

        let geometry = StreamGeometry::new();
        {
            let mut context = geometry.open();

            if is_horizontal {
                if forward {
                    let wave_x = width * (1.0 - progress);
                    context.begin_figure(Point::new(width, 0.0), true);
                    context.line_to(Point::new(wave_x, 0.0), true);
                    context.cubic_bezier_to(
                        Point::new(wave_x, wave_center - bulge_cross),
                        Point::new(wave_x - bulge_main, wave_center - bulge_cross * 0.5),
                        Point::new(wave_x - bulge_main, wave_center),
                        true,
                    );
                    context.cubic_bezier_to(
                        Point::new(wave_x - bulge_main, wave_center + bulge_cross * 0.5),
                        Point::new(wave_x, wave_center + bulge_cross),
                        Point::new(wave_x, height),
                        true,
                    );
                    context.line_to(Point::new(width, height), true);
                    context.end_figure(true);
                } else {
                    let wave_x = width * progress;
                    context.begin_figure(Point::new(0.0, 0.0), true);
                    context.line_to(Point::new(wave_x, 0.0), true);
                    context.cubic_bezier_to(
                        Point::new(wave_x, wave_center - bulge_cross),
                        Point::new(wave_x + bulge_main, wave_center - bulge_cross * 0.5),
                        Point::new(wave_x + bulge_main, wave_center),
                        true,
                    );
                    context.cubic_bezier_to(
                        Point::new(wave_x + bulge_main, wave_center + bulge_cross * 0.5),
                        Point::new(wave_x, wave_center + bulge_cross),
                        Point::new(wave_x, height),
                        true,
                    );
                    context.line_to(Point::new(0.0, height), true);
                    context.end_figure(true);
                }
            } else if forward {
                let wave_y = height * (1.0 - progress);
                context.begin_figure(Point::new(0.0, height), true);
                context.line_to(Point::new(0.0, wave_y), true);
                context.cubic_bezier_to(
                    Point::new(wave_center - bulge_cross, wave_y),
                    Point::new(wave_center - bulge_cross * 0.5, wave_y - bulge_main),
                    Point::new(wave_center, wave_y - bulge_main),
                    true,
                );
                context.cubic_bezier_to(
                    Point::new(wave_center + bulge_cross * 0.5, wave_y - bulge_main),
                    Point::new(wave_center + bulge_cross, wave_y),
                    Point::new(width, wave_y),
                    true,
                );
                context.line_to(Point::new(width, height), true);
                context.end_figure(true);
            } else {
                let wave_y = height * progress;
                context.begin_figure(Point::new(0.0, 0.0), true);
                context.line_to(Point::new(0.0, wave_y), true);
                context.cubic_bezier_to(
                    Point::new(wave_center - bulge_cross, wave_y),
                    Point::new(wave_center - bulge_cross * 0.5, wave_y + bulge_main),
                    Point::new(wave_center, wave_y + bulge_main),
                    true,
                );
                context.cubic_bezier_to(
                    Point::new(wave_center + bulge_cross * 0.5, wave_y + bulge_main),
                    Point::new(wave_center + bulge_cross, wave_y),
                    Point::new(width, wave_y),
                    true,
                );
                context.line_to(Point::new(width, 0.0), true);
                context.end_figure(true);
            }

            context.dispose();
        }

        geometry.upcast()
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn defaults_and_the_constructor_with_a_duration() {
        let transition = WaveRevealPageTransition::new();
        assert_eq!(120.0, transition.max_bulge());
        assert_eq!(0.35, transition.bulge_factor());
        assert_eq!(0.3, transition.cross_bulge_factor());
        assert_eq!(0.0, transition.wave_center_offset());
        assert_eq!(1.0, transition.center_sensitivity());
        assert_eq!(1.0, transition.bulge_exponent());

        let transition = WaveRevealPageTransition::with_duration(TimeSpan::from_seconds(0.8), SlideAxis::Vertical);
        assert_eq!(TimeSpan::from_seconds(0.8), transition.page_slide().duration());
        assert_eq!(SlideAxis::Vertical, transition.page_slide().orientation());
    }

    #[test]
    fn the_visible_fraction_of_a_page() {
        let size = Size::new(300.0, 200.0);
        assert_eq!(1.0, WaveRevealPageTransition::get_visible_fraction(0.0, size, 0.0, true));
        assert_eq!(0.0, WaveRevealPageTransition::get_visible_fraction(0.0, Size::new(0.0, 0.0), 100.0, true));
        assert_eq!(1.0, WaveRevealPageTransition::get_visible_fraction(0.0, size, 300.0, true));
        assert_eq!(0.5, WaveRevealPageTransition::get_visible_fraction(0.5, size, 300.0, true));
        assert_eq!(1.0, WaveRevealPageTransition::get_visible_fraction(1.0, size, 100.0, true));
        assert_eq!(0.0, WaveRevealPageTransition::get_visible_fraction(-2.5, size, 100.0, true));
    }
}
