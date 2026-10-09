//! A method of the class of a document named in markup (docs/porting/xaml.md, 9.4.4): the
//! handler of a routed event, the handler of an event that is not one, a handler with
//! wider parameters than the delegate passes, a method as the value of a property of a
//! delegate type, and a handler named inside a template.
//!
//! The documents are the class documents of the corpus (`corpus::CLASS_DOCUMENTS`). Each
//! is compiled as the document of its class by both hosts of the emitter, against the
//! run-time type system (the registries of the process) and against the build-time type
//! system (the scanned models), and the two texts are compared byte for byte; the text is
//! checked in (`generated_handlers/<module>.rs`) and compiled with this crate, and a class
//! populated by it is compared with a class populated by the run-time loader: the same
//! object tree, and the same handler called when the event is raised.
//!
//! The documents of `corpus::MISSING_METHOD_DOCUMENTS` name a method that is not found:
//! neither host compiles them, and both report the diagnostic that names the document, the
//! member and the method.
//!
//! To regenerate the checked-in files:
//!
//! ```text
//! cargo test -p ferroui-markup-xaml-tests --lib emitter::event_handlers::regenerate_class_documents -- --ignored --exact
//! ```
//!
//! Not from upstream, whose compiler has one back end.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::input::{InputElement, KeyModifiers, Pointer, PointerEventArgs, PointerPointProperties, PointerType, TappedEventArgs};
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};
use ferroui_base::{ObjectType, Point, Ref};
use ferroui_build::type_system::{ModelEmitTypes, ModelTypeSystem};
use ferroui_controls::primitives::{PopupFlyoutBase, TemplatedControl};
use ferroui_controls::{Button, Control, ToolTip};
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use ferroui_markup_xaml_loader::rust_emitter::{
    generate_class_file_with, generate_file_with, ClassGroup, DiagnosticHandler, EmitterHost, TransformOptions, GROUP_NOT_TRANSFORMED,
};
use xamlx::{XamlDiagnostic, XamlDiagnosticSeverity};

use super::corpus::{ClassDocument, MissingMethodDocument, CLASS_DOCUMENTS, MISSING_METHOD_DOCUMENTS};
use super::differential_tests::dump_root;
use super::generated_handlers;
use super::model_transform::scanned_models;
use crate::support::app::xaml_test_base;
use crate::support::helpers::boxed;
use crate::support::loader::load_with_root;
use crate::support::xaml::event_tests::{MyButton, MyHost, MyPanel};

/// The URI of a document is this followed by its name.
const ROOT_URI: &str = "ferres://FerroUI.Markup.Xaml.UnitTests/";

/// The directory of the checked-in output.
const GENERATED_DIRECTORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/emitter/generated_handlers");

/// The options of a compile of a document of this crate, with the diagnostics it reports
/// kept in `diagnostics`.
fn options(diagnostics: &Rc<RefCell<Vec<XamlDiagnostic>>>) -> TransformOptions {
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&crate::ASSEMBLY);
    let collected = diagnostics.clone();
    let handler: DiagnosticHandler = Rc::new(move |diagnostic: &XamlDiagnostic| {
        collected.borrow_mut().push(diagnostic.clone());
        diagnostic.severity
    });
    TransformOptions { diagnostic_handler: Some(handler), ..TransformOptions::of(&configuration) }
}

/// What a host makes of a document: the generated file, or why there is none, and the
/// diagnostics of the compile.
struct Compiled {
    source: Result<String, String>,
    diagnostics: Vec<XamlDiagnostic>,
}

