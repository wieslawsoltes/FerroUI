//! Explicit application quiescence while a replacement process restores state.
use crate::{InputPause, ReloadError};
use ferroui_base::input::InputElement;
use std::rc::Rc;

/// Application-owned work which must stop before restart state is captured.
/// Called on the UI thread. Return only after mutation has stopped; asynchronous
/// draining must finish before this call or return an error for a later retry.
/// On error or panic the participant must undo its own partial work. Previously
/// successful participants are resumed by the handoff. This is not a mechanism
/// for suspending arbitrary threads or rolling back arbitrary user side effects.
pub trait RestartParticipant {
    fn quiesce(&self) -> Result<RestartResume, ReloadError>;
}
impl<F: Fn() -> Result<RestartResume, ReloadError>> RestartParticipant for F {
    fn quiesce(&self) -> Result<RestartResume, ReloadError> {
        self()
    }
}

/// Resumes one participant when dropped on the UI thread. The closure must not
/// panic, and must restore the participant's prior running/paused state.
pub struct RestartResume(Option<Box<dyn FnOnce()>>);
impl RestartResume {
    pub fn new(resume: impl FnOnce() + 'static) -> Self {
        Self(Some(Box::new(resume)))
    }
}
impl Drop for RestartResume {
    fn drop(&mut self) {
        if let Some(resume) = self.0.take() {
            resume();
        }
    }
}

