//! Port of `StyledElement/StyledElementBehavior.cs`.

use crate::helpers::TemplatedParentHelper;
use crate::{DisposableAction, IBehavior, IBehaviorEventsHandler};
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_properties, BoxedValue, ElementRef, FerroObject, FerroObjectExtensions, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElement, StyledElementImpl, StyledProperty, WeakRef,
};
use ferroui_controls::primitives::FlyoutBase;
use ferroui_controls::{Control, TopLevel};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A base class for behaviors, implementing the basic plumbing of [`IBehavior`].
///
/// The behavior is a styled element: while its host is in the logical tree, the behavior is a
/// logical child of the host (one way: it is not in the logical children of the host), so it is
/// styled, resolves resources, follows the data context of its host and binds with `$parent`
/// and `TemplateBinding`.
///
/// The class is abstract: deriving classes override the members of [`StyledElementBehaviorImpl`].
/// See [`Behavior`](crate::Behavior) for the names of the hooks of the logical tree.
#[repr(C)]
pub struct StyledElementBehavior {
    base: StyledElement,
    data_context_disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    // Deviation (DEVIATIONS.md, Behaviors): the host is held weakly (the host holds its
    // behaviors through its behavior collection).
    associated_object: RefCell<Option<WeakRef<FerroObject>>>,
    is_attached_to_logical_tree: Cell<bool>,
    is_attached_to_visual_tree: Cell<bool>,
    is_initialized_notified: Cell<bool>,
    is_loaded: Cell<bool>,
}

ferro_class! {
    StyledElementBehavior: StyledElement, virtuals StyledElementBehaviorImpl: StyledElementImpl {
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
        /// Initializes the behavior (an internal member of the reference).
        #[doc(hidden)]
        fn initialize(this);
        /// Makes the behavior a logical child of its host (an internal member of the reference).
        #[doc(hidden)]
        fn attach_behavior_to_logical_tree(this);
        /// Removes the behavior from the logical tree (an internal member of the reference).
        #[doc(hidden)]
        fn detach_behavior_from_logical_tree(this);
    }
}

ferroui_base::ferro_impl_classes!(StyledElementBehavior: FerroObjectImpl, StyledElementImpl);

impl StyledElementBehaviorImpl for StyledElementBehavior {
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

    fn initialize(this: &Self) {
        this.initialize_if_needed();
    }

    fn attach_behavior_to_logical_tree(this: &Self) {
        let mut parent: Option<Ref<StyledElement>> = None;
        let mut templated_parent: Option<Ref<FerroObject>> = None;

        if let Some(associated_object) = this.associated_object() {
            if let Some(top_level) = associated_object.cast::<TopLevel>() {
                templated_parent = top_level.templated_parent();
                parent = Some(top_level.upcast());
            } else if let Some(flyout) = associated_object.downcast_ref::<FlyoutBase>() {
                if let Some(target) = flyout.target() {
                    templated_parent = target.templated_parent();
                    parent = Some(target.upcast());
                }
            } else if let Some(styled_element) = associated_object.cast::<StyledElement>() {
                if styled_element.parent().is_some() {
                    templated_parent = styled_element.templated_parent();
                    parent = Some(styled_element);
                }
            }
        }

        let Some(parent) = parent else {
            return;
        };

        // Required for $parent binding in markup.
        this.set_parent(&parent);

        // Required for TemplateBinding in markup.
        if templated_parent.is_some() {
            TemplatedParentHelper::set_templated_parent(this, templated_parent);
        }
    }

    fn detach_behavior_from_logical_tree(this: &Self) {
        this.set_parent(None);

        if this.templated_parent().is_some() {
            TemplatedParentHelper::set_templated_parent(this, None);
        }
    }
}

ferro_properties! {
    impl StyledElementBehavior {
        /// Identifies the `IsEnabled` property.
        pub fn is_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<StyledElementBehavior, _>("IsEnabled", true)
        }
    }
}

/// Removes the behavior from the logical tree when dropped, unless its host is a top level: the
/// `finally` of the detach handlers of the reference, which runs when the hook panics too.
struct DetachFromLogicalTree<'a>(&'a StyledElementBehavior);

impl Drop for DetachFromLogicalTree<'_> {
    fn drop(&mut self) {
        if !self.0.associated_object().is_some_and(|associated_object| associated_object.is::<TopLevel>()) {
            self.0.detach_behavior_from_logical_tree();
        }
    }
}

impl StyledElementBehavior {
    /// The static constructor: the class implements the behavior contracts.
    fn static_constructor() {
        <dyn IBehavior>::register::<StyledElementBehavior>();
        <dyn IBehaviorEventsHandler>::register::<StyledElementBehavior>();
    }

    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElement::construct(),
            data_context_disposable: RefCell::new(None),
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

