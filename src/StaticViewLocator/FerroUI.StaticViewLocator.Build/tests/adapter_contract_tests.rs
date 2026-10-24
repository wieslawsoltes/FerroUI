//! `StaticViewLocatorGeneratorAdapterContractTests.cs`.

use super::*;
use crate::diagnostics::{INCOMPATIBLE_ADAPTER_MEMBER, MISSING_ADAPTER_TYPE};

#[test]
fn missing_reactive_ui_reference_produces_targeted_diagnostic() {
    // The view locator of ReactiveUI needs the feature `reactiveui` of
    // ferroui-static-view-locator.
    let outcome = run_with(
        "#[ferroui_static_view_locator::static_view_locator(generate_view_locator = true)]
pub struct ViewLocator;
",
        |build| build.reactiveui_available(false),
    );
    let diagnostic = single(&outcome, MISSING_ADAPTER_TYPE);
    assert!(diagnostic.message.contains("ferroui_reactiveui::IViewLocator"));
    let source = file(&outcome, "ViewLocator.rs");
    assert!(!source.contains("reactiveui"), "{source}");
}

#[test]
fn incompatible_data_template_members_produce_targeted_diagnostics() {
    // An inherent `build` or `match_` would shadow the method of the generated data template.
    let outcome = run(
        "use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::Control;

#[ferroui_static_view_locator::static_view_locator(generate_data_template = true)]
pub struct ViewLocator;

impl ViewLocator {
    fn build(&self, value: Option<&BoxedValue>) -> Option<Ref<Control>> {
        None
    }

    pub fn match_(&self, value: Option<&BoxedValue>) -> i32 {
        0
    }
}
",
    );
    let found = diagnostics(&outcome, INCOMPATIBLE_ADAPTER_MEMBER);
    assert_eq!(found.len(), 2, "{:#?}", outcome.diagnostics);
    assert!(found.iter().any(|diagnostic| diagnostic.message.contains("ITemplateWithParam::build")));
    assert!(found.iter().any(|diagnostic| diagnostic.message.contains("IDataTemplate::match_")));
}

#[test]
fn incompatible_exact_factory_and_hook_produce_targeted_diagnostics() {
    let outcome = run(
        "use std::any::TypeId;

#[ferroui_static_view_locator::static_view_locator(generate_data_template = true)]
pub struct ViewLocator;

impl ViewLocator {
    fn try_create_view_exact(view_model_type: TypeId) -> i32 {
        0
    }

    fn build_fallback_view(&self) -> Option<i32> {
        None
    }
}
",
    );
    let found = diagnostics(&outcome, INCOMPATIBLE_ADAPTER_MEMBER);
    assert_eq!(found.len(), 2, "{:#?}", outcome.diagnostics);
    assert!(found.iter().any(|diagnostic| diagnostic.message.contains("try_create_view_exact")));
    assert!(found.iter().any(|diagnostic| diagnostic.message.contains("build_fallback_view")));
}

#[test]
fn incompatible_reactive_ui_resolve_method_produces_targeted_diagnostic() {
    let outcome = run(
        "#[ferroui_static_view_locator::static_view_locator(generate_view_locator = true)]
pub struct ViewLocator;

impl ViewLocator {
    pub fn resolve_view_default<VM>(&self, contract: &str) -> Option<i32> {
        None
    }
}
",
    );
    let diagnostic = single(&outcome, INCOMPATIBLE_ADAPTER_MEMBER);
    assert!(diagnostic.message.contains("resolve_view_default<VM>(&self)"), "{}", diagnostic.message);
}
