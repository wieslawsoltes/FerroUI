use ferroui_base::ferro_routed_event_args;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEvent, RoutedEventArgs};
use std::rc::Rc;

/// A text control's synchronous request to open its context menu.
///
/// The coordinates are device-independent pixels relative to the visual root.
/// Setting the inherited `Handled` flag cancels the associated menu request.
#[derive(Clone, Default)]
pub struct ContextMenuEventArgs {
    base: RoutedEventArgs,
    cursor_left: f64,
    cursor_top: f64,
}

ferro_routed_event_args!(ContextMenuEventArgs: RoutedEventArgs);

impl ContextMenuEventArgs {
    /// Creates arguments with fixed root-relative coordinates. The caller may
    /// associate a routed event before raising them.
    pub fn new(cursor_left: f64, cursor_top: f64) -> Self {
        Self {
            cursor_left,
            cursor_top,
            ..Self::default()
        }
    }

    pub fn cursor_left(&self) -> f64 {
        self.cursor_left
    }

    pub fn cursor_top(&self) -> f64 {
        self.cursor_top
    }
}

fn typed_args<R>(
    args: &Rc<dyn IRoutedEventArgs>,
    get: impl FnOnce(&ContextMenuEventArgs) -> R,
) -> Result<R, String> {
    args.downcast_ref::<ContextMenuEventArgs>()
        .map(get)
        .ok_or_else(|| "The arguments are not ContextMenuEventArgs.".to_owned())
}

ferroui_base::ferro_markup_type!(class ContextMenuEventArgs {
    namespace: "FerroUI.Controls",
    handles: [ContextMenuEventArgs],
    this: Rc<dyn IRoutedEventArgs>,
    base: Rc<dyn IRoutedEventArgs>,
    properties: [
        CursorLeft: f64 {
            try_get: |args: &Rc<dyn IRoutedEventArgs>| typed_args(args, ContextMenuEventArgs::cursor_left)
        },
        CursorTop: f64 {
            try_get: |args: &Rc<dyn IRoutedEventArgs>| typed_args(args, ContextMenuEventArgs::cursor_top)
        },
    ],
});

/// Metadata carrier for the closed context-menu routed-event type.
#[doc(hidden)]
pub struct ContextMenuOpeningRoutedEvent;
ferroui_base::ferro_markup_type!(class ContextMenuOpeningRoutedEvent as "RoutedEvent`1" {
    namespace: "FerroUI.Interactivity",
    handles: [RoutedEvent<ContextMenuEventArgs>, Option<RoutedEvent<ContextMenuEventArgs>>],
    base: RoutedEvent,
    generic: "RoutedEvent`1" [ContextMenuEventArgs],
});
