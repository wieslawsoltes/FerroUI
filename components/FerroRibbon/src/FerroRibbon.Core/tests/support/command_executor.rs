//! Development-only standard futures-rs executor and completion fixtures.
use ferroribbon_core::commands::{RibbonAsyncCommandFuture, RibbonCommandExecutor};
use ferroui_system::ObservableError;
use futures_channel::oneshot;
use futures_executor::LocalPool;
use futures_util::{FutureExt, future::Shared, task::LocalSpawnExt};
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
pub struct Executor {
    pool: LocalPool,
    unhandled: Rc<RefCell<Vec<ObservableError>>>,
}
impl Executor {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn spawner(&self) -> Rc<RibbonCommandExecutor> {
        let spawner = self.pool.spawner();
        let errors = self.unhandled.clone();
        Rc::new(move |future| {
            let errors = errors.clone();
            spawner
                .spawn_local(async move {
                    if let Err(error) = future.await {
                        errors.borrow_mut().push(error);
                    }
                })
                .unwrap();
        })
    }
    pub fn pump(&mut self) {
        self.pool.run_until_stalled();
    }
    pub fn wait(&mut self, future: RibbonAsyncCommandFuture) -> Result<(), ObservableError> {
        self.pool.run_until(future)
    }
    pub fn poll(
        &mut self,
        future: &mut RibbonAsyncCommandFuture,
    ) -> Option<Result<(), ObservableError>> {
        self.pump();
        future.as_mut().now_or_never()
    }
    pub fn take_unhandled(&self) -> Vec<ObservableError> {
        std::mem::take(&mut self.unhandled.borrow_mut())
    }
}
pub fn channel() -> (
    oneshot::Sender<Result<(), ObservableError>>,
    Shared<RibbonAsyncCommandFuture>,
) {
    let (sender, receiver) = oneshot::channel();
    let future: RibbonAsyncCommandFuture = Box::pin(async move { receiver.await.unwrap() });
    (sender, future.shared())
}
pub fn canceled() -> ObservableError {
    Rc::new(std::io::Error::new(
        std::io::ErrorKind::Interrupted,
        "cancelled",
    ))
}
