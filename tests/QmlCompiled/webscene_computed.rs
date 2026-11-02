use super::*;
use std::time::Duration;
include!(concat!(env!("OUT_DIR"), "/computed.rs"));

fn compiler() -> Compiler<'static> {
    Compiler::new()
        .javascript_entry("dual.js")
        .javascript_entry("plugins/en/index.js")
        .javascript_entry("plugins/pl/index.js")
        .javascript_entry("plugins/en/leaf.mjs")
}

fn exercise_computed(source: &impl ComponentSource) {
    let mut vm = Instance::with_engine(
        source,
        &TypeRegistry::new(),
        Default::default(),
        Rc::new(WebSceneEngine::from_env().unwrap()),
    )
    .unwrap();
    let root = vm.root();
    let wait = Duration::from_secs(1);
    let string = |s: &str| Value::String(s.to_owned());
    assert_eq!(
        vm.call(root, "evaluations", vec![]).unwrap(),
        Value::Number(0.)
    );
    for (method, locale, expected) in [
        ("fromQml", "pl", "pl"),
        ("fromChild", "en", "en"),
        ("fromClassic", "en", "en"),
        ("fromEsm", "pl", "pl"),
        ("fromNested", "en", "English leaf"),
    ] {
        assert_eq!(
            vm.call_and_await(root, method, vec![string(locale)], wait)
                .unwrap(),
            string(expected)
        );
    }
    assert_eq!(
        vm.call_and_await(root, "fromCoercion", vec![], wait)
            .unwrap(),
        string("en")
    );
    assert_eq!(
        vm.call_and_await(root, "sameNamespace", vec![], wait)
            .unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        vm.call(root, "evaluations", vec![]).unwrap(),
        Value::Number(2.)
    );
    assert_eq!(
        vm.call_and_await(root, "dualRole", vec![string("./dual.js")], wait)
            .unwrap(),
        string("1/undefined/2")
    );
    assert_eq!(
        vm.call_and_await(root, "rejectsClassicFactory", vec![], wait)
            .unwrap(),
        Value::Bool(true)
    );
    for path in [
        "./missing.js",
        "../outside.mjs",
        "https://example.invalid/module.mjs",
        "/absolute.mjs",
        "./bad%xx",
        "./plugins%2fen/index.js",
    ] {
        assert_eq!(
            vm.call_and_await(root, "rejectUnknown", vec![string(path)], wait)
                .unwrap(),
            Value::Bool(true),
            "{path}"
        );
    }
}

#[test]
fn dynamic_module_expressions_resolve_only_bundled_files_in_the_importers_directory() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("computed/Main.qml");
    exercise_computed(&compiler().load(path).unwrap().document);
    exercise_computed(&compiled_qml());
}
