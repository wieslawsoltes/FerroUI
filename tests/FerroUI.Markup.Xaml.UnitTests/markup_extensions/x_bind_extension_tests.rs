//! `{x:Bind}` in FerroUI XAML: step X1 of #525 (docs/porting/x-bind.md). Not a port of an
//! upstream file; the behaviour is WinUI's (the NOTICE.md of `ferroui-markup-xaml`).
//!
//! The fixtures in `x_bind/` run through the run-time loader and through the Rust emitter, and
//! the same assertions check both: the root source, the modes and update triggers, named
//! elements, attached properties, the typed template, and the lifecycle of the bindings of a root
//! (`Initialize`, `Update`, `StopTracking`, initialized when the root is attached to a window).
//! The diagnostics are load errors of the one and compile errors of the other.
//!
//! The emitter's output, `x_bind/x_bind.generated.rs`, is checked in, compiled with the tests and
//! compared with the current compiler. Regenerate it after reviewing a compiler change:
//! `cargo test -p ferroui-markup-xaml-tests --lib
//! markup_extensions::x_bind_extension_tests::regenerate_x_bind_fixture -- --ignored --exact`

use std::rc::Rc;

use ferroui_base::controls::NameScope;
use ferroui_base::input::{FocusChangedEventArgs, InputElement};
use ferroui_base::Ref;
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::{Button, ContentControl, Control, StackPanel, TextBlock, TextBox, Window};
use ferroui_markup_xaml::markup_extensions::XBindings;
use ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers;
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use ferroui_markup_xaml_loader::rust_emitter::generate_file;

use crate::support::app::styled_window_application;
use crate::support::loader::{describe, parse, try_parse};

#[allow(dead_code)]
#[rustfmt::skip]
#[path = "x_bind/x_bind.generated.rs"]
mod generated;

const ROOT: &str = include_str!("x_bind/x_bind_root.xaml");
const TEMPLATE: &str = include_str!("x_bind/x_bind_template.xaml");

const ASSEMBLY: &str = "FerroUI.XBind.Fixture";
const ROOT_URI: &str = "ferres://FerroUI.XBind.Fixture/";

// --- the root source ----------------------------------------------------------------------

struct RootParts {
    initial: Ref<TextBlock>,
    mirror: Ref<TextBlock>,
    editor: Ref<TextBox>,
    live: Ref<TextBox>,
    named: Ref<TextBlock>,
    attached: Ref<TextBlock>,
}

fn root_parts(root: &Ref<StackPanel>) -> RootParts {
    let scope = NameScope::get_name_scope(root).expect("the document root owns its name scope");
    RootParts {
        initial: scope.find_as::<TextBlock>("initial").unwrap(),
        mirror: scope.find_as::<TextBlock>("mirror").unwrap(),
        editor: scope.find_as::<TextBox>("editor").unwrap(),
        live: scope.find_as::<TextBox>("live").unwrap(),
        named: scope.find_as::<TextBlock>("named").unwrap(),
        attached: scope.find_as::<TextBlock>("attached").unwrap(),
    }
}

fn tag_text(root: &StackPanel) -> Option<String> {
    root.tag().and_then(|tag| tag.downcast_ref::<String>().cloned())
}

fn set_tag(root: &StackPanel, text: &str) {
    root.set_tag(Some(Rc::new(text.to_string())));
}

