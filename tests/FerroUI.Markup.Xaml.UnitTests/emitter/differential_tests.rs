//! The differential harness of the Rust emitter: every document of the
//! corpus is built by the run-time loader (the interpreter) and by the
//! generated function of `generated.rs`, and the two object trees are
//! compared through a canonical dump.
//!
//! The generated functions are compiled with this crate (rustc checks every
//! emitted call); `generated_output_is_up_to_date` keeps the checked-in file
//! equal to what the emitter writes today. See the module documentation for
//! why the file is checked in and how to regenerate it.

use std::rc::Rc;

use ferroui_base::controls::{INameScope, NameScope};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::diagnostics::FerroObjectDiagnosticExtensions as _;
use ferroui_base::metadata::{IServiceProvider, MarkupType};
use ferroui_base::{BoxedValue, FerroObject, FerroProperty, FerroPropertyRegistry, Ref, StyledElement};
use ferroui_markup_xaml::xaml_il::runtime::compiled::CompiledLoadError;
use ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers;
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, XamlLoadException};
use ferroui_markup_xaml_loader::runtime::framework::XamlMemberException;
use ferroui_markup_xaml_loader::rust_emitter::{generate_file, GeneratedFile};
use xamlx::exceptions::XamlError;

use super::corpus::{DOCUMENTS, EXPECTED_ELIGIBLE, EXPECTED_NOT_ELIGIBLE};
use super::generated;
use crate::support::app::xaml_test_base;
use crate::support::loader::{describe, try_load};

/// The path of the checked-in output of the emitter.
const GENERATED_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/emitter/generated.rs");

/// The path of the checked-in position map of `generated.rs` (docs/porting/xaml.md, 9.3.6).
const POSITION_MAP_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/emitter/generated.map.json");

/// What the emitter writes for the corpus today.
fn generate() -> GeneratedFile {
    generate_file(generated::ASSEMBLY_NAME, generated::ROOT_URI, DOCUMENTS, &RuntimeXamlLoaderConfiguration::new())
}

/// A value in display form: in its untyped form (the contents of a nullable,
/// `null` for none), an object of the object model as its class, a type
/// reference as `typeof(<name>)`.
fn display(value: &BoxedValue) -> String {
    let Some(value) = ferroui_markup_xaml::xaml_il::runtime::compiled::to_untyped(value.clone()) else {
        return "null".to_string();
    };
    if let Some(class) = value.downcast_ref::<&'static ferroui_base::TypeInfo>() {
        return format!("typeof({})", class.full_name());
    }
    if let Some(handle) = value.downcast_ref::<ferroui_base::data::core::ValueType>() {
        return format!("typeof({})", handle.name());
    }
    if let Some(transitions) = value.downcast_ref::<ferroui_base::animation::Transitions>() {
        let items: Vec<String> = transitions.to_vec().iter().map(|transition| transition.debug_display()).collect();
        return format!("[{}]", items.join(", "));
    }
    match ValueTypes::as_object(&*value) {
        Some(inner) => match inner.cast::<ferroui_base::styling::ControlTheme>() {
            Some(theme) => display_control_theme(&theme),
            None => format!("<{}>", inner.get_type().full_name()),
        },
        None => display_enum(&value)
            .or_else(|| display_declared(&value))
            .unwrap_or_else(|| ValueTypes::to_display_string(Some(&value))),
    }
}

/// A value of an enumeration (or a set of flags) in display form: the
/// integer value whose metadata conversion gives an equal value, among the
/// members and their combinations.
fn display_enum(value: &BoxedValue) -> Option<String> {
    use ferroui_base::metadata::{MarkupType, MarkupTypeKind};
    let markup = MarkupType::find_by_handle(value.value_type_id())?;
    if markup.kind != MarkupTypeKind::Enum {
        return None;
    }
    let from_value = markup.enum_from_value?;
    let members: Vec<i64> = markup.enum_members.iter().map(|member| member.value).collect();
    let mut candidates: Vec<i64> = members.clone();
    if markup.is_flags && members.len() <= 16 {
        for mask in 0u32..(1 << members.len()) {
            let combined = (0..members.len()).filter(|bit| mask & (1 << bit) != 0).fold(0i64, |acc, bit| acc | members[bit]);
            candidates.push(combined);
        }
    }
    candidates
        .into_iter()
        .find(|candidate| from_value(*candidate).is_some_and(|converted| converted.any_value_eq(&**value)))
        .map(|found| format!("{}({found})", markup.name))
}

