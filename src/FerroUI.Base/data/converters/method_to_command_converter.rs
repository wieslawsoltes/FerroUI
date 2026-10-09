use crate::data::core::plugins::InpcPropertyAccessor;
use crate::data::core::{ValueType, ValueTypes};
use crate::data::model::{Event, ICommand};
use crate::metadata::{attributes, MarkupAttributeValue, MarkupDelegate, MarkupMethod, MarkupValue};
use crate::reactive::{Disposable, IDisposable};
use crate::threading::{Dispatcher, DispatcherPriority};
use crate::BoxedValue;
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// A command made from the delegate of a method: what a string-path binding
/// of a method to a command property produces.
///
/// Executing the command calls the method, with the command parameter
/// converted to the type of its parameter if it has one. A method
/// `Can<Name>` with one "any value" parameter, declared by the type that
/// declares the method (or one of its base types), answers whether the
/// command can execute; the properties named by its `DependsOn` attributes
/// re-query it when the target of the delegate reports that they changed.
///
/// A failure of the method (its `Err`, or a command parameter that cannot be
/// converted to a parameter that takes no null) panics: a command has no
/// other way to report what the managed original throws from `Execute`.
///
/// The command has the target of the delegate as the delegate has it. The
/// delegate a binding reads from its source
/// ([`MarkupDelegate::for_method_of_source`]) does not keep the source
/// alive (DEVIATIONS.md, Bindings): once the source is gone the command
/// cannot execute and executing it does nothing, as the command of a
/// compiled binding path
/// ([`MethodCommand`](crate::data::core::expression_nodes::MethodCommand)).
pub struct MethodToCommandConverter {
    this: Weak<MethodToCommandConverter>,
    action: MarkupDelegate,
    /// The type of the parameter of the method; `None` without a parameter.
    parameter_type: Option<ValueType>,
    can_execute: Option<&'static MarkupMethod>,
    dependency_properties: Vec<&'static str>,
    token: Cell<Option<u64>>,
    can_execute_changed: Event<()>,
}

impl MethodToCommandConverter {
    /// Creates the command of `action`: the delegate of a declared method
    /// ([`MarkupDelegate::for_method`]). For any other delegate the command
    /// calls it with the command parameter and can always execute.
    pub fn new(action: &MarkupDelegate) -> Rc<Self> {
        let method = action.method();
        let parameter_type = method.and_then(|method| method.parameters.first()).map(|parameter| parameter());

        let can_execute = method.and_then(|method| {
            let name = format!("Can{}", method.name);
            let mut current = action.declaring_type();
            let mut depth = 0;
            while let Some(markup) = current {
                let found = markup
                    .find_methods(&name)
                    .find(|m| m.parameters.len() == 1 && m.parameters[0]().is_object() && m.is_static == method.is_static);
                if found.is_some() {
                    return found;
                }
                depth += 1;
                current = if depth < 64 { markup.base_type() } else { None };
            }
            None
        });
        let dependency_properties: Vec<&'static str> = can_execute
            .map(|can_execute| {
                can_execute
                    .attributes
                    .iter()
                    .filter(|attribute| attribute.name == attributes::DEPENDS_ON)
                    .filter_map(|attribute| match attribute.arguments.first() {
                        Some(MarkupAttributeValue::Str(name)) => Some(*name),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let this = Rc::new_cyclic(|this: &Weak<Self>| Self {
            this: this.clone(),
            action: action.clone(),
            parameter_type,
            can_execute,
            dependency_properties,
            token: Cell::new(None),
            can_execute_changed: Event::new(),
        });

        if !this.dependency_properties.is_empty() {
            let target = action.target();
            if let Some(inpc) = target.as_ref().and_then(|target| InpcPropertyAccessor::find_notifier(&**target)) {
                // The target does not keep the command alive.
                let weak = this.this.clone();
                let token = inpc.property_changed().add(Rc::new(move |name: &str| {
                    if let Some(this) = weak.upgrade() {
                        this.on_property_changed(name);
                    }
                }));
                this.token.set(Some(token));
            }
        }
        this
    }

    fn on_property_changed(&self, name: &str) {
        if name.trim().is_empty() || self.dependency_properties.contains(&name) {
            let weak = self.this.clone();
            Dispatcher::current_dispatcher().post_local(
                move || {
                    if let Some(this) = weak.upgrade() {
                        this.can_execute_changed.raise(&());
                    }
                },
                DispatcherPriority::INPUT,
            );
        }
    }

    fn arguments(&self, parameter: Option<&BoxedValue>) -> Vec<MarkupValue> {
        match self.parameter_type {
            None => Vec::new(),
            Some(type_) if type_.is_object() => vec![parameter.cloned()],
            // A parameter that cannot be converted is passed as null.
            Some(type_) => vec![ValueTypes::try_convert(parameter, type_).flatten()],
        }
    }
}

impl ICommand for MethodToCommandConverter {
    fn can_execute(&self, parameter: Option<&BoxedValue>) -> bool {
        if !self.action.is_target_alive() {
            return false;
        }
        let Some(can_execute) = self.can_execute else { return true };
        let result = match can_execute.is_static {
            true => (can_execute.invoke)(&[parameter.cloned()]),
            false => (can_execute.invoke)(&[self.action.target(), parameter.cloned()]),
        };
        match result {
            Ok(value) => crate::metadata::from_markup_value::<bool>(&value).unwrap_or(false),
            Err(error) => panic!("{}: {error}", can_execute.name),
        }
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        let Some(method) = self.action.method() else {
            self.action.invoke(&[parameter.cloned()]);
            return;
        };
        if !self.action.is_target_alive() {
            return;
        }
        if let Err(error) = self.action.try_invoke(&self.arguments(parameter)) {
            panic!("{}: {error}", method.name);
        }
    }

    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.can_execute_changed.add(Rc::new(move |_: &()| handler()));
        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.can_execute_changed.remove(token);
            }
        })
    }
}

impl Drop for MethodToCommandConverter {
    fn drop(&mut self) {
        if let Some(token) = self.token.take() {
            let target = self.action.target();
            if let Some(inpc) = target.as_ref().and_then(|target| InpcPropertyAccessor::find_notifier(&**target)) {
                inpc.property_changed().remove(token);
            }
        }
    }
}

/// The command of a delegate for a command-typed target: the conversion the
/// converters of string-path bindings apply before any other. `None` if
/// `target_type` is not the command contract or `value` is not the delegate
/// of a declared method with at most one parameter.
pub(crate) fn method_to_command(value: &BoxedValue, target_type: ValueType) -> Option<BoxedValue> {
    let nullable = target_type.is::<Option<Rc<dyn ICommand>>>();
    if !nullable && !target_type.is::<Rc<dyn ICommand>>() {
        return None;
    }
    let delegate = value.downcast_ref::<MarkupDelegate>()?;
    if delegate.method()?.parameters.len() > 1 {
        return None;
    }
    let command: Rc<dyn ICommand> = MethodToCommandConverter::new(delegate);
    Some(match nullable {
        true => Rc::new(Some(command)),
        false => Rc::new(command),
    })
}