/// `name` compiled by `host`: as the document of `class` when it has one, else as a
/// document without a class.
fn compile(host: &EmitterHost<'_>, name: &str, class: Option<&str>, module: &str, xaml: &str) -> Compiled {
    let diagnostics: Rc<RefCell<Vec<XamlDiagnostic>>> = Rc::default();
    let options = options(&diagnostics);
    let source = match class {
        Some(class) => {
            let class_type = host.type_system.find_type(class).unwrap_or_else(|| panic!("the host has no type {class}"));
            let documents = [(name.to_string(), xaml.to_string(), Some(format!("{ROOT_URI}{name}")))];
            let module_path = format!("::ferroui_markup_xaml_tests::emitter::generated_handlers::{module}");
            let group = ClassGroup { class: class_type, documents: &documents, constructor: None, module_path: &module_path };
            generate_class_file_with(host, &group, &options).map(|file| file.source)
        }
        None => {
            let file = generate_file_with(host, crate::ASSEMBLY.name, ROOT_URI, &[(name, xaml)], &options);
            match file.documents.first().and_then(|(_, reason)| reason.clone()) {
                Some(reason) => Err(reason),
                None => Ok(file.source),
            }
        }
    };
    let diagnostics = diagnostics.borrow().clone();
    Compiled { source, diagnostics }
}

/// Runs `with` with the two hosts of the emitter: the run-time type system of the thread,
/// and the build-time type system over the scanned models of the framework crates and of
/// this crate.
fn with_hosts<R>(with: impl FnOnce(&EmitterHost<'_>, &EmitterHost<'_>) -> R) -> R {
    let runtime = EmitterHost::runtime(&[]).unwrap_or_else(|error| panic!("the run-time type system: {}", error.message()));
    let system = ModelTypeSystem::new(scanned_models());
    let types = ModelEmitTypes::new(system.clone(), Vec::new());
    let model = EmitterHost { type_system: system.as_type_system(), types: &types, parsers: Vec::new() };
    with(&runtime, &model)
}

fn generated_path(document: &ClassDocument) -> String {
    format!("{GENERATED_DIRECTORY}/{}.rs", document.module)
}

fn first_difference(expected: &str, found: &str) -> String {
    let line = expected
        .lines()
        .zip(found.lines())
        .position(|(expected, found)| expected != found)
        .unwrap_or_else(|| expected.lines().count().min(found.lines().count()));
    let show = |text: &str| text.lines().nth(line).unwrap_or("(the text ends)").trim().to_string();
    format!("line {}: `{}` and `{}`", line + 1, show(expected), show(found))
}

/// Both hosts write the same file for every class document of the corpus, and it is the
/// checked-in one.
#[test]
fn class_documents_are_emitted_the_same_by_both_hosts() {
    let _base = xaml_test_base();
    with_hosts(|runtime, model| {
        for document in CLASS_DOCUMENTS {
            let at_run_time = compile(runtime, document.name, Some(document.class), document.module, document.xaml);
            let on_models = compile(model, document.name, Some(document.class), document.module, document.xaml);
            let at_run_time = at_run_time.source.unwrap_or_else(|reason| panic!("{} is not eligible at run time: {reason}", document.name));
            let on_models = on_models.source.unwrap_or_else(|reason| panic!("{} is not eligible against the models: {reason}", document.name));
            assert!(
                at_run_time == on_models,
                "{} is emitted differently against the models ({})",
                document.name,
                first_difference(&at_run_time, &on_models)
            );
            let path = generated_path(document);
            let checked_in = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path} cannot be read: {error}"));
            assert!(
                checked_in == at_run_time,
                "{path} is not what the emitter writes for {} ({}).\nRegenerate it:\n  cargo test -p ferroui-markup-xaml-tests --lib \
                 emitter::event_handlers::regenerate_class_documents -- --ignored --exact",
                document.name,
                first_difference(&checked_in, &at_run_time)
            );
        }
    });
}

/// Rewrites `emitter/generated_handlers/<module>.rs` with the emitter's output for the class
/// documents of the corpus.
#[test]
#[ignore = "writes emitter/generated_handlers; run it to regenerate the checked-in output"]
fn regenerate_class_documents() {
    let _base = xaml_test_base();
    with_hosts(|runtime, model| {
        for document in CLASS_DOCUMENTS {
            let at_run_time = compile(runtime, document.name, Some(document.class), document.module, document.xaml);
            match at_run_time.source {
                Ok(source) => {
                    println!("eligible      {}", document.name);
                    std::fs::write(generated_path(document), source).expect("the generated file can be written");
                }
                Err(reason) => println!("not eligible  {}: {reason}", document.name),
            }
            if let Err(reason) = compile(model, document.name, Some(document.class), document.module, document.xaml).source {
                println!("not eligible against the models  {}: {reason}", document.name);
            }
        }
    });
}