/// A value of a plain (non-object) reference type with metadata in display
/// form: its type and the values of its declared properties, by name, each
/// in plain display form (one level).
fn display_declared(value: &BoxedValue) -> Option<String> {
    use ferroui_base::metadata::{MarkupType, MarkupTypeKind};
    let markup = MarkupType::find_by_handle(value.value_type_id())?;
    if markup.kind != MarkupTypeKind::Class || markup.properties.is_empty() {
        return None;
    }
    let mut properties: Vec<String> = Vec::new();
    for property in markup.properties {
        let Some(get) = property.get else { continue };
        let shown = match get(&[Some(value.clone())]) {
            Ok(Some(read)) => match ValueTypes::as_object(&*read) {
                Some(object) => format!("<{}>", object.get_type().full_name()),
                None => display_enum(&read).unwrap_or_else(|| ValueTypes::to_display_string(Some(&read))),
            },
            Ok(None) => "null".to_string(),
            Err(error) => format!("<error {error}>"),
        };
        properties.push(format!("{}={shown}", property.name));
    }
    properties.sort();
    Some(format!("{} {{{}}}", markup.full_name(), properties.join(", ")))
}

/// A control theme in display form: its target type, the target type of
/// the theme it is based on, and its setters (property and value).
fn display_control_theme(theme: &Ref<ferroui_base::styling::ControlTheme>) -> String {
    use ferroui_base::styling::{Setter, SetterValue};
    let type_name = |class: Option<&'static ferroui_base::TypeInfo>| class.map(|class| class.full_name()).unwrap_or_default();
    let mut setters = Vec::new();
    for setter in theme.setters().to_vec() {
        let text = match setter.as_any().and_then(|any| any.downcast_ref::<Setter>()) {
            Some(setter) => {
                let property = setter.property().map(|property| property.name().to_string()).unwrap_or_default();
                let value = match setter.value() {
                    None => "null".to_string(),
                    Some(SetterValue::Value(value)) => display(&value),
                    Some(SetterValue::Binding(_)) => "<observable>".to_string(),
                    Some(SetterValue::BindingBase(_)) => "<binding>".to_string(),
                    Some(SetterValue::Template(_)) => "<template>".to_string(),
                };
                format!("{property}={value}")
            }
            None => "<setter>".to_string(),
        };
        setters.push(text);
    }
    format!(
        "<ControlTheme {} based on {} [{}]>",
        type_name(theme.target_type()),
        theme.based_on().map(|based_on| type_name(based_on.target_type())).unwrap_or_else(|| "nothing".to_string()),
        setters.join(", ")
    )
}

