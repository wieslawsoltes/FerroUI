use ferroui_qml::{ir::parse,Instance,TypeRegistry,Value};

#[test]
fn update_converts_strings_and_logical_assignments_skip_unneeded_evaluation_and_write() {
    let doc=parse(r#"
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
    "#).unwrap();
    let mut instance=Instance::new(&doc,&TypeRegistry::new(),Default::default()).unwrap();
    let root=instance.root();
    instance.call(root,"change",vec![]).unwrap();
    assert_eq!(instance.get(root,"x").unwrap(),Value::Number(5.));
    instance.set(root,"input",Value::Number(7.)).unwrap();
    assert_eq!(instance.get(root,"bound").unwrap(),Value::Number(7.));
}
