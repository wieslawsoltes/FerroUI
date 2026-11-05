//! `{x:Bind}` in FerroUI XAML: steps X1 to X4 of #525 (docs/porting/x-bind.md). Not a port of
//! an upstream file; the behaviour is WinUI's (the NOTICE.md of `ferroui-markup-xaml`), X2 after
//! its regression projects `FunctionTests` and `CastingTests`, X3 after `TwoWayTests` and
//! `NullableTests`, X4 after `EventTests`.
//!
//! The fixtures in `x_bind/` run through the run-time loader and through the Rust emitter, and
//! the same assertions check both: the root source, the modes and update triggers, named
//! elements, attached properties, the typed template, and the lifecycle of the bindings of a root
//! (`Initialize`, `Update`, `StopTracking`, initialized when the root is attached to a window);
//! functions on the root, on a path and static, with path and literal arguments, called again
//! when an argument changes, `BindBack`, static members, indexers, casts and interface members;
//! converters with a parameter and a language, `ConvertBack`, fallback and target null values;
//! event bindings with each signature form, on a path, static, on an event that is not a routed
//! event and in a template, subscribed while the bindings are initialized.
//! The diagnostics are load errors of the one and compile errors of the other.
//!
//! The emitter's output, `x_bind/x_bind.generated.rs`, is checked in, compiled with the tests and
//! compared with the current compiler. Regenerate it after reviewing a compiler change:
//! `cargo test -p ferroui-markup-xaml-tests --lib
//! markup_extensions::x_bind_extension_tests::regenerate_x_bind_fixture -- --ignored --exact`

use std::rc::Rc;

use ferroui_base::controls::NameScope;
use ferroui_base::input::{FocusChangedEventArgs, InputElement};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::Ref;
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::{Button, ContentControl, Control, StackPanel, TextBlock, TextBox, Window};
use ferroui_markup_xaml::markup_extensions::XBindings;
use ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers;
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use ferroui_markup_xaml_loader::rust_emitter::generate_file;

use crate::support::app::styled_window_application;
use crate::support::loader::{describe, parse, try_parse};
use crate::support::x_bind::{XBindHelpers, XBindModel, XBindPage, XBindTagConverter};

#[allow(dead_code)]
#[rustfmt::skip]
#[path = "x_bind/x_bind.generated.rs"]
mod generated;

const ROOT: &str = include_str!("x_bind/x_bind_root.xaml");
const TEMPLATE: &str = include_str!("x_bind/x_bind_template.xaml");
const FUNCTIONS: &str = include_str!("x_bind/x_bind_functions.xaml");
const CASTS: &str = include_str!("x_bind/x_bind_casts.xaml");
const CONVERTERS: &str = include_str!("x_bind/x_bind_converters.xaml");
const EVENTS: &str = include_str!("x_bind/x_bind_events.xaml");

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

// --- functions (step X2, WinUI's FunctionTests) ----------------------------------------------

struct FunctionParts {
    root_once: Ref<TextBlock>,
    root_function: Ref<TextBlock>,
    path_function: Ref<TextBlock>,
    model_argument: Ref<TextBlock>,
    static_function: Ref<TextBlock>,
    literals: Ref<TextBlock>,
    notifying: Ref<TextBlock>,
    named_argument: Ref<TextBlock>,
    bind_back: Ref<TextBox>,
    path_bind_back: Ref<TextBox>,
    static_bind_back: Ref<TextBox>,
    static_member: Ref<TextBlock>,
    index: Ref<TextBlock>,
    key: Ref<TextBlock>,
    interface_member: Ref<TextBlock>,
    item_host: Ref<ContentControl>,
}

