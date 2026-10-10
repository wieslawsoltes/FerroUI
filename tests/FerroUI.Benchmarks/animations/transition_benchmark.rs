//! Running a transition of a number over a number of frames.

use crate::harness::Registry;
use ferroui_base::animation::{DoubleTransition, TimeSpan, Transition};
use ferroui_base::layout::Layoutable;
use ferroui_base::reactive::{IObservable, IObserver, LightweightSubject, ObservableError};
use ferroui_base::Ref;
use std::cell::RefCell;
use std::rc::Rc;

pub struct TransitionBenchmark {
    observer: Rc<AddValueObserver>,
    produced_values: Rc<RefCell<Vec<f64>>>,
    time_producer: Rc<LightweightSubject<f64>>,
    transition: Ref<DoubleTransition>,
    frame_count: i32,
}

impl TransitionBenchmark {
    pub fn new(frame_count: i32) -> Self {
        // The upstream constructor reads the parameter before the benchmark
        // library assigns it, when it still holds zero: the duration of the
        // transition is zero and the list of the values starts empty. The
        // benchmark gives the transition its progress itself, so the
        // duration is not read.
        let frame_count_in_constructor = 0;

        let transition = DoubleTransition::new();
        transition.set_duration(TimeSpan::from_milliseconds(f64::from(frame_count_in_constructor)));
        transition.set_property(Some(Layoutable::width_property().as_property()));

        let time_producer = Rc::new(LightweightSubject::new());
        let produced_values = Rc::new(RefCell::new(Vec::with_capacity(frame_count_in_constructor as usize)));

        let observer = Rc::new(AddValueObserver::new(produced_values.clone()));

        Self { observer, produced_values, time_producer, transition, frame_count }
    }

    pub fn new_transition(&self) {
        let progress: Rc<dyn IObservable<f64>> = self.time_producer.clone();
        let transition_obs = DoubleTransition::do_transition(&self.transition, progress, 0.0, 1.0);

        self.produced_values.borrow_mut().clear();

        let observer: Rc<dyn IObserver<f64>> = self.observer.clone();
        let transition_sub = transition_obs.subscribe(observer);

        for i in 0..self.frame_count {
            self.time_producer.on_next(f64::from(i) / 1000.0);
        }

        debug_assert!(self.produced_values.borrow().len() == self.frame_count as usize);

        transition_sub.dispose();
    }
}

struct AddValueObserver {
    values: Rc<RefCell<Vec<f64>>>,
}

impl AddValueObserver {
    fn new(values: Rc<RefCell<Vec<f64>>>) -> Self {
        Self { values }
    }
}

impl IObserver<f64> for AddValueObserver {
    fn on_completed(&self) {}

    fn on_error(&self, _error: ObservableError) {}

    fn on_next(&self, value: f64) {
        self.values.borrow_mut().push(value);
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("animations", "TransitionBenchmark");
    for frame_count in [10, 100] {
        class.benchmark(
            "new_transition",
            format!("FrameCount={frame_count}"),
            move || TransitionBenchmark::new(frame_count),
            |b| b.new_transition(),
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn transition_benchmark() {
        crate::harness::smoke_class(super::register, "TransitionBenchmark");
    }
}
