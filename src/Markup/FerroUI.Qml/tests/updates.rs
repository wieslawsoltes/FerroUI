use ferroui_qml::{ir::parse, Instance, TypeRegistry, Value};

#[test]
fn update_converts_strings_and_logical_assignments_skip_unneeded_evaluation_and_write() {
    let doc = parse(
        r#"
        import QtQml
        QtObject {
            property var x: "2"
            property int input: 5
            property int bound: input
            function change() {
                x++
                bound ||= missing()
                bound ??= missing()
                x &&= x + 2
            }
        }
    "#,
    )
    .unwrap();
    let mut instance = Instance::new(&doc, &TypeRegistry::new(), Default::default()).unwrap();
    let root = instance.root();
    instance.call(root, "change", vec![]).unwrap();
    assert_eq!(instance.get(root, "x").unwrap(), Value::Number(5.));
    instance.set(root, "input", Value::Number(7.)).unwrap();
    assert_eq!(instance.get(root, "bound").unwrap(), Value::Number(7.));
}

#[test]
fn loops_preserve_initializer_scope_and_propagate_nested_control_flow() {
    let doc = parse(
        r#"
        import QtQml
        QtObject {
            property int count: 0
            function execute() {
                let i = 100
                for (let i = 0; i < 8; ++i) {
                    if (i == 1) continue;
                    if (i == 5) break;
                    count += i
                }
                do { count += 1 } while (false);
                while (count < 15) { ++count }
                return i + count
            }
            function runaway() { count = 40; for (;;) {} }
            function early() { while (true) { for (;;) { return 3 } } }
        }
    "#,
    )
    .unwrap();
    let mut instance = Instance::new(&doc, &TypeRegistry::new(), Default::default()).unwrap();
    let root = instance.root();
    assert_eq!(
        instance.call(root, "execute", vec![]).unwrap(),
        Value::Number(115.)
    );
    assert_eq!(
        instance.call(root, "early", vec![]).unwrap(),
        Value::Number(3.)
    );
    assert!(instance.call(root, "runaway", vec![]).is_err());
    assert_eq!(instance.get(root, "count").unwrap(), Value::Number(15.));
    for bad in [
        "QtObject { function bad(){ break; } }",
        "QtObject { onReady: { continue; } }",
    ] {
        assert!(parse(bad).is_err());
    }
}
