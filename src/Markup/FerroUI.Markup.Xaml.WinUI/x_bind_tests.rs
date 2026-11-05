//! WinUI documents on the x:Bind core of the XAML compiler (#525, X6; docs/porting/x-bind.md).
//!
//! The importer keeps WinUI's `{x:Bind}` and its directives as written; the run-time loader
//! compiles them into `XBindExtension` and the per-root `XBindings` of `ferroui-markup-xaml`.
//! These tests load WinUI documents through `parse_winui_as` and check the lifecycle WinUI
//! generates (initialized when the root is attached to a visual tree, a template instance driven
//! by its data context or by its container phase by phase), the directives (`x:DefaultBindMode`,
//! `x:Phase`, `x:Load`, `x:DeferLoadStrategy`) and the compiler's WinUI-worded diagnostics as load
//! errors. `compiled_tests` runs the root and template fixtures through the Rust emitter too.

use std::rc::Rc;

use ferroui_base::controls::NameScope;
use ferroui_base::{ObjectType, Ref};
use ferroui_controls::templates::{IDataTemplate, IDataTemplateComponent, XamlBindingHelper};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, Button, ContentControl, Control, StackPanel, TextBlock, Window};
use ferroui_markup_xaml::markup_extensions::{DeferredElement, XBindings};
use ferroui_markup_xaml::XamlLoadError;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

use crate::{WinUIXBindPolicy, WinUIXamlOptions, WINUI_PRESENTATION_NAMESPACE};

const XAML_NAMESPACE: &str = "http://schemas.microsoft.com/winfx/2006/xaml";
const ROOT: &str = include_str!("fixtures/winui_root.xaml");

fn start() -> UnitTestApplicationScope {
    crate::register_types();
    let scope = UnitTestApplication::start(TestServices::styled_window());
    FerroRuntimeXamlLoader::register();
    scope
}

/// A WinUI document: `body` inside a root element `root` with the WinUI namespaces.
fn document(root: &str, attributes: &str, body: &str) -> String {
    format!("<{root} xmlns='{WINUI_PRESENTATION_NAMESPACE}' xmlns:x='{XAML_NAMESPACE}' {attributes}>{body}</{root}>")
}

fn load<T: Clone + 'static>(xaml: &str) -> T {
    match crate::parse_winui_as::<T>(xaml, None, &WinUIXamlOptions::default()) {
        Ok(loaded) => loaded,
        Err(error) => panic!("{}", describe(&error)),
    }
}

/// A load error with the errors that caused it.
fn describe(error: &XamlLoadError) -> String {
    let mut text = error.to_string();
    let mut source = std::error::Error::source(error);
    while let Some(inner) = source {
        text.push_str(&format!("\n ---> {inner}"));
        source = std::error::Error::source(inner);
    }
    text
}

fn load_error(xaml: &str, options: &WinUIXamlOptions) -> String {
    match crate::parse_winui(xaml, None, options) {
        Ok(_) => panic!("the document loaded: {xaml}"),
        Err(error) => describe(&error),
    }
}

fn set_tag(root: &StackPanel, text: &str) {
    root.set_tag(Some(Rc::new(text.to_string())));
}

fn text_of(root: &StackPanel, name: &str) -> Option<String> {
    NameScope::get_name_scope(root)
        .and_then(|scope| scope.find_as::<TextBlock>(name))
        .and_then(|block| block.text())
}

/// WinUI initializes the bindings of a page on `Loading`; the core initializes the root of a
/// document when it is attached to a visual tree, after code-behind has set what they read.
#[test]
fn root_is_initialized_when_it_is_attached_to_a_visual_tree() {
    let _app = start();
    let root: Ref<StackPanel> = load(ROOT);
    let bindings = XBindings::get_for_root(&root).expect("the first x:Bind of the document creates the bindings of its root");
    assert!(!bindings.is_initialized());
    assert_eq!(text_of(&root, "initial"), None);
    set_tag(&root, "set before loading");
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    assert!(root.is_attached_to_visual_tree(), "showing the window attaches its content");
    assert!(bindings.is_initialized());
    assert_eq!(text_of(&root, "initial").as_deref(), Some("set before loading"));
    assert_eq!(text_of(&root, "mirror").as_deref(), Some("set before loading"));
    set_tag(&root, "after loading");
    assert_eq!(text_of(&root, "initial").as_deref(), Some("set before loading"), "OneTime is the default");
    assert_eq!(text_of(&root, "mirror").as_deref(), Some("after loading"));
    assert!(bindings.diagnostics().is_empty());
}