/// The diagnostics of a compile that are errors, each as its code, document, line, position
/// and text.
fn errors(compiled: &Compiled) -> Vec<(String, Option<String>, Option<i32>, Option<i32>, String)> {
    compiled
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity >= XamlDiagnosticSeverity::Error)
        .map(|diagnostic| {
            (diagnostic.code.clone(), diagnostic.document.clone(), diagnostic.line_number, diagnostic.line_position, diagnostic.title.clone())
        })
        .collect()
}

/// A method name no method of the root object answers to is an error of the compile, the
/// same through both hosts: its code, the document, the place of the name in it, and a text
/// that names the method and the member it is named for.
#[test]
fn a_method_that_is_not_found_is_reported_with_the_document_the_member_and_the_method() {
    let _base = xaml_test_base();
    with_hosts(|runtime, model| {
        for MissingMethodDocument { name, class, xaml, diagnostic } in MISSING_METHOD_DOCUMENTS {
            let at_run_time = compile(runtime, name, *class, "missing", xaml);
            let on_models = compile(model, name, *class, "missing", xaml);
            let reason = at_run_time.source.as_ref().expect_err("a document that names a method that is not found is not compiled");
            assert_eq!(on_models.source.as_ref().err(), Some(reason), "{name}: the hosts refuse the document differently");
            assert!(reason.contains(diagnostic), "{name}: {reason}");
            if class.is_none() {
                assert!(reason.starts_with(GROUP_NOT_TRANSFORMED), "{name}: {reason}");
            }

            let found = errors(&at_run_time);
            assert_eq!(found, errors(&on_models), "{name}: the hosts report other diagnostics");
            let [(code, document, line, position, text)] = found.as_slice() else {
                panic!("{name}: one error is expected, found {found:?}");
            };
            assert_eq!(code, "FRN3000", "{name}");
            assert_eq!(document.as_deref(), Some(*name));
            assert!(text.starts_with(diagnostic), "{name}: {text}");
            // The place is the attribute that names the method.
            let method = diagnostic.strip_prefix("No method `").and_then(|rest| rest.split('`').next()).expect("the method of the diagnostic");
            let value = xaml.find(&format!("='{method}'")).expect("the method is named in the document");
            let attribute = xaml[..value].rfind(' ').expect("the attribute of the member");
            assert_eq!((*line, *position), (Some(1), Some(attribute as i32 + 2)), "{name}: {text}");
        }
    });
}

// --- The generated code, run -------------------------------------------------

fn document(module: &str) -> &'static ClassDocument {
    CLASS_DOCUMENTS.iter().find(|document| document.module == module).expect("a class document of the corpus")
}

/// An instance of the class of a document populated by the generated code, and one
/// populated by the run-time loader from the same document: the two object trees are the
/// same.
fn populated<T: ObjectType>(
    module: &str,
    new: fn() -> Ref<T>,
    populate: fn(Option<Rc<dyn ferroui_base::metadata::IServiceProvider>>, &Ref<T>) -> Result<(), ferroui_markup_xaml::XamlLoadException>,
) -> (Ref<T>, Ref<T>) {
    let document = document(module);
    let compiled = new();
    populate(None, &compiled).unwrap_or_else(|error| panic!("{}: the generated code fails: {error}", document.name));
    let loaded = new();
    load_with_root(document.xaml, Some(&crate::ASSEMBLY), boxed(loaded.clone()));
    let (from_code, from_loader) = (dump_root(&boxed(compiled.clone())), dump_root(&boxed(loaded.clone())));
    assert!(from_code == from_loader, "{}: the trees differ ({})", document.name, first_difference(&from_loader, &from_code));
    (compiled, loaded)
}

/// The arguments of a double tap on `target`.
fn double_tapped(target: &Ref<Button>) -> TappedEventArgs {
    let pointer_event = PointerEventArgs::new(
        None::<&RoutedEvent<PointerEventArgs>>,
        target,
        Pointer::new(0, PointerType::Mouse, true),
        None,
        Point::default(),
        0,
        PointerPointProperties::default(),
        KeyModifiers::NONE,
    );
    TappedEventArgs::new(Some(InputElement::double_tapped_event()), &pointer_event)
}