fn function_parts(root: &Ref<XBindPage>) -> FunctionParts {
    let scope = NameScope::get_name_scope(root).expect("the document root owns its name scope");
    let text_block = |name: &str| scope.find_as::<TextBlock>(name).unwrap_or_else(|| panic!("the text block {name}"));
    let text_box = |name: &str| scope.find_as::<TextBox>(name).unwrap_or_else(|| panic!("the text box {name}"));
    FunctionParts {
        root_once: text_block("rootOnce"),
        root_function: text_block("rootFunction"),
        path_function: text_block("pathFunction"),
        model_argument: text_block("modelArgument"),
        static_function: text_block("staticFunction"),
        literals: text_block("literals"),
        notifying: text_block("notifying"),
        named_argument: text_block("namedArgument"),
        bind_back: text_box("bindBack"),
        path_bind_back: text_box("pathBindBack"),
        static_bind_back: text_box("staticBindBack"),
        static_member: text_block("staticMember"),
        index: text_block("index"),
        key: text_block("key"),
        interface_member: text_block("interfaceMember"),
        item_host: scope.find_as::<ContentControl>("itemHost").expect("the item host"),
    }
}

fn text(block: &TextBlock) -> Option<String> {
    block.text()
}

fn box_text(text_box: &TextBox) -> Option<String> {
    text_box.text()
}

fn assert_function_behavior(root: Ref<XBindPage>) {
    let parts = function_parts(&root);
    let model = root.model();
    let bindings = XBindings::get_for_root(&root).expect("the first x:Bind of a document creates the bindings of its root");
    assert_eq!(bindings.binding_count(), 15);
    bindings.initialize().unwrap_or_else(|errors| panic!("{errors}"));

    // A function on the root, on a path and static, with path and literal arguments.
    assert_eq!(text(&parts.root_once).as_deref(), Some("Title: Ada"));
    assert_eq!(text(&parts.root_function).as_deref(), Some("Title: Ada"));
    assert_eq!(text(&parts.path_function).as_deref(), Some("Hello, world Ada x3"));
    assert_eq!(text(&parts.model_argument).as_deref(), Some("Grace+Grace"));
    assert_eq!(text(&parts.static_function).as_deref(), Some("Ada-Ada"));
    assert_eq!(text(&parts.literals).as_deref(), Some("quoted, with comma|-2.5|true|null"));
    assert_eq!(text(&parts.notifying).as_deref(), Some("stamp 1"));
    assert_eq!(text(&parts.named_argument).as_deref(), Some("Title: Title: Ada"));
    assert_eq!(box_text(&parts.bind_back).as_deref(), Some("Title: Ada"));
    assert_eq!(box_text(&parts.path_bind_back).as_deref(), Some("GRACE"));
    assert_eq!(box_text(&parts.static_bind_back).as_deref(), Some("Ada"));
    // A static member, an integer and a string indexer, an interface member.
    assert_eq!(text(&parts.static_member).as_deref(), Some("prefix"));
    assert_eq!(text(&parts.index).as_deref(), Some("item 1"));
    assert_eq!(text(&parts.key).as_deref(), Some("value of second key"));
    assert_eq!(text(&parts.interface_member).as_deref(), Some("named Grace"));

    // A OneWay function is called again when an argument path changes; OneTime is not.
    root.set_title("Grace");
    assert_eq!(text(&parts.root_once).as_deref(), Some("Title: Ada"));
    assert_eq!(text(&parts.root_function).as_deref(), Some("Title: Grace"));
    assert_eq!(text(&parts.path_function).as_deref(), Some("Hello, world Grace x3"));
    assert_eq!(text(&parts.static_function).as_deref(), Some("Grace-Grace"));
    assert_eq!(box_text(&parts.bind_back).as_deref(), Some("Title: Grace"));
    // The argument of namedArgument is the text of a OneTime target: unchanged.
    assert_eq!(text(&parts.named_argument).as_deref(), Some("Title: Title: Ada"));
    model.set_name("Ada");
    assert_eq!(text(&parts.model_argument).as_deref(), Some("Ada+Ada"));
    assert_eq!(box_text(&parts.path_bind_back).as_deref(), Some("ADA"));
    // A PropertyChanged with the name of a function calls it again.
    model.bump();
    assert_eq!(text(&parts.notifying).as_deref(), Some("stamp 2"));

    // TwoWay through a function writes back through BindBack, and the function is called
    // again with the new source.
    parts.bind_back.set_current_value(TextBox::text_property(), Some("Title: Lovelace".into()));
    assert_eq!(root.title().as_deref(), Some("Lovelace"));
    assert_eq!(root.parsed().last().map(String::as_str), Some("Title: Lovelace"));
    assert_eq!(text(&parts.root_function).as_deref(), Some("Title: Lovelace"));
    parts.bind_back.set_current_value(TextBox::text_property(), Some("Hopper".into()));
    assert_eq!(root.title().as_deref(), Some("Hopper"));
    assert_eq!(box_text(&parts.bind_back).as_deref(), Some("Title: Hopper"));
    // BindBack on the value of a path.
    parts.path_bind_back.set_current_value(TextBox::text_property(), Some("TURING".into()));
    assert_eq!(model.name(), "turing");
    assert_eq!(text(&parts.model_argument).as_deref(), Some("turing+turing"));
    // A static BindBack, on LostFocus (the default of TextBox.Text).
    let stored = XBindHelpers::stored().len();
    parts.static_bind_back.set_current_value(TextBox::text_property(), Some("stored text".into()));
    assert_eq!(XBindHelpers::stored().len(), stored);
    parts.static_bind_back.raise_event(&FocusChangedEventArgs::new(InputElement::lost_focus_event()));
    assert_eq!(XBindHelpers::stored().last().map(String::as_str), Some("stored text"));

    // Update calls every function again, OneTime included.
    bindings.update().unwrap_or_else(|errors| panic!("{errors}"));
    assert_eq!(text(&parts.root_once).as_deref(), Some("Title: Hopper"));

    // StopTracking releases what the arguments and the owners of the functions listen to.
    bindings.stop_tracking();
    root.set_title("Stopped");
    model.bump();
    assert_eq!(text(&parts.root_function).as_deref(), Some("Title: Hopper"));
    assert_eq!(text(&parts.notifying).as_deref(), Some("stamp 2"));
    bindings.initialize().unwrap_or_else(|errors| panic!("{errors}"));
    assert_eq!(text(&parts.root_function).as_deref(), Some("Title: Stopped"));
    assert_eq!(text(&parts.notifying).as_deref(), Some("stamp 3"));
    assert!(bindings.diagnostics().is_empty(), "{:?}", bindings.diagnostics());

    // A function on the item of a typed template, called again when its argument changes.
    let template = parts.item_host.content_template().expect("the item template");
    let instance = template.build(&None).unwrap();
    let item_text = instance.to_ref().cast::<TextBlock>().expect("the root of the template is a text block");
    let item = XBindModel::new("item");
    instance.set_data_context(Some(item.clone()));
    assert_eq!(text(&item_text).as_deref(), Some("ITEM"));
    item.set_name("changed item");
    assert_eq!(text(&item_text).as_deref(), Some("CHANGED ITEM"));
}

