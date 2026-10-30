//! Port of `Helpers/ReversiblePropertyChange.cs`.

use super::member_lookup::{object_of, property_value_type};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::BindingPriority;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{BoxedValue, FerroObject, FerroPropertyRegistry, PropertyValue, WeakRef};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

/// Reads the current value of the property: `Some(value)` when it is read (`TryGetValue(out value)`).
pub type TryGetValue<'a> = &'a dyn Fn() -> Option<Option<BoxedValue>>;

/// Sets the value of the property; false when it could not be set.
pub type Setter = Rc<dyn Fn(Option<&BoxedValue>) -> bool>;

/// Sets a temporary (animation priority) value of the property: `Some(reversion)` when it is set
/// (`TrySetTemporaryValue(value, out reversion)`).
pub type TrySetTemporaryValue = Rc<dyn Fn(Option<&BoxedValue>) -> Option<Rc<dyn IDisposable>>>;

struct PropertyFrame {
    value: RefCell<Option<BoxedValue>>,
    setter: RefCell<Setter>,
    temporary_setter: RefCell<TrySetTemporaryValue>,
    temporary_value: RefCell<Option<Rc<dyn IDisposable>>>,
}

struct PropertyStack {
    original_value: Option<BoxedValue>,
    original_setter: Setter,
    use_temporary_values: bool,
    frames: RefCell<Vec<Rc<PropertyFrame>>>,
}

/// The identity of a target (the key of the `ConditionalWeakTable` of the reference): an object
/// of the class model is held without keeping it alive; any other value is held, and compared by
/// identity.
enum TargetKey {
    Object(WeakRef<FerroObject>),
    Value(BoxedValue),
}

impl TargetKey {
    fn of(target: &BoxedValue) -> Self {
        match object_of(target) {
            Some(object) => Self::Object(object.to_weak()),
            None => Self::Value(target.clone()),
        }
    }

    fn is_alive(&self) -> bool {
        match self {
            Self::Object(object) => object.upgrade().is_some(),
            Self::Value(_) => true,
        }
    }

    /// Whether both keys are the same target (and it is alive).
    fn same(&self, other: &TargetKey) -> bool {
        self.is_alive()
            && match (self, other) {
                (Self::Object(a), Self::Object(b)) => a.ptr_eq(b),
                (Self::Value(a), Self::Value(b)) => ValueTypes::identity_equals(Some(a), Some(b)),
                _ => false,
            }
    }

    fn clone_key(&self) -> TargetKey {
        match self {
            Self::Object(object) => Self::Object(object.clone()),
            Self::Value(value) => Self::Value(value.clone()),
        }
    }
}

type PropertyStacks = Vec<(String, Rc<PropertyStack>)>;

ferroui_base::ferro_thread_local! {
    static PROPERTY_STACKS: RefCell<Vec<(TargetKey, PropertyStacks)>> = const { RefCell::new(Vec::new()) };
}

/// The index of the stacks of the property changes of a target, if it has any. Entries of
/// targets that are no longer alive are dropped.
fn stacks_of(target: &TargetKey) -> Option<usize> {
    PROPERTY_STACKS.with(|stacks| {
        let mut stacks = stacks.borrow_mut();
        stacks.retain(|(key, _)| key.is_alive());
        stacks.iter().position(|(key, _)| key.same(target))
    })
}

fn find_stack(target: &TargetKey, property_name: &str) -> Option<Rc<PropertyStack>> {
    let index = stacks_of(target)?;
    PROPERTY_STACKS.with(|stacks| {
        stacks.borrow()[index].1.iter().find(|(name, _)| name == property_name).map(|(_, stack)| stack.clone())
    })
}

fn add_stack(target: &TargetKey, property_name: &str, stack: Rc<PropertyStack>) {
    let index = stacks_of(target);
    PROPERTY_STACKS.with(|stacks| {
        let mut stacks = stacks.borrow_mut();
        match index {
            Some(index) => stacks[index].1.push((property_name.to_string(), stack)),
            None => stacks.push((target.clone_key(), vec![(property_name.to_string(), stack)])),
        }
    });
}

fn remove_stack(target: &TargetKey, property_name: &str) {
    let Some(index) = stacks_of(target) else {
        return;
    };
    PROPERTY_STACKS.with(|stacks| {
        let mut stacks = stacks.borrow_mut();
        stacks[index].1.retain(|(name, _)| name != property_name);
        if stacks[index].1.is_empty() {
            stacks.remove(index);
        }
    });
}

/// Coordinates reversible property changes of one property across action instances: the changes
/// of a property of a target form a stack, and reverting a change restores the value below it
/// (an internal class of the reference).
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference names this class and the typed public
/// class [`ReversiblePropertyChange`] alike (`ReversiblePropertyChange` and
/// `ReversiblePropertyChange<TTarget, TValue>`); one Rust name cannot be both.
pub struct UntypedReversiblePropertyChange {
    property_name: String,
    // Deviation (DEVIATIONS.md, Behaviors): a target that is an object is held weakly (the
    // reference holds it; an action that changed a property of its host would keep the host
    // alive).
    target: RefCell<Option<TargetKey>>,
    frame: RefCell<Option<Rc<PropertyFrame>>>,
}

