//! Port of `Xaml/StyleIncludeTests.cs`, against compiled documents.
//!
//! The document a test loads with a `StyleInclude` of `ferres://Tests/..` or of
//! a document of upstream's test assembly is a document of this crate
//! ([`crate::compiled_xaml`]) that includes a document of the crate
//! `xaml-include-fixture-theme` (the assembly `Tests`): the include is compiled
//! into a call of that document's build function. A relative include names a
//! document of the assembly of the including document, so both documents of
//! such a test are documents of `xaml-include-fixture-theme`, compiled as one
//! group there. `StyleInclude_Should_Be_Replaced_With_Direct_Call` includes the
//! class document of the Simple theme, compiled into a creation of its class.
//! `Style_Inside_Resources_Should_Produce_Warning` compiles its document in the
//! test to read the warning of the compiler. Not from upstream:
//! `fluent_theme_is_included_across_crates` and
//! `a_document_of_another_crate_that_is_not_public_is_not_included`.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

use ferroui_base::metadata::{from_markup_value, IServiceProvider};
use ferroui_base::platform::{AssetAssembly, IAssetLoader, StandardAssetLoader};
use ferroui_base::styling::{styles_as_style, IStyle, Style, Styles};
use ferroui_base::utilities::Uri;
use ferroui_base::{BoxedValue, FerroLocator, Ref};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{ContentControl, Control};
use ferroui_markup_xaml::styling::StyleInclude;
use ferroui_markup_xaml::xaml_il::runtime::{
    IFerroXamlIlEagerParentStackProvider, IFerroXamlIlParentStackProvider, XamlIlRuntimeHelpers,
};
use ferroui_markup_xaml::{
    IUriContext, RuntimeXamlDiagnostic, RuntimeXamlDiagnosticSeverity, RuntimeXamlLoaderConfiguration,
    ServiceProviderExtensions,
};
use ferroui_markup_xaml_loader::rust_emitter::compile_documents;
use ferroui_themes_fluent::FluentTheme;
use ferroui_themes_simple::SimpleTheme;
use xaml_include_fixture_theme::StyleWithServiceProvider;

use super::compiled_xaml_tests::{dependencies, root_uri};
use super::support::{assert_is_type, assert_value_is_type, build, describe, try_build, xaml_test_base, Build};
use crate::compiled_xaml;
use crate::documents::STYLE_INSIDE_RESOURCES_SHOULD_PRODUCE_WARNING;

/// `Assert.IsType<T>(style)` for an item of a style collection.
#[track_caller]
fn assert_style_is_type<T: ferroui_base::ObjectType>(style: &Rc<dyn IStyle>) {
    assert_is_type::<T>(style.as_object().expect("the style is an object of the class model"));
}

/// The tests whose two documents are compiled as one group of the library:
/// `objects[0]` is the included style, `objects[1]` the content control whose
/// resources include it.
#[track_caller]
fn resolved_with_two_files(style: Build<Style>, root: Build<ContentControl>) {
    let _base = xaml_test_base();
    let style = build(style);
    assert_is_type::<Style>(&style);
    let content_control = build(root);
    assert_is_type::<ContentControl>(&content_control);

    assert_value_is_type::<Style>(&content_control.resources().get(&"Include".into()));
}

#[test]
fn style_include_is_built() {
    let _base = xaml_test_base();
    let mut services = TestServices::styled_window();
    services.theme = Some(Rc::new(|| styles_as_style(&Styles::new())));
    let _app = UnitTestApplication::start(services);

    let window = build(compiled_xaml::build_styleinclude_is_built_xaml);

    assert_style_is_type::<Style>(&window.styles().get(0));
}

#[test]
fn style_include_is_built_resources() {
    let _base = xaml_test_base();
    let mut services = TestServices::styled_window();
    services.theme = Some(Rc::new(|| styles_as_style(&Styles::new())));
    let _app = UnitTestApplication::start(services);

    let content_control = build(compiled_xaml::build_styleinclude_is_built_resources_xaml);

    assert_value_is_type::<Style>(&content_control.resources().get(&"Include".into()));
}

#[test]
fn style_include_is_resolved_with_two_files() {
    resolved_with_two_files(
        xaml_include_fixture_theme::compiled_xaml::build_style_xaml,
        compiled_xaml::build_styleinclude_is_resolved_with_two_files_xaml,
    );
}

#[test]
fn relative_back_style_include_is_resolved_with_two_files() {
    resolved_with_two_files(
        xaml_include_fixture_theme::compiled_xaml::build_subfolder_style_xaml,
        xaml_include_fixture_theme::compiled_xaml::build_subfolder_folder_root_xaml,
    );
}

