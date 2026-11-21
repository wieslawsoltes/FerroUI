//! Ribbon.cs strings subscription and deferred root/tab/group refresh.
//! Copyright (c) 2026 Wiesław Šoltés. MIT.
use super::*;
use ferroribbon_core::localization::{RibbonStrings, RibbonStringsChanged};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_controls::ToolTip;
use std::{
    collections::HashMap,
    sync::{
        Arc, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

#[cfg(test)]
#[path = "../root_localization_tests.rs"]
mod tests;

#[derive(Default)]
pub(super) struct LocalizationState {
    subscription: RefCell<Option<StringsSubscription>>,
}
struct StringsSubscription {
    // The static event retains the loaded ribbon as upstream does. Keeping only
    // a weak link back to the event prevents an event/ribbon ownership cycle.
    event: Weak<RibbonStringsChanged>,
    token: u64,
}
impl Drop for StringsSubscription {
    fn drop(&mut self) {
        if let Some(event) = self.event.upgrade() {
            event.remove(self.token);
        }
    }
}
// Only native Ribbon receivers are stored here, on their owning UI thread.
// Global callbacks and queued Send jobs own a link, never a control reference.
ferroui_base::ferro_thread_local! { static RECEIVERS:RefCell<HashMap<u64,Ref<Ribbon>>>=RefCell::default(); }
static NEXT_RECEIVER: AtomicU64 = AtomicU64::new(1);
struct ReceiverLink {
    key: u64,
    dispatcher: Arc<Dispatcher>,
}
impl Drop for ReceiverLink {
    fn drop(&mut self) {
        let key = self.key;
        self.dispatcher.post(
            move || {
                let removed = RECEIVERS.with(|receivers| receivers.borrow_mut().remove(&key));
                drop(removed); // application cleanup may reenter the receiver registry
            },
            DispatcherPriority::NORMAL,
        );
    }
}
impl Ribbon {
    pub(super) fn attach_strings(&self) {
        self.detach_strings();
        let event = RibbonStrings::current_changed();
        let key = NEXT_RECEIVER.fetch_add(1, Ordering::Relaxed);
        RECEIVERS.with(|receivers| {
            receivers.borrow_mut().insert(key, self.to_ref());
        });
        let link = Arc::new(ReceiverLink {
            key,
            dispatcher: Dispatcher::current_dispatcher(),
        });
        let token = event.add(Arc::new(move |_| {
            let queued = link.clone();
            link.dispatcher.post(
                move || {
                    let root =
                        RECEIVERS.with(|receivers| receivers.borrow().get(&queued.key).cloned());
                    root.expect("live localization receiver ticket")
                        .refresh_strings();
                    drop(queued);
                },
                DispatcherPriority::NORMAL,
            );
        }));
        let old = self
            .localization
            .subscription
            .replace(Some(StringsSubscription {
                event: Arc::downgrade(&event),
                token,
            }));
        drop(old);
    }
    pub(super) fn detach_strings(&self) {
        drop(self.localization.subscription.take());
    }
    pub(super) fn refresh_display_options_strings(&self) {
        if self.display_options_button().is_none() {
            return;
        }
        {
            let button = self
                .display_options_button()
                .expect("display-options button before tooltip");
            let text = RibbonStrings::current()
                .ribbon_display_options()
                .map(|s| s.to_utf8().expect("native display-options tooltip"));
            ToolTip::set_tip(&button, text.map(|s| Rc::new(s) as BoxedValue));
        }
        {
            // Source reads the field and Current again after SetToolTip: either
            // may have changed reentrantly while the property was being set.
            let button = self
                .display_options_button()
                .expect("display-options button before name");
            let text = RibbonStrings::current()
                .ribbon_display_options()
                .map(|s| s.to_utf8().expect("native display-options name"));
            AutomationProperties::set_name(&button, text.as_deref());
        }
    }
    fn refresh_strings(&self) {
        self.update_chrome();
        self.refresh_display_options_strings();
        for tab in self.tabs.iter() {
            tab.expect("null RibbonTab during strings refresh")
                .on_strings_changed();
        }
    }
}
