//! `StaticViewLocatorGeneratorArchitectureTests.cs`.

use super::*;
use crate::diagnostics::{AMBIGUOUS_VIEW_MAPPING, DUPLICATE_LOCATOR_FILE, UNSUPPORTED_LOCATOR_TYPE, VIEW_CANNOT_BE_CONSTRUCTED};

#[test]
fn supports_internal_locator_in_global_namespace() {
    // A locator at the root of the crate, visible in the crate only.
    let outcome = generated(
        "#[ferroui_static_view_locator::static_view_locator(generate_data_template = true)]
pub(crate) struct ViewLocator;
",
    );
    let source = file(&outcome, "ViewLocator.rs");
    assert!(source.contains("for the locator `ViewLocator` (test_app)"), "{source}");
    assert!(source.contains("impl ViewLocator {"), "{source}");
    assert!(source.contains("::ferroui_static_view_locator::generated::IDataTemplate for ViewLocator {"), "{source}");
}

#[test]
fn uses_unique_hint_names_for_same_named_locators() {
    // Rust has no partial classes: `#[static_view_locator]` includes the file named after the
    // struct, and two locators of one name in a crate state distinct ids.
    let source = |first: &str, second: &str| {
        format!(
            "pub mod first {{
    #[ferroui_static_view_locator::static_view_locator({first})]
    pub struct ViewLocator;
}}

pub mod second {{
    #[ferroui_static_view_locator::static_view_locator({second})]
    pub struct ViewLocator;
}}
"
        )
    };
    let outcome = generated(&source("id = \"first_view_locator\"", "id = \"second_view_locator\""));
    assert!(outcome.file("first_view_locator.rs").is_some());
    assert!(outcome.file("second_view_locator.rs").is_some());

    let outcome = run(&source("", ""));
    assert_eq!(diagnostics(&outcome, DUPLICATE_LOCATOR_FILE).len(), 2, "{:#?}", outcome.diagnostics);
    assert!(outcome.files.is_empty());
}

#[test]
fn reports_mapped_view_without_usable_constructor() {
    // The constructor is private to the module of the view.
    let outcome = run(&format!(
        "pub mod view_models {{
    pub struct SampleViewModel;
}}

pub mod views {{
    {VIEW_IMPORTS}
    pub struct SampleView {{ base: UserControl }}
    ferro_class!(SampleView: UserControl);
    impl SampleView {{
        fn new() -> Ref<Self> {{
            unimplemented!()
        }}
    }}
}}

#[ferroui_static_view_locator::static_view_locator(generate_data_template = true)]
pub struct ViewLocator;
"
    ));
    let diagnostic = single(&outcome, VIEW_CANNOT_BE_CONSTRUCTED);
    assert!(diagnostic.message.contains("SampleView"));
}

#[test]
fn reports_non_partial_locator_before_emission() {
    // A Rust locator is extended by an `impl` block: the attribute is reported on an item that is
    // not a struct (the C# locator must be partial, non-static and not file-local).
    let outcome = run(
        "#[ferroui_static_view_locator::static_view_locator]
pub enum ViewLocator {}
",
    );
    let diagnostic = single(&outcome, UNSUPPORTED_LOCATOR_TYPE);
    assert!(diagnostic.message.contains("an enumeration"));
    assert!(outcome.file("ViewLocator.rs").is_none());
}

#[test]
fn reports_ambiguous_automatic_reactive_ui_mapping() {
    let outcome = run(&ambiguous_reactive_ui_source(false));
    let diagnostic = single(&outcome, AMBIGUOUS_VIEW_MAPPING);
    assert!(diagnostic.message.contains("FirstView"));
    assert!(diagnostic.message.contains("SecondView"));
}

#[test]
fn reports_ambiguous_configured_contract_mapping() {
    let outcome = run(&format!(
        "{VIEW_IMPORTS}
pub trait IViewFor<VM> {{}}
pub struct SampleModel;

{}
impl IViewFor<SampleModel> for FirstView {{}}

{}
impl IViewFor<SampleModel> for SecondView {{}}

#[ferroui_static_view_locator::static_view_locator(view_model_mapping_contracts = [IViewFor])]
pub struct ViewLocator;
",
        view("FirstView"),
        view("SecondView")
    ));
    let diagnostic = single(&outcome, AMBIGUOUS_VIEW_MAPPING);
    assert!(diagnostic.message.contains("FirstView"));
    assert!(diagnostic.message.contains("SecondView"));
}

#[test]
fn explicit_mapping_resolves_automatic_reactive_ui_ambiguity() {
    let outcome = generated(&ambiguous_reactive_ui_source(true));
    let source = file(&outcome, "ViewLocator.rs");
    assert!(source.contains(&view_entry("crate::SampleViewModel", "crate::FirstView::new")), "{source}");
    assert!(!source.contains("crate::SecondView::new"), "{source}");
}

/// Two views of one view model of ReactiveUI (`IControlViewFor`, the `IViewFor<TViewModel>` of a
/// control), with an explicit mapping to the first when `explicit_mapping`.
fn ambiguous_reactive_ui_source(explicit_mapping: bool) -> String {
    let mapping = if explicit_mapping { "#[ferroui_static_view_locator::static_view_mapping(SampleViewModel, FirstView)]" } else { "" };
    format!(
        "{VIEW_IMPORTS}
use ferroui_reactiveui_controls::IControlViewFor;

pub struct SampleViewModel;

{}
impl IControlViewFor for FirstView {{
    type ViewModel = SampleViewModel;
}}

{}
impl IControlViewFor for SecondView {{
    type ViewModel = SampleViewModel;
}}

#[ferroui_static_view_locator::static_view_locator(generate_view_locator = true)]
{mapping}
pub struct ViewLocator;
",
        view("FirstView"),
        view("SecondView")
    )
}
