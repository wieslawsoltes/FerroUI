//! Tests of the registration of the framework evaluators.

use super::*;

use crate::runtime::type_system::RuntimeTypeSystem;

/// The type names of the first column of the table of
/// `compiler_extensions/mod.rs`.
fn table_rows() -> Vec<String> {
    let source = include_str!("../../compiler_extensions/mod.rs");
    source
        .lines()
        .filter(|line| line.starts_with("//! | [`"))
        .filter_map(|line| {
            let name = line.split('`').nth(1)?;
            Some(name.rsplit("::").next().unwrap_or(name).to_string())
        })
        .collect()
}

#[test]
fn every_row_of_the_back_end_table_has_an_evaluator() {
    let rows = table_rows();
    assert!(rows.len() > 60, "the table was not found: {} rows", rows.len());
    for row in &rows {
        assert!(
            FRAMEWORK_EVALUATORS.iter().any(|(name, _)| name == row),
            "{row} of the back-end table has no evaluator"
        );
    }
    for (name, _) in FRAMEWORK_EVALUATORS {
        assert!(rows.iter().any(|row| row == name), "{name} is not a row of the back-end table");
    }
    let mut names: Vec<&str> = FRAMEWORK_EVALUATORS.iter().map(|(name, _)| *name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), FRAMEWORK_EVALUATORS.len(), "a row is listed twice");
}


fn type_system() -> (Rc<RuntimeTypeSystem>, Rc<dyn xamlx::type_system::IXamlTypeSystem>) {
    let system = crate::FerroXamlIlRuntimeCompiler::type_system();
    let type_system = system.as_type_system();
    (system, type_system)
}

#[test]
fn the_language_configures_over_the_registered_metadata() {
    let (_, type_system) = type_system();
    let (mut mappings, emit) = match crate::compiler_extensions::FerroXamlIlLanguage::configure(&type_system) {
        Ok(configured) => configured,
        Err(e) => panic!("{}", e.message()),
    };
    assert!(emit.context_type_builder_callback(&mappings).is_ok());

    // The deferred content customisation becomes a generic method definition.
    adapt_type_mappings(&mut mappings);
    let customization = mappings.deferred_content_executor_customization.clone().expect("customization");
    assert!(customization.is_generic_method_definition());
    assert_eq!(customization.name(), "DeferredTransformationFactoryV3");
    let control = type_system.get_type("FerroUI.Controls.Control").ok().expect("Control");
    let constructed = customization.make_generic_method(&[control.clone()]).ok().expect("constructed");
    assert!(!constructed.is_generic_method_definition());
    assert!(constructed.generic_arguments()[0].equals(&*control));
    assert!(customization.make_generic_method(&[]).is_err());
    assert!(constructed.as_any().downcast_ref::<DeferredTransformationFactoryMethod>().is_some());
}

#[test]
fn types_the_compiler_looks_up_by_name_are_found_before_any_use() {
    let (_, type_system) = type_system();
    for name in [
        "FerroUI.FerroProperty",
        "FerroUI.FerroProperty`1",
        "FerroUI.StyledProperty`1",
        "FerroUI.AttachedProperty`1",
        "System.Collections.Generic.IDictionary`2",
        "System.IObservable`1",
        "System.Threading.Tasks.Task`1",
        "System.WeakReference`1",
        "System.ComponentModel.CultureInfoConverter",
        "FerroUI.Data.AssignBindingAttribute",
        "FerroUI.Metadata.ContentAttribute",
    ] {
        assert!(type_system.find_type(name).is_some(), "{name}");
    }
    let styled = type_system.find_type("FerroUI.StyledProperty`1").unwrap();
    let double = type_system.find_type("System.Double").unwrap();
    let property = type_system.find_type("FerroUI.FerroProperty").unwrap();
    let styled_of_double = styled.make_generic_type(&[double]).ok().expect("instantiation");
    assert!(property.is_assignable_from(&*styled_of_double));
    let converter = type_system.find_type("System.ComponentModel.CultureInfoConverter").unwrap();
    assert!(type_system.find_type("System.ComponentModel.TypeConverter").unwrap().is_assignable_from(&*converter));
}

