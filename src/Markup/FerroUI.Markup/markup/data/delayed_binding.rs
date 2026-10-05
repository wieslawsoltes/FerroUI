use ferroui_base::data::core::IPropertyInfo;
use ferroui_base::data::{BindingBase, BindingError};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{BoxedValue, FerroObject, FerroProperty, Ref, StyledElement, WeakRef};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// Provides delayed bindings for elements.
///
/// The markup engine applies its bindings in a delayed manner: bindings are
/// only applied when an element has finished initializing. Applying bindings
/// as soon as elements are created would mean that long-form bindings
/// (binding objects whose properties are set one by one after creation)
/// don't work, as the binding would be applied to the property before the
/// properties of the binding object are set.
pub struct DelayedBinding;

enum Entry {
    Binding { property: &'static FerroProperty, binding: Rc<dyn BindingBase> },
    ClrPropertyValue { property: Rc<dyn IPropertyInfo>, value: Rc<dyn Fn(&StyledElement) -> Option<BoxedValue>> },
}

impl Entry {
    fn apply(&self, control: &StyledElement) {
        match self {
            Entry::Binding { property, binding } => {
                control.bind_binding(property, &**binding);
            }
            Entry::ClrPropertyValue { property, value } => {
                let target: Ref<FerroObject> = control.to_ref().upcast();
                if let Err(e) = property.set(&target, value(control).as_ref()) {
                    if let Some(log) = Logger::try_get(LogEventLevel::Error, LogArea::PROPERTY) {
                        let target_type = control.get_type().name();
                        log.log_with_values(
                            Some(control as &dyn Any),
                            "Error setting {Property} on {Target}: {Exception}",
                            &[&property.name(), &target_type, &e],
                        );
                    }
                }
            }
        }
    }
}

struct Pending {
    target: WeakRef<StyledElement>,
    entries: Vec<Entry>,
    subscription: Option<Rc<dyn IDisposable>>,
}

thread_local! {
    /// The pending entries per element. Elements are held weakly.
    static ENTRIES: RefCell<Vec<Pending>> = const { RefCell::new(Vec::new()) };
}

impl DelayedBinding {
    /// Adds a delayed binding to an element: the binding is applied to
    /// `property` when the element has finished initializing, or immediately
    /// if it already has.
    pub fn add(target: &StyledElement, property: &'static FerroProperty, binding: Rc<dyn BindingBase>) {
        if target.is_initialized() {
            target.bind_binding(property, &*binding);
        } else {
            Self::add_entry(target, Entry::Binding { property, binding });
        }
    }

    /// Adds a delayed value to an element: `value` is evaluated and written
    /// to `property` when the element has finished initializing, or
    /// immediately if it already has.
    ///
    /// When the value is written immediately, an error from the property
    /// setter is returned (the equivalent of the setter throwing to the
    /// caller); when it is written later, the error is logged.
    pub fn add_value(
        target: &StyledElement,
        property: Rc<dyn IPropertyInfo>,
        value: impl Fn(&StyledElement) -> Option<BoxedValue> + 'static,
    ) -> Result<(), BindingError> {
        if target.is_initialized() {
            let object: Ref<FerroObject> = target.to_ref().upcast();
            property.set(&object, value(target).as_ref())
        } else {
            Self::add_entry(target, Entry::ClrPropertyValue { property, value: Rc::new(value) });
            Ok(())
        }
    }

    fn add_entry(target: &StyledElement, entry: Entry) {
        let target_ref = target.to_ref();
        let exists = ENTRIES.with(|entries| {
            let mut entries = entries.borrow_mut();
            // Drop the entries of elements that no longer exist.
            entries.retain(|p| p.target.upgrade().is_some());
            match entries.iter_mut().find(|p| p.target.points_to(&target_ref)) {
                Some(pending) => {
                    pending.entries.push(entry);
                    None
                }
                None => Some(entry),
            }
        });

        if let Some(entry) = exists {
            ENTRIES.with(|entries| {
                entries.borrow_mut().push(Pending {
                    target: target_ref.downgrade(),
                    entries: vec![entry],
                    subscription: None,
                })
            });
            let weak = target_ref.downgrade();
            let subscription = target.initialized(move || {
                if let Some(target) = weak.upgrade() {
                    Self::apply_bindings(&target);
                }
            });
            ENTRIES.with(|entries| {
                if let Some(pending) = entries.borrow_mut().iter_mut().find(|p| p.target.points_to(&target_ref)) {
                    pending.subscription = Some(subscription);
                }
            });
        }
    }

    /// Applies any delayed bindings to an element.
    pub fn apply_bindings(control: &StyledElement) {
        let control_ref = control.to_ref();
        let pending = ENTRIES.with(|entries| {
            let mut entries = entries.borrow_mut();
            let index = entries.iter().position(|p| p.target.points_to(&control_ref))?;
            Some(entries.remove(index))
        });

        if let Some(pending) = pending {
            for entry in &pending.entries {
                entry.apply(control);
            }
            if let Some(subscription) = pending.subscription {
                subscription.dispose();
            }
        }
    }
}