/// The canonical dump of an object tree: the full name of the class and,
/// for a styled element, whether it is initialised; every registered
/// property (of the class, and attached) that is set, with the priority of
/// its value, and every direct property of the class; every plain property
/// the markup metadata of the class and its base classes declares with a
/// getter, read through that getter; all of them sorted, with the value in
/// display form (an object value is shown as its class) and, for a
/// collection, its items, an object item dumped recursively; for a named
/// element, whether the name scope of the root finds it under its name; the
/// resources of the element (its own dictionary: key and value, a deferred
/// value built); for an element with a name scope of its own, whether that
/// scope is completed; then the logical children of a styled element,
/// recursively.
fn dump(object: &Ref<FerroObject>, root_scope: Option<&Rc<dyn INameScope>>, indent: usize, output: &mut String) {
    let pad = "  ".repeat(indent);
    let class = object.get_type();
    let styled = object.cast::<StyledElement>();
    let initialized = match &styled {
        Some(styled) if styled.is_initialized() => " (initialized)",
        Some(_) => " (not initialized)",
        None => "",
    };
    output.push_str(&format!("{pad}{}{initialized}\n", class.full_name()));

    let registry = FerroPropertyRegistry::instance();
    let mut properties: Vec<&'static FerroProperty> = Vec::new();
    let registered = registry.get_registered(class);
    let attached = registry.get_registered_attached(class);
    for property in registered.iter().chain(attached.iter()) {
        if !property.is_direct() && !properties.iter().any(|known| std::ptr::eq(*known, *property)) {
            properties.push(*property);
        }
    }
    let mut lines: Vec<String> = Vec::new();
    for property in properties {
        if !object.is_set(property) {
            continue;
        }
        let priority = object.get_diagnostic(property).priority();
        let value = display_value(&object.get_value_untyped(property), indent + 1);
        lines.push(format!("{pad}  {}.{} = {value} ({priority:?})\n", property.owner_type().name(), property.name()));
    }
    for property in registry.get_registered_direct(class).iter() {
        let value = display_value(&object.get_value_untyped(property), indent + 1);
        lines.push(format!("{pad}  direct {}.{} = {value}\n", property.owner_type().name(), property.name()));
    }
    let mut current = Some(class);
    while let Some(type_) = current {
        for property in type_.markup().map(|markup| markup.properties).unwrap_or_default() {
            let Some(get) = property.get else { continue };
            let value = match get(&[Some(Rc::new(object.clone()) as BoxedValue)]) {
                Ok(value) => match value {
                    Some(value) => display_value(&value, indent + 1),
                    None => "null".to_string(),
                },
                Err(error) => format!("<error: {error}>"),
            };
            lines.push(format!("{pad}  plain {}.{} = {value}\n", type_.name(), property.name));
        }
        current = type_.base_type();
    }
    lines.sort();
    for line in &lines {
        output.push_str(line);
    }

    // A resource dictionary that is not an element: its resources, as for an element.
    if let Some(dictionary) = object.cast::<ferroui_base::controls::ResourceDictionary>() {
        let mut keys = dictionary.keys();
        keys.sort_by_key(|key| key.to_string());
        for key in keys {
            let value = dictionary.try_get_resource(&key, None).flatten();
            let value = value.as_ref().map(display).unwrap_or_else(|| "null".to_string());
            output.push_str(&format!("{pad}  resource {key} = {value}\n"));
        }
    }
    if let Some(styled) = &styled {
        if let Some(name) = styled.name() {
            let found = root_scope
                .and_then(|scope| scope.find(&name))
                .is_some_and(|found| std::ptr::eq(&*found, &**object));
            output.push_str(&format!("{pad}  named '{name}', found in the scope of the root: {found}\n"));
        }
        // The resources of the element itself, by key, each value as the element finds it
        // (deferred content built).
        if styled.has_resources() {
            let resources = styled.resources();
            let mut keys = resources.keys();
            keys.sort_by_key(|key| key.to_string());
            for key in keys {
                let value = styled.try_get_resource(&key, None).flatten();
                let value = value.as_ref().map(display).unwrap_or_else(|| "null".to_string());
                output.push_str(&format!("{pad}  resource {key} = {value}\n"));
            }
        }
        if let Some(scope) = NameScope::get_name_scope(styled) {
            output.push_str(&format!("{pad}  has a name scope, completed: {}\n", scope.0.is_completed()));
        }
        for child in styled.logical_children().to_vec() {
            dump(&child.upcast::<FerroObject>(), root_scope, indent + 1, output);
        }
    }
}

/// A value in display form ([`display`]) and, when it is a collection, its items: an
/// object item dumped recursively at `indent`, any other item in display form.
fn display_value(value: &BoxedValue, indent: usize) -> String {
    let shown = display(value);
    let Some(items) = collection_items(value) else {
        return shown;
    };
    let items = match items {
        Ok(items) => items,
        Err(error) => return format!("{shown} <{error}>"),
    };
    let pad = "  ".repeat(indent + 1);
    let mut text = format!("{shown} with {} items", items.len());
    for item in items {
        match item.as_ref().and_then(|item| ValueTypes::as_object(&**item)) {
            Some(object) if indent < 8 => {
                let mut inner = String::new();
                dump(&object, None, indent + 1, &mut inner);
                text.push('\n');
                text.push_str(inner.trim_end_matches('\n'));
            }
            _ => text.push_str(&format!("\n{pad}{}", item.as_ref().map(display).unwrap_or_else(|| "null".to_string()))),
        }
    }
    text
}

