//! Port of `Actions/RemoveElementAction.cs`.

use super::control_of;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::{ContentControl, Control, Decorator, ItemsControl, Panel};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Removes the [`target_object`](Self::target_object) (or the sender) from its parent: a panel,
/// a decorator, a content control, a content presenter or an items control.
#[repr(C)]
pub struct RemoveElementAction {
    base: StyledElementAction,
}

ferro_class!(RemoveElementAction: StyledElementAction);
ferro_class_info!(RemoveElementAction {
    new: RemoveElementAction::new,
    markup: {
        property_attributes: [
            TargetObject: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(RemoveElementAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl RemoveElementAction {
        /// Identifies the `TargetObject` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own its target, so the value
        /// is an element reference.
        pub fn target_object_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<RemoveElementAction, _>("TargetObject", None)
        }
    }
}

/// Whether an untyped content is the element (`Content == element`: reference equality).
fn is_element(content: Option<BoxedValue>, element: &Ref<Control>) -> bool {
    content.as_ref().and_then(Control::from_boxed).is_some_and(|content| content.ptr_eq(element))
}

impl StyledElementActionImpl for RemoveElementAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let element = if this.get_value(Self::target_object_property()).is_some() {
            this.target_object()
        } else {
            control_of(sender)
        };
        let Some(element) = element else {
            return Some(Rc::new(false));
        };

        let Some(parent) = element.parent() else {
            return Some(Rc::new(false));
        };

        if let Some(panel) = parent.cast::<Panel>() {
            panel.children().remove(&element);
            return Some(Rc::new(true));
        }

        if let Some(decorator) = parent.cast::<Decorator>() {
            if decorator.child().is_some_and(|child| child.ptr_eq(&element)) {
                decorator.set_child(None::<Ref<Control>>);
                return Some(Rc::new(true));
            }
        }

        if let Some(content_control) = parent.cast::<ContentControl>() {
            if is_element(content_control.content(), &element) {
                content_control.set_content(None);
                return Some(Rc::new(true));
            }
        }

        if let Some(presenter) = parent.cast::<ContentPresenter>() {
            if is_element(presenter.content(), &element) {
                presenter.set_content(None);
                return Some(Rc::new(true));
            }
        }

        if let Some(items_control) = parent.cast::<ItemsControl>() {
            let items = items_control.items();
            let item = items.iter().find(|item| is_element(item.clone(), &element));
            if let Some(item) = item {
                items.remove(&item);
                return Some(Rc::new(true));
            }
        }

        Some(Rc::new(false))
    }
}

impl RemoveElementAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the element to remove. If not set, the sender is used. Markup resolves a name
    /// given for it to the named element.
    pub fn target_object(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::target_object_property()))
    }

    pub fn set_target_object(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::target_object_property(), ElementRef::from_nullable(value.into().0))
    }
}
