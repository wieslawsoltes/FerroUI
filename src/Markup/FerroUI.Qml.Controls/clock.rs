use super::{Error, QmlView};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use std::{cell::Cell, rc::Rc, time::Duration};

pub(crate) struct ViewClock {
    timer: Rc<DispatcherTimer>,
    sample_ms: Cell<i64>,
    manual: Cell<bool>,
}

impl ViewClock {
    pub(crate) fn stop(&self) {
        self.timer.stop();
    }
}

impl QmlView {
    pub(crate) fn install_clock(&self) {
        if !self.0.instance.borrow().has_timers() {
            return;
        }
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
        self.0.subscriptions.borrow_mut().push(subscription);
        self.0
            .clock
            .set(ViewClock {
                timer,
                sample_ms: Cell::new(0),
                manual: Cell::new(false),
            })
            .ok()
            .expect("one view clock");
        self.refresh_clock();
    }

    pub(crate) fn refresh_clock(&self) {
        let Some(clock) = self.0.clock.get() else {
            return;
        };
        let active =
            !clock.manual.get() && self.0.instance.borrow().next_timer_deadline().is_some();
        if active && !clock.timer.is_enabled() {
            clock.sample_ms.set(clock.timer.dispatcher().now());
            clock.timer.start();
        } else if !active {
            clock.stop();
        }
    }

    pub(crate) fn post_deferred(&self) {
        if !self.0.instance.borrow().has_deferred_work() || self.0.deferred_posted.replace(true) {
            return;
        }
        let weak = Rc::downgrade(&self.0);
        ferroui_base::threading::Dispatcher::current_dispatcher().post_local(
            move || {
                let Some(inner) = weak.upgrade() else {
                    return;
                };
                let view = QmlView(inner);
                view.0.deferred_posted.set(false);
                if let Err(error) = view.process_deferred(64) {
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
        if let Some(clock) = self.0.clock.get() {
            clock.manual.set(true);
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