#[test]
fn relative_root_style_include_is_resolved_with_two_files() {
    resolved_with_two_files(
        xaml_include_fixture_theme::compiled_xaml::build_style_xaml,
        xaml_include_fixture_theme::compiled_xaml::build_folder_root_xaml,
    );
}

#[test]
fn relative_style_include_is_resolved_with_two_files() {
    resolved_with_two_files(
        xaml_include_fixture_theme::compiled_xaml::build_relative_styleinclude_is_resolved_with_two_files_folder_style_xaml,
        xaml_include_fixture_theme::compiled_xaml::build_relative_styleinclude_is_resolved_with_two_files_folder_root_xaml,
    );
}

#[test]
fn relative_dot_syntax_style_include_is_resolved_with_two_files() {
    resolved_with_two_files(
        xaml_include_fixture_theme::compiled_xaml::build_relative_dot_syntax_styleinclude_is_resolved_with_two_files_folder_style_xaml,
        xaml_include_fixture_theme::compiled_xaml::build_relative_dot_syntax_styleinclude_is_resolved_with_two_files_folder_root_xaml,
    );
}

#[test]
fn non_latin_style_include_is_resolved_with_two_files() {
    resolved_with_two_files(
        compiled_xaml::build_u30b9_u30bf_u30a4_u30eb_xaml,
        compiled_xaml::build_nonlatin_styleinclude_is_resolved_with_two_files_xaml,
    );
}

#[test]
fn missing_resource_key_in_style_include_does_not_cause_stack_overflow() {
    let _base = xaml_test_base();
    // Only the failure of the lookup of the missing key is expected (the key-not-found
    // error of the static resource); anything else fails.
    if let Err(error) = try_build(compiled_xaml::build_missing_resourcekey_in_styleinclude_does_not_cause_stackoverflow_xaml) {
        let description = describe(&error);
        assert!(description.contains("Static resource 'missing' not found."), "the document failed to load: {description}");
    }
}

#[test]
fn style_include_should_be_replaced_with_direct_call() {
    let _base = xaml_test_base();
    let _app = UnitTestApplication::start(TestServices::styled_window());

    let control = build(compiled_xaml::build_styleinclude_should_be_replaced_with_direct_call_xaml);
    assert_style_is_type::<SimpleTheme>(&control.styles().get(0));
    assert_style_is_type::<SimpleTheme>(&control.styles().get(1));
}

/// Not from upstream: the class document of the Fluent theme, included from another
/// crate, is created with its class, as the Simple theme's is.
#[test]
fn fluent_theme_is_included_across_crates() {
    let _base = xaml_test_base();
    let _app = UnitTestApplication::start(TestServices::styled_window());

    let control = build(compiled_xaml::build_fluent_theme_is_included_across_crates_xaml);
    assert_style_is_type::<FluentTheme>(&control.styles().get(0));
}

/// Not from upstream: a document of the Fluent theme that is not public
/// (`x:ClassModifier="internal"`) cannot be included from another crate: upstream's
/// diagnostic.
#[test]
fn a_document_of_another_crate_that_is_not_public_is_not_included() {
    let _base = xaml_test_base();
    let documents = [(
        "Internal.xaml",
        "
<ContentControl xmlns='https://github.com/ferroui'>
    <ContentControl.Styles>
        <StyleInclude Source='ferres://FerroUI.Themes.Fluent/Controls/FluentControls.xaml'/>
    </ContentControl.Styles>
</ContentControl>",
    )];
    let compiled = compile_documents(&documents, Some(&root_uri()), &RuntimeXamlLoaderConfiguration::new(), &dependencies());
    let reason = compiled[0].source.clone().expect_err("the document does not compile");
    assert!(
        reason.contains(
            "Unable to resolve XAML resource \"ferres://ferroui.themes.fluent/Controls/FluentControls.xaml\" in the \"ferroui.themes.fluent\" assembly. Make sure this file exists and is public."
        ),
        "{reason}"
    );
}