/// A routed event of the element and a routed event of another class: the method of the
/// class is called when the event is raised, and the handler does not keep the object
/// alive.
#[test]
fn routed_event_calls_the_method_of_the_class() {
    let _base = xaml_test_base();
    let (compiled, loaded) = populated("routed_event", MyButton::new, generated_handlers::routed_event::populate);
    for target in [compiled, loaded] {
        assert!(!target.was_clicked() && !target.was_tapped());
        target.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert!(target.was_clicked() && !target.was_tapped());
        target.raise_event(&RoutedEventArgs::with_event(InputElement::tapped_event()));
        assert!(target.was_tapped());

        // The object holds the handler, the handler holds the object weakly.
        let weak = target.downgrade();
        drop(target);
        assert!(weak.upgrade().is_none(), "the handler keeps its object alive");
    }
}

/// A routed event of a child handled on the root, and an event with arguments of a class
/// of their own handled on an element between them.
#[test]
fn attached_event_calls_the_method_of_the_class() {
    let _base = xaml_test_base();
    let (compiled, loaded) = populated("attached_event", MyPanel::new, generated_handlers::attached_event::populate);
    for host in [compiled, loaded] {
        let target = host.get_control::<Button>("target");
        target.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert!(host.was_clicked() && !host.was_tapped());
        target.raise_event(&double_tapped(&target));
        assert!(host.was_tapped());

        // A handler whose object is gone does nothing.
        let weak = host.downgrade();
        drop(host);
        assert!(weak.upgrade().is_none(), "the handlers keep their object alive");
        target.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    }
}

/// Events that are not routed events: the handler with wider parameters receives the
/// arguments the event raises, the handler of the property-changed event its arguments.
#[test]
fn plain_event_calls_the_method_of_the_class() {
    let _base = xaml_test_base();
    let (compiled, loaded) = populated("plain_event", MyHost::new, generated_handlers::plain_event::populate);
    for host in [compiled, loaded] {
        assert_eq!(host.openings(), 0);
        // The handler cancels the arguments it receives, which are cancellable ones.
        assert!(host.raise_opening());
        assert_eq!(host.openings(), 1);

        let target = host.get_control::<Control>("target");
        let before = host.changed_properties().len();
        target.set_opacity(0.5);
        assert_eq!(host.changed_properties()[before..], ["Opacity".to_string()]);
    }
}

/// A method named as the value of a property of a delegate type is the callback of the
/// property, and the handler of the event of a flyout is attached.
#[test]
fn delegate_property_holds_the_method_of_the_class() {
    let _base = xaml_test_base();
    let (compiled, loaded) = populated("delegate_property", MyHost::new, generated_handlers::delegate_property::populate);
    for host in [compiled, loaded] {
        let target = host.get_control::<Control>("target");
        assert!(ToolTip::get_custom_popup_placement_callback(&target).is_some());
        let flyout = host.get_control::<Button>("button").flyout().expect("the flyout of the button");
        let flyout = flyout.cast::<PopupFlyoutBase>().expect("a popup flyout");
        assert!(flyout.custom_popup_placement_callback().is_some());
        assert_eq!((host.placements(), host.openings()), (0, 0));
    }
}

/// A handler named inside a template: the content the template builds calls the method of
/// the root object of the document.
#[test]
fn event_of_a_template_calls_the_method_of_the_class() {
    let _base = xaml_test_base();
    let (compiled, loaded) = populated("template_event", MyPanel::new, generated_handlers::template_event::populate);
    for host in [compiled, loaded] {
        let target = host.get_control::<Button>("target");
        let template = target.template().expect("the template of the button");
        let built = template.build(target.upcast_ref::<TemplatedControl>()).expect("the template builds its content");
        let (part, _) = built.deconstruct();
        assert!(!host.was_tapped());
        part.raise_event(&RoutedEventArgs::with_event(InputElement::tapped_event()));
        assert!(host.was_tapped() && !host.was_clicked());
    }
}
