//! Port of `Events/Handlers/FlyoutEventHandler.cs`.

use crate::events::{AddEventHandlerCallback, IAddEventHandler};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::EventArgs;
use ferroui_base::BoxedValue;
use ferroui_controls::primitives::FlyoutBase;
use std::rc::Rc;

/// The `Opened` and `Closed` events of a flyout.
pub(crate) struct FlyoutEventHandler;

impl IAddEventHandler for FlyoutEventHandler {
    fn matches(&self, source: &BoxedValue, event_name: &str) -> bool {
        ValueTypes::as_object(&**source).is_some_and(|source| source.is::<FlyoutBase>())
            && matches!(event_name, "Opened" | "Closed")
    }

    fn add_handler(
        &self,
        source: &BoxedValue,
        event_name: &str,
        handler: AddEventHandlerCallback,
    ) -> Option<Rc<dyn IDisposable>> {
        let target = ValueTypes::as_object(&**source).and_then(|source| source.cast::<FlyoutBase>())?;

        // The sender of the events is the flyout; the handler does not keep it alive.
        let sender = target.downgrade();
        let event_handler = move || {
            let sender: Option<BoxedValue> = sender.upgrade().map(|sender| Rc::new(sender) as BoxedValue);
            let event_args: BoxedValue = Rc::new(EventArgs::EMPTY);
            handler(sender.as_ref(), &event_args);
        };

        match event_name {
            "Opened" => Some(target.opened(event_handler)),
            "Closed" => Some(target.closed(event_handler)),
            _ => None,
        }
    }
}