/// Whether the type with markup metadata `markup`, or one of its base types, declares
/// an `Add` method: the collections markup adds items to.
fn declares_add(markup: Option<&'static MarkupType>) -> bool {
    let mut current = markup;
    while let Some(type_) = current {
        if type_.methods.iter().any(|method| method.name == "Add") {
            return true;
        }
        current = type_.base_type();
    }
    false
}

/// The items of `value` when it is a collection markup adds to (its type declares `Add`):
/// `Some(Ok(items))`, or `Some(Err(..))` for a collection this harness cannot enumerate,
/// so that a difference in the contents of such a collection can never compare as equal.
/// `None` for any other value.
fn collection_items(value: &BoxedValue) -> Option<Result<Vec<Option<BoxedValue>>, String>> {
    use ferroui_base::collections::FerroList;
    use ferroui_base::controls::Classes;
    use ferroui_base::styling::Styles;
    use ferroui_controls::documents::{Inline, InlineCollection};
    use ferroui_controls::templates::{DataTemplates, IDataTemplate};
    use ferroui_controls::{ColumnDefinition, ColumnDefinitions, Control, Controls, ItemCollection, RowDefinition, RowDefinitions};

    let markup = match ValueTypes::as_object(&**value) {
        Some(object) => object.get_type().markup(),
        None => MarkupType::find_by_handle(value.value_type_id()),
    };
    if !declares_add(markup) {
        return None;
    }
    fn objects<T: ferroui_base::ObjectType>(items: Vec<Ref<T>>) -> Vec<Option<BoxedValue>> {
        items.into_iter().map(|item| Some(Rc::new(item.upcast::<FerroObject>()) as BoxedValue)).collect()
    }
    let any = &**value;
    if let Some(list) = any.downcast_ref::<Controls>() {
        return Some(Ok(objects(list.to_vec())));
    }
    if let Some(list) = any.downcast_ref::<FerroList<Ref<Control>>>() {
        return Some(Ok(objects(list.to_vec())));
    }
    if let Some(list) = any.downcast_ref::<RowDefinitions>() {
        return Some(Ok(objects::<RowDefinition>(list.to_vec())));
    }
    if let Some(list) = any.downcast_ref::<ColumnDefinitions>() {
        return Some(Ok(objects::<ColumnDefinition>(list.to_vec())));
    }
    if let Some(list) = any.downcast_ref::<InlineCollection>() {
        return Some(Ok(objects::<Inline>(list.to_vec())));
    }
    if let Some(list) = any.downcast_ref::<FerroList<Ref<Inline>>>() {
        return Some(Ok(objects(list.to_vec())));
    }
    if let Some(list) = any.downcast_ref::<ItemCollection>() {
        return Some(Ok(list.to_vec()));
    }
    if let Some(classes) = any.downcast_ref::<Classes>() {
        return Some(Ok((0..classes.count()).map(|index| Some(Rc::new(classes.get(index)) as BoxedValue)).collect()));
    }
    if let Some(templates) = any.downcast_ref::<DataTemplates>() {
        let items: Vec<Rc<dyn IDataTemplate>> = templates.to_vec();
        return Some(Ok(items.into_iter().map(|item| Some(Rc::new(item) as BoxedValue)).collect()));
    }
    if let Some(styles) = ValueTypes::as_object(any).and_then(|object| object.cast::<Styles>()) {
        return Some(Ok((0..styles.count()).map(|index| Some(Rc::new(styles.get(index)) as BoxedValue)).collect()));
    }
    Some(Err(format!("a collection of type {} the harness cannot enumerate", any.type_name())))
}

fn dump_root(root: &BoxedValue) -> String {
    let mut output = String::new();
    match ValueTypes::as_object(&**root) {
        Some(object) => {
            let scope = object.cast::<StyledElement>().and_then(|styled| NameScope::get_name_scope(&styled)).map(|scope| scope.0);
            dump(&object, scope.as_ref(), 0, &mut output);
        }
        None => output.push_str("<the root is not an object of the object model>\n"),
    }
    output
}

