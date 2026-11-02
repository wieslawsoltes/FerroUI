//! CPU workload for the public animation API. Pass normal, reverse, alternate, or attach.
use ferroui_base::animation::{
    Animation, Cue, FillMode, IClock, IterationCount, KeyFrame, PlayState, PlaybackDirection,
    TimeSpan,
};
use ferroui_base::layout::Layoutable;
use ferroui_base::reactive::{Disposable, IDisposable, IObservable, IObserver, Observable};
use ferroui_base::styling::Setter;
use std::cell::{Cell, RefCell};
use std::hint::black_box;
use std::rc::Rc;
use std::time::Instant;

struct ManualClock {
    observer: RefCell<Option<Rc<dyn IObserver<TimeSpan>>>>,
    state: Cell<PlayState>,
}

impl ManualClock {
    fn step(&self, milliseconds: i64) {
        let observer = self.observer.borrow().clone().unwrap();
        observer.on_next(TimeSpan::from_ticks(milliseconds * 10_000));
    }
}

impl IObservable<TimeSpan> for ManualClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        *self.observer.borrow_mut() = Some(observer);
        Disposable::empty()
    }
}

impl IClock for ManualClock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }
    fn set_play_state(&self, value: PlayState) {
        self.state.set(value)
    }
}

fn main() {
    let mode = std::env::args().nth(1).expect("workload");
    let direction = match mode.as_str() {
        "normal" | "attach" => PlaybackDirection::Normal,
        "reverse" => PlaybackDirection::Reverse,
        "alternate" => PlaybackDirection::AlternateReverse,
        _ => panic!("unknown workload"),
    };
    let target = Layoutable::new();
    target.set_width(100.0);
    let animation = Animation::new();
    animation.set_duration(TimeSpan::from_ticks(1_000_000));
    if mode == "alternate" {
        animation.set_delay(TimeSpan::from_ticks(300_000));
        animation.set_delay_between_iterations(TimeSpan::from_ticks(200_000));
    }
    animation.set_fill_mode(FillMode::Both);
    animation.set_playback_direction(direction);
    animation.set_iteration_count(IterationCount::INFINITE);
    for (cue, width) in [(0.0, 100.0), (0.5, 250.0), (1.0, 200.0)] {
        animation.children().add(KeyFrame::with_cue(
            Cue::new(cue),
            [Setter::new(Layoutable::width_property(), width) as _],
        ));
    }
    let clock = Rc::new(ManualClock {
        observer: RefCell::new(None),
        state: Cell::new(PlayState::Run),
    });
    let attach = || {
        animation.apply(
            &target,
            Some(clock.clone()),
            Observable::return_(true),
            None,
            false,
        )
    };
    let mut checksum = 0.0;
    let mut subscription = attach();
    clock.step(0);
    let n = if mode == "attach" { 1000 } else { 24000 };
    let started = Instant::now();
    for i in 1..=n {
        if mode == "attach" {
            subscription.dispose();
            subscription = attach();
            clock.step(0);
        }
        clock.step(i);
        checksum += black_box(target.width());
    }
    let elapsed = started.elapsed().as_nanos();
    subscription.dispose();
    assert!(checksum.is_finite() && checksum > 0.0);
    println!("{n} {checksum} {elapsed}");
}
