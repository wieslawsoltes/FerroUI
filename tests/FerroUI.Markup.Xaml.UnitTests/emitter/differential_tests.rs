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
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::{BoxedValue, FerroObject, FerroProperty, FerroPropertyRegistry, Ref, StyledElement};
use ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers;
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use ferroui_markup_xaml_loader::rust_emitter::{generate_file, GeneratedFile};

use super::corpus::{DOCUMENTS, EXPECTED_ELIGIBLE, EXPECTED_NOT_ELIGIBLE};
use super::generated;
use crate::support::app::xaml_test_base;
use crate::support::loader::{describe, try_load};

/// The path of the checked-in output of the emitter.
const GENERATED_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/emitter/generated.rs");

/// What the emitter writes for the corpus today.
fn generate() -> GeneratedFile {
    generate_file(generated::ASSEMBLY_NAME, generated::ROOT_URI, DOCUMENTS, &RuntimeXamlLoaderConfiguration::new())
}

/// A value in display form: an object of the object model as its class.
fn display(value: &BoxedValue) -> String {
    match ValueTypes::as_object(&**value) {
        Some(inner) => format!("<{}>", inner.get_type().full_name()),
        None => ValueTypes::to_display_string(Some(value)),
    }
}

/// The canonical dump of an object tree: the full name of the class and,
/// for a styled element, whether it is initialised; every registered
/// property (of the class, and attached) that is set, sorted by name, and
/// every direct property of the class, with its value in display form (an
/// object value is shown as its class); for a named element, whether the
/// name scope of the root finds it under its name; for an element with a
/// name scope of its own, whether that scope is completed; then the logical
/// children of a styled element, recursively.
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
        let value = display(&object.get_value_untyped(property));
        lines.push(format!("{pad}  {}.{} = {value}\n", property.owner_type().name(), property.name()));
    }
    for property in registry.get_registered_direct(class).iter() {
        let value = display(&object.get_value_untyped(property));
        lines.push(format!("{pad}  direct {}.{} = {value}\n", property.owner_type().name(), property.name()));
    }
    lines.sort();
    for line in &lines {
        output.push_str(line);
    }

    if let Some(styled) = &styled {
        if let Some(name) = styled.name() {
            let found = root_scope
                .and_then(|scope| scope.find(&name))
                .is_some_and(|found| std::ptr::eq(&*found, &**object));
            output.push_str(&format!("{pad}  named '{name}', found in the scope of the root: {found}\n"));
        }
        if let Some(scope) = NameScope::get_name_scope(styled) {
            output.push_str(&format!("{pad}  has a name scope, completed: {}\n", scope.0.is_completed()));
        }
        for child in styled.logical_children().to_vec() {
            dump(&child.upcast::<FerroObject>(), root_scope, indent + 1, output);
        }
    }
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

fn build_generated(name: &str) -> Option<Result<BoxedValue, String>> {
    let (_, build) = generated::DOCUMENTS.iter().find(|(document, _)| *document == name)?;
    // What the run-time loader gives `Build`: a root service provider with a fresh name scope.
    let provider: Rc<dyn IServiceProvider> = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    Some(build(Some(provider)).map_err(|error| error.message().to_string()))
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
        // A failed load is compared by its message, which carries the position.
        let interpreted = match try_load(xaml) {
            Ok(root) => dump_root(&root),
            Err(error) => format!("<error: {}>\n", error.message()),
        };
        let generated = match built {
            Ok(root) => dump_root(&root),
            Err(error) => format!("<error: {error}>\n"),
        };
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
    assert!(direct.is_some(), "{uri} is not found by the generated loader");
    assert!(generated::try_load(None, &format!("{}missing.xaml", generated::ROOT_URI)).is_none());
    assert!(generated::try_load(None, "ferres://Other.Assembly/border_empty.xaml").is_none());

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
