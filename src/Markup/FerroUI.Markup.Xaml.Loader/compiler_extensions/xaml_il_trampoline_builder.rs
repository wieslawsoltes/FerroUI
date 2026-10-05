//! Port of `CompilerExtensions/XamlIlTrampolineBuilder.cs`, reduced to its data contract.
//!
//! Upstream the builder generates static "trampoline" methods with the fixed signatures a
//! command needs (`void (object target, object parameter)` and
//! `bool (object target, object parameter)`) that call a view model method, so that a method
//! that is not public can be bound as a command. There is no IL back end here: the two
//! `emit_*` methods return a description of the call a back end has to materialise (a closure
//! in emitted Rust, a method invoker at run time), and keep the upstream cache so that a
//! method gets one trampoline.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use xamlx::type_system::{IXamlMethod, IXamlType};

/// The parameter of a command's execute method.
#[derive(Clone)]
pub struct XamlIlCommandParameter {
    pub parameter_type: Rc<dyn IXamlType>,
    /// `!parameterType.Is("System", "Object")`: the command parameter is converted with
    /// `unbox.any parameterType` (a cast for a reference type, an unboxing for a value type;
    /// either throws on a mismatch). A `System.Object` parameter is passed as is.
    pub unbox_any: bool,
}

/// `void Trampoline(object target, object parameter)` for the execute method of a command.
///
/// # What a back end has to materialise (upstream IL)
///
/// 1. Load `target` and convert it to `method.declaring_type()`: `unbox` when
///    [`this_is_value_type`](Self::this_is_value_type), `castclass` otherwise.
/// 2. When the method has a parameter ([`parameter`](Self::parameter)): load `parameter` and,
///    when [`XamlIlCommandParameter::unbox_any`], convert it with `unbox.any`.
/// 3. Call [`method`](Self::method) and discard a result if it has one.
///
/// Rust runtime: the `execute` closure of `CompiledBindingPathBuilder::command` /
/// `notifying_command` (`Fn(&Owner, Option<&BoxedValue>)`).
pub struct XamlIlCommandExecuteTrampoline {
    /// `<declaring type fqn>+<method name>_<parameter count>!CommandExecuteTrampoline`.
    pub name: String,
    pub method: Rc<dyn IXamlMethod>,
    pub this_is_value_type: bool,
    /// The single parameter of the method; `None` for a method without parameters.
    pub parameter: Option<XamlIlCommandParameter>,
}

/// `bool Trampoline(object target, object parameter)` for the `Can<Name>(object)` method of a
/// command.
///
/// # What a back end has to materialise (upstream IL)
///
/// 1. Load `target` and convert it to `method.declaring_type()`: `unbox` when
///    [`this_is_value_type`](Self::this_is_value_type), `castclass` otherwise.
/// 2. Load `parameter` (the method takes `System.Object`).
/// 3. Tail-call [`method`](Self::method) and return its boolean result.
///
/// Rust runtime: the `can_execute` closure of `CompiledBindingPathBuilder::command` /
/// `notifying_command` (`Fn(&Owner, Option<&BoxedValue>) -> bool`).
pub struct XamlIlCommandCanExecuteTrampoline {
    /// `<declaring type fqn>+<method name>!CommandCanExecuteTrampoline`.
    pub name: String,
    pub method: Rc<dyn IXamlMethod>,
    pub this_is_value_type: bool,
}

/// `XamlIlTrampolineBuilder`. Upstream it is a configuration extra created with the type
/// builder of the trampolines' type; here it needs no builder:
/// `configuration.get_or_create_extra::<XamlIlTrampolineBuilder>()`.
#[derive(Default)]
pub struct XamlIlTrampolineBuilder {
    execute_trampolines: RefCell<HashMap<String, Rc<XamlIlCommandExecuteTrampoline>>>,
    can_execute_trampolines: RefCell<HashMap<String, Rc<XamlIlCommandCanExecuteTrampoline>>>,
}

