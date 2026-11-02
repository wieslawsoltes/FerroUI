use ferroui_qml::{
    ir, ComponentSource, Error, Instance, ScriptContext, ScriptEngine, TypeDefinition,
    TypeRegistry, Value,
};
use std::{cell::RefCell, path::Path, rc::Rc};

include!(concat!(env!("OUT_DIR"), "/direct_reads.rs"));

#[derive(Default)]
struct Guest(RefCell<Vec<String>>);

impl ScriptEngine for Guest {
    fn evaluate(
        &self,
        _: &mut ScriptContext<'_, '_>,
        script: &ir::JavaScript,
    ) -> Result<Value, Error> {
        self.0.borrow_mut().push(script.source.clone());
        match script.source.as_str() {
            "Math.PI" | "dynamic.width" => Ok(Value::Number(3.)),
            "this.operation" => Ok(Value::Undefined),
            other => Err(Error::new(format!(
                "unexpected guest evaluation of {other}"
            ))),
        }
    }
}

fn exercise(source: &impl ComponentSource) {
    let mut types = TypeRegistry::new();
    types
        .register(
            "Probe.Types",
            "Surface",
            TypeDefinition::default().property("width", "real", Value::Number(0.)),
        )
        .unwrap();
    let guest = Rc::new(Guest::default());
    let mut model =
        Instance::with_engine(source, &types, Default::default(), guest.clone()).unwrap();
    let root = model.root();
    let pane = model.id("pane").unwrap();
    for (object, name, n) in [
        (root, "byId", 8.),
        (root, "own", 7.),
        (root, "bare", 7.),
        (pane, "fromParent", 7.),
        (pane, "baseView", 3.),
        (pane, "derivedView", 9.),
    ] {
        assert_eq!(model.get(object, name).unwrap(), Value::Number(n));
    }
    let mut called = guest.0.borrow().clone();
    called.sort();
    assert_eq!(called, ["Math.PI", "dynamic.width", "this.operation"]);
    guest.0.borrow_mut().clear();
    model.set(root, "width", Value::Number(19.)).unwrap();
    model.set(pane, "width", Value::Number(21.)).unwrap();
    model.set(pane, "level", Value::Number(23.)).unwrap();
    assert_eq!(model.get(root, "own").unwrap(), Value::Number(19.));
    assert_eq!(model.get(pane, "fromParent").unwrap(), Value::Number(19.));
    assert_eq!(model.get(root, "byId").unwrap(), Value::Number(21.));
    assert_eq!(model.get(pane, "baseView").unwrap(), Value::Number(3.));
    assert_eq!(model.get(pane, "derivedView").unwrap(), Value::Number(23.));
    model.set(root, "byId", Value::Number(99.)).unwrap();
    model.set(pane, "width", Value::Number(25.)).unwrap();
    assert_eq!(model.get(root, "byId").unwrap(), Value::Number(99.));
    assert!(guest.0.borrow().is_empty());
}

#[test]
fn source_and_cargo_elide_guest_calls_only_for_proven_reads() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("reads/Main.qml");
    let source = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .load(path)
        .unwrap();
    exercise(&source.document);
    exercise(&compiled_qml());
}

#[test]
fn static_imports_keep_their_first_guest_evaluation_before_specialization() {
    use std::cell::Cell;
    struct Engine(Cell<usize>);
    impl ScriptEngine for Engine {
        fn evaluate(
            &self,
            ctx: &mut ScriptContext<'_, '_>,
            _: &ir::JavaScript,
        ) -> Result<Value, Error> {
            self.0.set(self.0.get() + 1);
            ctx.get(ctx.owner(), "width")
        }
    }
    let mut doc = ir::parse_external(
        "import QtQml\nQtObject {property int width:3;property int observed:this.width}",
    )
    .unwrap();
    doc.scripts.insert(
        "test.mjs".into(),
        ir::ScriptResource {
            source: "export const x=1".into(),
            executable: String::new(),
            kind: ir::ScriptKind::Module,
            preload: true,
            imports: Default::default(),
            dependencies: Default::default(),
            exports: Vec::new(),
            writable_exports: Vec::new(),
            import_hook: None,
        },
    );
    let engine = Rc::new(Engine(Cell::new(0)));
    let mut model = Instance::with_engine(
        &doc,
        &TypeRegistry::new(),
        Default::default(),
        engine.clone(),
    )
    .unwrap();
    assert_eq!(engine.0.get(), 1);
    model.set(model.root(), "width", Value::Number(5.)).unwrap();
    assert_eq!(
        model.get(model.root(), "observed").unwrap(),
        Value::Number(5.)
    );
    assert_eq!(engine.0.get(), 1);
}
