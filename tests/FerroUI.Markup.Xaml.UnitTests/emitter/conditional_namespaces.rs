//! Conditional XML namespaces in both markup paths: each document is built by the run-time
//! loader and by the generated function of `generated_conditional.rs`, for each answer of the
//! custom conditions, and the object trees must be equal. A file of its own: the documents of
//! the corpus, and `generated.rs`, stay as they are.

use std::rc::Rc;

use ferroui_base::controls::NameScope;
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::BoxedValue;
use ferroui_markup_xaml::register_xaml_condition;
use ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers;
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use ferroui_markup_xaml_loader::rust_emitter::{generate_file, GeneratedFile};

use super::differential_tests::dump_root;
use super::generated_conditional as generated;
use crate::support::app::xaml_test_base;
use crate::support::loader::try_load;

const GENERATED_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/emitter/generated_conditional.rs");

const FERRO: &str = "https://github.com/ferroui";

/// The documents: every shape a condition can guard, in built-in and custom conditions.
const DOCUMENTS: &[(&str, &str)] = &[
    (
        "element.xaml",
        r#"<StackPanel xmlns="https://github.com/ferroui"
            xmlns:yes="https://github.com/ferroui?IsTypePresent(FerroUI.Controls.Button)"
            xmlns:no="https://github.com/ferroui?IsTypeNotPresent(FerroUI.Controls.Button)">
            <yes:Button Content="kept"/><no:Button Content="left out"/><TextBlock Text="after"/>
        </StackPanel>"#,
    ),
    (
        "unknown_type.xaml",
        r#"<StackPanel xmlns="https://github.com/ferroui"
            xmlns:gone="https://github.com/ferroui?IsTypePresent(No.Such.Type)">
            <gone:NoSuchControl Unknown="1"/><TextBlock gone:NoSuchProperty="1" Text="kept"/>
        </StackPanel>"#,
    ),
    (
        "attribute.xaml",
        r#"<Button xmlns="https://github.com/ferroui"
            xmlns:yes="https://github.com/ferroui?IsPropertyPresent(FerroUI.Controls.Button,Content)"
            xmlns:no="https://github.com/ferroui?IsPropertyNotPresent(FerroUI.Controls.Button,Content)"
            yes:Content="kept" no:Content="left out"/>"#,
    ),
    (
        "property_element.xaml",
        r#"<Button xmlns="https://github.com/ferroui"
            xmlns:yes="https://github.com/ferroui?IsApiContractPresent(FerroUI.ApiContract,0)"
            xmlns:no="https://github.com/ferroui?IsApiContractNotPresent(FerroUI.ApiContract,0)">
            <no:Button.Content>left out</no:Button.Content>
            <yes:Button.Content>kept</yes:Button.Content>
        </Button>"#,
    ),
    (
        "nesting.xaml",
        r#"<StackPanel xmlns="https://github.com/ferroui"
            xmlns:yes="https://github.com/ferroui?IsTypePresent(Button)"
            xmlns:no="https://github.com/ferroui?IsTypeNotPresent(Button)">
            <yes:Border><yes:StackPanel><no:Button/><yes:TextBlock Text="inner"/></yes:StackPanel></yes:Border>
            <no:Border><yes:TextBlock Text="left out with its parent"/></no:Border>
        </StackPanel>"#,
    ),
    (
        "custom.xaml",
        r#"<StackPanel xmlns="https://github.com/ferroui"
            xmlns:a="https://github.com/ferroui?local:Flag(A)"
            xmlns:b="https://github.com/ferroui?local:Flag(B)">
            <a:Button Content="a"/><b:Button Content="b"/>
            <TextBlock a:Text="text a" b:Text="text b"/>
            <Button>
                <a:Button.Content>content a</a:Button.Content>
                <b:Button.Content>content b</b:Button.Content>
            </Button>
            <a:Border><b:TextBlock Text="a and b"/></a:Border>
        </StackPanel>"#,
    ),
    (
        "style_setters.xaml",
        r#"<Border xmlns="https://github.com/ferroui"
            xmlns:a="https://github.com/ferroui?local:Flag(A)"
            xmlns:b="https://github.com/ferroui?local:Flag(B)">
            <Border.Styles>
                <Style Selector="Border">
                    <a:Setter Property="Width" Value="10"/>
                    <b:Setter Property="Height" Value="20"/>
                    <Setter Property="Opacity" Value="0.5"/>
                </Style>
            </Border.Styles>
        </Border>"#,
    ),
];

fn generate() -> GeneratedFile {
    generate_file(generated::ASSEMBLY_NAME, generated::ROOT_URI, DOCUMENTS, &RuntimeXamlLoaderConfiguration::new(), &[])
}

fn build_generated(name: &str) -> BoxedValue {
    let (_, build) = generated::DOCUMENTS.iter().find(|(document, _)| *document == name).expect("emitted");
    let provider: Rc<dyn IServiceProvider> = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    build(Some(provider)).expect("the generated function builds")
}

fn flags(a: bool, b: bool) {
    register_xaml_condition("Flag", move |argument: &str| match argument {
        "A" => a,
        "B" => b,
        _ => false,
    });
}