impl XamlIlTrampolineBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// `EmitCommandExecuteTrampoline(context, executeMethod)`: the trampoline of an instance
    /// method with zero or one parameter (upstream asserts both in debug builds only). The same
    /// `Rc` is returned for the same trampoline name.
    pub fn emit_command_execute_trampoline(
        &self,
        execute_method: &Rc<dyn IXamlMethod>,
    ) -> Rc<XamlIlCommandExecuteTrampoline> {
        let parameters = execute_method.parameters();
        let method_name = format!(
            "{}+{}_{}!CommandExecuteTrampoline",
            execute_method.declaring_type().get_fqn(),
            execute_method.name(),
            parameters.len()
        );
        if let Some(method) = self.execute_trampolines.borrow().get(&method_name) {
            return method.clone();
        }

        let trampoline = Rc::new(XamlIlCommandExecuteTrampoline {
            name: method_name.clone(),
            this_is_value_type: execute_method.declaring_type().is_value_type(),
            parameter: parameters.first().map(|parameter_type| XamlIlCommandParameter {
                unbox_any: !parameter_type.is("System", "Object"),
                parameter_type: parameter_type.clone(),
            }),
            method: execute_method.clone(),
        });

        self.execute_trampolines
            .borrow_mut()
            .insert(method_name, trampoline.clone());
        trampoline
    }

    /// `EmitCommandCanExecuteTrampoline(context, canExecuteMethod)`: the trampoline of an
    /// instance method `bool Can<Name>(object)` (upstream asserts the shape in debug builds
    /// only). The same `Rc` is returned for the same trampoline name.
    pub fn emit_command_can_execute_trampoline(
        &self,
        can_execute_method: &Rc<dyn IXamlMethod>,
    ) -> Rc<XamlIlCommandCanExecuteTrampoline> {
        let method_name = format!(
            "{}+{}!CommandCanExecuteTrampoline",
            can_execute_method.declaring_type().get_fqn(),
            can_execute_method.name()
        );
        if let Some(method) = self.can_execute_trampolines.borrow().get(&method_name) {
            return method.clone();
        }

        let trampoline = Rc::new(XamlIlCommandCanExecuteTrampoline {
            name: method_name.clone(),
            this_is_value_type: can_execute_method.declaring_type().is_value_type(),
            method: can_execute_method.clone(),
        });

        self.can_execute_trampolines
            .borrow_mut()
            .insert(method_name, trampoline.clone());
        trampoline
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::bindings::create_bindings_test_framework;

    #[test]
    fn trampolines_are_named_after_the_method_and_cached() {
        let fw = create_bindings_test_framework();
        let size = fw.t("FerroUI.Size");
        let type_ = fw.fake_type("FerroUI.Size");
        let execute: Rc<dyn IXamlMethod> =
            type_.add_method("Grow", fw.t("System.Void"), vec![fw.t("System.Double")], false);
        let can_execute: Rc<dyn IXamlMethod> = type_.add_method(
            "CanGrow",
            fw.t("System.Boolean"),
            vec![fw.t("System.Object")],
            false,
        );
        assert!(size.is_value_type());

        let builder = XamlIlTrampolineBuilder::new();
        let trampoline = builder.emit_command_execute_trampoline(&execute);
        assert_eq!(
            trampoline.name,
            "FerroUI.Base:FerroUI.Size+Grow_1!CommandExecuteTrampoline"
        );
        // A value type receiver is unboxed, a value type parameter too.
        assert!(trampoline.this_is_value_type);
        let parameter = trampoline.parameter.as_ref().expect("parameter");
        assert!(parameter.unbox_any);
        assert_eq!(parameter.parameter_type.full_name(), "System.Double");
        assert!(Rc::ptr_eq(
            &trampoline,
            &builder.emit_command_execute_trampoline(&execute)
        ));

        let can = builder.emit_command_can_execute_trampoline(&can_execute);
        assert_eq!(
            can.name,
            "FerroUI.Base:FerroUI.Size+CanGrow!CommandCanExecuteTrampoline"
        );
        assert!(can.this_is_value_type);
        assert!(Rc::ptr_eq(
            &can,
            &builder.emit_command_can_execute_trampoline(&can_execute)
        ));
    }
}
