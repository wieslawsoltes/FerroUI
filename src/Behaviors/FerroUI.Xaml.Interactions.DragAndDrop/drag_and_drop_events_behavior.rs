//! Port of `DragAndDropEventsBehavior.cs`.

use crate::routed_handlers::RoutedHandlers;
use crate::values::boxed_object;
use ferroui_base::input::{DragDrop, DragEventArgs};
use ferroui_base::interactivity::Interactive;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, BoxedValue, ElementRef, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StaticType, StyledElementImpl, StyledProperty, WeakRef,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::RefCell;

/// Base class for behaviors that respond to drag-and-drop events: the drag events of the
/// [`target_control`](Self::target_control), or else of the associated control, go to the members
/// of [`DragAndDropEventsBehaviorImpl`].
///
/// The class is abstract.
#[repr(C)]
pub struct DragAndDropEventsBehavior {
    base: StyledElementBehaviorOfT,
    // The handlers added to each control the events are attached to, in the order they were
    // attached (`AddHandler` and `RemoveHandler` of the reference).
    attached: RefCell<Vec<(WeakRef<Control>, RoutedHandlers)>>,
}

ferro_class! {
    DragAndDropEventsBehavior: StyledElementBehaviorOfT, virtuals DragAndDropEventsBehaviorImpl: StyledElementBehaviorImpl {
        /// Called when a drag-and-drop operation enters the element.
        fn on_drag_enter(this, sender: Option<&BoxedValue>, e: &DragEventArgs);
        /// Called when a drag-and-drop operation leaves the element.
        fn on_drag_leave(this, sender: Option<&BoxedValue>, e: &DragEventArgs);
        /// Called when a drag-and-drop operation is updated while over the element.
        fn on_drag_over(this, sender: Option<&BoxedValue>, e: &DragEventArgs);
        /// Called when a drag-and-drop operation should complete over the element.
        fn on_drop(this, sender: Option<&BoxedValue>, e: &DragEventArgs);
    }
}
ferro_class_info!(DragAndDropEventsBehavior {
    markup: {
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});

ferroui_base::ferro_impl_classes!(DragAndDropEventsBehavior: StyledElementImpl);

impl DragAndDropEventsBehaviorImpl for DragAndDropEventsBehavior {
    fn on_drag_enter(_this: &Self, _sender: Option<&BoxedValue>, _e: &DragEventArgs) {}

    fn on_drag_leave(_this: &Self, _sender: Option<&BoxedValue>, _e: &DragEventArgs) {}

    fn on_drag_over(_this: &Self, _sender: Option<&BoxedValue>, _e: &DragEventArgs) {}

    fn on_drop(_this: &Self, _sender: Option<&BoxedValue>, _e: &DragEventArgs) {}
}

ferro_properties! {
    impl DragAndDropEventsBehavior {
        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the control is held by an element reference (a
        /// weak reference), as the framework holds the elements a property names.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ferroui_base::data::core::ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<DragAndDropEventsBehavior, _>("TargetControl", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for DragAndDropEventsBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(target_control) = this.target_control().or_else(|| this.associated_object_as::<Control>()) else {
                return;
            };

            this.attach_events(&target_control);
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(target_control) = this.target_control().or_else(|| this.associated_object_as::<Control>()) else {
                return;
            };

            this.detach_events(&target_control);
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for DragAndDropEventsBehavior {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::target_control_property().as_property() {
                let old_value = ElementRef::resolve(&change.get_old_value::<Option<ElementRef<Control>>>().flatten());
                if let Some(old_value) = old_value {
                    this.detach_events(&old_value);
                }

                let new_value = ElementRef::resolve(&change.get_new_value::<Option<ElementRef<Control>>>());
                if let Some(new_value) = new_value {
                    this.attach_events(&new_value);
                }
            }
        }
    }
}

impl DragAndDropEventsBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE), attached: RefCell::new(Vec::new()) }
    }

    /// Gets or sets the control that is used as source for drag and drop events.
    pub fn target_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::target_control_property()))
    }

    pub fn set_target_control(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::target_control_property(), ElementRef::from_nullable(value))
    }

    fn attach_events(&self, target_control: &Ref<Control>) {
        DragDrop::set_allow_drop(target_control, true);

        let handlers = RoutedHandlers::new();
        let behavior = self.to_ref().downgrade();
        handlers.add(
            target_control,
            DragDrop::drag_enter_event(),
            move |sender: &Interactive, e: &DragEventArgs| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.on_drag_enter(Some(&boxed_object(&sender.to_ref())), e);
                }
            },
            Interactive::DEFAULT_ROUTES,
            false,
        );
        let behavior = self.to_ref().downgrade();
        handlers.add(
            target_control,
            DragDrop::drag_leave_event(),
            move |sender: &Interactive, e: &DragEventArgs| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.on_drag_leave(Some(&boxed_object(&sender.to_ref())), e);
                }
            },
            Interactive::DEFAULT_ROUTES,
            false,
        );
        let behavior = self.to_ref().downgrade();
        handlers.add(
            target_control,
            DragDrop::drag_over_event(),
            move |sender: &Interactive, e: &DragEventArgs| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.on_drag_over(Some(&boxed_object(&sender.to_ref())), e);
                }
            },
            Interactive::DEFAULT_ROUTES,
            false,
        );
        let behavior = self.to_ref().downgrade();
        handlers.add(
            target_control,
            DragDrop::drop_event(),
            move |sender: &Interactive, e: &DragEventArgs| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.on_drop(Some(&boxed_object(&sender.to_ref())), e);
                }
            },
            Interactive::DEFAULT_ROUTES,
            false,
        );

        self.attached.borrow_mut().push((target_control.downgrade(), handlers));
    }

    fn detach_events(&self, target_control: &Ref<Control>) {
        DragDrop::set_allow_drop(target_control, false);

        let detached = {
            let mut attached = self.attached.borrow_mut();
            attached
                .iter()
                .position(|(control, _)| control.upgrade().is_some_and(|control| control.ptr_eq(target_control)))
                .map(|index| attached.remove(index))
        };
        if let Some((_, handlers)) = detached {
            handlers.clear();
        }
    }
}
