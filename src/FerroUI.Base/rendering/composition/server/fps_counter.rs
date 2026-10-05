use super::DiagnosticTextRenderer;
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::{BoxShadows, Brushes, Colors, IBrush};
use crate::platform::IDrawingContextImpl;
use crate::Rect;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// An FPS counter helper that can draw itself on the render thread
///
/// The C# class owns a stopwatch started at construction. Here the caller
/// supplies the time instead: [`new`](Self::new) and
/// [`render_fps`](Self::render_fps) take the current reading of a monotonic
/// clock (any epoch), and the counter works with the time elapsed between
/// the two.
pub struct FpsCounter {
    start: Duration,
    text_renderer: Rc<DiagnosticTextRenderer>,

    frames_this_second: Cell<i32>,
    total_frames: Cell<i32>,
    fps: Cell<i32>,
    last_fps_update: Cell<Duration>,
}

impl FpsCounter {
    /// `now` is the current reading of the monotonic clock that is also
    /// passed to [`render_fps`](Self::render_fps).
    pub fn new(text_renderer: Rc<DiagnosticTextRenderer>, now: Duration) -> Self {
        Self {
            start: now,
            text_renderer,
            frames_this_second: Cell::new(0),
            total_frames: Cell::new(0),
            fps: Cell::new(0),
            last_fps_update: Cell::new(Duration::ZERO),
        }
    }

    pub fn fps_tick(&self) {
        self.frames_this_second.set(self.frames_this_second.get().wrapping_add(1));
    }

    /// Counts a frame and draws the counter at the origin of the current
    /// transform of `context`. Returns the rectangle that was covered.
    pub fn render_fps(
        &self,
        context: &mut dyn IDrawingContextImpl,
        aux: &str,
        has_layer: bool,
        old_rect: Option<Rect>,
        now: Duration,
    ) -> Rect {
        let now = now.saturating_sub(self.start);
        let elapsed = now.saturating_sub(self.last_fps_update.get());

        self.frames_this_second.set(self.frames_this_second.get().wrapping_add(1));
        self.total_frames.set(self.total_frames.get().wrapping_add(1));

        if elapsed.as_secs_f64() > 1.0 {
            self.fps.set((self.frames_this_second.get() as f64 / elapsed.as_secs_f64()) as i32);
            self.frames_this_second.set(0);
            self.last_fps_update.set(now);
        }

        let fps_line = format!("Frame #{:08} FPS: {:03} {}", self.total_frames.get(), self.fps.get(), aux);

        let size = self.text_renderer.measure_ascii_text(&fps_line);

        let mut rect = Rect::new(0.0, 0.0, size.width + 3.0, size.height + 3.0);
        if let (true, Some(old_rect)) = (has_layer, old_rect) {
            rect = rect.union(old_rect);
        }

        let layer_br = ImmutableSolidColorBrush::with_opacity(Colors::BLACK, 0.5);
        let black = Brushes::black();
        let background: &dyn IBrush = if has_layer { &layer_br } else { &*black };
        context.draw_rectangle(Some(background), None, rect.into(), &BoxShadows::default());
        self.text_renderer.draw_ascii_text(context, &fps_line, &*Brushes::white());
        rect
    }

    pub fn reset(&self) {
        self.frames_this_second.set(0);
        self.total_frames.set(0);
        self.fps.set(0);
    }
}

#[cfg(test)]
mod tests {
    use super::super::diagnostic_text_renderer::tests::create_renderer;
    use super::super::dirty_rects::test_mocks::MockDrawingContext;
    use super::*;

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    /// The text drawn by the last `render_fps`, rebuilt from the glyph log.
    fn drawn_text(ctx: &MockDrawingContext) -> String {
        ctx.log
            .iter()
            .filter_map(|l| l.strip_prefix("glyph "))
            .map(|l| char::from_u32(l.split(' ').next().unwrap().parse::<u32>().unwrap()).unwrap())
            .collect()
    }