fn assert_root_behavior(root: Ref<StackPanel>) {
    let parts = root_parts(&root);
    let bindings = XBindings::get_for_root(&root).expect("the first x:Bind of a document creates the bindings of its root");
    assert!(!bindings.is_template());
    assert_eq!(bindings.binding_count(), 6);

    // Nothing is read before the bindings are initialized: code-behind sets what they read
    // after construction.
    assert!(!bindings.is_initialized());
    assert_eq!(parts.initial.text(), None);
    assert_eq!(parts.mirror.text(), None);
    assert!(parts.attached.tag().is_none());
    set_tag(&root, "assigned after construction");
    bindings.initialize().unwrap();
    assert!(bindings.is_initialized());
    for text in [parts.initial.text(), parts.mirror.text(), parts.editor.text(), parts.live.text()] {
        assert_eq!(text.as_deref(), Some("assigned after construction"));
    }
    // A path that starts at a named element, and an attached property of the root.
    assert_eq!(parts.named.text(), parts.editor.text());
    assert_eq!(parts.attached.tag().and_then(|tag| tag.downcast_ref::<i32>().copied()), Some(2));

    // OneTime does not follow the source; OneWay and TwoWay do.
    set_tag(&root, "live change");
    assert_eq!(parts.initial.text().as_deref(), Some("assigned after construction"));
    assert_eq!(parts.mirror.text().as_deref(), Some("live change"));
    assert_eq!(parts.editor.text().as_deref(), Some("live change"));
    assert_eq!(parts.live.text().as_deref(), Some("live change"));
    assert_eq!(parts.named.text().as_deref(), Some("live change"));

    // TwoWay on TextBox.Text writes back when the text box loses focus, by default.
    parts.editor.set_current_value(TextBox::text_property(), Some("user draft".into()));
    assert_eq!(tag_text(&root).as_deref(), Some("live change"));
    assert_eq!(parts.mirror.text().as_deref(), Some("live change"));
    assert_eq!(parts.named.text().as_deref(), Some("user draft"));
    parts.editor.raise_event(&FocusChangedEventArgs::new(InputElement::lost_focus_event()));
    assert_eq!(tag_text(&root).as_deref(), Some("user draft"));
    assert_eq!(parts.mirror.text().as_deref(), Some("user draft"));

    // UpdateSourceTrigger=PropertyChanged writes back on every change.
    parts.live.set_current_value(TextBox::text_property(), Some("typed".into()));
    assert_eq!(tag_text(&root).as_deref(), Some("typed"));
    assert_eq!(parts.mirror.text().as_deref(), Some("typed"));
    assert_eq!(parts.editor.text().as_deref(), Some("typed"));

    // Update reads every binding again, OneTime included.
    assert_eq!(parts.initial.text().as_deref(), Some("assigned after construction"));
    bindings.update().unwrap();
    assert_eq!(parts.initial.text().as_deref(), Some("typed"));

    // StopTracking releases the sources and keeps the targets; Initialize reads them again.
    bindings.stop_tracking();
    assert!(!bindings.is_initialized());
    set_tag(&root, "while stopped");
    assert_eq!(parts.mirror.text().as_deref(), Some("typed"));
    parts.live.set_current_value(TextBox::text_property(), Some("blocked write".into()));
    assert_eq!(tag_text(&root).as_deref(), Some("while stopped"));
    bindings.initialize().unwrap();
    for text in [parts.initial.text(), parts.mirror.text(), parts.editor.text(), parts.live.text()] {
        assert_eq!(text.as_deref(), Some("while stopped"));
    }
    assert!(bindings.diagnostics().is_empty());
}

/// WinUI initializes the bindings of a page on `Loading`; here the root is initialized when it
/// is attached to a visual tree, and an initialized root is not read again by a later attach.
fn assert_attach_initializes(root: Ref<StackPanel>) {
    let parts = root_parts(&root);
    let bindings = XBindings::get_for_root(&root).unwrap();
    set_tag(&root, "set before loading");
    assert!(!bindings.is_initialized());
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    assert!(root.is_attached_to_visual_tree(), "showing the window attaches its content");
    assert!(bindings.is_initialized());
    assert_eq!(parts.initial.text().as_deref(), Some("set before loading"));
    assert_eq!(parts.mirror.text().as_deref(), Some("set before loading"));
    set_tag(&root, "after loading");
    assert_eq!(parts.initial.text().as_deref(), Some("set before loading"));
    assert_eq!(parts.mirror.text().as_deref(), Some("after loading"));
}

fn compiled_root() -> Ref<StackPanel> {
    let provider = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    generated::build_x_bind_root_xaml(Some(provider)).unwrap()
}

fn compiled_template_host() -> Ref<ContentControl> {
    let provider = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    generated::build_x_bind_template_xaml(Some(provider)).unwrap()
}

#[test]
fn interpreted_root_source_modes_named_elements_and_lifecycle() {
    let _app = styled_window_application();
    assert_root_behavior(parse::<Ref<StackPanel>>(ROOT));
}

#[test]
fn compiled_root_source_modes_named_elements_and_lifecycle() {
    let _app = styled_window_application();
    assert_root_behavior(compiled_root());
}

#[test]
fn interpreted_root_is_initialized_when_attached() {
    let _app = styled_window_application();
    assert_attach_initializes(parse::<Ref<StackPanel>>(ROOT));
}

#[test]
fn compiled_root_is_initialized_when_attached() {
    let _app = styled_window_application();
    assert_attach_initializes(compiled_root());
}

#[test]
fn the_bindings_do_not_keep_their_root_alive() {
    let _app = styled_window_application();
    let weak = {
        let root = parse::<Ref<StackPanel>>(ROOT);
        let bindings = XBindings::get_for_root(&root).unwrap();
        bindings.initialize().unwrap();
        root.downgrade()
    };
    assert!(weak.upgrade().is_none(), "the root, its targets and its bindings are released");
}