#[test]
fn generated_conditional_output_is_up_to_date() {
    let _base = xaml_test_base();
    assert!(generate().source == include_str!("generated_conditional.rs"), "regenerate generated_conditional.rs");
}

#[test]
#[ignore = "writes emitter/generated_conditional.rs; run it to regenerate the checked-in output"]
fn regenerate_conditional_output() {
    let _base = xaml_test_base();
    let file = generate();
    for (name, reason) in &file.documents {
        assert!(reason.is_none(), "{name} is not eligible: {reason:?}");
    }
    std::fs::write(GENERATED_PATH, file.source).unwrap();
}

#[test]
fn both_paths_build_the_same_tree_for_every_answer() {
    let _base = xaml_test_base();
    for (a, b) in [(true, true), (true, false), (false, true), (false, false)] {
        flags(a, b);
        for (name, source) in DOCUMENTS {
            let interpreted = dump_root(&try_load(source).unwrap_or_else(|error| panic!("{name}: {error:?}")));
            let emitted = dump_root(&build_generated(name));
            let difference = interpreted.lines().zip(emitted.lines()).find(|(left, right)| left != right);
            assert!(interpreted == emitted, "{name} with A={a} B={b}: the first difference is {difference:?}");
        }
    }
}

#[test]
fn a_false_condition_leaves_its_nodes_out() {
    let _base = xaml_test_base();
    flags(true, false);
    let dump_of = |name: &str| dump_root(&build_generated(name));
    let element = dump_of("element.xaml");
    assert!(element.contains("kept") && element.contains("after") && !element.contains("left out"), "{element}");
    let unknown = dump_of("unknown_type.xaml");
    assert!(unknown.contains("kept") && !unknown.contains("NoSuch"), "{unknown}");
    for name in ["attribute.xaml", "property_element.xaml"] {
        let dump = dump_of(name);
        assert!(dump.contains("kept") && !dump.contains("left out"), "{name}: {dump}");
    }
    let nesting = dump_of("nesting.xaml");
    assert!(nesting.contains("inner") && !nesting.contains("left out"), "{nesting}");
    let custom = dump_of("custom.xaml");
    assert!(custom.contains("text a") && !custom.contains("text b") && !custom.contains("a and b"), "{custom}");
}

#[test]
fn conditional_setters_of_a_style_follow_the_answer_in_both_paths() {
    use ferroui_controls::Border;
    let _base = xaml_test_base();
    let source = DOCUMENTS.iter().find(|document| document.0 == "style_setters.xaml").unwrap().1;
    let count = |root: BoxedValue| {
        let border = ferroui_base::data::core::ValueTypes::as_object(&*root).unwrap().cast::<Border>().unwrap();
        let style = border.styles().get(0);
        style.indexed_style().expect("a style").setters().count()
    };
    for (a, b, expected) in [(true, true, 3), (true, false, 2), (false, true, 2), (false, false, 1)] {
        flags(a, b);
        assert_eq!(count(try_load(source).unwrap()), expected, "loader, A={a} B={b}");
        assert_eq!(count(build_generated("style_setters.xaml")), expected, "generated, A={a} B={b}");
    }
}

/// The bytecode backend ends a program at a line it does not know: the tests stay around the
/// statements they guard, and the documents stay eligible.
#[test]
fn the_bytecode_backend_keeps_the_tests_of_custom_conditions() {
    use ferroui_markup_xaml_loader::rust_emitter::{with_xaml_backend, BytecodeImage, XamlBackend};
    let _base = xaml_test_base();
    let native = generate();
    let image = BytecodeImage::packed();
    let bytecode = with_xaml_backend(XamlBackend::Bytecode, &image, generate);
    for (name, reason) in &bytecode.documents {
        assert!(reason.is_none(), "{name} is not eligible: {reason:?}");
    }
    let tests = |source: &str| source.matches("if rt::xaml_condition(").count();
    assert_eq!(tests(&bytecode.source), tests(&native.source));
    let opened = bytecode.source.matches('{').count();
    assert_eq!(opened, bytecode.source.matches('}').count());
}

#[test]
fn a_conditional_root_is_an_error_in_both_paths() {
    let _base = xaml_test_base();
    let source = format!(r#"<Button xmlns="{FERRO}?IsTypePresent(No.Such.Type)"/>"#);
    assert!(try_load(&source).is_err());
    let file = generate_file(
        generated::ASSEMBLY_NAME, generated::ROOT_URI, &[("root.xaml", &source)],
        &RuntimeXamlLoaderConfiguration::new(), &[],
    );
    assert!(file.documents.iter().all(|(_, reason)| reason.is_some()), "{:?}", file.documents);
}

#[test]
fn an_unregistered_condition_is_false_in_both_paths() {
    let _base = xaml_test_base();
    let source = DOCUMENTS.iter().find(|document| document.0 == "custom.xaml").unwrap().1;
    let renamed = source.replace("local:Flag", "local:NotRegistered");
    let interpreted = dump_root(&try_load(&renamed).unwrap());
    assert!(!interpreted.contains("text a") && !interpreted.contains("text b"), "{interpreted}");
}
