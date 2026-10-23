//! `StaticViewLocatorGeneratorResolutionModeTests.cs`.
//!
//! A base class of a view model is a class of the object model (`ferro_class!`), whose values are
//! `Ref<T>`; an interface is a trait. The reference compiles the precedence into run-time type tests
//! for the types it cannot enumerate; Rust cannot test a value for a trait at run time, and the
//! generator flattens the precedence into the table of the views for every type of the crate.

use super::*;
use crate::diagnostics::{AMBIGUOUS_FALLBACK_MAPPING, UNSUPPORTED_OPEN_GENERIC_ADAPTER_MAPPING};

/// The handle of a class of the object model, as generated source keys it.
fn object(path: &str) -> String {
    format!("::ferroui_static_view_locator::generated::Ref<{path}>")
}

fn resolution_source(mode: &str) -> String {
    format!(
        "pub mod view_models {{
    use ferroui_base::{{ferro_class, FerroObject}};

    pub struct BaseViewModel {{ base: FerroObject }}
    ferro_class!(BaseViewModel: FerroObject);
    pub trait IContractViewModel {{}}
    pub struct DerivedViewModel {{ base: BaseViewModel }}
    ferro_class!(DerivedViewModel: BaseViewModel);
    impl IContractViewModel for DerivedViewModel {{}}
}}

pub mod views {{
    {VIEW_IMPORTS}
    {}
    {}
}}

#[ferroui_static_view_locator::static_view_mapping(view_models::BaseViewModel, views::BaseView)]
#[ferroui_static_view_locator::static_view_mapping(view_models::IContractViewModel, views::ContractView)]
#[ferroui_static_view_locator::static_view_locator(generate_data_template = true, generated_adapter_resolution_mode = {mode})]
pub struct ViewLocator;
",
        view("BaseView"),
        view("ContractView")
    )
}

/// The generated adapter path: from the resolver to the legacy helpers, if any.
fn generated_adapter_path(source: &str) -> &str {
    let start = source.find("fn try_get_resolved_view_factory").expect("the resolver is generated");
    match source.find("fn try_get_factory(") {
        Some(end) if end > start => &source[start..end],
        _ => &source[start..],
    }
}

#[test]
fn flattens_discovered_fallback_mappings_for_every_resolution_mode() {
    let modes = [
        ("Exact", ""),
        ("ExactThenBaseTypes", "crate::views::BaseView::new"),
        ("ExactThenInterfaces", "crate::views::ContractView::new"),
        ("ExactThenBaseTypesThenInterfaces", "crate::views::BaseView::new"),
        ("ExactThenInterfacesThenBaseTypes", "crate::views::ContractView::new"),
    ];
    for (mode, expected_view) in modes {
        let outcome = generated(&resolution_source(mode));
        let source = file(&outcome, "ViewLocator.rs");
        let derived = object("crate::view_models::DerivedViewModel");
        if expected_view.is_empty() {
            assert!(!source.contains(&view_entry_of(&derived)), "{mode}: {source}");
        } else {
            assert!(source.contains(&view_entry(&derived, expected_view)), "{mode}: {source}");
        }
        let adapter = generated_adapter_path(source);
        assert!(!adapter.contains("base_type"), "{mode}");
        assert!(!adapter.contains("interfaces"), "{mode}");
        assert!(!adapter.contains("is_assignable_from"), "{mode}");
    }
}

#[test]
fn emits_runtime_type_tests_in_configured_precedence_order() {
    // No run-time type test is emitted: the precedence decides the flattened entry of a type that
    // has both a mapped base class and a mapped trait, base first or trait first.
    let base_first = generated(&resolution_source("ExactThenBaseTypesThenInterfaces"));
    let interface_first = generated(&resolution_source("ExactThenInterfacesThenBaseTypes"));
    let derived = object("crate::view_models::DerivedViewModel");
    let base_first = file(&base_first, "ViewLocator.rs");
    let interface_first = file(&interface_first, "ViewLocator.rs");
    assert!(base_first.contains(&view_entry(&derived, "crate::views::BaseView::new")), "{base_first}");
    assert!(interface_first.contains(&view_entry(&derived, "crate::views::ContractView::new")), "{interface_first}");
    for source in [base_first, interface_first] {
        let adapter = generated_adapter_path(source);
        assert!(!adapter.contains(".is::<"), "{adapter}");
        assert!(!adapter.contains(".cast::<"), "{adapter}");
    }
}

