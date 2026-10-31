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
