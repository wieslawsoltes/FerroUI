//! Port of `Events/Handlers/ButtonClickEventHandler.cs`.

use crate::events::{AddEventHandlerCallback, IAddEventHandler};
use crate::DisposableAction;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::reactive::IDisposable;
use ferroui_base::BoxedValue;
use ferroui_controls::Button;
use std::rc::Rc;

/// The `Click` event of a button (the registry registers the general form of it).
#[allow(dead_code)]
pub(crate) struct ButtonClickEventHandler;

#[allow(dead_code)]
impl ButtonClickEventHandler {
    const EVENT_NAME: &'static str = "Click";
}

impl IAddEventHandler for ButtonClickEventHandler {
    fn matches(&self, source: &BoxedValue, event_name: &str) -> bool {
        ValueTypes::as_object(&**source).is_some_and(|source| source.is::<Button>()) && event_name == Self::EVENT_NAME
    }

    fn add_handler(
        &self,
        source: &BoxedValue,
        event_name: &str,
        handler: AddEventHandlerCallback,
    ) -> Option<Rc<dyn IDisposable>> {
        let target = ValueTypes::as_object(&**source).and_then(|source| source.cast::<Button>())?;
        if event_name != Self::EVENT_NAME {
            return None;
        }

        let token = target.click(move |sender, e| {
            let sender: BoxedValue = Rc::new(sender.to_ref());
            let e: BoxedValue = Rc::new(e.share());
            handler(Some(&sender), &e);
        });

        let target = target.downgrade();
        Some(DisposableAction::create(move || {
            if let Some(target) = target.upgrade() {
                target.remove_handler(Button::click_event(), token);
            }
        }))
    }
}