#[test]
fn uses_nearest_mapped_interface_before_its_mapped_ancestor() {
    let outcome = generated(&format!(
        "{VIEW_IMPORTS}
pub trait IBaseViewModel {{}}
pub trait IDerivedViewModel: IBaseViewModel {{}}
pub struct ConcreteViewModel;
impl IBaseViewModel for ConcreteViewModel {{}}
impl IDerivedViewModel for ConcreteViewModel {{}}
{}
{}

#[ferroui_static_view_locator::static_view_mapping(IBaseViewModel, BaseView)]
#[ferroui_static_view_locator::static_view_mapping(IDerivedViewModel, DerivedView)]
#[ferroui_static_view_locator::static_view_locator(generate_data_template = true, generated_adapter_resolution_mode = ExactThenInterfaces)]
pub struct ViewLocator;
",
        view("BaseView"),
        view("DerivedView")
    ));
    let source = file(&outcome, "ViewLocator.rs");
    assert!(source.contains(&view_entry("crate::ConcreteViewModel", "crate::DerivedView::new")), "{source}");
    let derived = source.find(&entry_of("dyn crate::IDerivedViewModel")).expect("the entry of the derived trait");
    let base = source.find(&entry_of("dyn crate::IBaseViewModel")).expect("the entry of the base trait");
    assert!(base < derived, "the entries are sorted by view model");
}

#[test]
fn reports_ambiguous_mapped_interfaces_and_requires_an_explicit_mapping() {
    let outcome = run(&format!(
        "{VIEW_IMPORTS}
pub trait IAlphaViewModel {{}}
pub trait IBetaViewModel {{}}
pub struct AmbiguousViewModel;
impl IAlphaViewModel for AmbiguousViewModel {{}}
impl IBetaViewModel for AmbiguousViewModel {{}}
{}
{}

#[ferroui_static_view_locator::static_view_mapping(IAlphaViewModel, AlphaView)]
#[ferroui_static_view_locator::static_view_mapping(IBetaViewModel, BetaView)]
#[ferroui_static_view_locator::static_view_locator(generate_data_template = true, generated_adapter_resolution_mode = ExactThenInterfacesThenBaseTypes)]
pub struct ViewLocator;
",
        view("AlphaView"),
        view("BetaView")
    ));
    let diagnostic = single(&outcome, AMBIGUOUS_FALLBACK_MAPPING);
    assert!(diagnostic.message.contains("IAlphaViewModel"));
    assert!(diagnostic.message.contains("IBetaViewModel"));
}

#[test]
fn reports_open_generic_mapping_without_a_compiled_fallback_contract() {
    let outcome = run(&format!(
        "pub mod view_models {{
    pub struct FilterViewModel<T>(T);
}}

pub mod views {{
    {VIEW_IMPORTS}
    {}
}}

#[ferroui_static_view_locator::static_view_locator(generate_data_template = true)]
pub struct ViewLocator;
",
        view("FilterView")
    ));
    single(&outcome, UNSUPPORTED_OPEN_GENERIC_ADAPTER_MAPPING);
}

#[test]
fn infers_an_unambiguous_non_generic_base_contract_for_an_open_generic_mapping() {
    // The reference infers the base class of the generic view model as the contract its adapter
    // tests at run time. A Rust generic struct derives from no class, and a trait it implements
    // cannot be tested at run time: the generator reports the open generic mapping (SVL0008); an
    // instantiation is mapped explicitly or through a contract of its view.
    let outcome = run(&format!(
        "pub mod view_models {{
    pub trait FilterViewModelBase {{}}
    pub struct FilterViewModel<T>(T);
    impl<T> FilterViewModelBase for FilterViewModel<T> {{}}
}}

pub mod views {{
    {VIEW_IMPORTS}
    {}
}}

#[ferroui_static_view_locator::static_view_locator(generate_data_template = true)]
pub struct ViewLocator;
",
        view("FilterView")
    ));
    single(&outcome, UNSUPPORTED_OPEN_GENERIC_ADAPTER_MAPPING);
    let source = file(&outcome, "ViewLocator.rs");
    assert!(!source.contains("FilterViewModel<"), "{source}");
}