#[test]
fn the_framework_namespace_spells_x_bind_as_xbind() {
    let _app = styled_window_application();
    let root = parse::<Ref<StackPanel>>(
        "<StackPanel xmlns='https://github.com/ferroui' Tag='root tag'><TextBlock Text='{XBind Tag}'/></StackPanel>",
    );
    let text = root.children().get(0).cast::<TextBlock>().unwrap();
    assert_eq!(text.text(), None);
    XBindings::get_for_root(&root).unwrap().initialize().unwrap();
    assert_eq!(text.text().as_deref(), Some("root tag"));
}

// --- the typed template -------------------------------------------------------------------

struct TemplateParts {
    initial: Ref<TextBlock>,
    editor: Ref<TextBox>,
    mirror: Ref<TextBlock>,
    echo: Ref<TextBlock>,
}

fn template_parts(root: &Control) -> TemplateParts {
    // The template's name scope is the presenter's; the built control is found by position.
    let panel = root.to_ref().cast::<StackPanel>().unwrap();
    let children = panel.children();
    TemplateParts {
        initial: children.get(0).cast::<TextBlock>().unwrap(),
        editor: children.get(1).cast::<TextBox>().unwrap(),
        mirror: children.get(2).cast::<TextBlock>().unwrap(),
        echo: children.get(3).cast::<TextBlock>().unwrap(),
    }
}

fn item(text: &str) -> Ref<Button> {
    let result = Button::new();
    result.set_content(Some(Rc::new(text.to_string())));
    result
}

fn content_text(item: &Button) -> Option<String> {
    item.content().and_then(|content| content.downcast_ref::<String>().cloned())
}

fn assert_template_behavior(template: Rc<dyn IDataTemplate>) {
    let first = template.build(&None).unwrap();
    let second = template.build(&None).unwrap();
    let parts = template_parts(&first);
    let other = template_parts(&second);
    let first_bindings = XBindings::get_for_root(&first).expect("each template instance has the bindings of its root");
    let second_bindings = XBindings::get_for_root(&second).unwrap();
    assert!(!Rc::ptr_eq(&first_bindings, &second_bindings));
    assert!(first_bindings.is_template());

    // An instance reads its item when its data context changes to it, as WinUI's
    // DataContextChanged handler: no explicit Initialize.
    assert!(!first_bindings.is_initialized());
    assert_eq!(parts.initial.text(), None);
    let first_item = item("first");
    first.set_data_context(Some(Rc::new(first_item.clone())));
    assert!(first_bindings.is_initialized());
    assert_eq!(parts.initial.text().as_deref(), Some("first"));
    assert_eq!(parts.editor.text(), parts.initial.text());
    assert_eq!(parts.mirror.text(), parts.initial.text());
    // `editor` is not a member of the data type: it names the element of the template.
    assert_eq!(parts.echo.text(), parts.initial.text());
    assert_eq!(other.initial.text(), None);
    let second_item = item("second");
    second.set_data_context(Some(Rc::new(second_item.clone())));
    assert_eq!(other.initial.text().as_deref(), Some("second"));

    first_item.set_content(Some(Rc::new("changed source".to_string())));
    assert_eq!(parts.initial.text().as_deref(), Some("first"));
    assert_eq!(parts.editor.text().as_deref(), Some("changed source"));
    assert_eq!(parts.mirror.text().as_deref(), Some("changed source"));
    parts.editor.set_current_value(TextBox::text_property(), Some("changed target".into()));
    assert_eq!(content_text(&first_item).as_deref(), Some("changed target"));
    assert_eq!(parts.mirror.text().as_deref(), Some("changed target"));
    assert_eq!(parts.echo.text().as_deref(), Some("changed target"));
    assert_eq!(other.editor.text().as_deref(), Some("second"));

    // A recycled instance reads its new item, OneTime included, and leaves the old one.
    let recycled = item("recycled item");
    first.set_data_context(Some(Rc::new(recycled.clone())));
    assert_eq!(parts.initial.text().as_deref(), Some("recycled item"));
    assert_eq!(parts.editor.text(), parts.initial.text());
    assert_eq!(parts.mirror.text(), parts.initial.text());
    first_item.set_content(Some(Rc::new("old item is detached".to_string())));
    assert_eq!(parts.mirror.text().as_deref(), Some("recycled item"));

    // An item that is not of the x:DataType is a diagnostic; the next item reads normally.
    first.set_data_context(Some(Rc::new("not a button".to_string())));
    assert!(first_bindings.diagnostics().iter().any(|d| d.message.contains("x:DataType")));
    first.set_data_context(Some(Rc::new(recycled.clone())));
    assert!(first_bindings.diagnostics().is_empty());
    assert_eq!(parts.mirror.text().as_deref(), Some("recycled item"));

    // Stopping one instance leaves the other live.
    first_bindings.stop_tracking();
    parts.editor.set_current_value(TextBox::text_property(), Some("blocked write".into()));
    assert_eq!(content_text(&recycled).as_deref(), Some("recycled item"));
    second_item.set_content(Some(Rc::new("second remains live".to_string())));
    assert_eq!(other.mirror.text().as_deref(), Some("second remains live"));
}

