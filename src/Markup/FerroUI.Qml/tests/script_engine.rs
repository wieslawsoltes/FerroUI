use ferroui_qml::{
    ir::{self, JavaScript, ScriptForm},
    Error, ExternalValue, Instance, ScriptContext, ScriptEngine, ScriptInstance, TypeRegistry,
    Value,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

/// This engine exercises the host protocol independently of any optional VM.
#[derive(Default)]
struct Probe {
    begins: Cell<usize>,
    commits: Cell<usize>,
    releases: RefCell<Vec<ScriptInstance>>,
    depth: Cell<usize>,
    max_depth: Cell<usize>,
    selected: Cell<usize>,
    foreign_field: Cell<f64>,
    checkpoint: Cell<f64>,
}

impl ScriptEngine for Probe {
    fn begin_transaction(&self, _: ScriptInstance) -> Result<(), Error> {
        self.begins.set(self.begins.get() + 1);
        self.checkpoint.set(self.foreign_field.get());
        Ok(())
    }
    fn end_transaction(&self, _: ScriptInstance, committed: bool) {
        if committed {
            self.commits.set(self.commits.get() + 1);
        } else {
            self.foreign_field.set(self.checkpoint.get());
        }
    }
    fn release_instance(&self, id: ScriptInstance) {
        self.releases.borrow_mut().push(id);
    }
    fn evaluate(
        &self,
        host: &mut ScriptContext<'_, '_>,
        script: &JavaScript,
    ) -> Result<Value, Error> {
        let depth = self.depth.get() + 1;
        self.depth.set(depth);
        self.max_depth.set(self.max_depth.get().max(depth));
        let result = (|| match script.source.as_str() {
            "select" => {
                assert!(host.in_binding());
                assert_eq!(host.expected_type(), Some("int"));
                assert!(!host.names().iter().any(|n| n.starts_with("$qml:")));
                self.selected.set(self.selected.get() + 1);
                let selected = if host.resolve("choose")? == Some(Value::Bool(true)) {
                    "left"
                } else {
                    "right"
                };
                host.resolve(selected)?
                    .ok_or_else(|| Error::new("missing selection"))
            }
            "outer" => {
                assert_eq!(host.expected_type(), Some("int"));
                assert_eq!(host.parameter_names(), &["n"]);
                assert_eq!(host.argument_type(host.owner(), "inner", 0), Some("int"));
                assert_eq!(host.property_type(host.owner(), "left"), Some("int"));
                let owner = host.method_owner("inner").unwrap();
                let value = host.resolve("n")?.unwrap();
                host.call(owner, "inner", vec![value])
            }
            "inner" => {
                let owner = host.owner();
                let n = host.resolve("arg")?.unwrap();
                host.set(owner, "left", n)?;
                host.get(owner, "selected")
            }
            "external" => Ok(host.external(7, Rc::new(String::from("callable")))),
            "fail" => {
                host.set(host.owner(), "left", Value::Number(91.))?;
                host.call(host.owner(), "published", vec![])?;
                self.foreign_field.set(99.);
                Err(Error::new("guest threw"))
            }
            "catch" => {
                assert!(host.call(host.owner(), "halfway", vec![]).is_err());
                Ok(Value::Number(100.))
            }
            "scope" => {
                assert!(host.resources().contains_key("local.mjs"));
                assert_eq!(
                    host.resolve("privateName")?,
                    Some(Value::Object(host.root()))
                );
                Ok(Value::Number(1.))
            }
            _ => Err(Error::new("unexpected guest source")),
        })();
        self.depth.set(depth - 1);
        result
    }
    fn get_external(
        &self,
        _: &mut ScriptContext<'_, '_>,
        receiver: &ExternalValue,
        name: &str,
    ) -> Result<Value, Error> {
        assert_eq!(receiver.downcast::<String>().unwrap().as_str(), "callable");
        match name {
            "field" => Ok(Value::Number(self.foreign_field.get())),
            _ => Ok(Value::Undefined),
        }
    }
    fn set_external(
        &self,
        _: &mut ScriptContext<'_, '_>,
        _: &ExternalValue,
        name: &str,
        value: Value,
    ) -> Result<(), Error> {
        if let ("field", Value::Number(v)) = (name, value) {
            self.foreign_field.set(v);
            return Ok(());
        }
        Err(Error::new("invalid external assignment"))
    }
    fn call_external(
        &self,
        host: &mut ScriptContext<'_, '_>,
        function: &ExternalValue,
        receiver: Value,
        args: Vec<Value>,
    ) -> Result<Value, Error> {
        assert_eq!(function.instance(), host.instance());
        assert_eq!(receiver, Value::Object(host.owner()));
        Ok(args[0].clone())
    }
}

fn script(source: &str, form: ScriptForm) -> JavaScript {
    JavaScript {
        executable: None,
        import_hook: None,
        source: source.into(),
        form,
        origin: "fixture.qml".into(),
        references: Vec::new(),
        span: ir::SourceSpan { start: 1, end: 2 },
        root: None,
        ids: Default::default(),
        imports: Default::default(),
    }
}

fn fixture() -> ir::Document {
    let mut doc=ir::parse("import QtQml\nQtObject { id: actual; property bool choose:true; property int left: 2; property int right:5; property int selected: 1; property var functionHandle: null; signal published(); function halfway() { left=33; absent() }; function update(n:int):int { return n }; function inner(arg:int):int {return arg}; function invokeStored() {functionHandle.field=13; return functionHandle(functionHandle.field)} }").unwrap();
    for m in &mut doc.root.members {
        match &mut m.kind {
            ir::MemberKind::Property { name, value, .. }
                if name == "selected" || name == "functionHandle" =>
            {
                let text = if name == "selected" {
                    "select"
                } else {
                    "external"
                };
                *value = Some(ir::Binding::Expr(ir::Expr::JavaScript(script(
                    text,
                    ScriptForm::Expression,
                ))));
            }
            ir::MemberKind::Function { name, body, .. } if name == "update" || name == "inner" => {
                let text = if name == "update" { "outer" } else { "inner" };
                *body = vec![ir::Stmt::JavaScript(script(text, ScriptForm::FunctionBody))];
            }
            _ => (),
        }
    }
    doc.scripts.insert(
        "local.mjs".into(),
        ir::ScriptResource {
            executable: String::new(),
            import_hook: None,
            preload: true,
            imports: Default::default(),
            source: "export const n=1".into(),
            kind: ir::ScriptKind::Module,
            dependencies: Default::default(),
            exports: vec!["n".into()],
        },
    );
    doc
}

#[test]
fn host_access_is_live_tracks_only_fields_read_and_nested_guest_calls_share_transactions() {
    let engine = Rc::new(Probe::default());
    let mut ui = Instance::with_engine(
        &fixture(),
        &TypeRegistry::new(),
        BTreeMap::new(),
        engine.clone(),
    )
    .unwrap();
    let root = ui.root();
    assert_eq!(ui.get(root, "selected").unwrap(), Value::Number(2.));
    let reads = engine.selected.get();
    ui.set(root, "right", Value::Number(12.)).unwrap();
    assert_eq!(engine.selected.get(), reads);
    let starts = engine.begins.get();
    assert_eq!(
        ui.call(root, "update", vec![Value::Number(8.)]).unwrap(),
        Value::Number(8.)
    );
    assert_eq!(engine.begins.get(), starts + 1);
    assert!(engine.max_depth.get() >= 3);
    ui.set(root, "choose", Value::Bool(false)).unwrap();
    assert_eq!(ui.get(root, "selected").unwrap(), Value::Number(12.));
    let reads = engine.selected.get();
    ui.set(root, "left", Value::Number(1.)).unwrap();
    assert_eq!(reads, engine.selected.get());
    assert_eq!(
        ui.call(root, "invokeStored", vec![]).unwrap(),
        Value::Number(13.)
    );
}

#[test]
fn script_errors_and_caught_partial_host_failures_restore_state_and_release_the_realm() {
    let engine = Rc::new(Probe::default());
    let doc = fixture();
    let mut ui =
        Instance::with_engine(&doc, &TypeRegistry::new(), BTreeMap::new(), engine.clone()).unwrap();
    let root = ui.root();
    ui.watch_signal(root, "published").unwrap();
    assert!(ui
        .evaluate_script(root, &script("fail", ScriptForm::FunctionBody))
        .unwrap_err()
        .message
        .contains("fixture.qml[1..2]: guest threw"));
    assert_eq!(ui.get(root, "left").unwrap(), Value::Number(2.));
    assert!(ui.drain_signals().is_empty());
    assert_eq!(engine.foreign_field.get(), 0.);
    assert!(ui
        .evaluate_script(root, &script("catch", ScriptForm::FunctionBody))
        .is_err());
    assert_eq!(ui.get(root, "selected").unwrap(), Value::Number(2.));
    assert!(ui.drain_changes().is_empty());
    let mut aliased = script("scope", ScriptForm::Expression);
    aliased.ids.insert("privateName".into(), "actual".into());
    assert_eq!(
        ui.evaluate_script(root, &aliased).unwrap(),
        Value::Number(1.)
    );
    let value = ui.get(root, "functionHandle").unwrap();
    let mut other =
        Instance::with_engine(&doc, &TypeRegistry::new(), BTreeMap::new(), engine.clone()).unwrap();
    assert!(other.set(other.root(), "functionHandle", value).is_err());
    let first = ui.script_instance();
    let second = other.script_instance();
    drop(ui);
    drop(other);
    assert_eq!(&*engine.releases.borrow(), &[first, second]);
}

#[test]
fn requesting_a_script_without_attaching_an_engine_returns_a_diagnostic() {
    assert!(
        Instance::new(&fixture(), &TypeRegistry::new(), BTreeMap::new())
            .err()
            .unwrap()
            .message
            .contains("requires a JavaScript engine")
    );
}

#[derive(Default)]
struct CapturingEngine {
    saved: RefCell<Option<ferroui_qml::ScriptScope>>,
}

impl ScriptEngine for CapturingEngine {
    fn evaluate(
        &self,
        ctx: &mut ScriptContext<'_, '_>,
        script: &JavaScript,
    ) -> Result<Value, Error> {
        match script.source.as_str() {
            "capture" => {
                *self.saved.borrow_mut() = Some(ctx.capture_scope());
                Ok(Value::Undefined)
            }
            "later" => {
                let scope = self.saved.borrow().clone().unwrap();
                ctx.in_scope(&scope, |lexical| {
                    assert_eq!(
                        lexical.resolve("Ui.Local")?,
                        Some(Value::Object(lexical.owner()))
                    );
                    let Value::Number(old) = lexical.resolve("count")?.unwrap() else {
                        return Err(Error::new("expected count"));
                    };
                    lexical.assign("count", Value::Number(old + 1.))?;
                    Ok(Value::Number(old))
                })
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn retained_lexical_scope_reenters_with_live_values_on_another_owner_but_never_another_instance() {
    let doc=ir::parse("import QtQml\nQtObject { id: root; property int count: 99; property QtObject nested: QtObject { id: child; property int count: 4 } }").unwrap();
    let engine = Rc::new(CapturingEngine::default());
    let mut ui = Instance::with_engine(
        &doc,
        &TypeRegistry::new(),
        Default::default(),
        engine.clone(),
    )
    .unwrap();
    let child = ui.id("child").unwrap();
    let mut capture = script("capture", ScriptForm::FunctionBody);
    capture.ids.insert("Ui.Local".into(), "child".into());
    ui.evaluate_script(child, &capture).unwrap();
    assert_eq!(
        ui.evaluate_script(ui.root(), &script("later", ScriptForm::FunctionBody))
            .unwrap(),
        Value::Number(4.)
    );
    assert_eq!(ui.get(child, "count").unwrap(), Value::Number(5.));
    assert_eq!(ui.get(ui.root(), "count").unwrap(), Value::Number(99.));
    let mut second =
        Instance::with_engine(&doc, &TypeRegistry::new(), Default::default(), engine).unwrap();
    assert!(second
        .evaluate_script(second.root(), &script("later", ScriptForm::FunctionBody))
        .unwrap_err()
        .message
        .contains("foreign"));
}