#[test]
fn does_not_infer_a_broad_base_contract_with_conflicting_descendant_views() {
    let outcome = run(&format!(
        "pub mod view_models {{
    pub trait RootViewModel {{}}
    pub struct FilterViewModel<T>(T);
    impl<T> RootViewModel for FilterViewModel<T> {{}}
    pub struct OtherViewModel;
    impl RootViewModel for OtherViewModel {{}}
}}

pub mod views {{
    {VIEW_IMPORTS}
    {}
    {}
}}

#[ferroui_static_view_locator::static_view_locator(generate_data_template = true)]
pub struct ViewLocator;
",
        view("FilterView"),
        view("OtherView")
    ));
    single(&outcome, UNSUPPORTED_OPEN_GENERIC_ADAPTER_MAPPING);
    let source = file(&outcome, "ViewLocator.rs");
    assert!(!source.contains(&view_entry_of("dyn crate::view_models::RootViewModel")), "{source}");
}

#[test]
fn does_not_infer_a_broad_base_contract_with_an_unresolved_descendant() {
    let outcome = run(&format!(
        "pub mod view_models {{
    pub trait RootViewModel {{}}
    pub struct FilterViewModel<T>(T);
    impl<T> RootViewModel for FilterViewModel<T> {{}}
    pub struct UnresolvedViewModel;
    impl RootViewModel for UnresolvedViewModel {{}}
}}

pub mod views {{
    {VIEW_IMPORTS}
    {}
}}

#[ferroui_static_view_locator::static_view_locator(generate_data_template = true)]
pub struct ViewLocator;
",
        view("FilterView")
    ));
    single(&outcome, UNSUPPORTED_OPEN_GENERIC_ADAPTER_MAPPING);
    let source = file(&outcome, "ViewLocator.rs");
    assert!(!source.contains(&view_entry_of("dyn crate::view_models::RootViewModel")), "{source}");
}

#[test]
fn reports_ambiguous_open_generic_fallback_contracts() {
    let outcome = run(&format!(
        "{VIEW_IMPORTS}
pub trait IAlphaViewModel {{}}
pub trait IBetaViewModel {{}}
pub struct GenericViewModel<T>(T);
impl<T> IAlphaViewModel for GenericViewModel<T> {{}}
impl<T> IBetaViewModel for GenericViewModel<T> {{}}
{}
{}
{}

#[ferroui_static_view_locator::static_view_mapping(IAlphaViewModel, AlphaView)]
#[ferroui_static_view_locator::static_view_mapping(IBetaViewModel, BetaView)]
#[ferroui_static_view_locator::static_view_locator(generate_data_template = true, generated_adapter_resolution_mode = ExactThenInterfaces)]
pub struct ViewLocator;
",
        view("GenericView"),
        view("AlphaView"),
        view("BetaView")
    ));
    single(&outcome, AMBIGUOUS_FALLBACK_MAPPING);
    assert!(diagnostics(&outcome, UNSUPPORTED_OPEN_GENERIC_ADAPTER_MAPPING).is_empty());
}

#[test]
fn reports_when_nearest_base_shadows_the_open_generic_target() {
    // A generic struct has no base class: the open generic mapping is reported whatever the
    // mappings of the traits it implements.
    let outcome = run(&format!(
        "{VIEW_IMPORTS}
pub trait RootViewModel {{}}
pub trait IntermediateViewModel: RootViewModel {{}}
pub struct GenericViewModel<T>(T);
impl<T> RootViewModel for GenericViewModel<T> {{}}
impl<T> IntermediateViewModel for GenericViewModel<T> {{}}
{}
{}

#[ferroui_static_view_locator::static_view_mapping(RootViewModel, GenericView)]
#[ferroui_static_view_locator::static_view_mapping(IntermediateViewModel, OtherView)]
#[ferroui_static_view_locator::static_view_locator(generate_data_template = true, generated_adapter_resolution_mode = ExactThenBaseTypes)]
pub struct ViewLocator;
",
        view("GenericView"),
        view("OtherView")
    ));
    single(&outcome, UNSUPPORTED_OPEN_GENERIC_ADAPTER_MAPPING);
}

