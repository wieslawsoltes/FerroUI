use super::{Error, QmlView};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use std::{cell::Cell, rc::Rc, time::Duration};

const DEFERRED_BUDGET_MS: i64 = 4;
const FRAME_INTERVAL_MS: i64 = 16;

pub(crate) struct ViewClock {
    timer: Rc<DispatcherTimer>,
    sample_ms: Cell<i64>,
    _subscription: crate::tree::Subscriptions,
}

impl ViewClock {
    pub(crate) fn stop(&self) {
        self.timer.stop();
    }
}

impl QmlView {
    fn install_clock(&self) {
        let timer = DispatcherTimer::with_priority(DispatcherPriority::RENDER);
        // QML timers are animation-clock participants. A single dispatcher
        // subscription services every timer in the component on every backend.
        timer.set_interval(Duration::from_millis(16));
        let weak = Rc::downgrade(&self.0);
        let subscription = timer.tick(move |timer| {
            let Some(inner) = weak.upgrade() else {
                timer.stop();
                return;
            };
            let view = QmlView(inner);
            let clock = view.0.clock.get().expect("installed view clock");
            let now = timer.dispatcher().now();
            let elapsed = now.saturating_sub(clock.sample_ms.replace(now)).max(0) as u64;
            if let Err(error) = view.step_clock(Duration::from_millis(elapsed)) {
                timer.stop();
                (view.0.on_error)(error);
            }
        });
        self.0
            .clock
            .set(ViewClock {
                timer,
                sample_ms: Cell::new(0),
                _subscription: crate::tree::Subscriptions(vec![subscription]),
            })
            .ok()
            .expect("one view clock");
    }

    pub(crate) fn refresh_clock(&self) {
        if self.0.clock.get().is_none() {
            if !self.0.instance.borrow().has_timers() || self.0.manual_clock.get() {
                return;
            }
            self.install_clock();
        }
        let clock = self.0.clock.get().expect("installed view clock");
        let active =
            !self.0.manual_clock.get() && self.0.instance.borrow().next_timer_deadline().is_some();
        if active && !clock.timer.is_enabled() {
            clock.sample_ms.set(clock.timer.dispatcher().now());
            clock.timer.start();
        } else if !active {
            clock.stop();
        }
    }

    pub(crate) fn post_deferred(&self) {
        if !self.0.instance.borrow().has_deferred_work() {
            if let Some(timer) = self.0.deferred_resume.borrow_mut().take() {
                timer.dispose();
                self.0.deferred_posted.set(false);
            }
            return;
        }
        if self.0.deferred_posted.replace(true) {
            return;
        }
        let weak = Rc::downgrade(&self.0);
        Dispatcher::current_dispatcher().post_local(
            move || {
                let Some(inner) = weak.upgrade() else {
                    return;
                };
                let view = QmlView(inner);
                view.0.deferred_posted.set(false);
                let dispatcher = Dispatcher::current_dispatcher();
                let start = dispatcher.now();
                // Leave room for input and rendering on a busy dispatcher. One
                // already-running activation or guest callback is indivisible.
                let result = view.0.instance.borrow_mut().process_deferred_while(64, || {
                    dispatcher.now().saturating_sub(start) < DEFERRED_BUDGET_MS
                });
                let elapsed = dispatcher.now().saturating_sub(start);
                let should_yield = {
                    let model = view.0.instance.borrow();
                    model.has_incubating_loader()
                        || (elapsed >= DEFERRED_BUDGET_MS && model.has_deferred_work())
                };
                if should_yield {
                    // A dispatcher may drain newly posted background jobs in this
                    // OS callback. Resume via a timer so the next slice actually
                    // yields the thread to platform input and rendering.
                    view.0.deferred_posted.set(true);
                    let weak = Rc::downgrade(&view.0);
                    let timer = DispatcherTimer::run_once(
                        move || {
                            if let Some(inner) = weak.upgrade() {
                                inner.deferred_resume.borrow_mut().take();
                                inner.deferred_posted.set(false);
                                QmlView(inner).post_deferred();
                            }
                        },
                        Duration::from_millis(
                            (FRAME_INTERVAL_MS.saturating_sub(elapsed)).max(1) as u64
                        ),
                        DispatcherPriority::BACKGROUND,
                    );
                    view.0.deferred_resume.replace(Some(timer));
                }
                if let Err(error) = view.flush().and(result) {
                    (view.0.on_error)(error);
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }

    /// Deliver one bounded turn of deferred bindings and JavaScript callbacks.
    pub fn process_deferred(&self, limit: usize) -> Result<usize, Error> {
        let result = self.0.instance.borrow_mut().process_deferred(limit);
        self.flush()?;
        result
    }

    fn step_clock(&self, elapsed: Duration) -> Result<(), Error> {
        self.0.instance.borrow_mut().advance_time(elapsed)?;
        self.flush()
    }

    /// Advance the component from a deterministic clock. The first call takes
    /// ownership of ticking from the dispatcher for the lifetime of this view.
    pub fn advance_time(&self, elapsed: Duration) -> Result<(), Error> {
        self.0.manual_clock.set(true);
        if let Some(clock) = self.0.clock.get() {
            clock.stop();
        }
        self.step_clock(elapsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ControlRegistry;
    use ferroui_controls::{
        testing::{TestServices, UnitTestApplication},
        TextBlock,
    };
    use ferroui_qml::ir::parse;
    use std::collections::BTreeMap;

    #[test]
    fn unfinished_loader_preparation_yields_the_dispatcher_and_deactivation_cancels_the_timer() {
        use ferroui_qml::Value;

        let _scope = UnitTestApplication::start(TestServices::new());
        let source = format!(
            "import QtQuick\nLoader {{ asynchronous: true; sourceComponent: Item {{ {} }} }}",
            "Rectangle {}\n".repeat(96)
        );
        let view = QmlView::new(
            &parse(&source).unwrap(),
            &ControlRegistry::standard().unwrap(),
            BTreeMap::new(),
        )
        .unwrap();
        let root = view.0.instance.borrow().root();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(view.get(root, "item").unwrap(), Value::Null);
        assert!(view.0.instance.borrow().has_incubating_loader());
        assert!(view.0.deferred_resume.borrow().is_some());
        view.set(root, "active", Value::Bool(false)).unwrap();
        assert!(view.0.deferred_resume.borrow().is_none());
        assert!(!view.0.instance.borrow().has_deferred_work());
    }

    #[test]
    fn mounted_timer_uses_a_disposable_dispatcher_driver_and_manual_ticks_update_the_control() {
        let _scope = UnitTestApplication::start(TestServices::new());
        let doc=parse("import QtQml\nimport QtQuick\nColumn { id: root; property int count: 0; Timer { running: true; interval: 16; onTriggered: root.count++ }; Text { id: label; text: 'ticks ' + root.count } }").unwrap();
        let view =
            QmlView::new(&doc, &ControlRegistry::standard().unwrap(), BTreeMap::new()).unwrap();
        let timer = view.0.clock.get().unwrap().timer.clone();
        let label = view
            .control(view.id("label").unwrap())
            .unwrap()
            .cast::<TextBlock>()
            .unwrap();
        assert!(timer.is_enabled());
        view.advance_time(Duration::from_millis(16)).unwrap();
        assert_eq!(label.text().as_deref(), Some("ticks 1"));
        assert!(!timer.is_enabled());
        drop(view);
        assert!(!timer.is_enabled());
    }
}
