#[cfg(test)]
mod anchors;
#[cfg(test)]
mod buttons;
#[cfg(test)]
mod delegates;
#[cfg(test)]
mod geometry;
#[cfg(test)]
mod incubators;
#[cfg(test)]
mod nested_roles;
#[cfg(test)]
mod ranges;
#[cfg(test)]
mod remote_scripts;
#[cfg(test)]
mod states;
#[cfg(test)]
mod views;
#[cfg(test)]
fn quick_registry() -> ferroui_qml::TypeRegistry {
    use ferroui_qml::{TypeDefinition, TypeRegistry, Value};
    let mut types = TypeRegistry::new();
    types.register_quick_states().unwrap();
    types
        .register(
            "QtQuick",
            "Item",
            TypeDefinition::default()
                .with_quick_states()
                .inherits("QtQml", "QtObject")
                .property("x", "real", Value::Number(0.))
                .property("y", "real", Value::Number(0.))
                .property("width", "real", Value::Undefined)
                .property("height", "real", Value::Undefined)
                .property("implicitWidth", "real", Value::Number(0.))
                .property("implicitHeight", "real", Value::Number(0.))
                .property("data", "list<QtObject>", Value::List(vec![]))
                .default_property("data")
                .visual_children("data"),
        )
        .unwrap();
    types
        .register("QtQuick", "Loader", TypeDefinition::quick_loader())
        .unwrap();
    types
        .register("QtQuick", "Repeater", TypeDefinition::quick_repeater())
        .unwrap();
    types
        .register("QtQuick", "ListView", TypeDefinition::quick_list_view())
        .unwrap();
    types.reexport("QtQuick", "QtQml").unwrap();
    types
}

#[cfg(test)]
mod loader;
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

#[cfg(all(test, feature = "diagnostics"))]
mod diagnostics;

#[cfg(test)]
mod engine_roots;

#[cfg(test)]
mod singleton_tests {
    use ferroui_qml::{Instance, TypeRegistry, Value};
    include!(concat!(env!("OUT_DIR"), "/singletons.rs"));

