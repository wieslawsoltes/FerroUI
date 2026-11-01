//! A capacity-one mailbox wakes the local dispatcher. The owning UI thread
//! delivers all QML signals and applies visual updates between events.
use super::*;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use std::sync::Arc;

impl QmlView {
    pub(super) fn install_worker_dispatch(&self) {
        if self.0.worker_wake.get().is_some() || !self.0.instance.borrow().has_worker_scripts() {
            return;
        }
        let (wake, notifications) = async_channel::bounded(1);
        self.0
            .worker_wake
            .set(wake.clone())
            .expect("one worker dispatcher");
        self.0
            .instance
            .borrow_mut()
            .set_worker_waker(Some(Arc::new(move || {
                let _ = wake.try_send(());
            })));
        let weak = Rc::downgrade(&self.0);
        Dispatcher::current_dispatcher().invoke_async_task_local_with_priority(
            move || async move {
                while notifications.recv().await.is_ok() {
                    loop {
                        let Some(inner) = weak.upgrade() else {
                            return;
                        };
                        let view = QmlView(inner);
                        let mut drained = false;
                        for _ in 0..64 {
                            match view.poll_worker_scripts(1) {
                                Ok(0) => {
                                    drained = true;
                                    break;
                                }
                                Ok(_) => {}
                                Err(error) => (view.0.on_error)(error),
                            }
                        }
                        if drained {
                            break;
                        }
                        drop(view);
                        Dispatcher::yield_with_priority(DispatcherPriority::BACKGROUND).await;
                    }
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }

    /// Dispatch pending WorkerScript messages on the view's owner. A mounted
    /// view also receives worker notifications automatically, without a timer.
    pub fn poll_worker_scripts(&self, limit: usize) -> Result<usize, Error> {
        let delivered = self.0.instance.borrow_mut().poll_worker_scripts(limit);
        self.flush()?;
        delivered
    }
}
