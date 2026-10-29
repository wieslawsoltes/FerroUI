//! Port of `DropBehaviorBase.cs`.

use crate::routed_handlers::RoutedHandlers;
use crate::values::boxed_object;
use crate::IDropHandler;
use ferroui_base::input::{DragDrop, DragEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class, BoxedValue, FerroObjectImpl, StyledElementImpl};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{InvokeCommandBehaviorBase, StyledElementBehaviorImpl};
use std::cell::RefCell;
use std::rc::Rc;

/// Base class for behaviors that respond to drag-and-drop events: the drag events of the
/// associated control go to the [`handler`](Self::handler), and the deriving class executes the
/// command of the behavior with [`execute_command`](Self::execute_command).
///
/// The class is abstract.
#[repr(C)]
pub struct DropBehaviorBase {
    base: InvokeCommandBehaviorBase,
    handler: RefCell<Option<Rc<dyn IDropHandler>>>,
    handlers: RoutedHandlers,
}

ferro_class!(DropBehaviorBase: InvokeCommandBehaviorBase);
ferroui_base::ferro_impl_classes!(DropBehaviorBase: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for DropBehaviorBase {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return;
            };

            DragDrop::set_allow_drop(&associated_object, true);
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                DragDrop::drag_enter_event(),
                move |sender: &Interactive, e: &DragEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.drag_enter(&boxed_object(&sender.to_ref()), e);
                    }
                },
                Interactive::DEFAULT_ROUTES,
                false,
            );
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                DragDrop::drag_leave_event(),
                move |sender: &Interactive, e: &DragEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.drag_leave(&boxed_object(&sender.to_ref()), e);
                    }
                },
                Interactive::DEFAULT_ROUTES,
                false,
            );
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                DragDrop::drag_over_event(),
                move |sender: &Interactive, e: &DragEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.drag_over(&boxed_object(&sender.to_ref()), e);
                    }
                },
                Interactive::DEFAULT_ROUTES,
                false,
            );
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                DragDrop::drop_event(),
                move |sender: &Interactive, e: &DragEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.drop_event_handler(&boxed_object(&sender.to_ref()), e);
                    }
                },
                Interactive::DEFAULT_ROUTES,
                false,
            );
        }

        fn on_detached_from_visual_tree(this: &Self) {
            if let Some(associated_object) = this.associated_object_as::<Control>() {
                DragDrop::set_allow_drop(&associated_object, false);
            }
            this.handlers.clear();
        }
    }
}

impl DropBehaviorBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InvokeCommandBehaviorBase::construct(), handler: RefCell::new(None), handlers: RoutedHandlers::new() }
    }

    /// Gets or sets the handler responsible for processing drop events (a protected member of the
    /// reference).
    pub fn handler(&self) -> Option<Rc<dyn IDropHandler>> {
        self.handler.borrow().clone()
    }

    pub fn set_handler(&self, value: Option<Rc<dyn IDropHandler>>) {
        *self.handler.borrow_mut() = value;
    }

    fn drag_enter(&self, sender: &BoxedValue, e: &DragEventArgs) {
        if let Some(handler) = self.handler() {
            handler.enter(Some(sender), e, None, None);
        }
    }

    fn drag_leave(&self, sender: &BoxedValue, e: &RoutedEventArgs) {
        if let Some(handler) = self.handler() {
            handler.leave(Some(sender), e);
        }
    }

    fn drag_over(&self, sender: &BoxedValue, e: &DragEventArgs) {
        if let Some(handler) = self.handler() {
            handler.over(Some(sender), e, None, None);
        }
    }

    // `Drop` of the reference: a method named `drop` would be taken for the destructor.
    fn drop_event_handler(&self, sender: &BoxedValue, e: &DragEventArgs) {
        if let Some(handler) = self.handler() {
            IDropHandler::drop(&*handler, Some(sender), e, None, None);
        }
    }

    /// Executes the associated command (a protected member of the reference): when the behavior
    /// is enabled, its control is visible and enabled, and the command can execute with the
    /// resolved parameter.
    pub fn execute_command(&self, parameter: Option<&BoxedValue>) {
        let Some(command) = self.command().filter(|_| self.is_enabled()) else {
            return;
        };

        let Some(associated_object) = self.associated_object_as::<Control>() else {
            return;
        };
        if !(associated_object.is_visible() && associated_object.is_enabled()) {
            return;
        }

        let resolved_parameter = self.resolve_parameter(parameter);

        if !command.can_execute(resolved_parameter.as_ref()) {
            return;
        }

        command.execute(resolved_parameter.as_ref());
    }
}
