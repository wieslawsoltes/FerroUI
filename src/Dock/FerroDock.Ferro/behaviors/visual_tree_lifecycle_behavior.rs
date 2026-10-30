// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::services::IVisualTreeLifecycle;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_properties, AttachedProperty, BoxedValue, FerroProperty, FerroPropertyChangedEventArgs,
    StyledElement, WeakRef,
};
use ferroui_controls::Control;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Invokes the original lifecycle hooks on controls and their DataContexts.
pub struct VisualTreeLifecycleBehavior;
ferroui_base::ferro_static_type!(VisualTreeLifecycleBehavior);
ferroui_base::ferro_markup_type!(static VisualTreeLifecycleBehavior {type_info:VisualTreeLifecycleBehavior});
ferro_properties! {impl VisualTreeLifecycleBehavior {
    /// Identifies the source IsEnabled attached property, false by default.
    pub fn is_enabled_property()->AttachedProperty<bool> {
        FerroProperty::register_attached::<VisualTreeLifecycleBehavior,Control,_>("IsEnabled",false)
    }
}}
struct Subscriptions {
    attached: Rc<dyn IDisposable>,
    detached: Rc<dyn IDisposable>,
}
struct Entry {
    control: WeakRef<Control>,
    subscriptions: Vec<Subscriptions>,
}
// Native events remove subscription tokens, whereas C# removes a static delegate.
// This private weak table retains those tokens without adding observable control
// properties or changing class-handler order. It never owns a control/ViewModel.
ferroui_base::ferro_thread_local! {
    static SUBSCRIPTIONS:RefCell<HashMap<usize,Entry>>=RefCell::new(HashMap::new());
}
impl VisualTreeLifecycleBehavior {
    fn static_constructor() {
        Self::is_enabled_property()
            .changed()
            .add_class_handler::<Control>(Self::on_is_enabled_changed);
        StyledElement::data_context_property()
            .changed()
            .add_class_handler::<Control>(Self::on_data_context_changed);
    }
    /// Gets whether the source lifecycle behavior is enabled on the control.
    pub fn get_is_enabled(element: &Control) -> bool {
        element.get_value(Self::is_enabled_property())
    }
    /// Enables or disables the source hooks, including an immediate attached-tree transition.
    pub fn set_is_enabled(element: &Control, value: bool) {
        element.set_value(Self::is_enabled_property(), value);
    }
    fn on_is_enabled_changed(element: &Control, e: &FerroPropertyChangedEventArgs<'_>) {
        let Some(&enabled) = e.new_value().downcast_ref::<bool>() else {
            return;
        };
        if enabled {
            let weak = element.to_ref().downgrade();
            let attached = element.attached_to_visual_tree(move |_| {
                if let Some(element) = weak.upgrade() {
                    Self::invoke_attached(&element);
                }
            });
            let weak = element.to_ref().downgrade();
            let detached = element.detached_from_visual_tree(move |_| {
                if let Some(element) = weak.upgrade() {
                    Self::invoke_detached(&element);
                }
            });
            SUBSCRIPTIONS.with(|store| {
                let mut table = store.borrow_mut();
                table.retain(|_, entry| entry.control.upgrade().is_some());
                table
                    .entry(std::ptr::from_ref(element) as usize)
                    .or_insert_with(|| Entry {
                        control: element.to_ref().downgrade(),
                        subscriptions: Vec::new(),
                    })
                    .subscriptions
                    .push(Subscriptions { attached, detached });
            });
            if element.is_attached_to_visual_tree() {
                Self::invoke_attached(element);
            }
        } else {
            let removed = SUBSCRIPTIONS.with(|store| {
                let mut table = store.borrow_mut();
                table.retain(|_, entry| entry.control.upgrade().is_some());
                let key = std::ptr::from_ref(element) as usize;
                let entry = table.get_mut(&key)?;
                // Delegate -= removes the last matching registration, not all
                // registrations if native property notifications repeated one.
                let pair = entry.subscriptions.pop();
                if entry.subscriptions.is_empty() {
                    table.remove(&key);
                }
                pair
            });
            if let Some(pair) = removed {
                pair.attached.dispose();
                pair.detached.dispose();
            }
            if element.is_attached_to_visual_tree() {
                Self::invoke_detached(element);
            }
        }
    }
    fn on_data_context_changed(element: &Control, e: &FerroPropertyChangedEventArgs<'_>) {
        if !Self::get_is_enabled(element) || !element.is_attached_to_visual_tree() {
            return;
        }
        if let Some(old) = e
            .old_value()
            .and_then(|v| v.downcast_ref::<Option<BoxedValue>>())
            .and_then(Option::as_ref)
            .and_then(lifecycle)
        {
            old.on_detached_from_visual_tree();
        }
        // Use the event's new value after the old callback. Nested context
        // changes must not replace the value of this original notification.
        if let Some(new) = e
            .new_value()
            .downcast_ref::<Option<BoxedValue>>()
            .and_then(Option::as_ref)
            .and_then(lifecycle)
        {
            new.on_attached_to_visual_tree();
        }
    }
    fn invoke_attached(element: &Control) {
        let own = lifecycle(&Control::boxed(element.to_ref()));
        if let Some(own) = &own {
            own.on_attached_to_visual_tree();
        }
        // The element hook can change DataContext: preserve this live lookup.
        if let Some(context) = element.data_context() {
            if let Some(model) = lifecycle(&context) {
                if !same_element(element, &context, own.as_ref(), &model) {
                    model.on_attached_to_visual_tree();
                }
            }
        }
    }
    fn invoke_detached(element: &Control) {
        if let Some(context) = element.data_context() {
            if let Some(model) = lifecycle(&context) {
                let own = lifecycle(&Control::boxed(element.to_ref()));
                if !same_element(element, &context, own.as_ref(), &model) {
                    model.on_detached_from_visual_tree();
                }
            }
        }
        // Query the control after its context hook, retaining source callback order.
        if let Some(own) = lifecycle(&Control::boxed(element.to_ref())) {
            own.on_detached_from_visual_tree();
        }
    }
}
fn lifecycle(value: &BoxedValue) -> Option<Rc<dyn IVisualTreeLifecycle>> {
    if let Some(value) = value.downcast_ref::<Rc<dyn IVisualTreeLifecycle>>() {
        return Some(value.clone());
    }
    ValueTypes::try_cast(value, ValueType::of::<Rc<dyn IVisualTreeLifecycle>>())?
        .downcast_ref::<Rc<dyn IVisualTreeLifecycle>>()
        .cloned()
}
fn same_element(
    element: &Control,
    context: &BoxedValue,
    own: Option<&Rc<dyn IVisualTreeLifecycle>>,
    model: &Rc<dyn IVisualTreeLifecycle>,
) -> bool {
    ValueTypes::as_object(&**context).is_some_and(|object| object.ptr_eq(&element.to_ref()))
        || own.is_some_and(|own| Rc::ptr_eq(own, model))
}