    /// Gets the styled element to which this behavior is attached.
    pub fn associated_styled_element(&self) -> Option<Ref<StyledElement>> {
        self.associated_object().and_then(|associated_object| associated_object.cast::<StyledElement>())
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
        let data_context_disposable = self.synchronize_data_context(&associated_object);
        *self.data_context_disposable.borrow_mut() = data_context_disposable;

        // NOTE: Special case handling for TopLevel as it does not trigger attached to logical or
        // visual tree events.
        if associated_object.is::<TopLevel>() {
            self.attach_behavior_to_logical_tree();
        }

        self.on_attached();
    }

    /// Detaches the behaviors from the [`associated_object`](Self::associated_object).
    pub fn detach(&self) {
        self.on_detaching();

        if self.parent().is_some() || self.templated_parent().is_some() {
            self.detach_behavior_from_logical_tree();
        }

        let data_context_disposable = self.data_context_disposable.borrow_mut().take();
        if let Some(data_context_disposable) = data_context_disposable {
            data_context_disposable.dispose();
        }
        self.is_attached_to_logical_tree.set(false);
        self.is_attached_to_visual_tree.set(false);
        self.is_initialized_notified.set(false);
        self.is_loaded.set(false);
        *self.associated_object.borrow_mut() = None;
    }

    fn synchronize_data_context(&self, associated_object: &Ref<FerroObject>) -> Option<Rc<dyn IDisposable>> {
        let this = self.to_ref().downgrade();
        let data_context_property = StyledElement::data_context_property();

        if let Some(styled_element) = associated_object.downcast_ref::<StyledElement>() {
            // Setting the initial data context value immediately, so that bindings are
            // available before the first data context change, is disabled in the reference
            // (`#if false`); the subscription below sets it on subscription.

            // Required for data context binding in markup.
            return Some(styled_element.get_observable(data_context_property).subscribe_fn(
                move |x: Option<BoxedValue>| {
                    if let Some(this) = this.upgrade() {
                        this.set_current_value(data_context_property, x);
                    }
                },
            ));
        }

        if let Some(flyout) = associated_object.downcast_ref::<FlyoutBase>() {
            let target_data_context_subscription: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));

            let subscription = target_data_context_subscription.clone();
            let target_changed = move |target: Option<Ref<Control>>| {
                let Some(this) = this.upgrade() else {
                    return;
                };

                let previous = subscription.borrow_mut().take();
                if let Some(previous) = previous {
                    previous.dispose();
                }

                if this.parent().is_some() || this.templated_parent().is_some() {
                    this.detach_behavior_from_logical_tree();
                }

                let Some(target) = target else {
                    this.set_current_value(data_context_property, None);
                    return;
                };

                this.attach_behavior_to_logical_tree();
                let behavior = this.downgrade();
                let target_object: &FerroObject = &target;
                let target_subscription = FerroObjectExtensions::get_observable(target_object, data_context_property)
                    .subscribe_fn(move |x: Option<BoxedValue>| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.set_current_value(data_context_property, x);
                        }
                    });
                *subscription.borrow_mut() = Some(target_subscription);
            };

            let target_subscription = flyout
                .get_observable(FlyoutBase::target_property())
                .subscribe_fn(move |target: Option<ElementRef<Control>>| target_changed(ElementRef::resolve(&target)));

            return Some(DisposableAction::create(move || {
                target_subscription.dispose();
                let target_data_context = target_data_context_subscription.borrow().clone();
                if let Some(target_data_context) = target_data_context {
                    target_data_context.dispose();
                }
            }));
        }

        None
    }
}

impl IBehavior for StyledElementBehavior {
    fn associated_object(&self) -> Option<Ref<FerroObject>> {
        StyledElementBehavior::associated_object(self)
    }

    fn attach(&self, associated_object: Option<Ref<FerroObject>>) {
        StyledElementBehavior::attach(self, associated_object)
    }

    fn detach(&self) {
        StyledElementBehavior::detach(self)
    }
}

impl IBehaviorEventsHandler for StyledElementBehavior {
    fn attached_to_visual_tree_event_handler(&self) {
        if self.is_attached_to_visual_tree.get() {
            return;
        }

        self.is_attached_to_visual_tree.set(true);
        self.attach_behavior_to_logical_tree();

        self.on_attached_to_visual_tree();
    }

    fn detached_from_visual_tree_event_handler(&self) {
        if !self.is_attached_to_visual_tree.get() {
            return;
        }

        self.is_attached_to_visual_tree.set(false);
        let _finally = DetachFromLogicalTree(self);
        self.on_detached_from_visual_tree();
    }

    fn attached_to_logical_tree_event_handler(&self) {
        if self.is_attached_to_logical_tree.get() {
            return;
        }

        self.is_attached_to_logical_tree.set(true);
        self.attach_behavior_to_logical_tree();

        self.on_attached_to_logical_tree_event();
    }

    fn detached_from_logical_tree_event_handler(&self) {
        if !self.is_attached_to_logical_tree.get() {
            return;
        }

        self.is_attached_to_logical_tree.set(false);
        let _finally = DetachFromLogicalTree(self);
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
        self.initialize();

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
