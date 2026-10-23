//! `StaticViewLocatorGeneratorDataTemplateMatchTests.cs`.

use super::*;

#[test]
fn default_generated_match_requires_a_resolved_view_factory() {
    // The reference maps a base class explicitly and resolves the generic view models derived
    // from it through a run-time type test. A Rust struct derives from no class: the view model
    // implements a trait mapped explicitly, and the mapping is flattened onto it. The generated
    // `match_` asks for a resolved factory only: no lookup of the missing views, no generic
    // definition.
    let outcome = generated(&format!(
        "pub mod view_models {{
    pub trait WidgetViewModelBase {{}}
    pub struct WidgetViewModel;
    impl WidgetViewModelBase for WidgetViewModel {{}}
}}

pub mod views {{
    {VIEW_IMPORTS}
    {}
}}

#[ferroui_static_view_locator::static_view_mapping(view_models::WidgetViewModelBase, views::WidgetView)]
#[ferroui_static_view_locator::static_view_locator(generate_data_template = true)]
pub struct ViewLocator;
",
        view("WidgetView")
    ));
    let source = file(&outcome, "ViewLocator.rs");
    assert!(source.contains(&view_entry("crate::view_models::WidgetViewModel", "crate::views::WidgetView::new")), "{source}");
    assert!(source.contains(&view_entry("dyn crate::view_models::WidgetViewModelBase", "crate::views::WidgetView::new")), "{source}");
    assert!(source.contains("        Self::try_get_resolved_view_factory(data).is_some()\n"), "{source}");
    assert!(!source.contains("s_missing_views().contains"), "{source}");
    assert!(!source.contains("generic_definition"), "{source}");
}

#[test]
fn custom_missing_view_hook_omits_the_missing_view_table() {
    let outcome = generated(
        "use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::Control;
use ferroui_static_view_locator::RuntimeType;

pub struct UnresolvedViewModel;

#[ferroui_static_view_locator::static_view_locator(generate_data_template = true, generate_runtime_type_fallback_methods = false)]
pub struct ViewLocator;

impl ViewLocator {
    fn build_missing_view(&self, data: Option<&BoxedValue>, view_model_type: RuntimeType) -> Option<Ref<Control>> {
        None
    }
}
",
    );
    let source = file(&outcome, "ViewLocator.rs");
    assert!(!source.contains("s_missing_views"), "{source}");
    assert!(source.contains("self.build_missing_view(Some(data), view_model_type).into()"), "{source}");
}
