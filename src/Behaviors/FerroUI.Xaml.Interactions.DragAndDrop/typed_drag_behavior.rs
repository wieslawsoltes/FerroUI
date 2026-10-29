//! Port of `TypedDragBehavior.cs`.

use crate::{TypedDragBehaviorBase, TypedDragBehaviorBaseImpl, IDragHandler};
use ferroui_base::input::PointerEventArgs;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::StyledElementBehaviorImpl;
use std::rc::Rc;

/// Behavior that initiates a drag operation for a specific data type, and notifies its
/// [`handler`](Self::handler) before and after the drag.
#[repr(C)]
pub struct TypedDragBehavior {
    base: TypedDragBehaviorBase,
}

ferro_class!(TypedDragBehavior: TypedDragBehaviorBase);
ferro_class_info!(TypedDragBehavior { new: TypedDragBehavior::new });
ferroui_base::ferro_impl_classes!(TypedDragBehavior: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

ferro_properties! {
    impl TypedDragBehavior {
        /// Identifies the `Handler` property.
        pub fn handler_property() -> StyledProperty<Option<Rc<dyn IDragHandler>>> {
            FerroProperty::register::<TypedDragBehavior, _>("Handler", None)
        }
    }
}

impl TypedDragBehaviorBaseImpl for TypedDragBehavior {
    fn on_before_drag_drop(this: &Self, sender: Option<&BoxedValue>, e: &PointerEventArgs, context: Option<&BoxedValue>) {
        if let Some(handler) = this.handler() {
            handler.before_drag_drop(sender, e, context);
        }
    }

    fn on_after_drag_drop(this: &Self, sender: Option<&BoxedValue>, e: &PointerEventArgs, context: Option<&BoxedValue>) {
        if let Some(handler) = this.handler() {
            handler.after_drag_drop(sender, e, context);
        }
    }
}

impl TypedDragBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: TypedDragBehaviorBase::construct() }
    }

    /// Creates the behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the handler that receives drag notifications.
    pub fn handler(&self) -> Option<Rc<dyn IDragHandler>> {
        self.get_value(Self::handler_property())
    }

    pub fn set_handler(&self, value: Option<Rc<dyn IDragHandler>>) {
        self.set_value(Self::handler_property(), value)
    }
}
