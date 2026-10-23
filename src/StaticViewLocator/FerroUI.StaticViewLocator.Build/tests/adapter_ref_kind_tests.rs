//! `StaticViewLocatorGeneratorAdapterRefKindTests.cs`.

use super::*;
use crate::diagnostics::INCOMPATIBLE_ADAPTER_MEMBER;

#[test]
fn ref_kind_overloads_do_not_suppress_by_value_adapter_members() {
    // The reference keeps generating a member when the locator declares an overload of it with
    // `ref` or `in` parameters. Rust has no overloads: a method of the name of a hook is the hook,
    // and one that takes `&mut self` or other parameters cannot stand for it. It is reported
    // rather than taken silently.
    let outcome = run(
        "use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::Control;

#[ferroui_static_view_locator::static_view_locator(
    generate_view_locator = true,
    generate_data_template = true,
    generate_runtime_type_fallback_methods = false,
)]
pub struct ViewLocator;

impl ViewLocator {
    fn build_fallback_view(&mut self, value: &mut Option<BoxedValue>) -> Option<Ref<Control>> {
        None
    }

    fn match_data_template(&mut self, value: Option<&BoxedValue>) -> bool {
        false
    }
}
",
    );
    let found = diagnostics(&outcome, INCOMPATIBLE_ADAPTER_MEMBER);
    assert_eq!(found.len(), 2, "{:#?}", outcome.diagnostics);
    assert!(found.iter().any(|diagnostic| diagnostic.message.contains("build_fallback_view")));
    assert!(found.iter().any(|diagnostic| diagnostic.message.contains("match_data_template")));
    // The members of the other adapters are still generated.
    let source = file(&outcome, "ViewLocator.rs");
    assert!(source.contains("fn resolve_view(&self, view_model: "), "{source}");
    assert!(source.contains("fn build_resolved_view(&self, param: "), "{source}");
}