/// Holds input and registered application participants paused until dropped.
/// Capture restart state only after `begin` succeeds. Keep this guard until the
/// old process exits or handoff fails; dropping it resumes participants in reverse
/// registration order, then releases the temporary input contribution.
pub struct RestartHandoff {
    participants: Vec<RestartResume>,
    input: Option<InputPause>,
}
impl RestartHandoff {
    pub fn begin(
        root: &InputElement,
        participants: impl IntoIterator<Item = Rc<dyn RestartParticipant>>,
    ) -> Result<Self, ReloadError> {
        let mut handoff = Self {
            input: Some(InputPause::new(root)?),
            participants: Vec::new(),
        };
        for participant in participants {
            handoff.participants.push(participant.quiesce()?);
        }
        Ok(handoff)
    }
}
impl Drop for RestartHandoff {
    fn drop(&mut self) {
        while let Some(participant) = self.participants.pop() {
            drop(participant);
        }
        self.input.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
    use ferroui_controls::Border;
    use std::{cell::RefCell, time::Duration};

    #[test]
    fn failed_participant_resumes_prior_work_before_reenabling_input() {
        let _dispatcher = Dispatcher::unit_test_scope();
        let root = Border::new();
        let events = Rc::new(RefCell::new(Vec::new()));
        let participant = |name| -> Rc<dyn RestartParticipant> {
            let root = root.clone();
            let events = events.clone();
            Rc::new(move || {
                assert!(!root.is_enabled());
                events.borrow_mut().push((name, "pause"));
                let root = root.clone();
                let events = events.clone();
                Ok(RestartResume::new(move || {
                    assert!(!root.is_enabled(), "input must remain paused during resume");
                    events.borrow_mut().push((name, "resume"));
                }))
            })
        };
        let failure: Rc<dyn RestartParticipant> =
            Rc::new(|| Err(ReloadError::Failed("worker is still draining".into())));
        let result = RestartHandoff::begin(&root, [participant(1), participant(2), failure]);
        assert!(result.is_err());
        assert_eq!(
            *events.borrow(),
            [(1, "pause"), (2, "pause"), (2, "resume"), (1, "resume")]
        );
        assert!(root.is_enabled());
    }

    #[test]
    fn completed_handoff_retains_quiescence_until_dropped_and_unwinds_safely() {
        let _dispatcher = Dispatcher::unit_test_scope();
        let root = Border::new();
        let timer = DispatcherTimer::with_callback(
            Duration::from_secs(1),
            DispatcherPriority::BACKGROUND,
            |_| {},
        );
        let participant: Rc<dyn RestartParticipant> = Rc::new({
            let timer = timer.clone();
            move || {
                let was_running = timer.is_enabled();
                timer.stop();
                let timer = timer.clone();
                Ok(RestartResume::new(move || {
                    if was_running {
                        timer.start();
                    }
                }))
            }
        });
        for initially_running in [true, false] {
            if initially_running {
                timer.start();
            } else {
                timer.stop();
            }
            let handoff = RestartHandoff::begin(&root, [participant.clone()]).unwrap();
            assert!(!timer.is_enabled());
            assert!(!root.is_enabled());
            drop(handoff);
            assert_eq!(timer.is_enabled(), initially_running);
            assert!(root.is_enabled());
        }
        timer.start();
        let panic: Rc<dyn RestartParticipant> = Rc::new(|| panic!("participant failed"));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = RestartHandoff::begin(&root, [participant, panic]);
        }));
        assert!(result.is_err());
        assert!(timer.is_enabled());
        assert!(root.is_enabled());
        timer.stop();
    }

    #[test]
    fn background_producer_acknowledges_quiescence_and_resumes_queued_work() {
        use std::sync::mpsc::{self, Sender};
        enum Command {
            Produce(Sender<u64>),
            Pause(Sender<bool>),
            Inspect(Sender<(u64, bool, usize)>),
            Resume,
            Stop,
        }
        struct Worker {
            send: Sender<Command>,
            join: Option<std::thread::JoinHandle<()>>,
        }
        impl Drop for Worker {
            fn drop(&mut self) {
                let _ = self.send.send(Command::Stop);
                self.join.take().unwrap().join().unwrap();
            }
        }
        let _dispatcher = Dispatcher::unit_test_scope();
        let root = Border::new();
        let (send, receive) = mpsc::channel();
        let worker = Worker {
            send,
            join: Some(std::thread::spawn(move || {
                let mut state = 0;
                let mut paused = false;
                let mut pending = Vec::new();
                while let Ok(command) = receive.recv() {
                    match command {
                        Command::Produce(done) if paused => pending.push(done),
                        Command::Produce(done) => {
                            state += 1;
                            let _ = done.send(state);
                        }
                        Command::Pause(done) => {
                            let was_paused = paused;
                            paused = true;
                            let _ = done.send(was_paused);
                        }
                        Command::Inspect(done) => {
                            let _ = done.send((state, paused, pending.len()));
                        }
                        Command::Resume => {
                            paused = false;
                            for done in pending.drain(..) {
                                state += 1;
                                let _ = done.send(state);
                            }
                        }
                        Command::Stop => break,
                    }
                }
            })),
        };
        let inspect = || {
            let (send, receive) = mpsc::channel();
            worker.send.send(Command::Inspect(send)).unwrap();
            receive.recv_timeout(Duration::from_secs(5)).unwrap()
        };
        // The application adapter waits for the producer's acknowledgement; an
        // input-only pause would not stop this non-UI state from changing.
        let participant: Rc<dyn RestartParticipant> = Rc::new({
            let commands = worker.send.clone();
            move || {
                let (send, receive) = mpsc::channel();
                commands.send(Command::Pause(send)).unwrap();
                let was_paused = receive.recv_timeout(Duration::from_secs(5)).unwrap();
                let commands = commands.clone();
                Ok(RestartResume::new(move || {
                    if !was_paused {
                        commands.send(Command::Resume).unwrap();
                    }
                }))
            }
        });
        let handoff = RestartHandoff::begin(&root, [participant.clone()]).unwrap();
        let (done, produced) = mpsc::channel();
        worker.send.send(Command::Produce(done)).unwrap();
        // Inspect is a FIFO barrier: the worker has seen the production request
        // and queued it, so this assertion does not depend on scheduling/sleeps.
        assert_eq!(inspect(), (0, true, 1));
        assert!(matches!(
            produced.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        assert!(!root.is_enabled());
        drop(handoff);
        assert_eq!(produced.recv_timeout(Duration::from_secs(5)).unwrap(), 1);
        assert_eq!(inspect(), (1, false, 0));
        assert!(root.is_enabled());

        // Failure in another participant must release this background producer,
        // just as cancellation of a replacement startup does.
        let failure: Rc<dyn RestartParticipant> =
            Rc::new(|| Err(ReloadError::Failed("replacement is not ready".into())));
        assert!(RestartHandoff::begin(&root, [participant.clone(), failure]).is_err());
        assert_eq!(inspect(), (1, false, 0));
        assert!(root.is_enabled());

        // A producer paused by the application before handoff stays paused.
        let (send, receive) = mpsc::channel();
        worker.send.send(Command::Pause(send)).unwrap();
        assert!(!receive.recv_timeout(Duration::from_secs(5)).unwrap());
        drop(RestartHandoff::begin(&root, [participant]).unwrap());
        assert_eq!(inspect(), (1, true, 0));
    }
}
