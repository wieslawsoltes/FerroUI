use crate::{CompositionAnimation, NormalizeResult};
use std::time::Duration;

#[derive(Default)]
pub(crate) struct Playback {
    elapsed: Duration,
    pub position: Duration,
    last_frame: Option<Duration>,
    completed_loops: u128,
    pub running: bool,
    finished: bool,
}

impl Playback {
    pub fn start(&mut self, repeats: i32) {
        *self = Self {
            running: repeats != 0,
            ..Self::default()
        };
    }

    pub fn stop(&mut self) {
        *self = Self::default();
    }

    pub fn pause(&mut self) {
        self.running = false;
        self.reset_clock();
    }

    pub fn resume(&mut self, repeats: i32) {
        self.running = repeats != 0 && !self.finished;
        self.reset_clock();
    }

    pub fn seek(&mut self, position: Duration, duration: Option<Duration>) {
        let position = duration.map_or(position, |duration| position.min(duration));
        self.position = position;
        self.elapsed = position;
        self.completed_loops = 0;
        self.finished = false;
        self.reset_clock();
    }

    pub fn reset_clock(&mut self) {
        self.last_frame = None;
    }

    pub fn advance(
        &mut self,
        now: Duration,
        rate: f64,
        repeats: i32,
        animation: &mut dyn CompositionAnimation,
    ) -> Option<Duration> {
        let previous = self.last_frame.replace(now)?;
        let delta_seconds = now.saturating_sub(previous).as_secs_f64() * rate;
        let delta = Duration::try_from_secs_f64(delta_seconds).unwrap_or(Duration::MAX);
        if delta.is_zero() {
            return None;
        }

        let elapsed = self.elapsed.saturating_add(delta);
        if let Some(duration) = animation.duration().filter(|duration| !duration.is_zero()) {
            let (remainder, loops) = split_loops(elapsed, duration);
            if self.count_loops(loops, repeats) {
                self.position = duration;
                self.elapsed = duration;
            } else {
                self.elapsed = remainder;
                self.position = animation.on_normalize_elapsed(remainder).effective_elapsed;
            }
        } else {
            let normalized = animation.on_normalize_elapsed(elapsed);
            self.position = normalized.effective_elapsed;
            self.elapsed = if normalized.has_looped {
                self.position
            } else {
                elapsed
            };
            self.count_loops(u128::from(normalized.has_looped), repeats);
        }
        Some(delta)
    }

    fn count_loops(&mut self, loops: u128, repeats: i32) -> bool {
        self.completed_loops = self.completed_loops.saturating_add(loops);
        if repeats > 0 && self.completed_loops >= repeats as u128 {
            self.finished = true;
            self.pause();
        }
        self.finished
    }
}

fn split_loops(elapsed: Duration, duration: Duration) -> (Duration, u128) {
    const NANOS_PER_SECOND: u128 = 1_000_000_000;
    let loops = elapsed.as_nanos() / duration.as_nanos();
    let remainder = elapsed.as_nanos() % duration.as_nanos();
    let remainder = Duration::new(
        (remainder / NANOS_PER_SECOND) as u64,
        (remainder % NANOS_PER_SECOND) as u32,
    );
    (remainder, loops)
}

pub(crate) fn normalize_elapsed(elapsed: Duration, duration: Duration) -> NormalizeResult {
    if duration.is_zero() {
        return NormalizeResult::new(elapsed, false);
    }
    let (remainder, loops) = split_loops(elapsed, duration);
    NormalizeResult::new(remainder, loops > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::{Rect, Size};
    use skia_safe::Canvas;

    struct Animation;

    impl CompositionAnimation for Animation {
        fn source_size(&self) -> Size {
            Size::new(100.0, 100.0)
        }

        fn duration(&self) -> Option<Duration> {
            Some(Duration::from_secs(2))
        }

        fn on_render(&mut self, _: &Canvas, _: Rect, _: Duration, _: bool) {}
    }

    fn seconds(seconds: f64) -> Duration {
        Duration::from_secs_f64(seconds)
    }

    #[test]
    fn pause_resume_seek_and_stop_preserve_the_expected_position() {
        let mut playback = Playback::default();
        let mut animation = Animation;
        playback.start(-1);
        assert_eq!(
            playback.advance(seconds(10.0), 2.0, -1, &mut animation),
            None
        );
        assert_eq!(
            playback.advance(seconds(10.25), 2.0, -1, &mut animation),
            Some(seconds(0.5))
        );
        assert_eq!(playback.position, seconds(0.5));
        playback.pause();
        assert!(!playback.running);
        playback.seek(seconds(1.0), animation.duration());
        playback.resume(-1);
        playback.advance(seconds(30.0), 1.0, -1, &mut animation);
        playback.advance(seconds(30.25), 1.0, -1, &mut animation);
        assert_eq!(playback.position, seconds(1.25));
        playback.stop();
        assert_eq!(playback.position, Duration::ZERO);
        assert!(!playback.running);
    }

    #[test]
    fn counts_skipped_loops_and_holds_the_end_frame() {
        let mut playback = Playback::default();
        let mut animation = Animation;
        playback.start(3);
        playback.advance(seconds(0.0), 1.0, 3, &mut animation);
        playback.advance(seconds(4.5), 1.0, 3, &mut animation);
        assert_eq!(playback.position, seconds(0.5));
        assert!(playback.running);
        playback.advance(seconds(6.0), 1.0, 3, &mut animation);
        assert_eq!(playback.position, seconds(2.0));
        assert!(!playback.running);
        playback.resume(3);
        assert!(!playback.running);
    }

    #[test]
    fn zero_repeats_and_seek_at_end_do_not_wrap_the_displayed_frame() {
        let mut playback = Playback::default();
        playback.start(0);
        assert!(!playback.running);
        playback.seek(seconds(30.0), Some(seconds(2.0)));
        assert_eq!(playback.position, seconds(2.0));
        playback.start(1);
        assert_eq!(playback.position, Duration::ZERO);
    }
}