impl UntypedReversiblePropertyChange {
    /// Creates the coordinator of the property `property_name`.
    ///
    /// # Panics
    /// Panics if the name is empty.
    pub fn new(property_name: &str) -> Self {
        assert!(!property_name.is_empty(), "The value cannot be an empty string. (Parameter 'propertyName')");
        Self { property_name: property_name.to_string(), target: RefCell::new(None), frame: RefCell::new(None) }
    }

    fn same_target(&self, target: &TargetKey) -> bool {
        self.target.borrow().as_ref().is_some_and(|current| current.same(target))
    }

    /// Applies `value` to the property of `target`, keeping what [`revert`](Self::revert) needs.
    /// Returns false when the value was not applied (a direct property, or a getter or setter
    /// that failed).
    pub fn apply(
        &self,
        target: &BoxedValue,
        value: Option<&BoxedValue>,
        getter: TryGetValue<'_>,
        setter: Setter,
        temporary_setter: TrySetTemporaryValue,
        is_direct_property: bool,
    ) -> bool {
        if is_direct_property {
            return false;
        }

        let target = &TargetKey::of(target);
        let current_frame = self.frame.borrow().clone();
        if let Some(frame) = current_frame.as_ref().filter(|_| self.same_target(target)) {
            if let Some(existing_stack) = find_stack(target, &self.property_name) {
                let frame_index = existing_stack.frames.borrow().iter().position(|f| Rc::ptr_eq(f, frame));
                if let Some(frame_index) = frame_index {
                    let is_top = frame_index == existing_stack.frames.borrow().len() - 1;
                    if is_top && !Self::try_apply_value(&existing_stack, frame, value, &setter, &temporary_setter) {
                        return false;
                    }

                    *frame.value.borrow_mut() = value.cloned();
                    *frame.setter.borrow_mut() = setter;
                    *frame.temporary_setter.borrow_mut() = temporary_setter;
                    return true;
                }
            }

            self.clear_state();
        } else if current_frame.is_some() && !self.revert() {
            return false;
        }

        let existing = find_stack(target, &self.property_name);
        let is_new_stack = existing.is_none();
        let mut temporary_value: Option<Rc<dyn IDisposable>> = None;

        let property_stack = match existing {
            None => {
                let Some(original_value) = getter() else {
                    return false;
                };

                let reversion = temporary_setter(value);
                let use_temporary_values = reversion.is_some();
                temporary_value = reversion;
                let property_stack = Rc::new(PropertyStack {
                    original_value,
                    original_setter: setter.clone(),
                    use_temporary_values,
                    frames: RefCell::new(Vec::new()),
                });
                if !use_temporary_values && !setter(value) {
                    return false;
                }
                property_stack
            }
            Some(property_stack) => {
                if property_stack.use_temporary_values {
                    match temporary_setter(value) {
                        Some(reversion) => temporary_value = Some(reversion),
                        None => return false,
                    }
                } else if !setter(value) {
                    return false;
                }
                property_stack
            }
        };

        let frame = Rc::new(PropertyFrame {
            value: RefCell::new(value.cloned()),
            setter: RefCell::new(setter),
            temporary_setter: RefCell::new(temporary_setter),
            temporary_value: RefCell::new(temporary_value),
        });
        property_stack.frames.borrow_mut().push(frame.clone());
        if is_new_stack {
            add_stack(target, &self.property_name, property_stack);
        }

        *self.target.borrow_mut() = Some(target.clone_key());
        *self.frame.borrow_mut() = Some(frame);
        true
    }

    /// Reverts the value this coordinator applied. Returns false when there is nothing to revert
    /// or the value below could not be restored.
    pub fn revert(&self) -> bool {
        let target = self.target.borrow().as_ref().map(TargetKey::clone_key);
        let frame = self.frame.borrow().clone();
        let (Some(target), Some(frame)) = (target, frame) else {
            return false;
        };
        let Some(property_stack) = find_stack(&target, &self.property_name) else {
            return false;
        };

        let frame_index = property_stack.frames.borrow().iter().position(|f| Rc::ptr_eq(f, &frame));
        let Some(frame_index) = frame_index else {
            self.clear_state();
            return false;
        };

        let is_top_frame = frame_index == property_stack.frames.borrow().len() - 1;
        if property_stack.use_temporary_values {
            let temporary_value = frame.temporary_value.borrow().clone();
            if let Some(temporary_value) = temporary_value {
                temporary_value.dispose();
            }
            if is_top_frame && frame_index > 0 {
                let preceding_frame = property_stack.frames.borrow()[frame_index - 1].clone();
                let temporary_setter = preceding_frame.temporary_setter.borrow().clone();
                let preceding_value = preceding_frame.value.borrow().clone();
                let Some(replacement) = temporary_setter(preceding_value.as_ref()) else {
                    return false;
                };

                let previous = preceding_frame.temporary_value.borrow_mut().replace(replacement);
                if let Some(previous) = previous {
                    previous.dispose();
                }
            }
        } else if is_top_frame {
            let restored = if frame_index > 0 {
                let preceding_frame = property_stack.frames.borrow()[frame_index - 1].clone();
                let setter = preceding_frame.setter.borrow().clone();
                let preceding_value = preceding_frame.value.borrow().clone();
                setter(preceding_value.as_ref())
            } else {
                (property_stack.original_setter)(property_stack.original_value.as_ref())
            };
            if !restored {
                return false;
            }
        }

        property_stack.frames.borrow_mut().remove(frame_index);
        if property_stack.frames.borrow().is_empty() {
            remove_stack(&target, &self.property_name);
        }

        self.clear_state();
        true
    }