#[test]
fn interpreted_typed_template_reads_its_item_and_recycles() {
    let _app = styled_window_application();
    let host = parse::<Ref<ContentControl>>(TEMPLATE);
    assert_template_behavior(host.content_template().unwrap());
}

#[test]
fn compiled_typed_template_reads_its_item_and_recycles() {
    let _app = styled_window_application();
    assert_template_behavior(compiled_template_host().content_template().unwrap());
}

// --- diagnostics --------------------------------------------------------------------------

/// A document with one x:Bind on its second line, and what its error says. The run-time
/// loader and the emitter run the same transformers: the error is the same in both.
const DIAGNOSTICS: &[(&str, &str)] = &[
    ("<TextBlock Text='{x:Bind Missing}'/>", "Unable to resolve property or method of name 'Missing'"),
    ("<TextBlock Tag='{x:Bind Children, Mode=TwoWay}'/>", "cannot write to read-only source member 'Children'"),
    ("<TextBox x:Name='box'/><TextBlock Tag='{x:Bind box, Mode=TwoWay}'/>", "cannot write to the named element 'box'"),
    ("<TextBlock Text='{x:Bind Tag, Mode=OneWay, UpdateSourceTrigger=PropertyChanged}'/>", "may only be used with binding 'Mode=TwoWay'"),
    ("<TextBox Text='{x:Bind Tag, Mode=TwoWay, UpdateSourceTrigger=Explicit}'/>", "'Explicit' is not a supported value"),
    ("<TextBlock Text='{x:Bind Tag, Mode=Default}'/>", "x:Bind Mode must be OneTime, OneWay or TwoWay"),
    ("<TextBlock Text='{x:Bind $self.Tag}'/>", "'$self' is not x:Bind syntax"),
    ("<TextBlock Text='{x:Bind ((TextBlock)Tag).Text}'/>", "a cast is not supported yet"),
    (
        "<ContentControl><ContentControl.ContentTemplate><DataTemplate><TextBlock Text='{x:Bind Tag}'/></DataTemplate></ContentControl.ContentTemplate></ContentControl>",
        "DataTemplates containing x:Bind need a DataType",
    ),
    (
        "<StackPanel.Styles><Style Selector='TextBlock'><Setter Property='Tag' Value='{x:Bind Tag}'/></Style></StackPanel.Styles>",
        "Cannot use x:Bind in Styles",
    ),
];

fn diagnostic_document(body: &str) -> String {
    format!(
        "<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>\n  {body}\n</StackPanel>"
    )
}

#[test]
fn diagnostics_are_load_errors_with_the_position() {
    let _app = styled_window_application();
    for (body, expected) in DIAGNOSTICS {
        let document = diagnostic_document(body);
        let error = match try_parse::<Ref<StackPanel>>(&document) {
            Ok(_) => panic!("the document loaded: {body}"),
            Err(error) => describe(&error),
        };
        assert!(error.contains(expected), "{body}: {error}");
        assert!(error.contains("line 2"), "{body}: the error has the position of the binding: {error}");
    }
}

#[test]
fn diagnostics_are_compile_errors() {
    let _app = styled_window_application();
    for (body, expected) in DIAGNOSTICS {
        let document = diagnostic_document(body);
        let file = generate_file(
            ASSEMBLY,
            ROOT_URI,
            &[("diagnostic.xaml", document.as_str())],
            &RuntimeXamlLoaderConfiguration::new(),
            &[],
        );
        let reason = file.documents[0].1.clone().unwrap_or_else(|| panic!("the document compiled: {body}"));
        assert!(reason.contains(expected), "{body}: {reason}");
    }
}

// --- the compiled fixture -----------------------------------------------------------------

fn emitted_fixture() -> String {
    crate::register_types();
    let file = generate_file(
        ASSEMBLY,
        ROOT_URI,
        &[("x_bind_root.xaml", ROOT), ("x_bind_template.xaml", TEMPLATE)],
        &RuntimeXamlLoaderConfiguration::new(),
        &[],
    );
    for (document, reason) in &file.documents {
        assert!(reason.is_none(), "{document}: {reason:?}");
    }
    file.source
}

#[test]
fn generated_x_bind_fixture_matches_the_current_compiler() {
    assert_eq!(
        include_str!("x_bind/x_bind.generated.rs"),
        emitted_fixture(),
        "regenerate the fixture with the command in the header of this module"
    );
}

#[test]
#[ignore = "updates the checked-in fixture; run explicitly after reviewing compiler changes"]
fn regenerate_x_bind_fixture() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("markup_extensions/x_bind/x_bind.generated.rs");
    std::fs::write(path, emitted_fixture()).unwrap();
}