/// The first line the two dumps differ in.
fn first_difference(interpreted: &str, generated: &str) -> String {
    let mut left = interpreted.lines();
    let mut right = generated.lines();
    let mut line = 1;
    loop {
        match (left.next(), right.next()) {
            (None, None) => return "no difference".to_string(),
            (a, b) if a == b => line += 1,
            (a, b) => {
                return format!(
                    "line {line}: interpreter `{}`, generated `{}`",
                    a.unwrap_or("<end>"),
                    b.unwrap_or("<end>")
                )
            }
        }
    }
}

/// The type name of the exception a failed load reports: the type of the error of the
/// compiler the run-time loader wraps (`XamlError::type_name`, a failed member being a
/// `TargetInvocationException` that wraps a [`XamlMemberException`]), or the type a step of
/// generated code names ([`CompiledLoadError`]). `None` for an error without either.
fn exception_type(error: &XamlLoadException) -> Option<&'static str> {
    let inner = error.inner_exception()?;
    if let Some(error) = inner.downcast_ref::<XamlError>() {
        return Some(error.type_name());
    }
    if inner.downcast_ref::<XamlMemberException>().is_some() {
        return Some("TargetInvocationException");
    }
    inner.downcast_ref::<CompiledLoadError>().map(CompiledLoadError::type_name)
}

/// A failed load as the harness compares it: the exception type and the message, which
/// carries the position.
fn failure(error: &XamlLoadException) -> String {
    format!("<error {}: {}>\n", exception_type(error).unwrap_or("<no exception type>"), error.message())
}

fn build_generated(name: &str) -> Option<Result<BoxedValue, XamlLoadException>> {
    let (_, build) = generated::DOCUMENTS.iter().find(|(document, _)| *document == name)?;
    // What the run-time loader gives `Build`: a root service provider with a fresh name scope.
    let provider: Rc<dyn IServiceProvider> = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    Some(build(Some(provider)))
}

#[test]
fn generated_output_is_up_to_date() {
    let _base = xaml_test_base();
    let file = generate();
    let checked_in = include_str!("generated.rs");
    if file.source != checked_in {
        panic!(
            "emitter/generated.rs is not what the emitter writes for the corpus ({}).\n\
             Regenerate it:\n  cargo test -p ferroui-markup-xaml-tests --lib \
             emitter::differential_tests::regenerate_emitter_output -- --ignored --exact",
            first_difference(checked_in, &file.source)
        );
    }
    let map = include_str!("generated.map.json");
    assert!(
        file.position_map == map,
        "emitter/generated.map.json is not the position map of the emitter's output ({}); regenerate it as above",
        first_difference(map, &file.position_map)
    );
}

/// Rewrites `emitter/generated.rs` with the emitter's output for the corpus.
#[test]
#[ignore = "writes emitter/generated.rs; run it to regenerate the checked-in output"]
fn regenerate_emitter_output() {
    let _base = xaml_test_base();
    let file = generate();
    for (name, reason) in &file.documents {
        match reason {
            None => println!("eligible      {name}"),
            Some(reason) => println!("not eligible  {name}: {reason}"),
        }
    }
    std::fs::write(GENERATED_PATH, file.source).expect("emitter/generated.rs can be written");
    std::fs::write(POSITION_MAP_PATH, file.position_map).expect("emitter/generated.map.json can be written");
}

/// Not from upstream. The emitter's output depends only on the documents and
/// the declarations, never on what ran earlier on the thread: the corpus
/// compiled on a fresh thread is the corpus compiled on a thread that first
/// initialised every registered type (in reverse order of registration),
/// loaded every document through the run-time loader and compiled the
/// corpus once already.
#[test]
fn output_does_not_depend_on_what_ran_before() {
    let fresh = std::thread::spawn(|| {
        let _base = xaml_test_base();
        generate().source
    })
    .join()
    .expect("the fresh thread compiles the corpus");
    let after = std::thread::spawn(|| {
        let _base = xaml_test_base();
        for type_ in ferroui_base::TypeInfo::registered_types().into_iter().rev() {
            type_.ensure_class_init();
        }
        for (_, xaml) in DOCUMENTS {
            let _ = try_load(xaml);
        }
        let _ = generate();
        generate().source
    })
    .join()
    .expect("the second thread compiles the corpus");
    assert!(fresh == after, "the output differs: {}", first_difference(&fresh, &after));
    assert_eq!(fresh, include_str!("generated.rs"));
}

