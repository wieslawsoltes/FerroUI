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
        assert_eq!(ferroui_qml::ComponentSource::document(&document).singletons.len(), 1);
        let registry = TypeRegistry::new();
        let mut first = Instance::new(&document, &registry, Default::default()).unwrap();
        let second = Instance::new(&document, &registry, Default::default()).unwrap();
        let root = first.root();
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
