use crate::{Repeat, Timeline};
use ferroui_base::animation::{Animatable, Animation, IClock, PlayState, SignedDuration};
use ferroui_base::reactive::{Disposable, IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::threading::Dispatcher;
use ferroui_base::Ref;
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// Inherited clock for an artboard whose timeline owns its time.
pub struct FrozenPreviewClock;

impl IObservable<SignedDuration> for FrozenPreviewClock {
    fn subscribe(&self, _observer: Rc<dyn IObserver<SignedDuration>>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

impl IClock for FrozenPreviewClock {
    fn play_state(&self) -> PlayState {
        PlayState::Pause
    }

    fn set_play_state(&self, _value: PlayState) {}
}

/// The elapsed-time bounds of a loaded animation, including delays and repetitions.
pub fn animation_timing(animation: &Animation) -> Option<Timeline> {
    let mut timing = Timeline::new(animation.duration().total_seconds()).ok()?;
    timing.delay = animation.delay().total_seconds();
    timing.delay_between_iterations = animation.delay_between_iterations().total_seconds();
    timing.speed_ratio = animation.speed_ratio();
    let iterations = animation.iteration_count();
    timing.repeat = if iterations.is_infinite() {
        Repeat::Infinite
    } else {
        Repeat::Count(iterations.value())
    };
    timing.validate().ok()?;
    Some(timing)
}

/// Owns the disposable animation-priority values for a set of native clips.
/// The host chooses targets and drives the clock. The application owns interpolation,
/// visibility, bindings and native fill behavior.
#[derive(Default)]
pub struct NativePlayback {
    runs: Vec<NativeRun>,
    elapsed: f64,
}

impl NativePlayback {
    pub fn elapsed(&self) -> f64 {
        self.elapsed
    }

    pub fn is_active(&self) -> bool {
        !self.runs.is_empty()
    }

    pub fn stop(&mut self) {
        self.runs.clear();
        self.elapsed = 0.0;
    }

    /// Replace the active bindings and reconstruct each clock from time zero.
    /// Use this after a backward seek or a change of targets.
    pub fn seek(
        &mut self,
        bindings: Vec<(Ref<Animation>, Ref<Animatable>)>,
        elapsed: f64,
    ) -> Result<bool, String> {
        validate_time(elapsed)?;
        if bindings.is_empty() {
            return Err("Choose live preview targets for this animation program".into());
        }
        self.stop();
        let mut runs = Vec::with_capacity(bindings.len());
        for (animation, target) in bindings {
            runs.push(NativeRun::new(&animation, &target)?);
        }
        pulse(&runs, 0.0)?;
        pulse(&runs, elapsed)?;
        self.runs = runs;
        self.elapsed = elapsed;
        Ok(self.complete())
    }

    /// Pulse existing subscriptions forward, preserving individual visibility clocks.
    pub fn advance_to(&mut self, elapsed: f64) -> Result<bool, String> {
        validate_time(elapsed)?;
        if !self.is_active() || elapsed < self.elapsed {
            return Err("Start a native animation before advancing its clock".into());
        }
        if let Err(error) = pulse(&self.runs, elapsed) {
            self.stop();
            return Err(error);
        }
        self.elapsed = elapsed;
        Ok(self.complete())
    }

    fn complete(&self) -> bool {
        self.runs.iter().all(|run| run.complete.get())
    }
}

struct NativeRun {
    clock: Rc<NativeClock>,
    subscription: Rc<dyn IDisposable>,
    complete: Rc<Cell<bool>>,
}

impl NativeRun {
    fn new(animation: &Animation, target: &Animatable) -> Result<Self, String> {
        let clock = Rc::new(NativeClock::default());
        let complete = Rc::new(Cell::new(false));
        let completed = complete.clone();
        capture_preview_errors(|| {
            let subscription = animation
                .try_apply_for_preview(
                    target,
                    Some(clock.clone()),
                    Some(Rc::new(move || completed.set(true))),
                )
                .map_err(|error| error.to_string())?;
            Ok(Self {
                clock,
                subscription,
                complete,
            })
        })
        .map_err(|message| format!("Native animation rejected the program: {message}"))
    }
}

impl Drop for NativeRun {
    fn drop(&mut self) {
        self.clock.state.set(PlayState::Stop);
        self.subscription.dispose();
    }
}

fn pulse(runs: &[NativeRun], seconds: f64) -> Result<(), String> {
    capture_preview_errors(|| {
        for run in runs {
            run.clock
                .subject
                .on_next(SignedDuration::from_seconds(seconds));
        }
        Ok(())
    })
    .map_err(|message| format!("Native animation frame failed: {message}"))
}

fn capture_preview_errors<T>(work: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let dispatcher = Dispatcher::current_dispatcher();
    catch_unwind(AssertUnwindSafe(|| dispatcher.capture_unhandled_exceptions(work)))
        .map_err(panic_message)??
}

fn validate_time(value: f64) -> Result<(), String> {
    if value.is_finite() && value >= 0.0 && value * 10_000_000.0 < i64::MAX as f64 {
        Ok(())
    } else {
        Err("Native animation time is outside the clock range".into())
    }
}

fn panic_message(value: Box<dyn std::any::Any + Send>) -> String {
    value
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            value
                .downcast_ref::<&str>()
                .map(|value| (*value).to_owned())
        })
        .unwrap_or_else(|| "runtime callback failed".into())
}

struct NativeClock {
    subject: LightweightSubject<SignedDuration>,
    state: Cell<PlayState>,
}

impl Default for NativeClock {
    fn default() -> Self {
        Self {
            subject: LightweightSubject::new(),
            state: Cell::new(PlayState::Run),
        }
    }
}

impl IObservable<SignedDuration> for NativeClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<SignedDuration>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl IClock for NativeClock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }

    fn set_play_state(&self, value: PlayState) {
        self.state.set(value);
    }
}