#[test]
fn eligibility_of_the_corpus_is_as_expected() {
    let _base = xaml_test_base();
    let file = generate();
    let mut wrong = Vec::new();
    let mut eligible = 0;
    for (name, reason) in &file.documents {
        match reason {
            None => {
                eligible += 1;
                println!("eligible      {name}");
                if EXPECTED_NOT_ELIGIBLE.contains(&name.as_str()) {
                    wrong.push(format!("{name} is eligible but must not be"));
                }
            }
            Some(reason) => {
                println!("not eligible  {name}: {reason}");
                if EXPECTED_ELIGIBLE.contains(&name.as_str()) {
                    wrong.push(format!("{name} must be eligible: {reason}"));
                }
            }
        }
    }
    println!("{eligible} of {} documents are eligible", file.documents.len());
    assert!(wrong.is_empty(), "eligibility differs from the corpus:\n{}", wrong.join("\n"));
}

#[test]
fn both_back_ends_build_equal_object_trees() {
    let _base = xaml_test_base();
    let (mut matches, mut mismatches, mut not_eligible) = (0, Vec::new(), 0);
    for (name, xaml) in DOCUMENTS {
        let Some(built) = build_generated(name) else {
            not_eligible += 1;
            println!("not eligible  {name}");
            continue;
        };
        // A failed load is compared by its exception type and its message, which carries
        // the position.
        let interpreted = match try_load(xaml) {
            Ok(root) => dump_root(&root),
            Err(error) => failure(&error),
        };
        let generated = match built {
            Ok(root) => dump_root(&root),
            Err(error) => failure(&error),
        };
        // `FERROUI_EMITTER_DUMP=<name>` prints the dump of a document (diagnostic).
        if std::env::var("FERROUI_EMITTER_DUMP").is_ok_and(|dumped| dumped == *name) {
            println!("--- {name}, interpreter\n{interpreted}--- {name}, generated\n{generated}");
        }
        if interpreted == generated {
            matches += 1;
            println!("match         {name}");
        } else {
            let difference = first_difference(&interpreted, &generated);
            println!("MISMATCH      {name}: {difference}");
            mismatches.push(format!("{name}: {difference}\n--- interpreter\n{interpreted}--- generated\n{generated}"));
        }
    }
    println!("{matches} match, {} mismatch, {not_eligible} not eligible", mismatches.len());
    assert!(mismatches.is_empty(), "the back ends differ:\n{}", mismatches.join("\n"));
    assert!(matches > 0 || generated::DOCUMENTS.is_empty(), "no document was compared");
}

/// Not from upstream. A failed build of generated code reports the exception type the
/// run-time loader reports, not only its message: a duplicate name is the
/// `ArgumentException` of the name scope in both back ends.
#[test]
fn a_failed_build_reports_the_exception_type_of_the_run_time_loader() {
    let _base = xaml_test_base();
    let name = "duplicate_name.xaml";
    let (_, xaml) = DOCUMENTS.iter().find(|(document, _)| *document == name).expect("a corpus document");
    let interpreted = try_load(xaml).err().expect("the run-time loader fails");
    let generated = build_generated(name).expect("the document is generated").err().expect("the generated build fails");
    assert_eq!(exception_type(&interpreted), Some("ArgumentException"));
    assert_eq!(exception_type(&generated), Some("ArgumentException"));
    assert_eq!(failure(&generated), failure(&interpreted));
}

/// Both back ends' failure of the corpus document `name`, which must fail.
fn failures_of(name: &str) -> (XamlLoadException, XamlLoadException) {
    let (_, xaml) = DOCUMENTS.iter().find(|(document, _)| *document == name).expect("a corpus document");
    let interpreted = try_load(xaml).err().expect("the run-time loader fails");
    let generated = build_generated(name).expect("the document is generated").err().expect("the generated build fails");
    (interpreted, generated)
}