#[test]
fn the_bindings_do_not_keep_the_root_of_a_winui_document_alive() {
    let _app = start();
    let weak = {
        let root: Ref<StackPanel> = load(ROOT);
        XBindings::get_for_root(&root).unwrap().initialize().unwrap();
        root.downgrade()
    };
    assert!(weak.upgrade().is_none(), "the root, its targets and its bindings are released");
}

/// `x:DefaultBindMode` gives the bindings of its subtree their mode; an explicit mode wins.
#[test]
fn default_bind_mode_of_a_winui_document() {
    let _app = start();
    let xaml = document(
        "StackPanel",
        "x:DefaultBindMode='OneWay' Tag='construction value'",
        "<TextBlock x:Name='scoped' Text='{x:Bind Tag}'/><TextBlock x:Name='explicit' Text='{x:Bind Tag, Mode=OneTime}'/>",
    );
    let root: Ref<StackPanel> = load(&xaml);
    XBindings::get_for_root(&root).unwrap().initialize().unwrap();
    assert_eq!(text_of(&root, "scoped").as_deref(), Some("construction value"));
    set_tag(&root, "changed");
    assert_eq!(text_of(&root, "scoped").as_deref(), Some("changed"), "OneWay from x:DefaultBindMode");
    assert_eq!(text_of(&root, "explicit").as_deref(), Some("construction value"), "an explicit Mode wins");
}

fn found_as<T: ObjectType>(root: &StackPanel, name: &str) -> Option<Ref<T>> {
    DeferredElement::find_name(root, name).unwrap().and_then(|found| found.cast::<T>())
}

/// `x:Load="False"` and `x:DeferLoadStrategy="Lazy"`: a placeholder holds the place of the
/// element until `FindName` creates it; its bindings read the root of the document.
#[test]
fn deferred_elements_of_a_winui_document_load_with_find_name() {
    let _app = start();
    let xaml = document(
        "StackPanel",
        "Tag='root tag'",
        "<Border x:Name='lazy' x:Load='False'><TextBlock x:Name='lazyText' Text='{x:Bind Tag, Mode=OneWay}'/></Border><TextBlock x:Name='later' x:DeferLoadStrategy='Lazy' Text='{x:Bind Tag}'/>",
    );
    let root: Ref<StackPanel> = load(&xaml);
    let children = root.children();
    assert!(children.get(0).cast::<DeferredElement>().is_some());
    assert!(children.get(1).cast::<DeferredElement>().is_some());
    XBindings::get_for_root(&root).unwrap().initialize().unwrap();
    assert!(children.get(0).cast::<DeferredElement>().is_some(), "x:Load='False' waits for FindName");
    let lazy = found_as::<Border>(&root, "lazy").expect("FindName loads the element");
    assert!(children.get(0).cast::<Border>() == Some(lazy.clone()));
    let lazy_text = found_as::<TextBlock>(&root, "lazyText").expect("a named element of a loaded element");
    assert_eq!(lazy_text.text().as_deref(), Some("root tag"));
    let later = found_as::<TextBlock>(&root, "later").expect("x:DeferLoadStrategy='Lazy' loads with FindName");
    assert_eq!(later.text().as_deref(), Some("root tag"));
    set_tag(&root, "changed");
    assert_eq!(lazy_text.text().as_deref(), Some("changed"));
}

fn phased_item(content: &str, tag: &str) -> Ref<Button> {
    let result = Button::new();
    result.set_content(Some(Rc::new(content.to_string())));
    result.set_tag(Some(Rc::new(tag.to_string())));
    result
}

