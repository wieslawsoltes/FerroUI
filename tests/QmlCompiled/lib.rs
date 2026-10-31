#[cfg(test)]
mod tests {
    use ferroui_qml::{Instance, TypeRegistry, Value};
    include!(concat!(env!("OUT_DIR"), "/component.rs"));

    #[test]
    fn rust_compiled_component_links_typed_instances_with_private_ids() {
        let mut instance =
            Instance::new(&compiled_qml(), &TypeRegistry::new(), Default::default()).unwrap();
        let root = instance.root();
        let left = instance.id("left").unwrap();
        let right = instance.id("right").unwrap();
        assert_eq!(instance.get(root, "first").unwrap(), Value::Object(left));
        assert_eq!(
            instance.get(root, "caption").unwrap(),
            Value::String("Count: 17".into())
        );
        assert!(instance.id("privateModel").is_none());
        assert_eq!(
            instance
                .call(left, "increment", vec![Value::Number(4.)])
                .unwrap(),
            Value::Number(7.)
        );
        assert_eq!(instance.get(root, "sum").unwrap(), Value::Number(25.));
        assert_eq!(instance.get(right, "count").unwrap(), Value::Number(11.));
        assert!(instance.set(root, "first", Value::Object(right)).is_err());
        assert!(instance.set(root, "second", Value::Object(left)).is_ok());
    }
}

#[cfg(test)]
mod singleton_tests {
    use ferroui_qml::{Instance, TypeRegistry, Value};
    include!(concat!(env!("OUT_DIR"), "/singletons.rs"));

    #[test]
    fn source_singleton_is_shared_by_components_and_isolated_between_engines() {
        let document = compiled_qml();
        assert_eq!(
            ferroui_qml::ComponentSource::document(&document)
                .singletons
                .len(),
            1
        );
        let registry = TypeRegistry::new();
        let mut first = Instance::new(&document, &registry, Default::default()).unwrap();
        let second = Instance::new(&document, &registry, Default::default()).unwrap();
        let root = first.root();
        assert_eq!(first.get(root, "contrast").unwrap(), Value::Number(3.));
        let Value::Object(state) = first.get(root, "state").unwrap() else {
            panic!("missing state");
        };
        let Value::Object(consumer) = first.get(root, "first").unwrap() else {
            panic!("missing consumer");
        };
        assert_eq!(first.parent(state), None);
        first.call(consumer, "tick", vec![]).unwrap();
        assert_eq!(first.get(root, "total").unwrap(), Value::Number(9.));
        assert_eq!(
            second.get(second.root(), "total").unwrap(),
            Value::Number(6.)
        );
    }
}

#[cfg(test)]
mod scripts {
    use ferroui_qml::{ir, ComponentSource, Instance, TypeRegistry, Value};
    include!(concat!(env!("OUT_DIR"), "/scripts.rs"));

    fn run(source: &impl ComponentSource) -> Vec<Value> {
        let mut vm = Instance::new(source, &TypeRegistry::new(), Default::default()).unwrap();
        let root = vm.root();
        let mut observed = vec![vm.get(root, "reading").unwrap()];
        let n = Value::Number;
        vm.emit(root, "calculate", vec![n(9.)]).unwrap();
        observed.push(vm.get(root, "amount").unwrap());
        observed.push(vm.get(root, "reading").unwrap());
        observed.push(vm.call(root, "fold", vec![n(3.)]).unwrap());
        observed.push(vm.call(root, "fold", vec![n(8.)]).unwrap());
        observed.push(vm.call(root, "lazy", vec![]).unwrap());
        assert!(vm.call(root, "fail", vec![]).is_err());
        observed.push(vm.get(root, "amount").unwrap());
        vm.set(root, "active", Value::Bool(false)).unwrap();
        observed.push(vm.get(root, "reading").unwrap());
        observed
    }

    #[test]
    fn native_and_source_scripts_agree_on_control_flow_and_transaction_rollback() {
        let compiled = compiled_qml();
        assert!(!compiled.scripts().is_empty());
        assert!(!compiled.evaluators().is_empty());
        let source = ir::parse(include_str!("scripts/ControlFlow.qml")).unwrap();
        let actual = run(&compiled);
        assert_eq!(actual, run(&source));
        let n = Value::Number;
        assert_eq!(
            actual,
            vec![
                n(4.),
                n(8.),
                n(8.),
                n(9.),
                n(-1.),
                Value::Bool(true),
                n(8.),
                n(7.)
            ]
        );
    }
}

#[cfg(test)]
#[path = "reference/fixture.rs"]
mod qt_reference;

#[test]
fn both_backends_match_qt_6_12_behaviors() {
    qt_reference::run(|mode, case, actual, expected| assert_eq!(actual, expected, "{mode}/{case}"));
}