#[test]
fn the_generic_and_untyped_members_of_the_root_class_are_projected() {
    use ferroui_base::data::BindingPriority;
    use ferroui_base::metadata::from_markup_value;
    use xamlx::type_system::IXamlMethod;

    use crate::runtime::type_system::{RuntimeConstructor, RuntimeField, RuntimeMethod};

    let (system, type_system) = type_system();
    let object = type_system.get_type("FerroUI.FerroObject").ok().expect("FerroObject");
    let priority = type_system.get_type("FerroUI.Data.BindingPriority").ok().expect("BindingPriority");
    let double = type_system.get_type("System.Double").ok().expect("Double");

    // `SetValue<T>(StyledProperty<T>, T, BindingPriority)`, found as the well-known types find it.
    let set_styled = object
        .get_method(|m| {
            let parameters = m.parameters();
            m.is_public()
                && !m.is_static()
                && m.name() == "SetValue"
                && parameters.len() == 3
                && parameters[0].name() == "StyledProperty`1"
                && parameters[2].equals(&*priority)
        })
        .ok()
        .expect("SetValue<T>");
    assert!(set_styled.is_generic_method_definition());
    assert_eq!(set_styled.generic_parameters().len(), 1);
    assert_eq!(set_styled.return_type().full_name(), "System.IDisposable");
    let set_double = set_styled.make_generic_method(&[double.clone()]).ok().expect("SetValue<double>");
    assert!(!set_double.is_generic_method_definition());
    let parameters = set_double.parameters();
    assert_eq!(parameters[0].full_name(), "FerroUI.StyledProperty`1[System.Double]");
    assert!(parameters[1].equals(&*double));
    assert!(set_double.make_generic_method(&[double.clone()]).is_err());
    assert!(set_styled.make_generic_method(&[]).is_err());

    // The untyped accessors.
    let property = type_system.get_type("FerroUI.FerroProperty").ok().expect("FerroProperty");
    let set_untyped = object
        .get_method(|m| m.name() == "SetValue" && m.parameters().first().is_some_and(|p| p.equals(&*property)))
        .ok()
        .expect("SetValue");
    let get_untyped = object.get_method(|m| m.name() == "GetValue").ok().expect("GetValue");
    let bind = object.get_method(|m| m.name() == "Bind").ok().expect("Bind");
    assert_eq!(bind.return_type().full_name(), "FerroUI.Data.BindingExpressionBase");
    assert_eq!(bind.parameters()[1].full_name(), "FerroUI.Data.BindingBase");

    // They work on a live object: a brush has a styled opacity.
    let brush = type_system.get_type("FerroUI.Media.SolidColorBrush").ok().expect("SolidColorBrush");
    let constructor = brush.get_constructor(None).ok().expect("constructor");
    let instance = constructor.as_any().downcast_ref::<RuntimeConstructor>().unwrap().invoke(&[]).ok().expect("brush");
    let field = brush.get_all_fields().into_iter().find(|f| f.name() == "OpacityProperty").expect("OpacityProperty");
    let opacity = field.as_any().downcast_ref::<RuntimeField>().unwrap().get().ok().expect("field value");
    let local = evaluator_value(BindingPriority::LocalValue);
    let invoke = |method: &Rc<dyn IXamlMethod>, arguments: &[ferroui_base::metadata::MarkupValue]| {
        method.as_any().downcast_ref::<RuntimeMethod>().unwrap().invoke(arguments)
    };
    invoke(&set_double, &[instance.clone(), opacity.clone(), evaluator_value(0.25f64), local.clone()]).ok().expect("set");
    let read = invoke(&get_untyped, &[instance.clone(), opacity.clone()]).ok().expect("get");
    assert_eq!(from_markup_value::<f64>(&read), Some(0.25));
    invoke(&set_untyped, &[instance.clone(), opacity.clone(), evaluator_value(0.5f64), local.clone()]).ok().expect("set");
    let read = invoke(&get_untyped, &[instance.clone(), opacity.clone()]).ok().expect("get");
    assert_eq!(from_markup_value::<f64>(&read), Some(0.5));
    // A value of another type is rejected, not converted.
    assert!(invoke(&set_untyped, &[instance.clone(), opacity.clone(), evaluator_value("x".to_string()), local.clone()])
        .is_err());
    let _ = (instance, opacity, local);
    let _ = system;
}

fn evaluator_value<T: PartialEq + 'static>(value: T) -> ferroui_base::metadata::MarkupValue {
    let value: ferroui_base::BoxedValue = Rc::new(value);
    Some(value)
}

#[test]
fn the_add_handler_methods_of_interactive_are_projected() {
    let (_, type_system) = type_system();
    let well_known = type_system.well_known_types();
    let interactivity = match crate::compiler_extensions::transformers::InteractivityWellKnownTypes::new(
        &type_system,
        &well_known,
    ) {
        Ok(types) => types,
        Err(e) => panic!("{}", e.message()),
    };
    assert!(!interactivity.add_handler.is_generic_method_definition());
    assert!(interactivity.add_handler_t.is_generic_method_definition());
    let constructed = interactivity
        .add_handler_t
        .make_generic_method(std::slice::from_ref(&interactivity.routed_event_args))
        .ok()
        .expect("AddHandler<RoutedEventArgs>");
    let parameters = constructed.parameters();
    assert!(parameters[1].equals(&*interactivity.routed_event_handler));
    assert!(interactivity.routed_event.is_assignable_from(&*parameters[0]));
}

/// The diagnostic of the well-known types over the real registries: the
/// first member the compiler cannot resolve yet. Run with `--ignored`.
#[test]
#[ignore = "diagnostic: fails until every well-known member has metadata"]
fn the_well_known_types_resolve_over_the_registered_metadata() {
    let (_, type_system) = type_system();
    let result = crate::compiler_extensions::transformers::FerroXamlIlWellKnownTypes::new(&type_system);
    assert!(result.is_ok(), "well-known types: {:?}", result.err().map(|e| e.message()));
}

