//! A capacity-one mailbox wakes the local dispatcher. The owning UI thread
//! delivers all QML signals and applies visual updates between events.
use super::*;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use std::sync::Arc;

impl QmlView {
    pub(super) fn install_worker_dispatch(&self) {
        let model = self.0.instance.borrow();
        if self.0.worker_wake.get().is_some()
            || !(model.has_worker_scripts() || model.has_pending_component_loads())
        {
            return;
        }
        drop(model);
        let (wake, notifications) = async_channel::bounded(1);
        self.0
            .worker_wake
            .set(wake.clone())
            .expect("one worker dispatcher");
        let notify: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            let _ = wake.try_send(());
        });
        {
            let mut model = self.0.instance.borrow_mut();
            model.set_worker_waker(Some(notify.clone()));
            model.set_component_waker(Some(notify));
        }
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
                            let delivered = {
                                let mut model = view.0.instance.borrow_mut();
                                let network = model.poll_component_loads(1);
                                let workers = model.poll_worker_scripts(1);
                                network.and_then(|n| workers.map(|w| n + w))
                            };
                            match view.flush().and(delivered) {
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

    /// Incorporate downloaded QML on the owner. Mounted views are also woken
    /// automatically when their configured transport completes a request.
    pub fn poll_component_loads(&self, limit: usize) -> Result<usize, Error> {
        let delivered = self.0.instance.borrow_mut().poll_component_loads(limit);
        self.flush()?;
        delivered
    }

    /// Dispatch pending WorkerScript messages on the view's owner. A mounted
    /// view also receives worker notifications automatically, without a timer.
    pub fn poll_worker_scripts(&self, limit: usize) -> Result<usize, Error> {
        let delivered = self.0.instance.borrow_mut().poll_worker_scripts(limit);
        self.flush()?;
        delivered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_controls::{
        testing::{TestServices, UnitTestApplication},
        TextBlock,
    };
    use ferroui_qml::{
        ir,
        workers::{WorkerDelivery, WorkerEndpoint, WorkerEvent, WorkerValue},
        ScriptContext,
    };

    struct Backend(RefCell<Option<WorkerDelivery>>);
    struct Endpoint;
    impl WorkerEndpoint for Endpoint {
        fn send(&self, _: WorkerValue) -> Result<(), Error> {
            Ok(())
        }
    }

    impl ScriptEngine for Backend {
        fn evaluate(
            &self,
            _: &mut ScriptContext<'_, '_>,
            _: &ir::JavaScript,
        ) -> Result<Value, Error> {
            Err(Error::new(
                "the dispatcher fixture uses native QML expressions",
            ))
        }
        fn start_worker(
            &self,
            _: Arc<BTreeMap<String, ir::ScriptResource>>,
            _: &str,
            _: &str,
            events: WorkerDelivery,
        ) -> Result<Box<dyn WorkerEndpoint>, Error> {
            self.0.replace(Some(events));
            Ok(Box::new(Endpoint))
        }
    }

    #[test]
    fn worker_thread_wakes_the_owner_dispatcher_and_updates_the_native_peer() {
        let _scope = UnitTestApplication::start(TestServices::new());
        let document = ir::parse(r#"
            import QtQuick
            import QtQml.WorkerScript
            Column {
                id: root
                property string output: "waiting"
                WorkerScript { source: "echo.js"; onMessage: function(messageObject) { root.output = messageObject } }
                Text { id: label; text: root.output }
            }
        "#).unwrap();
        let backend = Rc::new(Backend(RefCell::default()));
        let view = QmlView::with_script_engine(
            &document,
            &ControlRegistry::standard().unwrap(),
            Default::default(),
            backend.clone(),
        )
        .unwrap();
        let label = view
            .control(view.id("label").unwrap())
            .unwrap()
            .cast::<TextBlock>()
            .unwrap();
        let producer = backend
            .0
            .take()
            .expect("worker registered before view mount");
        std::thread::spawn(move || {
            assert!(producer.publish(WorkerEvent::Message(WorkerValue::String(
                "delivered".into()
            ))));
        })
        .join()
        .unwrap();
        assert_eq!(label.text().as_deref(), Some("waiting"));
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(label.text().as_deref(), Some("delivered"));
    }
}
