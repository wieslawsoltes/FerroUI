use ferroui_base::Size;
use futures_util::task::AtomicWaker;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::task::Poll;
use std::time::Duration;

pub(crate) enum Report {
    Ready {
        size: Size,
        duration: Duration,
    },
    LoadError(String),
    Renderer(bool),
    Frame {
        revision: u64,
        position: Duration,
        delta: Duration,
    },
    Started,
    Stopped,
}

/// The render thread wakes one dispatcher task. Pending frame positions and
/// deltas coalesce when the dispatcher is busy.
#[derive(Default)]
pub(crate) struct Feedback {
    queue: Mutex<VecDeque<(u64, Report)>>,
    waker: AtomicWaker,
}

impl Feedback {
    pub fn push(&self, generation: u64, report: Report) {
        {
            let mut queue = self
                .queue
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            match (queue.back_mut(), &report) {
                (
                    Some((
                        last_generation,
                        Report::Frame {
                            revision,
                            position,
                            delta,
                        },
                    )),
                    Report::Frame {
                        revision: next_revision,
                        position: next_position,
                        delta: next_delta,
                    },
                ) if *last_generation == generation && *revision == *next_revision => {
                    *position = *next_position;
                    *delta = delta.saturating_add(*next_delta);
                }
                _ => queue.push_back((generation, report)),
            }
        }
        self.waker.wake();
    }

    pub async fn next(&self) -> (u64, Report) {
        std::future::poll_fn(|context| {
            self.waker.register(context.waker());
            let mut queue = self
                .queue
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            queue.pop_front().map_or(Poll::Pending, Poll::Ready)
        })
        .await
    }
}
