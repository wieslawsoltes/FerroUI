//! Port of `ContextDropBehavior.cs`.

use crate::{ContextDropBehaviorBase, ContextDropBehaviorBaseImpl, IDropHandler};
use ferroui_base::input::DragEventArgs;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::StyledElementBehaviorImpl;
use std::rc::Rc;

/// Behavior that enables dropping context data onto the associated control using a predefined
/// [`IDropHandler`].
#[repr(C)]
pub struct ContextDropBehavior {
    base: ContextDropBehaviorBase,
}

ferro_class!(ContextDropBehavior: ContextDropBehaviorBase);
ferro_class_info!(ContextDropBehavior { new: ContextDropBehavior::new });
ferroui_base::ferro_impl_classes!(ContextDropBehavior: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

ferro_properties! {
    impl ContextDropBehavior {
        /// Identifies the `Handler` property.
        pub fn handler_property() -> StyledProperty<Option<Rc<dyn IDropHandler>>> {
            FerroProperty::register::<ContextDropBehavior, _>("Handler", None)
        }
    }
}

impl ContextDropBehaviorBaseImpl for ContextDropBehavior {
    fn on_enter(
        this: &Self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
    ) {
        if let Some(handler) = this.handler() {
            handler.enter(sender, e, source_context, target_context);
        }
    }

    fn on_leave(this: &Self, sender: Option<&BoxedValue>, e: &RoutedEventArgs) {
        if let Some(handler) = this.handler() {
            handler.leave(sender, e);
        }
    }

    fn on_over(
        this: &Self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
    ) {
        if let Some(handler) = this.handler() {
            handler.over(sender, e, source_context, target_context);
        }
    }

    fn on_drop(
        this: &Self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
    ) {
        if let Some(handler) = this.handler() {
            IDropHandler::drop(&*handler, sender, e, source_context, target_context);
        }
    }
}

impl ContextDropBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ContextDropBehaviorBase::construct() }
    }

    /// Creates the behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the drop handler that receives drop notifications.
    pub fn handler(&self) -> Option<Rc<dyn IDropHandler>> {
        self.get_value(Self::handler_property())
    }

    pub fn set_handler(&self, value: Option<Rc<dyn IDropHandler>>) {
        self.set_value(Self::handler_property(), value)
    }
}
