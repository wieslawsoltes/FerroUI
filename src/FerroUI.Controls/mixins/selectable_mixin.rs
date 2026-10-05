use crate::primitives::SelectingItemsControl;
use crate::Control;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{ObjectType, StyledProperty, Upcast};

/// Adds selectable functionality to control classes.
///
/// The [`SelectableMixin`] adds behavior to a control which can be selected.
/// It adds the following behavior:
///
/// - Adds a ":selected" class when the control is selected.
/// - Raises a `SelectingItemsControl::is_selected_changed_event` when the
///   selected state changes.
pub struct SelectableMixin;

impl SelectableMixin {
    /// Initializes the mixin for the control class `TControl`.
    ///
    /// `is_selected` is the `IsSelected` property.
    pub fn attach<TControl: ObjectType + Upcast<Control>>(is_selected: &'static StyledProperty<bool>) {
        is_selected.changed().subscribe(|x| {
            if let Some(sender) = x.sender().downcast_ref::<TControl>() {
                let sender: &Control = sender.upcast();
                sender.pseudo_classes().set(":selected", x.get_new_value::<bool>());

                sender.raise_event(&RoutedEventArgs::with_event(SelectingItemsControl::is_selected_changed_event()));
            }
        });
    }
}
