use ferroui_qml::{
    ir::qmldir::StaticPlugin, ComponentSource, Error, Instance, TypeDefinition, TypeRegistry, Value,
};
use ferroui_qml_build::Compiler;

mod compiled {
    include!(concat!(env!("OUT_DIR"), "/static_plugin.rs"));
}

fn registry() -> TypeRegistry {
    let mut registry = TypeRegistry::new();
    let plugin = StaticPlugin::new("Workshop.Gauges", "gaugeplugin", 1, 3);
    assert!(registry
        .install_plugin(plugin.clone(), |types| {
            types.register("Workshop.Gauges", "Partial", TypeDefinition::default())?;
            Err(Error::new("incomplete initialization"))
        })
        .is_err());
    assert!(!registry.modules().any(|module| module == "Workshop.Gauges"));
    registry
        .install_plugin(plugin.clone(), |types| {
            types.register(
                "Workshop.Gauges",
                "Gauge",
                TypeDefinition::default()
                    .inherits("QtQml", "QtObject")
                    .property("value", "int", Value::Number(0.)),
            )
        })
        .unwrap();
    registry
        .install_plugin(plugin, |_| panic!("already initialized"))
        .unwrap();
    registry
}

#[test]
fn plugin_and_source_exports_share_a_versioned_namespace() {
    let types = registry();
    let compiler = || Compiler::new().import_path("plugins/modules");
    let missing = compiler().load("plugins/Main.qml").err().unwrap();
    assert!(missing.to_string().contains("gaugeplugin"), "{missing}");
    let compiler = compiler().static_plugin(types.static_plugins().next().unwrap().clone());
    let source = compiler.load("plugins/Main.qml").unwrap();
    assert!(source
        .dependencies
        .iter()
        .any(|p| p.ends_with("Workshop/Gauges/qmldir")));
    fn check(source: &impl ComponentSource, types: &TypeRegistry) {
        let mut instance = Instance::new(source, types, Default::default()).unwrap();
        let other = instance.get(instance.root(), "other").unwrap();
        assert_eq!(
            instance
                .call(instance.root(), "echo", vec![other.clone()])
                .unwrap(),
            other
        );
        assert_eq!(
            instance.get(instance.root(), "total").unwrap(),
            Value::Number(45.)
        );
        instance
            .set(instance.root(), "value", Value::Number(9.))
            .unwrap();
        assert_eq!(
            instance.get(instance.root(), "total").unwrap(),
            Value::Number(21.)
        );
    }
    check(&source.document, &types);
    check(&compiled::compiled_qml(), &types);
}
