//! Ported from `tests/Xaml.Behaviors.Interactivity.UnitTests/TestBehaviors.cs` of Xaml.Behaviors.

use crate::{Behavior, BehaviorImpl, BehaviorImplExt, BehaviorOfT};
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref, StaticType};
use ferroui_controls::Button;
use std::cell::Cell;

/// A behavior that counts the calls of each of its hooks.
#[repr(C)]
pub(crate) struct TestBehavior {
    base: Behavior,
    attached_called: Cell<i32>,
    detaching_called: Cell<i32>,
    visual_attach_called: Cell<i32>,
    visual_detach_called: Cell<i32>,
    logical_attach_called: Cell<i32>,
    logical_detach_called: Cell<i32>,
    loaded_called: Cell<i32>,
    unloaded_called: Cell<i32>,
    initialized_called: Cell<i32>,
    data_context_changed_called: Cell<i32>,
    resources_changed_called: Cell<i32>,
    theme_changed_called: Cell<i32>,
}

ferro_class!(TestBehavior: Behavior);
ferroui_base::ferro_impl_classes!(TestBehavior: FerroObjectImpl);

fn increment(counter: &Cell<i32>) {
    counter.set(counter.get() + 1);
}

impl BehaviorImpl for TestBehavior {
    fn on_attached(this: &Self) {
        increment(&this.attached_called);
    }

    fn on_detaching(this: &Self) {
        increment(&this.detaching_called);
    }

    fn on_attached_to_visual_tree(this: &Self) {
        increment(&this.visual_attach_called);
    }

    fn on_detached_from_visual_tree(this: &Self) {
        increment(&this.visual_detach_called);
    }

    fn on_attached_to_logical_tree_event(this: &Self) {
        increment(&this.logical_attach_called);
    }

    fn on_detached_from_logical_tree_event(this: &Self) {
        increment(&this.logical_detach_called);
    }

    fn on_loaded(this: &Self) {
        increment(&this.loaded_called);
    }

    fn on_unloaded(this: &Self) {
        increment(&this.unloaded_called);
    }

    fn on_initialized_event(this: &Self) {
        increment(&this.initialized_called);
    }

    fn on_data_context_changed_event(this: &Self) {
        increment(&this.data_context_changed_called);
    }

    fn on_resources_changed_event(this: &Self) {
        increment(&this.resources_changed_called);
    }

    fn on_actual_theme_variant_changed_event(this: &Self) {
        increment(&this.theme_changed_called);
    }
}

#[allow(dead_code)]
impl TestBehavior {
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self {
            base: Behavior::construct(),
            attached_called: Cell::new(0),
            detaching_called: Cell::new(0),
            visual_attach_called: Cell::new(0),
            visual_detach_called: Cell::new(0),
            logical_attach_called: Cell::new(0),
            logical_detach_called: Cell::new(0),
            loaded_called: Cell::new(0),
            unloaded_called: Cell::new(0),
            initialized_called: Cell::new(0),
            data_context_changed_called: Cell::new(0),
            resources_changed_called: Cell::new(0),
            theme_changed_called: Cell::new(0),
        })
    }

    pub(crate) fn attached_called(&self) -> i32 {
        self.attached_called.get()
    }

    pub(crate) fn detaching_called(&self) -> i32 {
        self.detaching_called.get()
    }

    pub(crate) fn visual_attach_called(&self) -> i32 {
        self.visual_attach_called.get()
    }

    pub(crate) fn visual_detach_called(&self) -> i32 {
        self.visual_detach_called.get()
    }

    pub(crate) fn logical_attach_called(&self) -> i32 {
        self.logical_attach_called.get()
    }

    pub(crate) fn logical_detach_called(&self) -> i32 {
        self.logical_detach_called.get()
    }

    pub(crate) fn loaded_called(&self) -> i32 {
        self.loaded_called.get()
    }

    pub(crate) fn unloaded_called(&self) -> i32 {
        self.unloaded_called.get()
    }

    pub(crate) fn initialized_called(&self) -> i32 {
        self.initialized_called.get()
    }

    pub(crate) fn data_context_changed_called(&self) -> i32 {
        self.data_context_changed_called.get()
    }

    pub(crate) fn resources_changed_called(&self) -> i32 {
        self.resources_changed_called.get()
    }

    pub(crate) fn theme_changed_called(&self) -> i32 {
        self.theme_changed_called.get()
    }
}

/// A behavior of buttons (`Behavior<Button>`) that counts its attachments and detachments.
#[repr(C)]
pub(crate) struct TestBehaviorOfT {
    base: BehaviorOfT,
    attached_called: Cell<i32>,
    detaching_called: Cell<i32>,
}

ferro_class!(TestBehaviorOfT: BehaviorOfT);
ferroui_base::ferro_impl_classes!(TestBehaviorOfT: FerroObjectImpl);

impl BehaviorImpl for TestBehaviorOfT {
    fn on_attached(this: &Self) {
        Self::parent_on_attached(this);
        increment(&this.attached_called);
    }

    fn on_detaching(this: &Self) {
        Self::parent_on_detaching(this);
        increment(&this.detaching_called);
    }
}

impl TestBehaviorOfT {
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self {
            base: BehaviorOfT::construct(<Button as StaticType>::TYPE),
            attached_called: Cell::new(0),
            detaching_called: Cell::new(0),
        })
    }

    pub(crate) fn attached_called(&self) -> i32 {
        self.attached_called.get()
    }

    pub(crate) fn detaching_called(&self) -> i32 {
        self.detaching_called.get()
    }
}