#[test]
fn reports_when_preferred_interface_shadows_the_open_generic_target_base() {
    // `GenericViewModel<T>` maps to `GenericView` by its name; the trait the interface-first mode
    // prefers maps to another view. A generic struct has no base class to fall back to.
    let outcome = run(&format!(
        "{VIEW_IMPORTS}
pub trait IOtherViewModel {{}}
pub struct GenericViewModel<T>(T);
impl<T> IOtherViewModel for GenericViewModel<T> {{}}
{}
{}

#[ferroui_static_view_locator::static_view_mapping(IOtherViewModel, OtherView)]
#[ferroui_static_view_locator::static_view_locator(generate_data_template = true, generated_adapter_resolution_mode = ExactThenInterfacesThenBaseTypes)]
pub struct ViewLocator;
",
        view("GenericView"),
        view("OtherView")
    ));
    single(&outcome, UNSUPPORTED_OPEN_GENERIC_ADAPTER_MAPPING);
}

#[test]
fn generated_resolver_covers_all_paths_without_allocating_or_constructing_during_match() {
    // The resolver of the generated adapters: an exact lookup of the run-time type, where the
    // precedence is flattened at build time; `match_` creates no view. The run-time half of the
    // reference's test (no view constructed and no allocation during `match_`) is in
    // `tests/FerroUI.StaticViewLocator.Tests`.
    let outcome = generated(&format!(
        "{VIEW_IMPORTS}
use ferroui_base::FerroObject;

pub struct BaseViewModel {{ base: FerroObject }}
ferro_class!(BaseViewModel: FerroObject);
pub struct DerivedViewModel {{ base: BaseViewModel }}
ferro_class!(DerivedViewModel: BaseViewModel);
pub trait IContractViewModel {{}}
pub struct ContractViewModel;
impl IContractViewModel for ContractViewModel {{}}
pub struct ExplicitViewModel;
pub struct FilteredViewModel;
pub struct UnresolvedViewModel;
{}
{}
{}
{}

#[ferroui_static_view_locator::static_view_mapping(BaseViewModel, BaseView)]
#[ferroui_static_view_locator::static_view_mapping(IContractViewModel, ContractView)]
#[ferroui_static_view_locator::static_view_mapping(ExplicitViewModel, ExplicitOverrideView)]
#[ferroui_static_view_locator::static_view_mapping(FilteredViewModel, FilteredView)]
#[ferroui_static_view_locator::static_view_locator(
    generate_data_template = true,
    generated_adapter_resolution_mode = ExactThenBaseTypesThenInterfaces,
    data_template_match_types = [BaseViewModel, IContractViewModel, ExplicitViewModel],
)]
pub struct ViewLocator;
",
        view("BaseView"),
        view("ContractView"),
        view("ExplicitOverrideView"),
        view("FilteredView")
    ));
    let source = file(&outcome, "ViewLocator.rs");
    let adapter = generated_adapter_path(source);
    assert!(!adapter.contains("base_type"));
    assert!(!adapter.contains("interfaces"));
    assert!(!adapter.contains("generic_definition"));
    // The types `data_template_match_types` accepts, decided at build time.
    let start = source.find("fn s_match_types").expect("the match types are generated");
    let match_types = &source[start..source[start..].find("&TABLE").map(|end| start + end).unwrap_or(source.len())];
    for accepted in [object("crate::BaseViewModel"), object("crate::DerivedViewModel"), "crate::ContractViewModel".to_string(), "crate::ExplicitViewModel".to_string()] {
        assert!(match_types.contains(&entry_of(&accepted)), "{accepted}: {match_types}");
    }
    assert!(!match_types.contains("FilteredViewModel"), "{match_types}");
    assert!(!match_types.contains("UnresolvedViewModel"), "{match_types}");
}