fn compiled_functions() -> Ref<XBindPage> {
    let provider = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    generated::build_x_bind_functions_xaml(Some(provider)).unwrap()
}

#[test]
fn interpreted_functions_arguments_bind_back_static_members_and_indexers() {
    let _app = styled_window_application();
    assert_function_behavior(parse::<Ref<XBindPage>>(FUNCTIONS));
}

#[test]
fn compiled_functions_arguments_bind_back_static_members_and_indexers() {
    let _app = styled_window_application();
    assert_function_behavior(compiled_functions());
}

// --- casts (step X2, WinUI's CastingTests) ------------------------------------------------

struct CastParts {
    cast_root: Ref<TextBlock>,
    cast_member: Ref<TextBlock>,
    cast_two_way: Ref<TextBox>,
    cast_leaf: Ref<TextBlock>,
    cast_function: Ref<TextBlock>,
    cast_interface: Ref<TextBlock>,
    attached_path: Ref<TextBlock>,
}

fn cast_parts(root: &Ref<StackPanel>) -> CastParts {
    let scope = NameScope::get_name_scope(root).expect("the document root owns its name scope");
    let text_block = |name: &str| scope.find_as::<TextBlock>(name).unwrap_or_else(|| panic!("the text block {name}"));
    CastParts {
        cast_root: text_block("castRoot"),
        cast_member: text_block("castMember"),
        cast_two_way: scope.find_as::<TextBox>("castTwoWay").expect("the text box castTwoWay"),
        cast_leaf: text_block("castLeaf"),
        cast_function: text_block("castFunction"),
        cast_interface: text_block("castInterface"),
        attached_path: text_block("attachedPath"),
    }
}

