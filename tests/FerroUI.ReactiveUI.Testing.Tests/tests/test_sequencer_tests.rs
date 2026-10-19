//! `TestSequencerTests` (`ReactiveUI.Testing.Tests/TestSequencerTests.cs`).
//!
//! The participant that the subscription starts is a task of the current thread, which waits at
//! once; upstream waits briefly (`Task.Delay`) for its handler, which runs on another thread, to
//! start.

use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_reactive::{IObserver, ObservableExt};
use ferroui_reactiveui_testing::TestSequencer;
use ferroui_system::threading::tasks::{block_on, Task};
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

const INITIAL_PHASE: i64 = 0;
const FIRST_PHASE: i64 = 1;
const SECOND_PHASE: i64 = 2;

#[test]
fn should_execute_tests_in_order() {
    let test_sequencer = Rc::new(TestSequencer::new());
    let subject = BehaviorSubject::new(());

    // Tracks the asynchronous participant.
    let advance_count = Rc::new(Cell::new(0));

    let sequencer = test_sequencer.clone();
    let count = advance_count.clone();
    let _subscription = subject.as_observable().skip(1).subscribe_next(move |_| {
        let sequencer = sequencer.clone();
        let count = count.clone();
        let _ = Task::spawn_local(async move {
            sequencer.advance_phase_async().await?;
            count.set(count.get() + 1);
            Ok(())
        });
    });

    assert_eq!(test_sequencer.current_phase(), INITIAL_PHASE);
    assert_eq!(test_sequencer.completed_phases(), INITIAL_PHASE);

    // Trigger first advance from subscription.
    subject.on_next(());

    assert_eq!(test_sequencer.current_phase(), FIRST_PHASE);
    assert_eq!(test_sequencer.completed_phases(), INITIAL_PHASE);

    // Complete Phase 1 from the test.
    block_on(test_sequencer.advance_phase_async_with("Phase 1")).expect("phase 1 completes");
    assert_eq!(test_sequencer.current_phase(), FIRST_PHASE);
    assert_eq!(test_sequencer.completed_phases(), FIRST_PHASE);
    assert_eq!(advance_count.get(), 1);

    // Trigger second advance from subscription.
    subject.on_next(());

    assert_eq!(test_sequencer.current_phase(), SECOND_PHASE);
    assert_eq!(test_sequencer.completed_phases(), FIRST_PHASE);

    // Complete Phase 2 from the test.
    block_on(test_sequencer.advance_phase_async_with("Phase 2")).expect("phase 2 completes");
    assert_eq!(test_sequencer.current_phase(), SECOND_PHASE);
    assert_eq!(test_sequencer.completed_phases(), SECOND_PHASE);
    assert_eq!(advance_count.get(), 2);

    test_sequencer.dispose();
    subject.dispose();
}