    fn render(counter: &FpsCounter, aux: &str, now: Duration) -> String {
        let mut ctx = MockDrawingContext::default();
        counter.render_fps(&mut ctx, aux, false, None, now);
        drawn_text(&ctx)
    }

    #[test]
    fn counts_frames_and_updates_fps_once_more_than_a_second_elapsed() {
        // The clock does not start at zero: only the elapsed time matters.
        let start = ms(5000);
        let counter = FpsCounter::new(Rc::new(create_renderer()), start);

        assert_eq!("Frame #00000001 FPS: 000 aux", render(&counter, "aux", start + ms(100)));
        assert_eq!("Frame #00000002 FPS: 000 ", render(&counter, "", start + ms(500)));
        // Exactly one second is not enough.
        assert_eq!("Frame #00000003 FPS: 000 ", render(&counter, "", start + ms(1000)));
        // 4 frames in 2 seconds.
        assert_eq!("Frame #00000004 FPS: 002 ", render(&counter, "", start + ms(2000)));
        // The window restarted at 2s; fps is kept until the next update.
        assert_eq!("Frame #00000005 FPS: 002 ", render(&counter, "", start + ms(2500)));
        // 2 frames in 1.25 seconds: 1.6 truncates to 1.
        assert_eq!("Frame #00000006 FPS: 001 ", render(&counter, "", start + ms(3250)));
    }

    #[test]
    fn fps_tick_counts_extra_frames() {
        let counter = FpsCounter::new(Rc::new(create_renderer()), Duration::ZERO);
        for _ in 0..119 {
            counter.fps_tick();
        }
        assert_eq!("Frame #00000001 FPS: 060 ", render(&counter, "", ms(2000)));
    }

    #[test]
    fn reset_clears_counters_but_not_the_update_time() {
        let counter = FpsCounter::new(Rc::new(create_renderer()), Duration::ZERO);
        render(&counter, "", ms(1500));
        render(&counter, "", ms(1600));
        counter.reset();
        assert_eq!("Frame #00000001 FPS: 000 ", render(&counter, "", ms(1700)));
        // 2 frames since the reset, 1.5s since the last update at 1.5s.
        assert_eq!("Frame #00000002 FPS: 001 ", render(&counter, "", ms(3000)));
    }

    #[test]
    fn draws_background_then_text_and_returns_the_rect() {
        let counter = FpsCounter::new(Rc::new(create_renderer()), Duration::ZERO);
        let mut ctx = MockDrawingContext::default();
        // "Frame #00000001 FPS: 000 x" is 26 glyphs of 6x10.
        let rect = counter.render_fps(&mut ctx, "x", false, Some(Rect::new(0.0, 0.0, 500.0, 500.0)), ms(10));
        assert_eq!(Rect::new(0.0, 0.0, 159.0, 13.0), rect);
        assert_eq!("rect #ff000000@1 0, 0, 159, 13", ctx.log[0]);
        assert_eq!(27, ctx.log.len());
        assert_eq!("glyph 70 #ffffffff@1 at 0,0", ctx.log[1]);
        assert_eq!("glyph 114 #ffffffff@1 at 6,0", ctx.log[2]);
    }

    #[test]
    fn with_a_layer_the_old_rect_is_covered_with_a_translucent_background() {
        let counter = FpsCounter::new(Rc::new(create_renderer()), Duration::ZERO);
        let mut ctx = MockDrawingContext::default();
        let rect = counter.render_fps(&mut ctx, "x", true, Some(Rect::new(0.0, 0.0, 200.0, 10.0)), ms(10));
        assert_eq!(Rect::new(0.0, 0.0, 200.0, 13.0), rect);
        assert_eq!("rect #ff000000@0.5 0, 0, 200, 13", ctx.log[0]);

        let mut ctx = MockDrawingContext::default();
        let rect = counter.render_fps(&mut ctx, "x", true, None, ms(20));
        assert_eq!(Rect::new(0.0, 0.0, 159.0, 13.0), rect);
    }
}