fn assert_cast_behavior(root: Ref<StackPanel>) {
    let parts = cast_parts(&root);
    let bindings = XBindings::get_for_root(&root).unwrap();
    assert_eq!(bindings.binding_count(), 7);
    let model = XBindModel::new("grace");
    root.set_tag(Some(model.clone()));
    bindings.initialize().unwrap_or_else(|errors| panic!("{errors}"));

    // A cast passes on a value of the type: the root, a member of it, a function of it.
    let cast_model = parts.cast_root.tag().and_then(|tag| tag.downcast_ref::<XBindModel>().map(|cast| std::ptr::eq(cast, &*model)));
    assert_eq!(cast_model, Some(true));
    assert_eq!(text(&parts.cast_member).as_deref(), Some("grace"));
    assert_eq!(box_text(&parts.cast_two_way).as_deref(), Some("grace"));
    assert_eq!(text(&parts.cast_function).as_deref(), Some("GRACE"));
    // A cast to another type passes on null.
    assert_eq!(text(&parts.cast_leaf), None);
    // The member of an interface, and an attached property after a path.
    assert_eq!(text(&parts.cast_interface).as_deref(), Some("named grace"));
    assert_eq!(parts.attached_path.tag().and_then(|tag| tag.downcast_ref::<i32>().copied()), Some(4));

    model.set_name("ada");
    assert_eq!(text(&parts.cast_member).as_deref(), Some("ada"));
    assert_eq!(text(&parts.cast_function).as_deref(), Some("ADA"));
    // TwoWay through a cast inside the path (WinUI: `((local:T)DataContext).Username`).
    parts.cast_two_way.set_current_value(TextBox::text_property(), Some("turing".into()));
    assert_eq!(model.name(), "turing");
    assert_eq!(text(&parts.cast_member).as_deref(), Some("turing"));

    // A value of another type: the casts to the model pass on null, the cast to a text passes.
    root.set_tag(Some(Rc::new("plain text".to_string())));
    assert!(parts.cast_root.tag().is_none());
    assert_eq!(text(&parts.cast_leaf).as_deref(), Some("plain text"));
    assert_eq!(text(&parts.cast_member), None);
}

fn compiled_casts() -> Ref<StackPanel> {
    let provider = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    generated::build_x_bind_casts_xaml(Some(provider)).unwrap()
}

#[test]
fn interpreted_casts_interface_members_and_attached_properties() {
    let _app = styled_window_application();
    assert_cast_behavior(parse::<Ref<StackPanel>>(CASTS));
}

#[test]
fn compiled_casts_interface_members_and_attached_properties() {
    let _app = styled_window_application();
    assert_cast_behavior(compiled_casts());
}

// --- converters and substitute values (step X3, WinUI's TwoWayTests and NullableTests) ------

struct ConverterParts {
    converted: Ref<TextBlock>,
    parameter: Ref<TextBlock>,
    convert_back: Ref<TextBox>,
    fallback_once: Ref<TextBlock>,
    fallback: Ref<TextBlock>,
    fallback_two_way: Ref<TextBox>,
    null_fallback: Ref<TextBlock>,
    null_value: Ref<TextBlock>,
    null_two_way: Ref<TextBox>,
}

fn converter_parts(root: &Ref<XBindPage>) -> ConverterParts {
    let scope = NameScope::get_name_scope(root).expect("the document root owns its name scope");
    let text_block = |name: &str| scope.find_as::<TextBlock>(name).unwrap_or_else(|| panic!("the text block {name}"));
    let text_box = |name: &str| scope.find_as::<TextBox>(name).unwrap_or_else(|| panic!("the text box {name}"));
    ConverterParts {
        converted: text_block("converted"),
        parameter: text_block("parameter"),
        convert_back: text_box("convertBack"),
        fallback_once: text_block("fallbackOnce"),
        fallback: text_block("fallback"),
        fallback_two_way: text_box("fallbackTwoWay"),
        null_fallback: text_block("nullFallback"),
        null_value: text_block("nullValue"),
        null_two_way: text_box("nullTwoWay"),
    }
}

fn starts_with(value: Option<String>, prefix: &str) -> bool {
    value.is_some_and(|value| value.starts_with(prefix))
}