    fn try_apply_value(
        property_stack: &PropertyStack,
        frame: &PropertyFrame,
        value: Option<&BoxedValue>,
        setter: &Setter,
        temporary_setter: &TrySetTemporaryValue,
    ) -> bool {
        if !property_stack.use_temporary_values {
            return setter(value);
        }

        let Some(replacement) = temporary_setter(value) else {
            return false;
        };

        let previous = frame.temporary_value.borrow_mut().replace(replacement);
        if let Some(previous) = previous {
            previous.dispose();
        }
        true
    }

    fn clear_state(&self) {
        *self.target.borrow_mut() = None;
        *self.frame.borrow_mut() = None;
    }
}

/// Coordinates reversible, typed property changes across action instances.
///
/// `TTarget` is the handle of the target (`Ref<Button>`, `Rc<ViewModel>`), `TValue` the type of
/// the value of the property.
pub struct ReversiblePropertyChange<TTarget, TValue> {
    property_name: String,
    change: UntypedReversiblePropertyChange,
    _marker: PhantomData<fn(TTarget, TValue)>,
}

impl<TTarget: Clone + PartialEq + 'static, TValue: PropertyValue> ReversiblePropertyChange<TTarget, TValue> {
    /// Initializes a new reversible property-change coordinator for the property `property_name`.
    ///
    /// # Panics
    /// Panics if the name is empty.
    pub fn new(property_name: &str) -> Self {
        assert!(!property_name.is_empty(), "The value cannot be an empty string. (Parameter 'propertyName')");
        Self {
            property_name: property_name.to_string(),
            change: UntypedReversiblePropertyChange::new(property_name),
            _marker: PhantomData,
        }
    }

    /// Applies a value while preserving the state required to revert it. `getter` gets the current
    /// property value, `setter` sets it. Returns true when the reversible value was applied.
    pub fn apply(
        &self,
        target: &TTarget,
        value: TValue,
        getter: impl Fn(&TTarget) -> TValue + 'static,
        setter: impl Fn(&TTarget, TValue) + 'static,
    ) -> bool {
        let target_object: BoxedValue = Rc::new(target.clone());
        let ferro_object = object_of(&target_object);
        let ferro_property = ferro_object
            .as_ref()
            .and_then(|object| FerroPropertyRegistry::instance().find_registered_for(object, &self.property_name));

        let try_get_value = {
            let target = target.clone();
            move || -> Option<Option<BoxedValue>> { Some(Some(Rc::new(getter(&target)) as BoxedValue)) }
        };

        let set_value: Setter = {
            let target = target.clone();
            let setter = Rc::new(setter);
            Rc::new(move |new_value: Option<&BoxedValue>| {
                match new_value.and_then(|new_value| new_value.downcast_ref::<TValue>()) {
                    Some(new_value) => {
                        setter(&target, new_value.clone());
                        true
                    }
                    None => false,
                }
            })
        };

        let set_temporary_value: TrySetTemporaryValue = {
            let object = ferro_object.as_ref().map(|object| object.to_weak());
            Rc::new(move |new_value: Option<&BoxedValue>| {
                let (Some(property), Some(object)) = (ferro_property, object.as_ref().and_then(WeakRef::upgrade)) else {
                    return None;
                };
                let converted = ValueTypes::try_convert(new_value, property_value_type(property)).flatten()?;
                object.set_value_untyped(property, converted.as_any(), BindingPriority::Animation)
            })
        };

        let value: BoxedValue = Rc::new(value);
        self.change.apply(
            &target_object,
            Some(&value),
            &try_get_value,
            set_value,
            set_temporary_value,
            ferro_property.is_some_and(|property| property.is_direct()),
        )
    }

    /// Reverts the value previously applied by this coordinator. Returns true when an applied
    /// value was reverted.
    pub fn revert(&self, _setter: impl Fn(&TTarget, TValue)) -> bool {
        self.change.revert()
    }

    /// The name of the property.
    pub fn property_name(&self) -> &str {
        &self.property_name
    }
}
