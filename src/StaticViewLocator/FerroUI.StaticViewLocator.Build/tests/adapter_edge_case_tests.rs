//! `StaticViewLocatorGeneratorAdapterEdgeCaseTests.cs`.

use super::*;
use crate::diagnostics::INCOMPATIBLE_ADAPTER_MEMBER;

#[test]
fn unrelated_overloads_do_not_suppress_generated_adapter_members() {
    // Rust has no overloads: members of other names than the adapter members (the reference's
    // overloads of other parameter types) leave them generated.
    let outcome = generated(&format!(
        "pub mod view_models {{
    pub struct SampleViewModel;
}}

pub mod views {{
    {VIEW_IMPORTS}
    use ferroui_reactiveui_controls::IControlViewFor;
    {}
    impl IControlViewFor for SampleView {{
        type ViewModel = crate::view_models::SampleViewModel;
    }}
}}

use ferroui_base::Ref;
use ferroui_controls::Control;

#[ferroui_static_view_locator::static_view_locator(
    generate_view_locator = true,
    generate_data_template = true,
    generate_runtime_type_fallback_methods = false,
)]
pub struct ViewLocator;

impl ViewLocator {{
    pub fn build_text(&self, value: Option<&str>) -> Option<Ref<Control>> {{
        None
    }}

    pub fn match_text(&self, value: Option<&str>) -> bool {{
        false
    }}

    pub fn resolve_view_text(&self, value: Option<&str>) -> Option<i32> {{
        None
    }}

    fn build_fallback_view_text(&self, value: Option<&str>) -> Option<Ref<Control>> {{
        None
    }}
}}
",
        view("SampleView")
    ));
    let source = file(&outcome, "ViewLocator.rs");
    assert!(source.contains("fn build(&self, param: &::ferroui_static_view_locator::generated::Option<"), "{source}");
    assert!(source.contains("fn match_(&self, data: "), "{source}");
    assert!(source.contains("fn resolve_view(&self, view_model: "), "{source}");
    assert!(source.contains("fn build_fallback_view(&self, param: "), "{source}");
}

#[test]
fn configured_mapping_contract_takes_precedence_over_automatic_reactive_ui_mapping() {
    let outcome = generated(&format!(
        "{VIEW_IMPORTS}
use ferroui_reactiveui_controls::IControlViewFor;

pub trait IConfiguredView<VM> {{}}
pub struct DashboardModel;

{}
impl IConfiguredView<DashboardModel> for ConfiguredDashboardView {{}}

{}
impl IControlViewFor for ReactiveDashboardView {{
    type ViewModel = DashboardModel;
}}

#[ferroui_static_view_locator::static_view_locator(
    generate_view_locator = true,
    generate_runtime_type_fallback_methods = false,
    view_model_mapping_contracts = [IConfiguredView],
)]
pub struct ViewLocator;
",
        view("ConfiguredDashboardView"),
        view("ReactiveDashboardView")
    ));
    let source = file(&outcome, "ViewLocator.rs");
    assert!(source.contains(&view_entry("crate::DashboardModel", "crate::ConfiguredDashboardView::new")), "{source}");
    assert!(!source.contains("crate::ReactiveDashboardView::new"), "{source}");
}

#[test]
fn generated_i_data_template_does_not_emit_unused_runtime_fallback_helpers() {
    let outcome = generated(&format!(
        "pub mod view_models {{
    pub struct SampleViewModel;
}}

pub mod views {{
    {VIEW_IMPORTS}
    {}
}}

#[ferroui_static_view_locator::static_view_locator(generate_data_template = true)]
pub struct ViewLocator;
",
        view("SampleView")
    ));
    let source = file(&outcome, "ViewLocator.rs");
    assert!(source.contains("fn build(&self, param: "), "{source}");
    assert!(!source.contains("try_get_factory_from_interfaces"), "{source}");
    assert!(!source.contains("try_get_missing_view_from_interfaces"), "{source}");
}

#[test]
fn existing_exact_factory_is_reused_instead_of_generated_again() {
    let outcome = generated(
        "use ferroui_base::Ref;
use ferroui_controls::Control;
use std::any::TypeId;

#[ferroui_static_view_locator::static_view_locator(generate_view_locator = true, generate_runtime_type_fallback_methods = false)]
pub struct ViewLocator;

impl ViewLocator {
    fn try_create_view_exact(view_model_type: TypeId) -> Option<Ref<Control>> {
        None
    }
}
",
    );
    let source = file(&outcome, "ViewLocator.rs");
    assert!(!source.contains("fn try_create_view_exact("), "{source}");
}

#[test]
fn covariant_build_hook_return_type_compiles() {
    // A hook may return any type that converts into `Option<Ref<Control>>` (`Ref<Control>` here):
    // the generated `build` converts its result. The tests of `FerroUI.StaticViewLocator.Tests`
    // compile such a hook.
    let outcome = generated(
        "use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::Control;

#[ferroui_static_view_locator::static_view_locator(generate_data_template = true, generate_runtime_type_fallback_methods = false)]
pub struct ViewLocator;

impl ViewLocator {
    fn build_resolved_view(&self, value: Option<&BoxedValue>) -> Ref<Control> {
        unimplemented!()
    }
}
",
    );
    assert!(diagnostics(&outcome, INCOMPATIBLE_ADAPTER_MEMBER).is_empty());
    let source = file(&outcome, "ViewLocator.rs");
    assert!(!source.contains("fn build_resolved_view("), "{source}");
    assert!(source.contains("self.build_resolved_view(Some(data)).into()"), "{source}");
}

#[test]
fn existing_reactive_ui_resolve_methods_are_not_duplicated() {
    let outcome = generated(
        "use ferroui_reactiveui::{IEnableLogger, IViewFor, IViewForOf, IViewLocator, ViewModelType};
use std::any::Any;
use std::rc::Rc;

#[ferroui_static_view_locator::static_view_locator(generate_view_locator = true, generate_runtime_type_fallback_methods = false)]
pub struct ViewLocator;

impl ViewLocator {
    pub fn resolve_view_default<VM: 'static>(&self) -> Option<Rc<dyn IViewForOf<VM>>> {
        None
    }

    pub fn resolve_view_for<VM: 'static>(&self, contract: Option<&str>) -> Option<Rc<dyn IViewForOf<VM>>> {
        None
    }
}

impl IEnableLogger for ViewLocator {}

impl IViewLocator for ViewLocator {
    fn resolve_view_typed(&self, view_model: Rc<dyn Any>, view_model_type: &ViewModelType, contract: Option<&str>) -> Option<Rc<dyn IViewFor>> {
        None
    }

    fn resolve_view(&self, view_model: Option<Rc<dyn Any>>, contract: Option<&str>) -> Option<Rc<dyn IViewFor>> {
        None
    }

    fn resolve_view_unsafe(&self, view_model: Option<Rc<dyn Any>>, contract: Option<&str>) -> Option<Rc<dyn IViewFor>> {
        None
    }
}
",
    );
    let source = file(&outcome, "ViewLocator.rs");
    assert!(!source.contains("fn resolve_view_default"), "{source}");
    assert!(!source.contains("fn resolve_view_for"), "{source}");
    assert!(!source.contains("IViewLocator for ViewLocator"), "{source}");
    assert!(!source.contains("IEnableLogger for ViewLocator"), "{source}");
}
