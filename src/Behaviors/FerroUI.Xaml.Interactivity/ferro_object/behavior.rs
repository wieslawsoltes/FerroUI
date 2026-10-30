//! Port of `FerroObject/Behavior.cs`.

use crate::{IBehavior, IBehaviorEventsHandler};
use ferroui_base::{
    ferro_class, ferro_properties, FerroObject, FerroObjectImpl, FerroProperty, Nullable, Ref, StyledProperty, WeakRef,
};
use std::cell::{Cell, RefCell};

/// A base class for behaviors, implementing the basic plumbing of [`IBehavior`].
///
/// The class is abstract: deriving classes override the members of [`BehaviorImpl`].
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference names the hooks of the attachment of the
/// host to the logical tree `OnAttachedToLogicalTree` / `OnDetachedFromLogicalTree`; the styled
/// element behaviors would hide the members of the same name of the styled element, so both
/// families name them [`on_attached_to_logical_tree_event`](Behavior::on_attached_to_logical_tree_event)
/// and [`on_detached_from_logical_tree_event`](Behavior::on_detached_from_logical_tree_event), as
/// the reference names the other host hooks (`OnInitializedEvent`, `OnDataContextChangedEvent`).
#[repr(C)]
pub struct Behavior {
    base: FerroObject,
    // Deviation (DEVIATIONS.md, Behaviors): the host is held weakly (the host holds its
    // behaviors through its behavior collection).
    associated_object: RefCell<Option<WeakRef<FerroObject>>>,
    is_attached_to_logical_tree: Cell<bool>,
    is_attached_to_visual_tree: Cell<bool>,
    is_initialized_notified: Cell<bool>,
    is_loaded: Cell<bool>,
}

ferro_class! {
    Behavior: FerroObject, virtuals BehaviorImpl: FerroObjectImpl {
        /// Called after the behavior is attached to the associated object.
        ///
        /// Override this to hook up functionality to the associated object.
        fn on_attached(this);
        /// Called when the behavior is being detached from its associated object.
        ///
        /// Override this to unhook functionality from the associated object.
        fn on_detaching(this);
        /// Called after the associated object is attached to the visual tree.
        ///
        /// Invoked only when the associated object is a visual.
        fn on_attached_to_visual_tree(this);
        /// Called when the associated object is being detached from the visual tree.
        ///
        /// Invoked only when the associated object is a visual.
        fn on_detached_from_visual_tree(this);
        /// Called after the associated object is attached to the logical tree
        /// (`OnAttachedToLogicalTree`).
        ///
        /// Invoked only when the associated object is a styled element.
        fn on_attached_to_logical_tree_event(this);
        /// Called when the associated object is being detached from the logical tree
        /// (`OnDetachedFromLogicalTree`).
        ///
        /// Invoked only when the associated object is a styled element.
        fn on_detached_from_logical_tree_event(this);
        /// Called after the associated object is loaded.
        ///
        /// Invoked only when the associated object is a control.
        fn on_loaded(this);
        /// Called when the associated object is unloaded.
        ///
        /// Invoked only when the associated object is a control.
        fn on_unloaded(this);
        /// Called when the associated object is initialized.
        ///
        /// Invoked only when the associated object is a styled element.
        fn on_initialized_event(this);
        /// Called when the data context of the associated object changed.
        ///
        /// Invoked only when the associated object is a styled element.
        fn on_data_context_changed_event(this);
        /// Called when the resources of the associated object changed.
        ///
        /// Invoked only when the associated object is a styled element.
        fn on_resources_changed_event(this);
        /// Called when the actual theme variant of the associated object changed.
        ///
        /// Invoked only when the associated object is a styled element.
        fn on_actual_theme_variant_changed_event(this);
    }
}

ferroui_base::ferro_impl_classes!(Behavior: FerroObjectImpl);

impl BehaviorImpl for Behavior {
    fn on_attached(_this: &Self) {}

    fn on_detaching(_this: &Self) {}

    fn on_attached_to_visual_tree(_this: &Self) {}

    fn on_detached_from_visual_tree(_this: &Self) {}

    fn on_attached_to_logical_tree_event(_this: &Self) {}

    fn on_detached_from_logical_tree_event(_this: &Self) {}

    fn on_loaded(_this: &Self) {}

    fn on_unloaded(_this: &Self) {}

    fn on_initialized_event(_this: &Self) {}

    fn on_data_context_changed_event(_this: &Self) {}

    fn on_resources_changed_event(_this: &Self) {}