fn assert_converter_behavior(root: Ref<XBindPage>) {
    let parts = converter_parts(&root);
    let bindings = XBindings::get_for_root(&root).expect("the first x:Bind of a document creates the bindings of its root");
    assert_eq!(bindings.binding_count(), 9);
    bindings.initialize().unwrap_or_else(|errors| panic!("{errors}"));

    // The converter, with its parameter and the culture ConverterLanguage names (the current
    // culture without one).
    assert!(starts_with(text(&parts.converted), "Ada|-|"), "{:?}", text(&parts.converted));
    assert_eq!(text(&parts.parameter).as_deref(), Some("Ada|param|fr-FR"));
    assert!(starts_with(box_text(&parts.convert_back), "Ada|back|"), "{:?}", box_text(&parts.convert_back));

    // A path that cannot be read (a null in its middle) gives the fallback value, OneTime
    // included; a value that is there needs no substitute.
    assert_eq!(text(&parts.fallback_once).as_deref(), Some("no model"));
    assert_eq!(text(&parts.fallback).as_deref(), Some("no model"));
    assert_eq!(box_text(&parts.fallback_two_way).as_deref(), Some("no model"));
    assert_eq!(text(&parts.null_fallback), None);
    assert_eq!(text(&parts.null_value).as_deref(), Some("Ada"));
    assert_eq!(box_text(&parts.null_two_way).as_deref(), Some("Ada"));
    // A fallback value stands for the path: it is not a diagnostic.
    assert!(bindings.diagnostics().is_empty(), "{:?}", bindings.diagnostics());

    // A null value gives the target null value; a converter sees the null itself.
    root.clear_title();
    assert!(starts_with(text(&parts.converted), "null|-|"), "{:?}", text(&parts.converted));
    assert_eq!(text(&parts.null_value).as_deref(), Some("untitled"));
    assert_eq!(box_text(&parts.null_two_way).as_deref(), Some("untitled"));
    // A TwoWay binding with a target null value writes what the target is given.
    parts.null_two_way.set_current_value(TextBox::text_property(), Some("Hopper".into()));
    assert_eq!(root.title().as_deref(), Some("Hopper"));
    assert_eq!(text(&parts.null_value).as_deref(), Some("Hopper"));

    // TwoWay through a converter writes back through ConvertBack, with its parameter.
    parts.convert_back.set_current_value(TextBox::text_property(), Some("lovelace".into()));
    assert_eq!(root.title().as_deref(), Some("LOVELACE"));
    let converted_back = XBindTagConverter::converted_back();
    assert!(
        converted_back.last().is_some_and(|call| call.starts_with("lovelace|back|")),
        "{converted_back:?}"
    );
    assert!(starts_with(text(&parts.converted), "LOVELACE|-|"), "{:?}", text(&parts.converted));

    // The path can be read once the model is there; OneTime reads it on Update.
    let model = XBindModel::new("grace");
    root.set_tag(Some(model.clone()));
    assert_eq!(text(&parts.fallback_once).as_deref(), Some("no model"));
    assert_eq!(text(&parts.fallback).as_deref(), Some("grace"));
    assert_eq!(box_text(&parts.fallback_two_way).as_deref(), Some("grace"));
    assert_eq!(text(&parts.null_fallback).as_deref(), Some("grace"));
    parts.fallback_two_way.set_current_value(TextBox::text_property(), Some("turing".into()));
    assert_eq!(model.name(), "turing");
    assert_eq!(text(&parts.fallback).as_deref(), Some("turing"));
    bindings.update().unwrap_or_else(|errors| panic!("{errors}"));
    assert_eq!(text(&parts.fallback_once).as_deref(), Some("turing"));

    // The model gone again: the fallback values (x:Null for one), and no diagnostic.
    root.set_tag(None);
    assert_eq!(text(&parts.fallback).as_deref(), Some("no model"));
    assert_eq!(box_text(&parts.fallback_two_way).as_deref(), Some("no model"));
    assert_eq!(text(&parts.null_fallback), None);
    assert!(bindings.diagnostics().is_empty(), "{:?}", bindings.diagnostics());
}

