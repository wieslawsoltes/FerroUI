// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::presentation::IDeferredContentPresentationTarget;
use crate::{
    DeferredContentControl, DeferredContentPresentationTimeline, DeferredContentPresenter,
};
use ferroui_base::Ref;
use ferroui_controls::Control;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
pub(crate) struct TargetState {
    pub(crate) timeline: RefCell<Weak<DeferredContentPresentationTimeline>>,
    handle: RefCell<Weak<NativeTarget>>,
}
impl TargetState {
    pub(crate) fn new() -> Self {
        Self {
            timeline: RefCell::new(Weak::new()),
            handle: RefCell::new(Weak::new()),
        }
    }
    pub(crate) fn project(&self, control: &Control) -> Rc<dyn IDeferredContentPresentationTarget> {
        if let Some(handle) = self.handle.borrow().upgrade() {
            return handle;
        }
        let handle = Rc::new(NativeTarget(control.to_ref()));
        *self.handle.borrow_mut() = Rc::downgrade(&handle);
        handle
    }
    pub(crate) fn remove(&self, control: &Control) {
        let timeline = self.timeline.borrow().upgrade();
        if let Some(timeline) = timeline {
            timeline.remove(&self.project(control));
        }
    }
}
struct NativeTarget(Ref<Control>);
impl NativeTarget {
    fn with_state<T>(&self, action: impl FnOnce(&TargetState) -> T) -> T {
        if let Some(control) = self.0.cast::<DeferredContentControl>() {
            action(&control.target)
        } else {
            action(
                &self
                    .0
                    .cast::<DeferredContentPresenter>()
                    .expect("deferred target native class")
                    .target,
            )
        }
    }
}
impl IDeferredContentPresentationTarget for NativeTarget {
    fn enqueued_timeline(&self) -> Option<Rc<DeferredContentPresentationTimeline>> {
        self.with_state(|state| state.timeline.borrow().upgrade())
    }
    fn set_enqueued_timeline(&self, value: Option<Rc<DeferredContentPresentationTimeline>>) {
        self.with_state(|state| {
            *state.timeline.borrow_mut() =
                value.map_or_else(Weak::new, |timeline| Rc::downgrade(&timeline))
        });
    }
    fn retain_pending_presentation_on_failure(&self) -> bool {
        false
    }
    fn apply_deferred_presentation(&self) -> bool {
        if let Some(control) = self.0.cast::<DeferredContentControl>() {
            control.apply_deferred_presentation(true)
        } else {
            self.0
                .cast::<DeferredContentPresenter>()
                .expect("deferred presenter target")
                .apply_deferred_presentation(true)
        }
    }
}