/// Not from upstream. An error below the first line of a document carries its line in
/// both back ends: the second `x:Name='same'` is on line 4.
#[test]
fn a_failure_below_the_first_line_reports_its_line() {
    let _base = xaml_test_base();
    let (interpreted, generated) = failures_of("multiline_duplicate_name.xaml");
    assert_eq!(failure(&generated), failure(&interpreted));
    assert!(generated.message().contains("Line 4, position"), "{}", generated.message());
    assert!(generated.message().ends_with(")") && generated.message().contains("(line 4 position"), "{}", generated.message());
}

/// Not from upstream. A control whose `EndInit` fails fails the build where the
/// run-time loader fails, as the exception that wraps the failure of the member, at the
/// position of the control (line 3).
#[test]
fn a_failed_end_init_is_the_error_of_the_run_time_loader() {
    let _base = xaml_test_base();
    let (interpreted, generated) = failures_of("end_init_failure.xaml");
    assert_eq!(exception_type(&generated), Some("TargetInvocationException"));
    assert_eq!(failure(&generated), failure(&interpreted));
    assert!(
        generated.message().starts_with("Duplicate setter encountered for property 'Tag' in 'FailingEndInit'."),
        "{}",
        generated.message()
    );
    assert!(generated.message().contains("(line 3 position"), "{}", generated.message());
}

#[test]
fn compiled_documents_are_registered_by_uri() {
    use ferroui_base::platform::{AssetAssembly, IAssetLoader, StandardAssetLoader};
    use ferroui_base::utilities::{Uri, UriKind};
    use ferroui_base::FerroLocator;
    use ferroui_markup_xaml::FerroXamlLoader;

    let _base = xaml_test_base();
    let Some((name, _)) = generated::DOCUMENTS.first() else {
        panic!("generated.rs holds no document");
    };
    let uri = format!("{}{name}", generated::ROOT_URI);

    // The loader of the assembly finds the document by its URI, whatever its case, and
    // nothing else.
    let direct = generated::try_load(None, &uri.to_uppercase().replace("FERRES://", "ferres://"));
    assert!(matches!(direct, Ok(Some(_))), "{uri} is not found by the generated loader");
    assert!(matches!(generated::try_load(None, &format!("{}missing.xaml", generated::ROOT_URI)), Ok(None)));
    assert!(matches!(generated::try_load(None, "ferres://Other.Assembly/border_empty.xaml"), Ok(None)));

    // Through the loader of the runtime library: it asks the asset loader which assembly
    // the URI belongs to, and that assembly's registered loader for the document.
    let _locator_scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<dyn IAssetLoader>()
        .to_constant(Rc::new(StandardAssetLoader::new(Some(&AssetAssembly::new(generated::ASSEMBLY_NAME)))));
    generated::register_compiled_xaml();
    let parsed = Uri::new(&uri, UriKind::Absolute).expect("an absolute URI");
    let loaded = FerroXamlLoader::try_load_with_service_provider(None, &parsed, None);
    ferroui_markup_xaml::FerroXamlLoader::unregister_compiled_xaml(generated::ASSEMBLY_NAME);
    match loaded {
        Ok(root) => {
            let expected = build_generated(name).expect("the document is generated").expect("it builds");
            assert_eq!(dump_root(&root), dump_root(&expected));
        }
        Err(error) => panic!("{uri} did not load through FerroXamlLoader: {}", describe(&error)),
    }
}