fn compiled_converters() -> Ref<XBindPage> {
    let provider = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    generated::build_x_bind_converters_xaml(Some(provider)).unwrap()
}

#[test]
fn interpreted_converters_fallback_and_target_null_values() {
    let _app = styled_window_application();
    assert_converter_behavior(parse::<Ref<XBindPage>>(CONVERTERS));
}

#[test]
fn compiled_converters_fallback_and_target_null_values() {
    let _app = styled_window_application();
    assert_converter_behavior(compiled_converters());
}

// --- events (step X4, WinUI's EventTests) --------------------------------------------------

fn click(button: &Ref<Button>) {
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

fn assert_event_behavior(root: Ref<XBindPage>) {
    let scope = NameScope::get_name_scope(&root).expect("the document root owns its name scope");
    let button = |name: &str| scope.find_as::<Button>(name).unwrap_or_else(|| panic!("the button {name}"));
    let regular = button("regular");
    let no_arguments = button("noArguments");
    let base_arguments = button("baseArguments");
    let model_path = button("modelPath");
    let current_path = button("currentPath");
    let static_handler = button("staticHandler");
    let item_host = scope.find_as::<ContentControl>("itemHost").expect("the item host");
    let bindings = XBindings::get_for_root(&root).expect("the first x:Bind of a document creates the bindings of its root");
    assert_eq!(bindings.binding_count(), 0);
    assert_eq!(bindings.event_binding_count(), 7);

    // Nothing is subscribed before the bindings are initialized.
    assert_eq!(bindings.subscribed_event_count(), 0);
    assert_eq!(root.pinged_handler_count(), 0);
    click(&regular);
    assert!(root.clicks().is_empty());
    bindings.initialize().unwrap_or_else(|errors| panic!("{errors}"));
    assert_eq!(bindings.subscribed_event_count(), 7);
    assert_eq!(root.pinged_handler_count(), 1);

    // A method with the event's signature, without parameters, and with base parameters.
    click(&regular);
    click(&no_arguments);
    click(&base_arguments);
    assert_eq!(root.clicks(), ["regular", "no arguments", "base"]);

    // A method on a path: the path is read when the event is raised.
    let model = root.model();
    click(&model_path);
    assert_eq!(model.pokes(), 1);
    // A null on the path calls nothing, and is not a failure.
    click(&current_path);
    let first = XBindModel::new("first");
    root.set_current(Some(first.clone()));
    click(&current_path);
    assert_eq!(first.pokes(), 1);
    // Another owner, after Update: the new one is called.
    let second = XBindModel::new("second");
    root.set_current(Some(second.clone()));
    bindings.update().unwrap_or_else(|errors| panic!("{errors}"));
    click(&current_path);
    assert_eq!(second.pokes(), 1);
    assert_eq!(first.pokes(), 1);
    // Update keeps one subscription per event.
    assert_eq!(root.pinged_handler_count(), 1);

    // A static handler, and an event that is not a routed event.
    let clicked = XBindHelpers::clicked_count();
    click(&static_handler);
    assert_eq!(XBindHelpers::clicked_count(), clicked + 1);
    root.ping("hello");
    assert_eq!(root.clicks().last().map(String::as_str), Some("pinged hello"));

    // StopTracking releases the handlers.
    bindings.stop_tracking();
    assert_eq!(bindings.subscribed_event_count(), 0);
    assert_eq!(root.pinged_handler_count(), 0);
    let calls = root.clicks().len();
    click(&regular);
    click(&model_path);
    click(&static_handler);
    root.ping("stopped");
    assert_eq!(root.clicks().len(), calls);
    assert_eq!(model.pokes(), 1);
    assert_eq!(XBindHelpers::clicked_count(), clicked + 1);

    // Initialize subscribes them again.
    bindings.initialize().unwrap_or_else(|errors| panic!("{errors}"));
    assert_eq!(bindings.subscribed_event_count(), 7);
    assert_eq!(root.pinged_handler_count(), 1);
    click(&regular);
    assert_eq!(root.clicks().last().map(String::as_str), Some("regular"));
    assert!(bindings.diagnostics().is_empty(), "{:?}", bindings.diagnostics());

    // An event on the item of a typed template (WinUI: event binding at the root of a
    // template); a recycled instance calls its new item.
    let template = item_host.content_template().expect("the item template");
    let instance = template.build(&None).unwrap();
    let item_button = instance.to_ref().cast::<Button>().expect("the root of the template is a button");
    let item = XBindModel::new("item");
    instance.set_data_context(Some(item.clone()));
    click(&item_button);
    assert_eq!(item.pokes(), 1);
    let next = XBindModel::new("next");
    instance.set_data_context(Some(next.clone()));
    click(&item_button);
    assert_eq!(next.pokes(), 1);
    assert_eq!(item.pokes(), 1);
}

fn compiled_events() -> Ref<XBindPage> {
    let provider = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    generated::build_x_bind_events_xaml(Some(provider)).unwrap()
}

#[test]
fn interpreted_events_signatures_paths_static_handlers_and_lifecycle() {
    let _app = styled_window_application();
    assert_event_behavior(parse::<Ref<XBindPage>>(EVENTS));
}

#[test]
fn compiled_events_signatures_paths_static_handlers_and_lifecycle() {
    let _app = styled_window_application();
    assert_event_behavior(compiled_events());
}

#[test]
fn event_bindings_do_not_keep_their_root_alive() {
    let _app = styled_window_application();
    let weak = {
        let root = parse::<Ref<XBindPage>>(EVENTS);
        XBindings::get_for_root(&root).unwrap().initialize().unwrap();
        root.downgrade()
    };
    assert!(weak.upgrade().is_none(), "the root, its targets, their handlers and the bindings are released");
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
    (
        "<ContentControl><ContentControl.ContentTemplate><DataTemplate><TextBlock Text='{x:Bind Tag}'/></DataTemplate></ContentControl.ContentTemplate></ContentControl>",
        "DataTemplates containing x:Bind need a DataType",
    ),
    (
        "<StackPanel.Styles><Style Selector='TextBlock'><Setter Property='Tag' Value='{x:Bind Tag}'/></Style></StackPanel.Styles>",
        "Cannot use x:Bind in Styles",
    ),
    // Step X2: functions, BindBack, casts and indexers, with WinUI's messages.
    ("<TextBlock Text='{x:Bind Missing(Name)}'/>", "Function 'Missing' not found on type"),
    ("<TextBlock Text='{x:Bind Name()}'/>", "Expecting a method"),
    ("<TextBlock Text='{x:Bind local:XBindHelpers.Join(Name)}'/>", "Mismatched function parameter count"),
    ("<TextBlock Text='{x:Bind local:XBindHelpers.Join(Name, 2, 1)}'/>", "Invalid or mismatched parameter at position '2'"),
    ("<TextBlock Text='{x:Bind local:XBindHelpers.Join(Name, Name, IsVisible)}'/>", "Invalid or mismatched parameter at position '3'"),
    ("<TextBlock IsVisible='{x:Bind local:XBindHelpers.Join(Name, Name, 1)}'/>", "Function return type 'System.String' must match binding target type"),
    ("<TextBlock Text='{x:Bind local:XBindModel.Shout(Name)}'/>", "to be a static function"),
    ("<TextBlock Text='{x:Bind local:XBindHelpers.Join(Name, Name, 1).Length}'/>", "Functions are only supported at the end of the bind path"),
    ("<TextBlock Text='{x:Bind local:XBindHelpers.Join(Name, Format(Name), 1)}'/>", "Functions cannot have other functions as parameters"),
    ("<TextBox Text='{x:Bind local:XBindHelpers.Join(Name, Name, 1), Mode=TwoWay}'/>", "BindBack was expected, but not found"),
    ("<TextBlock Text='{x:Bind local:XBindHelpers.Join(Name, Name, 1), BindBack=local:XBindHelpers.Store}'/>", "Unexpected BindBack found"),
    (
        "<TextBox Text='{x:Bind local:XBindHelpers.Join(Name, Name, 1), Mode=TwoWay, BindBack=local:XBindHelpers.Join}'/>",
        "BindBack must point to a method that takes one argument of type",
    ),
    ("<TextBox Text='{x:Bind (x:String)Tag, Mode=TwoWay}'/>", "TwoWay binding is invalid when the binding expression ends with a cast"),
    ("<TextBlock Text='{x:Bind ((local:XBindModel)IsVisible).Name}'/>", "Unable to cast type"),
    ("<TextBlock Text='{x:Bind IsVisible[0]}'/>", "does not have an indexer"),
    ("<TextBlock Text='{x:Bind Name Other}'/>", "Invalid binding path 'Name Other' : Syntax error at 'Other'"),
    // Step X3: converters, fallback values and target null values, with WinUI's messages.
    ("<TextBlock Text='{x:Bind Tag, ConverterParameter=p}'/>", "ConverterParameter cannot be specified without an actual converter"),
    ("<TextBlock Text='{x:Bind Tag, ConverterLanguage=fr-FR}'/>", "ConverterLanguage cannot be specified without an actual converter"),
    (
        "<StackPanel.Resources><local:XBindTagConverter x:Key='tag'/></StackPanel.Resources><TextBlock Text='{x:Bind local:XBindHelpers.Join(Name, Name, 1), Converter={StaticResource tag}}'/>",
        "Using a converter with function binding is not supported",
    ),
    (
        "<StackPanel.Resources><x:String x:Key='text'>plain</x:String></StackPanel.Resources><TextBlock Text='{x:Bind Tag, Converter={StaticResource text}}'/>",
        "The Converter of an x:Bind must be an IValueConverter",
    ),
    ("<TextBlock IsVisible='{x:Bind Tag, TargetNullValue=True}'/>", "TargetNullValue cannot be applied on 'IsVisible'"),
    ("<TextBlock IsVisible='{x:Bind Tag, FallbackValue=maybe}'/>", "The FallbackValue 'maybe' of an x:Bind cannot be converted"),
    (
        "<StackPanel.Resources><x:String x:Key='text'>plain</x:String></StackPanel.Resources><TextBlock Text='{x:Bind Tag, FallbackValue={StaticResource text}}'/>",
        "Only string and {x:Null} are supported for FallbackValue",
    ),
    // Step X4: events, with WinUI's messages.
    ("<Button Click='{x:Bind Missing}'/>", "Unable to resolve property or method of name 'Missing'"),
    ("<Button Click='{x:Bind Tag}'/>", "Event 'Click' can only be bound to properties of delegate type"),
    (
        "<Button Click='{x:Bind local:XBindHelpers.Store}'/>",
        "Invalid signature for event 'Click'. Events can only be bound to methods that match the event signature or are parameterless",
    ),
    ("<Button Click='{x:Bind local:XBindHelpers.Mismatched}'/>", "Invalid signature for event 'Click'. Parameter 1 of type 'String'"),
    ("<Button Click='{x:Bind local:XBindHelpers.Overloaded}'/>", "Events can only be bound to non-overloaded methods"),
    ("<Button Click='{x:Bind local:XBindHelpers.Clicked()}'/>", "binds to a method, not to a call"),
    ("<Button Click='{x:Bind local:XBindHelpers.Clicked, Mode=OneWay}'/>", "'Mode' cannot be set on an x:Bind of the event 'Click'"),
    (
        "<StackPanel.Resources><local:XBindTagConverter x:Key='tag'/></StackPanel.Resources><Button Click='{x:Bind local:XBindHelpers.Clicked, Converter={StaticResource tag}}'/>",
        "'Converter' cannot be set on an x:Bind of the event 'Click'",
    ),
    ("<Button Click='{x:Bind local:XBindHelpers.Clicked, FallbackValue=none}'/>", "'FallbackValue' cannot be set on an x:Bind of the event 'Click'"),
];

fn diagnostic_document(body: &str) -> String {
    format!(
        "<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests;assembly=FerroUI.Markup.Xaml.UnitTests'>\n  {body}\n</StackPanel>"
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
    crate::register_types();
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
        &[
            ("x_bind_root.xaml", ROOT),
            ("x_bind_template.xaml", TEMPLATE),
            ("x_bind_functions.xaml", FUNCTIONS),
            ("x_bind_casts.xaml", CASTS),
            ("x_bind_converters.xaml", CONVERTERS),
            ("x_bind_events.xaml", EVENTS),
        ],
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
