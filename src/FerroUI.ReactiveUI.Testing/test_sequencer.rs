//! `TestSequencer` (`ReactiveUI.Testing/TestSequencer.cs`).

use ferroui_system::threading::tasks::{Task, TaskCompletionSource};
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::fmt;

/// Two participants of a test that wait for each other phase by phase: each calls
/// [`advance_phase_async`](Self::advance_phase_async) at the end of its part of a phase, and the
/// phase completes when both have (`TestSequencer`, a `Barrier` of two participants).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the observables are single-threaded, so the two
/// participants are two tasks of the current thread instead of two threads: the task of the first
/// participant to arrive ends when the second one arrives, and the task of the second one has
/// ended when [`advance_phase_async`](Self::advance_phase_async) returns. The continuations of a
/// task run when it ends, so upstream's `Task.Yield()` after the barrier has no counterpart.
#[derive(Default)]
pub struct TestSequencer {
    /// The phases both participants completed (`Barrier.CurrentPhaseNumber`).
    completed_phases: Cell<i64>,
    /// The phase the participants are in (`CurrentPhase`).
    current_phase: Cell<i64>,
    /// The first participant of the current phase, waiting for the second one.
    waiting: RefCell<Option<TaskCompletionSource<()>>>,
    disposed: Cell<bool>,
}

impl fmt::Debug for TestSequencer {
    /// `{CompletedPhases}, {CurrentPhase}` (`DebuggerDisplay`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}", self.completed_phases(), self.current_phase())
    }
}

impl TestSequencer {
    /// A sequencer before its first phase.
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of phases both participants completed (`CompletedPhases`).
    pub fn completed_phases(&self) -> i64 {
        self.completed_phases.get()
    }

    /// The phase the participants are in, set when the first of them arrives (`CurrentPhase`).
    pub fn current_phase(&self) -> i64 {
        self.current_phase.get()
    }

    /// Ends the part of the current phase of the calling participant; the task ends when the other
    /// participant has ended its part too (`AdvancePhaseAsync()`).
    pub fn advance_phase_async(&self) -> Task<()> {
        self.advance_phase_async_with("")
    }

    /// [`advance_phase_async`](Self::advance_phase_async), with a comment that names the phase for
    /// the reader of the test (`AdvancePhaseAsync(string)`).
    ///
    /// # Panics
    /// When the sequencer is disposed, where upstream's barrier throws `ObjectDisposedException`.
    pub fn advance_phase_async_with(&self, comment: &str) -> Task<()> {
        let _ = comment;
        assert!(!self.disposed.get(), "Cannot access a disposed object. Object name: 'TestSequencer'.");
        let waiting = self.waiting.borrow_mut().take();
        match waiting {
            // The first participant to arrive starts the phase and waits for the other one.
            None => {
                self.current_phase.set(self.completed_phases.get() + 1);
                let source = TaskCompletionSource::new();
                let task = source.task();
                *self.waiting.borrow_mut() = Some(source);
                task
            }
            // The second one completes the phase and releases the first.
            Some(first) => {
                self.completed_phases.set(self.completed_phases.get() + 1);
                first.set_result(());
                Task::from_result(())
            }
        }
    }
}

impl IDisposable for TestSequencer {
    /// Disposes the sequencer (`Dispose`); a participant still waiting has its task cancelled.
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        let waiting = self.waiting.borrow_mut().take();
        if let Some(waiting) = waiting {
            let _ = waiting.try_set_canceled();
        }
    }
}