    fn on_actual_theme_variant_changed_event(_this: &Self) {}
}

ferro_properties! {
    impl Behavior {
        /// Identifies the `IsEnabled` property.
        pub fn is_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<Behavior, _>("IsEnabled", true)
        }
    }
}

impl Behavior {
    /// The static constructor: the class implements the behavior contracts.
    fn static_constructor() {
        <dyn IBehavior>::register::<Behavior>();
        <dyn IBehaviorEventsHandler>::register::<Behavior>();
    }

    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            associated_object: RefCell::new(None),
            is_attached_to_logical_tree: Cell::new(false),
            is_attached_to_visual_tree: Cell::new(false),
            is_initialized_notified: Cell::new(false),
            is_loaded: Cell::new(false),
        }
    }

    /// Gets the object to which the behavior is attached.
    pub fn associated_object(&self) -> Option<Ref<FerroObject>> {
        self.associated_object.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// Gets or sets a value indicating whether this instance is enabled.
    pub fn is_enabled(&self) -> bool {
        self.get_value(Self::is_enabled_property())
    }

    pub fn set_is_enabled(&self, value: bool) {
        self.set_value(Self::is_enabled_property(), value)
    }

    /// Attaches the behavior to the specified object.
    ///
    /// # Panics
    /// Panics if the behavior is attached to another object.
    pub fn attach(&self, associated_object: impl Into<Nullable<FerroObject>>) {
        let associated_object = associated_object.into().0;
        let current = self.associated_object();
        if associated_object == current {
            return;
        }

        if current.is_some() {
            panic!("An instance of a behavior cannot be attached to more than one object at a time.");
        }

        debug_assert!(associated_object.is_some(), "Cannot attach the behavior to a null object.");
        let Some(associated_object) = associated_object else {
            panic!("Value cannot be null. (Parameter 'associatedObject')");
        };
        *self.associated_object.borrow_mut() = Some(associated_object.downgrade());

        self.on_attached();
    }

    /// Detaches the behaviors from the [`associated_object`](Self::associated_object).
    pub fn detach(&self) {
        self.on_detaching();
        self.is_attached_to_logical_tree.set(false);
        self.is_attached_to_visual_tree.set(false);
        self.is_initialized_notified.set(false);
        self.is_loaded.set(false);
        *self.associated_object.borrow_mut() = None;
    }
}

impl IBehavior for Behavior {
    fn associated_object(&self) -> Option<Ref<FerroObject>> {
        Behavior::associated_object(self)
    }

    fn attach(&self, associated_object: Option<Ref<FerroObject>>) {
        Behavior::attach(self, associated_object)
    }

    fn detach(&self) {
        Behavior::detach(self)
    }
}

impl IBehaviorEventsHandler for Behavior {
    fn attached_to_visual_tree_event_handler(&self) {
        if self.is_attached_to_visual_tree.get() {
            return;
        }

        self.is_attached_to_visual_tree.set(true);
        self.on_attached_to_visual_tree();
    }

    fn detached_from_visual_tree_event_handler(&self) {
        if !self.is_attached_to_visual_tree.get() {
            return;
        }

        self.is_attached_to_visual_tree.set(false);
        self.on_detached_from_visual_tree();
    }

    fn attached_to_logical_tree_event_handler(&self) {
        if self.is_attached_to_logical_tree.get() {
            return;
        }

        self.is_attached_to_logical_tree.set(true);
        self.on_attached_to_logical_tree_event();
    }

    fn detached_from_logical_tree_event_handler(&self) {
        if !self.is_attached_to_logical_tree.get() {
            return;
        }

        self.is_attached_to_logical_tree.set(false);
        self.on_detached_from_logical_tree_event();
    }

    fn loaded_event_handler(&self) {
        if self.is_loaded.get() {
            return;
        }

        self.is_loaded.set(true);
        self.on_loaded();
    }

    fn unloaded_event_handler(&self) {
        if !self.is_loaded.get() {
            return;
        }

        self.is_loaded.set(false);
        self.on_unloaded();
    }

    fn initialized_event_handler(&self) {
        if self.is_initialized_notified.get() {
            return;
        }

        self.is_initialized_notified.set(true);
        self.on_initialized_event();
    }

    fn data_context_changed_event_handler(&self) {
        self.on_data_context_changed_event();
    }

    fn resources_changed_event_handler(&self) {
        self.on_resources_changed_event();
    }

    fn actual_theme_variant_changed_event_handler(&self) {
        self.on_actual_theme_variant_changed_event();
    }
}
