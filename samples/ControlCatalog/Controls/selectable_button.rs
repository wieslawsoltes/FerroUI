//! Port of `Controls/SelectableButton.cs`.

use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_routed_event, instantiate,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl,
    StyledProperty, StyledPropertyOptions, VisualImpl,
};
use ferroui_controls::primitives::{TemplatedControlImpl, ToggleButton, ToggleButtonImpl};
use ferroui_controls::{register_selectable, ButtonImpl, ContentControlImpl, ControlImpl, ISelectable};

#[repr(C)]
pub struct SelectableButton {
    base: ToggleButton,
}

ferro_class! {
    SelectableButton: ToggleButton, virtuals SelectableButtonImpl: ToggleButtonImpl {
        /// Called when `IsSelected` changes. `e` are the event arguments
        /// for the routed event that is raised by the default
        /// implementation of this method.
        fn on_is_selected_changed(this, e: &RoutedEventArgs);
    }
}
ferro_class_info!(SelectableButton {
    new: SelectableButton::new,
    markup: {
        attributes: [PseudoClasses(":selected")],
    },
});
ferro_impl_classes!(
    SelectableButton: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    ButtonImpl,
    ToggleButtonImpl
);

impl FerroObjectImpl for SelectableButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_pseudo_classes(this.is_selected());
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::is_selected_property().as_property() {
            let new_value = change.get_new_value::<bool>();
            this.update_pseudo_classes(new_value);
            this.on_is_selected_changed(&RoutedEventArgs::with_event(Self::is_selected_changed_event()));
        }
    }
}

impl SelectableButtonImpl for SelectableButton {
    fn on_is_selected_changed(this: &Self, e: &RoutedEventArgs) {
        this.raise_event(e);
    }
}

impl ISelectable for SelectableButton {
    fn is_selected(&self) -> bool {
        SelectableButton::is_selected(self)
    }

    fn set_is_selected(&self, value: bool) {
        SelectableButton::set_is_selected(self, value)
    }
}

ferro_properties! {
    impl SelectableButton {
        /// Defines the `IsSelected` property.
        pub fn is_selected_property() -> StyledProperty<bool> {
            FerroProperty::register_with::<SelectableButton, _>(
                "IsSelected",
                StyledPropertyOptions::new(false).default_binding_mode(BindingMode::TwoWay),
            )
        }
    }
}

impl SelectableButton {
    ferro_routed_event!(
        /// Defines the `IsSelectedChanged` event.
        pub fn is_selected_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<SelectableButton, _>("IsSelectedChanged", RoutingStrategies::BUBBLE)
        }
    );

    fn static_constructor() {
        register_selectable::<SelectableButton>();
    }

    pub fn construct() -> Self {
        Self { base: ToggleButton::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Whether the button is selected.
    pub fn is_selected(&self) -> bool {
        self.get_value(Self::is_selected_property())
    }

    pub fn set_is_selected(&self, value: bool) {
        self.set_value(Self::is_selected_property(), value)
    }

    /// Raised when the `IsSelected` property value changes.
    pub fn is_selected_changed(
        &self,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::is_selected_changed_event(), handler)
    }

    fn update_pseudo_classes(&self, is_selected: bool) {
        self.pseudo_classes().set(":selected", is_selected);
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;
    use ferroui_controls::as_selectable;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn selecting_sets_the_pseudo_class_and_raises_the_event() {
        let button = SelectableButton::new();
        let raised = Rc::new(Cell::new(0));
        let sink = raised.clone();
        button.is_selected_changed(move |_, _| sink.set(sink.get() + 1));

        assert!(!button.classes().contains(":selected"));
        button.set_is_selected(true);
        assert!(button.classes().contains(":selected"));
        assert_eq!(1, raised.get());

        let selectable = as_selectable(&button).expect("a selectable");
        selectable.set_is_selected(false);
        assert!(!button.is_selected());
        assert_eq!(2, raised.get());
    }
}
