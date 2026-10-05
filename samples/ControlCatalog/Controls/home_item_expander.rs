//! Port of `Controls/HomeItemExpander.cs`.

use ferroui_base::data::BindingMode;
use ferroui_base::input::{ICommand, InputElementImpl};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, DirectProperty,
    DirectPropertyMetadata, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    StyledElementImpl, StyledProperty, StyledPropertyOptions, VisualImpl,
};
use ferroui_controls::primitives::{SelectingItemsControl, TemplatedControlImpl};
use ferroui_controls::{
    register_selectable, Button, ContentControlImpl, ControlImpl, Expander, ExpanderImpl, ISelectable,
};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct HomeItemExpander {
    base: Expander,
    is_effectively_expanded: Cell<bool>,
}

ferro_class!(HomeItemExpander: Expander);
ferro_class_info!(HomeItemExpander { new: HomeItemExpander::new });
ferro_impl_classes!(
    HomeItemExpander: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    ExpanderImpl
);

impl FerroObjectImpl for HomeItemExpander {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::can_expand_property().as_property()
            || change.property() == Expander::is_expanded_property().as_property()
        {
            this.set_is_effectively_expanded(this.can_expand() && this.is_expanded());
        }
    }
}

impl ISelectable for HomeItemExpander {
    fn is_selected(&self) -> bool {
        HomeItemExpander::is_selected(self)
    }

    fn set_is_selected(&self, value: bool) {
        HomeItemExpander::set_is_selected(self, value)
    }
}

ferro_properties! {
    impl HomeItemExpander {
        /// Defines the `Command` property.
        pub fn command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            Button::command_property().add_owner::<HomeItemExpander>()
        }

        /// Defines the `CommandParameter` property.
        pub fn command_parameter_property() -> StyledProperty<Option<BoxedValue>> {
            Button::command_parameter_property().add_owner::<HomeItemExpander>()
        }

        /// Defines the `CanExpand` property.
        pub fn can_expand_property() -> StyledProperty<bool> {
            FerroProperty::register_with::<HomeItemExpander, _>(
                "CanExpand",
                StyledPropertyOptions::new(false).default_binding_mode(BindingMode::TwoWay),
            )
        }

        /// Defines the `IsEffectivelyExpanded` property.
        pub fn is_effectively_expanded_property() -> DirectProperty<HomeItemExpander, bool> {
            FerroProperty::register_direct_with::<HomeItemExpander, _>(
                "IsEffectivelyExpanded",
                HomeItemExpander::is_effectively_expanded,
                Some(HomeItemExpander::set_is_effectively_expanded),
                DirectPropertyMetadata::new(None).with_default_binding_mode(BindingMode::TwoWay),
            )
        }

        /// Defines the `IsSelected` property.
        pub fn is_selected_property() -> StyledProperty<bool> {
            SelectingItemsControl::is_selected_property().add_owner::<HomeItemExpander>()
        }
    }
}

impl HomeItemExpander {
    fn static_constructor() {
        register_selectable::<HomeItemExpander>();
    }

    pub fn construct() -> Self {
        Self { base: Expander::construct(), is_effectively_expanded: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// An `ICommand` to be invoked when the button is clicked.
    pub fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::command_property())
    }

    pub fn set_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::command_property(), value)
    }

    /// A parameter to be passed to the `Command`.
    pub fn command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::command_parameter_property())
    }

    pub fn set_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::command_parameter_property(), value)
    }

    /// A value indicating whether the content area can be open and visible.
    pub fn can_expand(&self) -> bool {
        self.get_value(Self::can_expand_property())
    }

    pub fn set_can_expand(&self, value: bool) {
        self.set_value(Self::can_expand_property(), value)
    }

    /// The selection state of the item.
    pub fn is_selected(&self) -> bool {
        self.get_value(Self::is_selected_property())
    }

    pub fn set_is_selected(&self, value: bool) {
        self.set_value(Self::is_selected_property(), value)
    }

    /// Whether the expander is expanded.
    pub fn is_effectively_expanded(&self) -> bool {
        self.is_effectively_expanded.get()
    }

    fn set_is_effectively_expanded(&self, value: bool) {
        self.set_and_raise_cell(Self::is_effectively_expanded_property(), &self.is_effectively_expanded, value);
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn is_effectively_expanded_needs_both_can_expand_and_is_expanded() {
        let expander = HomeItemExpander::new();
        assert!(!expander.is_effectively_expanded());

        expander.set_is_expanded(true);
        assert!(!expander.is_effectively_expanded());

        expander.set_can_expand(true);
        assert!(expander.is_effectively_expanded());

        expander.set_is_expanded(false);
        assert!(!expander.is_effectively_expanded());
    }
}
