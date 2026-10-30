//! Port of `SplitView/SplitViewTogglePaneAction.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::SplitView;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Toggles the [`is_pane_open`](SplitView::is_pane_open) state of a [`SplitView`] when executed.
#[repr(C)]
pub struct SplitViewTogglePaneAction {
    base: StyledElementAction,
}

ferro_class!(SplitViewTogglePaneAction: StyledElementAction);
ferro_class_info!(SplitViewTogglePaneAction {
    new: SplitViewTogglePaneAction::new,
    markup: {
        property_attributes: [
            TargetSplitView: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(SplitViewTogglePaneAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl SplitViewTogglePaneAction {
        /// Identifies the `TargetSplitView` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the split view, so the
        /// value is an element reference.
        pub fn target_split_view_property() -> StyledProperty<Option<ElementRef<SplitView>>> {
            ValueTypes::register_element_ref::<SplitView>();
            FerroProperty::register::<SplitViewTogglePaneAction, _>("TargetSplitView", None)
        }
    }
}

impl StyledElementActionImpl for SplitViewTogglePaneAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let split_view = match this.target_split_view() {
            Some(split_view) => Some(split_view),
            None => crate::part_c::sender_as::<SplitView>(sender),
        };
        let Some(split_view) = split_view else {
            return Some(Rc::new(false));
        };

        split_view.set_is_pane_open(!split_view.is_pane_open());
        Some(Rc::new(true))
    }
}

impl SplitViewTogglePaneAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the target [`SplitView`]. If not set, the sender is used. This is a property
    /// of the object model; markup resolves a name given for it to the named split view.
    pub fn target_split_view(&self) -> Option<Ref<SplitView>> {
        ElementRef::resolve(&self.get_value(Self::target_split_view_property()))
    }

    pub fn set_target_split_view(&self, value: impl Into<Nullable<SplitView>>) {
        self.set_value(Self::target_split_view_property(), ElementRef::from_nullable(value.into().0))
    }
}
