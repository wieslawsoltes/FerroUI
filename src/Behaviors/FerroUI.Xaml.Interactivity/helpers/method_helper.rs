//! Port of `Helpers/MethodHelper.cs`.

use super::member_lookup::{find_methods, object_of};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::metadata::MarkupMethod;
use ferroui_base::{BoxedValue, FerroObject, TypeInfo, WeakRef};
use std::any::TypeId;
use std::cell::RefCell;

/// The identity of the type of a target: the class of an object of the class model, or the Rust
/// type of any other value (`Type`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TargetType {
    Class(*const TypeInfo),
    Value(TypeId),
}

/// The target a method helper finds methods on, held without keeping an object alive.
enum TargetValue {
    Object(WeakRef<FerroObject>),
    Value(BoxedValue),
}

impl TargetValue {
    fn get(&self) -> Option<BoxedValue> {
        match self {
            Self::Object(object) => object.upgrade().map(|object| std::rc::Rc::new(object) as BoxedValue),
            Self::Value(value) => Some(value.clone()),
        }
    }
}

/// Finds the method a call-method action calls on its target: a public method of the name that
/// returns nothing (or a task) and takes no parameters, or two parameters of which the first is
/// an untyped value (the sender).
///
/// Deviation (DEVIATIONS.md, Behaviors): the methods are the methods of the markup metadata of
/// the type of the target (`GetRuntimeMethods()` of the reference); see
/// [`member_lookup`](super::member_lookup).
#[derive(Default)]
pub struct MethodHelper {
    cached_method_descriptor: RefCell<Option<MethodDescriptor>>,
    method_descriptors: RefCell<Vec<MethodDescriptor>>,
    target_object_type: RefCell<Option<TargetType>>,
    target: RefCell<Option<TargetValue>>,
}

impl MethodHelper {
    /// Creates a helper with no target.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the helper has a target type (`TargetObjectType is not null`).
    pub fn has_target_object_type(&self) -> bool {
        self.target_object_type.borrow().is_some()
    }

    /// Finds the method whose second parameter is closest to the type of `parameter`, or the
    /// parameterless method (or the one method that accepts null as its second parameter).
    pub fn find_best_method(&self, parameter: Option<&BoxedValue>) -> Option<MethodDescriptor> {
        let Some(parameter) = parameter else {
            return *self.cached_method_descriptor.borrow();
        };

        let parameter_type_info = ValueType::of_value(&**parameter);

        let mut most_derived_method: Option<MethodDescriptor> = None;

        // Loop over the methods looking for the one whose type is closest to the type of the given
        // parameter.
        for current_method in self.method_descriptors.borrow().iter() {
            let Some(current_type_info) = current_method.second_parameter_type_info() else {
                continue;
            };

            if ValueTypes::is_assignable(parameter_type_info, current_type_info) {
                let replaces = match &most_derived_method {
                    None => true,
                    Some(most_derived) => !most_derived
                        .second_parameter_type_info()
                        .is_some_and(|most_derived_type| ValueTypes::is_assignable(most_derived_type, current_type_info)),
                };
                if replaces {
                    most_derived_method = Some(*current_method);
                }
            }
        }

        most_derived_method.or(*self.cached_method_descriptor.borrow())
    }

    /// Sets the target the methods are found on, and finds the methods named `method_name` when
    /// the type of the target changed (`UpdateTargetType(target.GetType(), methodName)`).
    pub fn update_target_type(&self, target: &BoxedValue, method_name: Option<&str>) {
        let (new_target_type, new_target) = match object_of(target) {
            Some(object) => (TargetType::Class(object.get_type()), TargetValue::Object(object.to_weak())),
            None => (TargetType::Value(ValueType::of_value(&**target).id()), TargetValue::Value(target.clone())),
        };
        *self.target.borrow_mut() = Some(new_target);

        if Some(new_target_type) == *self.target_object_type.borrow() {
            return;
        }

        *self.target_object_type.borrow_mut() = Some(new_target_type);

        self.update_method_descriptors(method_name);
    }

    /// Finds the methods named `method_name` on the target.
    pub fn update_method_descriptors(&self, method_name: Option<&str>) {
        self.method_descriptors.borrow_mut().clear();
        *self.cached_method_descriptor.borrow_mut() = None;

        let target = self.target.borrow().as_ref().and_then(TargetValue::get);
        let (Some(method_name), Some(target)) = (method_name.filter(|name| !name.is_empty()), target) else {
            return;
        };

        // Find all public methods that match the given name and have either no parameters, or
        // two parameters where the first is an untyped value.
        let mut cached_method_descriptor: Option<MethodDescriptor> = None;
        let mut method_descriptors: Vec<MethodDescriptor> = Vec::new();
        for method in find_methods(&*target, method_name) {
            if method.is_static || !returns_nothing_or_task(method) {
                continue;
            }

            let parameters = method.parameters;
            if parameters.is_empty() {
                // There can be only one parameterless method of the given name.
                cached_method_descriptor = Some(MethodDescriptor::new(method));
            } else if parameters.len() == 2 && parameters[0]().is_object() {
                method_descriptors.push(MethodDescriptor::new(method));
            }
        }

        // We didn't find a parameterless method, so we want to find a method that accepts null as
        // a second parameter, but if we have more than one of these it is ambiguous which we
        // should call, so we do nothing.
        if cached_method_descriptor.is_none() {
            for method in &method_descriptors {
                if let Some(type_info) = method.second_parameter_type_info() {
                    if ValueTypes::accepts_null(type_info) {
                        if cached_method_descriptor.is_some() {
                            cached_method_descriptor = None;
                            break;
                        }

                        cached_method_descriptor = Some(*method);
                    }
                }
            }
        }

        *self.method_descriptors.borrow_mut() = method_descriptors;
        *self.cached_method_descriptor.borrow_mut() = cached_method_descriptor;
    }
}

/// Whether a method returns nothing or a task (`ReturnType == typeof(void) || ReturnType == typeof(Task)`).
fn returns_nothing_or_task(method: &MarkupMethod) -> bool {
    match method.return_type {
        None => true,
        Some(return_type) => {
            let name = return_type().name();
            name.contains("DispatcherTask") || name.ends_with("Task") || name.contains("Task<")
        }
    }
}

/// A method found by a [`MethodHelper`].
#[derive(Clone, Copy)]
pub struct MethodDescriptor {
    method_info: &'static MarkupMethod,
}

impl MethodDescriptor {
    /// Describes `method_info`.
    pub fn new(method_info: &'static MarkupMethod) -> Self {
        Self { method_info }
    }

    /// The method.
    pub fn method_info(&self) -> &'static MarkupMethod {
        self.method_info
    }

    /// The types of the parameters of the method.
    pub fn parameters(&self) -> Vec<ValueType> {
        self.method_info.parameters.iter().map(|parameter| parameter()).collect()
    }

    /// The number of parameters of the method.
    pub fn parameter_count(&self) -> usize {
        self.method_info.parameters.len()
    }

    /// The type of the second parameter, if the method has two.
    pub fn second_parameter_type_info(&self) -> Option<ValueType> {
        if self.parameter_count() < 2 {
            None
        } else {
            Some(self.method_info.parameters[1]())
        }
    }
}

impl std::fmt::Debug for MethodDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.method_info.name)
    }
}