#[test]
fn style_inside_resources_should_produce_warning() {
    let _base = xaml_test_base();
    let _app = UnitTestApplication::start(TestServices::styled_window());

    // The diagnostics are the compiler's: the document is compiled here with a handler.
    let diagnostics: Rc<RefCell<Vec<RuntimeXamlDiagnostic>>> = Rc::new(RefCell::new(Vec::new()));
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&crate::ASSEMBLY);
    configuration.diagnostic_handler = Some(Rc::new({
        let diagnostics = diagnostics.clone();
        move |diagnostic: &RuntimeXamlDiagnostic| {
            diagnostics.borrow_mut().push(diagnostic.clone());
            diagnostic.severity
        }
    }));
    let compiled = compile_documents(
        &[("Style_Inside_Resources_Should_Produce_Warning.xaml", STYLE_INSIDE_RESOURCES_SHOULD_PRODUCE_WARNING)],
        Some(&root_uri()),
        &configuration,
        &dependencies(),
    );
    if let Err(reason) = &compiled[0].source {
        panic!("the document does not compile: {reason}");
    }

    let control = build(compiled_xaml::build_style_inside_resources_should_produce_warning_xaml);
    let merged = control.resources().merged_dictionaries().get(0);
    let merged_object = merged.as_object().expect("the merged dictionary is an object of the class model").to_ref();
    assert!(from_markup_value::<Rc<dyn IStyle>>(&Some(Rc::new(merged_object) as BoxedValue)).is_some());
    let diagnostics = diagnostics.borrow();
    assert_eq!(1, diagnostics.len(), "{diagnostics:?}");
    let warning = &diagnostics[0];
    assert_eq!(RuntimeXamlDiagnosticSeverity::Warning, warning.severity);
}

#[test]
fn style_include_from_code_behind_resolves_compiled() {
    let _base = xaml_test_base();
    let locator_scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<dyn IAssetLoader>()
        .to_constant(Rc::new(StandardAssetLoader::new(Some(&AssetAssembly::new(xaml_include_fixture_theme::ASSEMBLY.name)))));

    let sp = TestServiceProvider::new();
    let service_provider: Rc<dyn IServiceProvider> = sp.clone();
    let style_include = StyleInclude::with_service_provider(service_provider.clone());
    style_include.set_source(Some(Uri::absolute("ferres://Tests/Xaml/StyleWithServiceProvider.xaml").expect("a valid URI")));

    let loaded = style_include.loaded();
    let loaded = loaded.as_object().expect("the loaded style is an object of the class model");
    assert_is_type::<StyleWithServiceProvider>(loaded);
    let loaded: Ref<StyleWithServiceProvider> = loaded.to_ref().cast().expect("a StyleWithServiceProvider");
    let loaded_service_provider = loaded.service_provider().expect("the style has a service provider");

    assert!(
        service_provider.get_required_service::<Rc<dyn IFerroXamlIlParentStackProvider>>().parents()
            == loaded_service_provider.get_required_service::<Rc<dyn IFerroXamlIlParentStackProvider>>().parents()
    );

    locator_scope.dispose();
}

/// Port of `TestServiceProvider` of `Xaml/StyleIncludeTests.cs`: a service provider
/// that is its own URI context and parent stack provider; every other service comes
/// from a root service provider.
struct TestServiceProvider {
    this: Weak<TestServiceProvider>,
    root: Rc<dyn IServiceProvider>,
    base_uri: RefCell<Option<Uri>>,
    parents_stack: Rc<Vec<BoxedValue>>,
}

impl TestServiceProvider {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            root: XamlIlRuntimeHelpers::create_root_service_provider_v2(),
            base_uri: RefCell::new(None),
            parents_stack: Rc::new(vec![Control::boxed(ContentControl::new())]),
        })
    }

    fn this(&self) -> Rc<TestServiceProvider> {
        self.this.upgrade().expect("the object is alive while it is used")
    }
}

impl IServiceProvider for TestServiceProvider {
    fn get_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
        if service_type == TypeId::of::<Rc<dyn IUriContext>>() {
            let context: Rc<dyn IUriContext> = self.this();
            return Some(Rc::new(context));
        }
        if service_type == TypeId::of::<Rc<dyn IFerroXamlIlParentStackProvider>>() {
            let provider: Rc<dyn IFerroXamlIlParentStackProvider> = self.this();
            return Some(Rc::new(provider));
        }
        self.root.get_service(service_type)
    }
}

impl IUriContext for TestServiceProvider {
    fn base_uri(&self) -> Option<Uri> {
        self.base_uri.borrow().clone()
    }

    fn set_base_uri(&self, value: Option<Uri>) {
        *self.base_uri.borrow_mut() = value;
    }
}

impl IFerroXamlIlParentStackProvider for TestServiceProvider {
    fn parents(&self) -> Vec<BoxedValue> {
        self.parents_stack.iter().rev().cloned().collect()
    }

    fn as_eager_parent_stack_provider(self: Rc<Self>) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        Some(self)
    }
}

impl IFerroXamlIlEagerParentStackProvider for TestServiceProvider {
    fn direct_parents_stack(&self) -> Rc<Vec<BoxedValue>> {
        self.parents_stack.clone()
    }

    fn parent_provider(&self) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        None
    }
}