    #[test]
    fn source_singleton_is_shared_by_components_and_isolated_between_engines() {
        let document = compiled_qml();
        assert_eq!(
            ferroui_qml::ComponentSource::document(&document)
                .singletons
                .len(),
            1
        );
        let registry = TypeRegistry::new();
        let mut first = Instance::new(&document, &registry, Default::default()).unwrap();
        let second = Instance::new(&document, &registry, Default::default()).unwrap();
        let root = first.root();
        assert_eq!(first.get(root, "contrast").unwrap(), Value::Number(3.));
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

#[cfg(test)]
mod scripts {
    use ferroui_qml::{ir, ComponentSource, Instance, TypeRegistry, Value};
    include!(concat!(env!("OUT_DIR"), "/scripts.rs"));

    fn run(source: &impl ComponentSource) -> Vec<Value> {
        let mut vm = Instance::new(source, &TypeRegistry::new(), Default::default()).unwrap();
        let root = vm.root();
        let mut observed = vec![vm.get(root, "reading").unwrap()];
        let n = Value::Number;
        vm.emit(root, "calculate", vec![n(9.)]).unwrap();
        observed.push(vm.get(root, "amount").unwrap());
        observed.push(vm.get(root, "reading").unwrap());
        observed.push(vm.call(root, "fold", vec![n(3.)]).unwrap());
        observed.push(vm.call(root, "fold", vec![n(8.)]).unwrap());
        observed.push(vm.call(root, "lazy", vec![]).unwrap());
        assert!(vm.call(root, "fail", vec![]).is_err());
        observed.push(vm.get(root, "amount").unwrap());
        vm.set(root, "active", Value::Bool(false)).unwrap();
        observed.push(vm.get(root, "reading").unwrap());
        observed
    }

    #[test]
    fn native_compiler_preserves_restore_value_and_first_read_metadata() {
        let compiled = compiled_qml();
        assert_eq!(compiled.first_reads().len(), compiled.evaluators().len());
        assert!(compiled.first_reads().iter().any(Option::is_some));
        let mut vm = Instance::new(&compiled, &TypeRegistry::new(), Default::default()).unwrap();
        let root = vm.root();
        assert_eq!(vm.get(root, "staticInitial").unwrap(), Value::Number(27.));
        vm.set(
            vm.id("literalOverride").unwrap(),
            "when",
            Value::Bool(false),
        )
        .unwrap();
        assert_eq!(vm.get(root, "staticInitial").unwrap(), Value::Number(-8.));
    }

    #[test]
    fn native_and_source_scripts_agree_on_control_flow_and_transaction_rollback() {
        let compiled = compiled_qml();
        assert!(!compiled.scripts().is_empty());
        assert!(!compiled.evaluators().is_empty());
        let source = ir::parse(include_str!("scripts/ControlFlow.qml")).unwrap();
        let actual = run(&compiled);
        assert_eq!(actual, run(&source));
        let n = Value::Number;
        assert_eq!(
            actual,
            vec![
                n(4.),
                n(8.),
                n(8.),
                n(9.),
                n(-1.),
                Value::Bool(true),
                n(8.),
                n(7.)
            ]
        );
    }
}

#[cfg(test)]
#[path = "reference/fixture.rs"]
mod qt_reference;

#[test]
fn both_backends_match_qt_6_12_behaviors() {
    qt_reference::run(|mode, case, actual, expected| assert_eq!(actual, expected, "{mode}/{case}"));
}

#[cfg(test)]
mod shadows {
    use ferroui_qml::{Instance, TypeRegistry, Value};
    include!(concat!(env!("OUT_DIR"), "/shadow.rs"));
    #[test]
    fn each_declaration_retains_independent_storage_notify_and_lexical_access() {
        let mut engine =
            Instance::new(&compiled_qml(), &TypeRegistry::new(), Default::default()).unwrap();
        let root = engine.root();
        assert_eq!(
            engine.call(root, "observe", Vec::new()).unwrap(),
            Value::String("2/7/25/2/2/2/7/25".into())
        );
        assert!(engine
            .property_names(root)
            .iter()
            .all(|p| !p.starts_with('$')));
        assert!(engine
            .drain_changes()
            .iter()
            .all(|change| !change.property.starts_with('$')));
    }
}

#[cfg(test)]
mod models {
    use ferroui_qml::{ir, ComponentSource, Instance, TypeRegistry, Value};
    include!(concat!(env!("OUT_DIR"), "/inventory.rs"));

    fn verify(source: &impl ComponentSource) {
        let mut instance = Instance::new(source, &TypeRegistry::new(), Default::default()).unwrap();
        let root = instance.root();
        let model = instance.id("stock").unwrap();
        assert_eq!(
            instance.get(root, "firstName").unwrap(),
            Value::String("A".into())
        );
        assert_eq!(
            instance.call(root, "edit", vec![]).unwrap(),
            Value::String("3/C/4/8".into())
        );
        assert_eq!(
            instance.get(root, "firstName").unwrap(),
            Value::String("C".into())
        );
        let Value::ModelRow(row) = instance
            .call(model, "get", vec![Value::Number(1.)])
            .unwrap()
        else {
            panic!("missing row")
        };
        assert!(instance.call(root, "invalid", vec![]).is_err());
        assert_eq!(instance.get(model, "count").unwrap(), Value::Number(3.));
        assert_eq!(
            instance.model_role(row, "price").unwrap(),
            Value::Number(8.)
        );
        assert!(instance.set(model, "count", Value::Number(1.)).is_err());
        instance.call(model, "clear", vec![]).unwrap();
        assert!(instance.model_role(row, "price").is_err());
        assert_eq!(
            instance.get(root, "firstName").unwrap(),
            Value::String(String::new())
        );
    }

    #[test]
    fn qml_models_support_editing_reordering_and_transactional_row_handles() {
        verify(&compiled_qml());
        verify(&ir::parse(include_str!("models/Inventory.qml")).unwrap());
    }
}

#[cfg(all(test, feature = "webscene"))]
mod webscene;

#[cfg(all(test, feature = "webscene"))]
mod direct_reads;

#[cfg(test)]
mod native_host;

#[cfg(test)]
mod dynamic_components;
#[cfg(all(test, feature = "webscene"))]
mod workers;

#[cfg(test)]
mod remote_components;
