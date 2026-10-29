//! Port of `ContextDragBehavior.cs`.

use crate::{ContextDragBehaviorBase, ContextDragBehaviorBaseImpl, IDragHandler};
use ferroui_base::input::PointerEventArgs;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::StyledElementBehaviorImpl;
use std::rc::Rc;

/// Behavior that starts a drag operation using the associated context data, and notifies its
/// [`handler`](Self::handler) before and after the drag.
#[repr(C)]
pub struct ContextDragBehavior {
    base: ContextDragBehaviorBase,
}

ferro_class!(ContextDragBehavior: ContextDragBehaviorBase);
ferro_class_info!(ContextDragBehavior { new: ContextDragBehavior::new });
ferroui_base::ferro_impl_classes!(ContextDragBehavior: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

ferro_properties! {
    impl ContextDragBehavior {
        /// Identifies the `Handler` property.
        pub fn handler_property() -> StyledProperty<Option<Rc<dyn IDragHandler>>> {
            FerroProperty::register::<ContextDragBehavior, _>("Handler", None)
        }
    }
}

impl ContextDragBehaviorBaseImpl for ContextDragBehavior {
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

impl ContextDragBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ContextDragBehaviorBase::construct() }
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