/// Not from upstream. A compiled document that fails to build is an error of the load
/// through the compiled loader, as it is through the run-time loader: the same exception
/// type and message, and no panic.
#[test]
fn a_compiled_document_that_fails_to_build_fails_the_load() {
    use ferroui_base::platform::{AssetAssembly, IAssetLoader, StandardAssetLoader};
    use ferroui_base::utilities::{Uri, UriKind};
    use ferroui_base::FerroLocator;
    use ferroui_markup_xaml::FerroXamlLoader;

    let _base = xaml_test_base();
    let name = "duplicate_name.xaml";
    let (_, xaml) = DOCUMENTS.iter().find(|(document, _)| *document == name).expect("a corpus document");
    let interpreted = try_load(xaml).err().expect("the run-time loader fails");
    let uri = format!("{}{name}", generated::ROOT_URI);

    let direct = generated::try_load(None, &uri).err().expect("the generated loader fails");
    assert_eq!(failure(&direct), failure(&interpreted));

    let _locator_scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<dyn IAssetLoader>()
        .to_constant(Rc::new(StandardAssetLoader::new(Some(&AssetAssembly::new(generated::ASSEMBLY_NAME)))));
    generated::register_compiled_xaml();
    let parsed = Uri::new(&uri, UriKind::Absolute).expect("an absolute URI");
    let loaded = FerroXamlLoader::try_load_with_service_provider(None, &parsed, None);
    FerroXamlLoader::unregister_compiled_xaml(generated::ASSEMBLY_NAME);
    let error = loaded.err().expect("the load through FerroXamlLoader fails");
    assert_eq!(failure(&error), failure(&interpreted));
}

/// Prints the transformed tree the emitter walks for the corpus document named by the
/// environment variable `FERROUI_EMITTER_DOCUMENT`: a diagnostic for a document that is
/// not eligible.
///
/// ```text
/// FERROUI_EMITTER_DOCUMENT=border_child.xaml cargo test -p ferroui-markup-xaml-tests --lib \
///     emitter::differential_tests::print_transformed_tree -- --ignored --exact --nocapture
/// ```
#[test]
#[ignore = "a diagnostic: prints the transformed tree of one corpus document"]
fn print_transformed_tree() {
    let _base = xaml_test_base();
    let Ok(name) = std::env::var("FERROUI_EMITTER_DOCUMENT") else {
        println!("FERROUI_EMITTER_DOCUMENT names no document");
        return;
    };
    let Some((_, xaml)) = DOCUMENTS.iter().find(|(document, _)| *document == name) else {
        println!("{name} is not a document of the corpus");
        return;
    };
    match ferroui_markup_xaml_loader::rust_emitter::transformed_tree(&name, xaml, &RuntimeXamlLoaderConfiguration::new()) {
        Ok(tree) => println!("{tree}"),
        Err(error) => println!("{name} does not transform: {error}"),
    }
}

/// Not from upstream. A list built from text (`RowDefinitions='Auto,*'`) is created once:
/// its capacity, every `Add` and the property setter receive the same list, so the compiled
/// grid has the definitions the text states, as the run-time loader's has.
#[test]
fn a_list_built_from_text_is_one_list() {
    use ferroui_controls::Grid;

    let _base = xaml_test_base();
    let built = build_generated("grid_definitions_text.xaml").expect("the document is generated").expect("it builds");
    let grid = ValueTypes::as_object(&*built).and_then(|object| object.cast::<Grid>()).expect("the root is a grid");
    assert_eq!(grid.row_definitions().count(), 2);
    assert_eq!(grid.column_definitions().count(), 3);
}

/// Not from upstream. The position map leads from a generated line back to the XAML node it
/// was emitted for: the line that creates the `TextBlock` of `multiline_document.xaml` (on
/// line 5 of the document) is in a range of that document's line 5.
#[test]
fn the_position_map_leads_back_to_the_document() {
    let source = include_str!("generated.rs");
    let map = include_str!("generated.map.json");
    let (index, _) = source
        .lines()
        .enumerate()
        .skip_while(|(_, line)| !line.starts_with("pub fn build_multiline_document_xaml("))
        .find(|(_, line)| line.contains("::ferroui_controls::TextBlock::new()"))
        .expect("the generated build function of multiline_document.xaml creates a TextBlock");
    let number = index + 1;
    let entry = map
        .lines()
        .filter(|line| line.contains("\"document\": \"multiline_document.xaml\""))
        .find(|line| {
            let range = line.split("[").nth(1).and_then(|rest| rest.split(']').next()).unwrap_or_default();
            let (first, last) = range.split_once(", ").unwrap_or_default();
            first.parse::<usize>().is_ok_and(|first| first <= number) && last.parse::<usize>().is_ok_and(|last| number <= last)
        })
        .expect("the line is in a range of the map");
    assert!(entry.contains("\"line\": 5,"), "{entry}");
}
