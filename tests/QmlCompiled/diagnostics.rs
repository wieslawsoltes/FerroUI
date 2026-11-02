use ferroui_qml::diagnostics::{Breakpoint, Debugger, Kind, Phase, Resume};
use ferroui_qml::{ComponentSource, Instance, ScriptEngine, TypeRegistry, Value};
use std::cell::RefCell;
use std::rc::Rc;
include!(concat!(env!("OUT_DIR"), "/debug_probe.rs"));

fn exercise(source: &impl ComponentSource, engine: Option<Rc<dyn ScriptEngine>>) {
    let debug = Debugger::default();
    let events = Rc::new(RefCell::new(Vec::new()));
    let trace = events.clone();
    debug.on_trace(move |event| {
        trace.borrow_mut().push((
            matches!(event.phase, Phase::Enter),
            event.depth,
            event.site.name.clone(),
        ));
    });
    let mut ui = Instance::with_debugger(
        source,
        &TypeRegistry::new(),
        Default::default(),
        engine,
        debug.clone(),
    )
    .unwrap();
    let root = ui.root();
    let init = debug.take_profile();
    assert!(init.iter().any(|(s, m)| s.name == "Component.onCompleted"
        && s.kind == Kind::Handler
        && m.calls == 1));
    let binding = init
        .keys()
        .find(|s| s.kind == Kind::Binding && s.name == "doubled")
        .unwrap();
    let span = binding.span.unwrap();
    assert!(include_str!("diagnostics/Probe.qml")[span.start..span.end].contains("count * 2"));
    let stops = Rc::new(RefCell::new(Vec::new()));
    let copy = stops.clone();
    debug.set_breakpoints([Breakpoint::Member {
        kind: Kind::Method,
        name: "outer".into(),
    }]);
    debug.on_pause(move |frame| {
        let mut stops = copy.borrow_mut();
        assert_eq!(
            frame.parameters[0].name,
            if frame.site.name == "add" {
                "amount"
            } else {
                "arg"
            }
        );
        stops.push((
            frame.site.name.clone(),
            frame.depth,
            frame.arguments[0].clone(),
        ));
        match stops.len() {
            1 | 2 => Resume::StepInto,
            3 => Resume::StepOut,
            _ => Resume::StepOver,
        }
    });
    let result = ui.call(root, "outer", vec![Value::Number(2.)]).unwrap();
    assert_eq!(result, Value::Number(7.));
    assert_eq!(
        stops.borrow().as_slice(),
        [
            ("outer".into(), 0, Value::Number(2.)),
            ("nested".into(), 1, Value::Number(2.)),
            ("add".into(), 2, Value::Number(2.)),
            ("nested".into(), 1, Value::Number(3.)),
        ]
    );
    let profile = debug.take_profile();
    let (site, metrics) = profile.iter().find(|(s, _)| s.name == "add").unwrap();
    assert!(site.source.as_deref().unwrap().ends_with("/Part.qml"));
    assert_eq!((metrics.calls, metrics.failures), (2, 0));
    assert_eq!(metrics.inclusive, metrics.exclusive);
    let outer = profile.iter().find(|(s, _)| s.name == "outer").unwrap().1;
    assert!(outer.inclusive >= metrics.inclusive);
    assert!(outer.exclusive <= outer.inclusive);
    let mut stack = Vec::new();
    for (enter, depth, name) in events.borrow().iter() {
        if *enter {
            assert_eq!(stack.len(), *depth);
            stack.push(name.clone());
        } else {
            assert_eq!(stack.pop().as_ref(), Some(name));
        }
    }
    assert!(stack.is_empty());
    // A source-offset breakpoint inside a flattened composite uses its defining file.
    // Resume the lingering step-over once, then arm the source breakpoint.
    debug.on_pause(|_| Resume::Continue);
    ui.call(root, "nested", vec![Value::Number(0.)]).unwrap();
    debug.set_breakpoints([Breakpoint::Source {
        url: site.source.clone().unwrap(),
        offset: include_str!("diagnostics/Part.qml").find("add(").unwrap(),
    }]);
    debug.on_pause(|frame| {
        assert_eq!(frame.site.name, "add");
        Resume::Abort
    });
    assert!(ui.call(root, "mutate", vec![]).is_err());
    assert_eq!(ui.get(root, "count").unwrap(), Value::Number(1.));
    let stats = debug.profile();
    assert!(stats
        .iter()
        .any(|(s, m)| s.name == "mutate" && m.failures == 1));
    ui.set_debugger(None);
    ui.call(root, "mutate", vec![]).unwrap();
    assert_eq!(ui.get(root, "count").unwrap(), Value::Number(2.));
}

#[test]
fn source_and_aot_declaration_stepping_and_profiling() {
    let doc = ferroui_qml_build::Compiler::new()
        .load("diagnostics/Probe.qml")
        .unwrap()
        .document;
    exercise(&doc, None);
    exercise(&compiled_qml(), None);
}

#[cfg(feature = "webscene")]
#[test]
fn guest_declaration_breakpoints_and_abort_preserve_qml_transactions() {
    let doc = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .load("diagnostics/Probe.qml")
        .unwrap()
        .document;
    let vm = Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap());
    exercise(&doc, Some(vm));
}
