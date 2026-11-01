use ferroui_qml::{ir, Instance, TypeRegistry, Value};

#[test]
fn forward_chains_use_an_explicit_stack_and_opaque_calls_fail_with_a_diagnostic() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let count = 1000;
            let chain = (0..count)
                .map(|i| format!("property int p{i}: p{} + 1;\n", i + 1))
                .collect::<String>();
            let source = format!("import QtQml\nQtObject {{ {chain} property int p{count}: 1 }}");
            let doc = ir::parse(&source).unwrap();
            let mut vm = Instance::new(&doc, &TypeRegistry::new(), Default::default()).unwrap();
            assert_eq!(vm.get(vm.root(), "p0").unwrap(), Value::Number(1001.));
            vm.set(vm.root(), "p1000", Value::Number(2.)).unwrap();
            assert_eq!(vm.get(vm.root(), "p0").unwrap(), Value::Number(1002.));
            let chain = (0..100)
                .map(|i| {
                    format!(
                        "property int p{i}: f{i}(); function f{i}(): int {{ return p{} }}\n",
                        i + 1
                    )
                })
                .collect::<String>();
            let doc = ir::parse(&format!(
                "import QtQml\nQtObject {{ {chain} property int p100: 1 }}"
            ))
            .unwrap();
            let result = Instance::new(&doc, &TypeRegistry::new(), Default::default());
            assert!(matches!(result, Err(e) if e.message.contains("stack limit")));
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn mandatory_read_hint_does_not_schedule_an_id_as_its_shadowed_property() {
    let doc = ir::parse("import QtQml\nQtObject { id: same; property var same: same }").unwrap();
    let vm = Instance::new(&doc, &TypeRegistry::new(), Default::default()).unwrap();
    assert_eq!(vm.get(vm.root(), "same").unwrap(), Value::Object(vm.root()));
}
