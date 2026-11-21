//! Source checked events are local snapshots, independent of routed-event vetoes.
use ferroui_base::{BoxedValue, interactivity::RoutedEventArgs, metadata::MarkupDelegate};
use ferroui_controls::Control;
use std::{cell::RefCell, rc::Rc};
#[derive(Default)]
pub(super) struct CheckedEvents([RefCell<Option<MarkupDelegate>>; 3]);
impl CheckedEvents {
    pub fn add(&self, index: usize, handler: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.0[index].borrow().as_ref(), handler.as_ref());
        self.0[index].replace(next);
    }
    pub fn remove(&self, index: usize, handler: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.0[index].borrow().as_ref(), handler.as_ref());
        self.0[index].replace(next);
    }
    pub fn raise(&self, owner: &Control, state: Option<bool>, before: impl FnOnce()) {
        let index = match state {
            Some(true) => 0,
            Some(false) => 1,
            None => 2,
        };
        let handler = self.0[index].borrow().clone();
        before();
        if let Some(handler) = handler {
            let args = RoutedEventArgs::new();
            args.set_source(owner.to_ref().upcast::<ferroui_base::FerroObject>());
            handler
                .try_invoke(&[
                    Some(Control::boxed(owner.to_ref())),
                    Some(Rc::new(args) as BoxedValue),
                ])
                .expect("checked multicast handler");
        }
    }
}
pub(super) fn click(button: &ferroui_controls::primitives::ToggleButton) {
    use ferroui_controls::{
        Button,
        platform::{FeedbackAction, PlatformFeedbackExtensions},
    };
    if !button.is_effectively_enabled() {
        return;
    }
    button.toggle();
    button.perform_feedback(FeedbackAction::click());
    let args = RoutedEventArgs::with_event(Button::click_event());
    button.raise_event_local(&args);
    if let Some(command) = button.command() {
        let parameter = button.command_parameter();
        if command.can_execute(button.command_parameter().as_ref()) {
            command.execute(parameter.as_ref());
        }
    }
    // Source ToggleButton derives ButtonBase, not the native Button flyout extension.
}
pub(super) fn parameter(state: Option<bool>) -> Option<BoxedValue> {
    state.map(|s| Rc::new(s) as BoxedValue)
}
