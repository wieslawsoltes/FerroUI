//! Port of `ListBoxItem/SelectListBoxItemOnPointerMovedBehavior.cs`.

use ferroui_base::input::{InputElement, PointerEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StaticType, StyledElementImpl, WeakRef,
};
use ferroui_controls::{Control, ListBoxItem};
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::RefCell;

/// Sets the `IsSelected` property of the [`ListBoxItem`] parent of the associated control to true
/// on its pointer moved event.
#[repr(C)]
pub struct SelectListBoxItemOnPointerMovedBehavior {
    base: StyledElementBehaviorOfT,
    // The handler of the pointer moved event (`PointerMoved -= PointerMoved` of the reference).
    pointer_moved: RefCell<Option<(WeakRef<Control>, RoutedEventHandlerToken)>>,
}

ferro_class!(SelectListBoxItemOnPointerMovedBehavior: StyledElementBehaviorOfT);
ferro_class_info!(SelectListBoxItemOnPointerMovedBehavior { new: SelectListBoxItemOnPointerMovedBehavior::new });
ferroui_base::ferro_impl_classes!(SelectListBoxItemOnPointerMovedBehavior: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for SelectListBoxItemOnPointerMovedBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            if let Some(associated_object) = this.associated_object_as::<Control>() {
                this.remove_handler();
                let behavior = this.to_ref().downgrade();
                let token = associated_object.add_handler(
                    InputElement::pointer_moved_event(),
                    move |_sender: &Interactive, args: &PointerEventArgs| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.pointer_moved(args);
                        }
                    },
                );
                *this.pointer_moved.borrow_mut() = Some((associated_object.downgrade(), token));
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.remove_handler();
        }
    }
}

impl SelectListBoxItemOnPointerMovedBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE), pointer_moved: RefCell::new(None) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn remove_handler(&self) {
        let handler = self.pointer_moved.borrow_mut().take();
        if let Some((control, token)) = handler {
            if let Some(control) = control.upgrade() {
                control.remove_handler(InputElement::pointer_moved_event(), token);
            }
        }
    }

    fn pointer_moved(&self, _args: &PointerEventArgs) {
        let item = self
            .associated_object_as::<Control>()
            .and_then(|associated_object| associated_object.parent())
            .and_then(|parent| parent.cast::<ListBoxItem>());
        if let Some(item) = item {
            item.set_current_value(ListBoxItem::is_selected_property(), true);
            Dispatcher::ui_thread().post_local(
                move || {
                    item.focus();
                },
                DispatcherPriority::DEFAULT,
            );
        }
    }
}