/// `x:Phase` in an item template: a container drives the instance through its template
/// component (WinUI's `ProcessBindings` and `Recycle`); the elements of a later phase are hidden
/// until their phase.
#[test]
fn phases_of_a_winui_item_template() {
    let _app = start();
    let xaml = document(
        "ContentControl",
        "",
        "<ContentControl.ContentTemplate><DataTemplate x:DataType='Button'><StackPanel><TextBlock Text='{x:Bind Content}'/><TextBlock x:Phase='1' Text='{x:Bind Tag}'/></StackPanel></DataTemplate></ContentControl.ContentTemplate>",
    );
    let host: Ref<ContentControl> = load(&xaml);
    let template: Rc<dyn IDataTemplate> = host.content_template().unwrap();
    let root = template.build(&None).unwrap();
    let bindings = XBindings::get_for_root(&root).expect("the bindings of the template instance");
    assert!(bindings.is_template());
    let component: Rc<dyn IDataTemplateComponent> = XamlBindingHelper::get_data_template_component(&root)
        .expect("the root of a typed template instance carries its bindings as its template component");
    let panel = root.to_ref().cast::<StackPanel>().unwrap();
    let first = panel.children().get(0).cast::<TextBlock>().unwrap();
    let later = panel.children().get(1).cast::<TextBlock>().unwrap();
    let item = phased_item("content", "tag");

    component.recycle();
    assert_eq!(component.process_bindings(Some(Rc::new(item.clone())), 0, 0), 1);
    assert!(bindings.is_container_driven());
    assert_eq!(first.text().as_deref(), Some("content"));
    assert_eq!(later.text(), None);
    assert!(!later.is_visible(), "an element of a later phase is hidden until its phase");
    assert_eq!(component.process_bindings(Some(Rc::new(item.clone())), -1, 1), -1);
    assert_eq!(later.text().as_deref(), Some("tag"));
    assert!(later.is_visible());
    assert!(bindings.diagnostics().is_empty());
}

/// The forms WinUI's compiler rejects are load errors with WinUI's messages, from the x:Bind
/// core; the importer adds nothing to them.
#[test]
fn winui_diagnostics_of_the_core_are_load_errors() {
    let _app = start();
    let options = WinUIXamlOptions::default();
    for (xaml, expected) in [
        (
            document("ContentControl", "", "<ContentControl.ContentTemplate><DataTemplate><TextBlock Text='{x:Bind Name}'/></DataTemplate></ContentControl.ContentTemplate>"),
            "DataTemplates containing x:Bind need a DataType to be specified using 'x:DataType'",
        ),
        (
            document("StackPanel", "", "<TextBlock Text='{x:Bind Missing}'/>"),
            "Unable to resolve property or method of name 'Missing'",
        ),
        (
            document("StackPanel", "", "<TextBox Text='{x:Bind Tag, Mode=TwoWay, UpdateSourceTrigger=Explicit}'/>"),
            "'Explicit' is not a supported value for 'UpdateSourceTrigger'",
        ),
        (
            document("StackPanel", "", "<TextBlock x:Phase='1' Text='{x:Bind Tag}'/>"),
            "Phase can only be used within a DataTemplate",
        ),
        (
            document("StackPanel", "x:DefaultBindMode='Default'", "<TextBlock Text='{x:Bind Tag}'/>"),
            "Invalid value specified for attribute 'DefaultBindMode'",
        ),
    ] {
        let error = load_error(&xaml, &options);
        assert!(error.contains(expected), "{xaml}: {error}");
    }
}

/// With `WinUIXBindPolicy::Reject` the migration reports the x:Bind, and nothing is loaded.
#[test]
fn reject_policy_is_a_migration_error() {
    let _app = start();
    let options = WinUIXamlOptions {
        x_bind: WinUIXBindPolicy::Reject,
        ..Default::default()
    };
    let error = load_error(ROOT, &options);
    assert!(error.contains("WXAML1007"), "{error}");
}
