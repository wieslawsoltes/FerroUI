use ferroui_base::ferro_routed_event_args;
use ferroui_base::input::platform::IClipboard;
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};
use std::rc::Rc;

/// Provides data specific to a `TextBox::pasting_from_clipboard` event.
#[derive(Clone, Default)]
pub struct PastingFromClipboardEventArgs {
    base: RoutedEventArgs,
    clipboard: Option<Rc<dyn IClipboard>>,
}

ferro_routed_event_args!(PastingFromClipboardEventArgs: RoutedEventArgs);

impl PastingFromClipboardEventArgs {
    /// Initializes a new instance of the `PastingFromClipboardEventArgs`
    /// class.
    ///
    /// `routed_event` is the routed event associated with these event args,
    /// `clipboard` the clipboard being pasted from.
    pub fn new<T: ?Sized>(routed_event: &RoutedEvent<T>, clipboard: Option<Rc<dyn IClipboard>>) -> Self {
        Self { base: RoutedEventArgs::with_event(routed_event), clipboard }
    }

    /// The clipboard being pasted from. This is either the system clipboard
    /// or, when pasting via middle-click on platforms supporting it, the
    /// primary selection.
    ///
    /// `None` when no clipboard is available; a handler can still handle the
    /// event to provide custom paste behavior.
    pub fn clipboard(&self) -> Option<Rc<dyn IClipboard>> {
        self.clipboard.clone()
    }
}
