//! `StaticViewLocatorGeneratorSnapshotTests.cs`.

use super::*;
use crate::diagnostics::{INVALID_MAPPING_CONTRACT, VIEW_CANNOT_BE_CONSTRUCTED};

#[test]
fn reports_invalid_mapping_contracts() {
    // A trait without a generic parameter or an associated type, and a closed generic trait (the
    // reference: an interface without a type parameter and `IViewFor<string>`).
    let outcome = run(
        "use ferroui_static_view_locator::static_view_locator;

pub trait IInvalidContract {}
pub trait IViewFor<VM> {}

#[static_view_locator(view_model_mapping_contracts = [IInvalidContract, IViewFor<String>])]
pub struct ViewLocator;
",
    );
    let found = diagnostics(&outcome, INVALID_MAPPING_CONTRACT);
    assert_eq!(found.len(), 2, "{:#?}", outcome.diagnostics);
    assert!(found.iter().any(|diagnostic| diagnostic.message.contains("IInvalidContract")));
    assert!(found.iter().any(|diagnostic| diagnostic.message.contains("IViewFor<String>")));
}

#[test]
fn generates_factories_for_constructors_callable_without_arguments() {
    // A Rust function has no optional or `params` parameters: a view is created by `new()` or by
    // the constructor its class states (`ferro_class_info!(V { new: V::create })`).
    let outcome = generated(&format!(
        "pub mod view_models {{
    pub struct OptionalViewModel;
    pub struct ParamsViewModel;
}}

pub mod views {{
    {VIEW_IMPORTS}
    use ferroui_base::ferro_class_info;

    {}
    pub struct ParamsView {{ base: UserControl }}
    ferro_class!(ParamsView: UserControl);
    ferro_class_info!(ParamsView {{ new: ParamsView::create }});
    impl ParamsView {{
        pub fn create() -> Ref<Self> {{
            unimplemented!()
        }}
    }}
}}

#[ferroui_static_view_locator::static_view_locator]
pub struct ViewLocator;
",
        view("OptionalView")
    ));
    let source = file(&outcome, "ViewLocator.rs");
    assert!(source.contains(&view_entry("crate::view_models::OptionalViewModel", "crate::views::OptionalView::new")), "{source}");
    assert!(source.contains(&view_entry("crate::view_models::ParamsViewModel", "crate::views::ParamsView::create")), "{source}");
}

#[test]
fn generates_factory_when_locator_itself_has_private_constructor() {
    // The locator is a view, created through a constructor that is private to its module.
    let outcome = generated(&format!(
        "{VIEW_IMPORTS}
use ferroui_static_view_locator::{{static_view_locator, static_view_mapping}};

pub struct DashboardViewModel;

#[static_view_locator]
#[static_view_mapping(DashboardViewModel, ViewLocator)]
pub struct ViewLocator {{ base: UserControl }}
ferro_class!(ViewLocator: UserControl);
impl ViewLocator {{
    fn new() -> Ref<Self> {{
        unimplemented!()
    }}
}}
"
    ));
    let source = file(&outcome, "ViewLocator.rs");
    assert!(source.contains(&view_entry("crate::DashboardViewModel", "crate::ViewLocator::new")), "{source}");
}

#[test]
fn reports_view_without_constructor_callable_with_no_arguments() {
    let outcome = run(&format!(
        "pub mod view_models {{
    pub struct DashboardViewModel;
}}

pub mod views {{
    {VIEW_IMPORTS}
    pub struct DashboardView {{ base: UserControl }}
    ferro_class!(DashboardView: UserControl);
    impl DashboardView {{
        pub fn new(title: &str) -> Ref<Self> {{
            unimplemented!()
        }}
    }}
}}

#[ferroui_static_view_locator::static_view_locator]
pub struct ViewLocator;
"
    ));
    let diagnostic = single(&outcome, VIEW_CANNOT_BE_CONSTRUCTED);
    let source = file(&outcome, "ViewLocator.rs");
    assert!(diagnostic.message.contains("DashboardView"));
    assert!(!source.contains("crate::views::DashboardView::new"), "{source}");
    assert!(source.contains(&missing_entry("crate::view_models::DashboardViewModel", "Not Found: test_app::views::DashboardView")), "{source}");
}

#[test]
fn preserves_internal_locator_accessibility() {
    // The generated source extends the struct and leaves its visibility as declared: it does not
    // declare the struct again, and its members are private to the module of the locator.
    let outcome = generated(
        "#[ferroui_static_view_locator::static_view_locator]
pub(crate) struct ViewLocator;
",
    );
    let source = file(&outcome, "ViewLocator.rs");
    assert!(source.contains("impl ViewLocator {"), "{source}");
    assert!(!source.contains("struct ViewLocator"), "{source}");
    assert!(!source.contains("pub fn s_views"), "{source}");
}

#[test]
fn infers_mapping_from_configured_generic_contract_and_generates_factory_method() {
    let outcome = generated(&format!(
        "pub mod models {{
    pub struct DashboardModel;
}}

pub mod framework {{
    pub trait IViewFor<VM> {{}}
}}

pub mod screens {{
    {VIEW_IMPORTS}
    {}
    impl crate::framework::IViewFor<crate::models::DashboardModel> for DashboardScreen {{}}
}}

use ferroui_base::{{BoxedValue, Ref}};
use ferroui_controls::Control;

#[ferroui_static_view_locator::static_view_locator(
    generate_runtime_type_fallback_methods = false,
    generate_view_factory_methods = true,
    view_model_mapping_contracts = [framework::IViewFor],
)]
pub struct ViewLocator;

impl ViewLocator {{
    pub fn build(&self, data: Option<&BoxedValue>) -> Option<Ref<Control>> {{
        None
    }}
}}
",
        view("DashboardScreen")
    ));
    let source = file(&outcome, "ViewLocator.rs");
    assert!(source.contains(&view_entry("crate::models::DashboardModel", "crate::screens::DashboardScreen::new")), "{source}");
    assert!(source.contains("fn try_create_view_exact("), "{source}");
    assert!(!source.contains("s_interface_views"), "{source}");
}

#[test]
fn explicit_mapping_overrides_generic_base_contract_mapping() {
    // The contract of the reference is a generic base class (`FrameworkView<TViewModel>`); a Rust
    // view states its view model through a trait with an associated type.
    let outcome = generated(&format!(
        "pub mod models {{
    pub struct DashboardModel;
}}

pub mod framework {{
    pub trait FrameworkView {{
        type ViewModel;
    }}
}}

pub mod screens {{
    {VIEW_IMPORTS}
    {}
    impl crate::framework::FrameworkView for DashboardScreen {{
        type ViewModel = crate::models::DashboardModel;
    }}
    {}
}}

#[ferroui_static_view_locator::static_view_locator(view_model_mapping_contracts = [framework::FrameworkView])]
#[ferroui_static_view_locator::static_view_mapping(models::DashboardModel, screens::AlternateDashboardScreen)]
pub struct ViewLocator;
",
        view("DashboardScreen"),
        view("AlternateDashboardScreen")
    ));
    let source = file(&outcome, "ViewLocator.rs");
    assert!(source.contains(&view_entry("crate::models::DashboardModel", "crate::screens::AlternateDashboardScreen::new")), "{source}");
    assert!(!source.contains("crate::screens::DashboardScreen::new"), "{source}");
}

#[test]
fn does_not_choose_an_arbitrary_view_for_ambiguous_contract_mappings() {
    let outcome = run(&format!(
        "pub mod models {{
    pub struct DashboardViewModel;
}}

pub mod framework {{
    pub trait IViewFor<VM> {{}}
}}

pub mod screens {{
    {VIEW_IMPORTS}
    use ferroui_reactiveui_controls::IControlViewFor;
    use crate::models::DashboardViewModel;

    {}
    impl crate::framework::IViewFor<DashboardViewModel> for DashboardView {{}}
    impl IControlViewFor for DashboardView {{
        type ViewModel = DashboardViewModel;
    }}

    {}
    impl crate::framework::IViewFor<DashboardViewModel> for AlternateDashboardView {{}}
    impl IControlViewFor for AlternateDashboardView {{
        type ViewModel = DashboardViewModel;
    }}
}}

#[ferroui_static_view_locator::static_view_locator(generate_view_locator = true, view_model_mapping_contracts = [framework::IViewFor])]
pub struct ViewLocator;
",
        view("DashboardView"),
        view("AlternateDashboardView")
    ));
    let diagnostic = single(&outcome, crate::diagnostics::AMBIGUOUS_VIEW_MAPPING);
    let source = file(&outcome, "ViewLocator.rs");
    assert!(diagnostic.message.contains("DashboardViewModel"));
    assert!(!source.contains("TypeId::of::<crate::models::DashboardViewModel>"), "{source}");
    assert!(!source.contains("crate::screens::DashboardView::new"), "{source}");
    assert!(!source.contains("crate::screens::AlternateDashboardView::new"), "{source}");
}

#[test]
fn generates_attribute_and_locator_sources() {
    // The attribute is not generated source in Rust (a proc macro of
    // ferroui-static-view-locator-macros); the generated locator is compared in full.
    let outcome = generated(&format!(
        "pub mod view_models {{
    pub struct ViewModelBase;
    pub struct MainWindowViewModel;
    pub struct TestViewModel;
    pub struct IgnoredViewModel;
}}

pub mod views {{
    {VIEW_IMPORTS}
    {}
}}

#[ferroui_static_view_locator::static_view_locator]
pub struct ViewLocator;
",
        view("TestView")
    ));
    assert_eq!(outcome.files.len(), 1);
    assert_eq!(file(&outcome, "ViewLocator.rs"), EXPECTED_LEGACY_LOCATOR);
}

/// The legacy locator of `generates_attribute_and_locator_sources`.
const EXPECTED_LEGACY_LOCATOR: &str = r#"// @generated by ferroui-static-view-locator-build for the locator `ViewLocator` (test_app).
// The build script of the crate writes this file again; do not edit it.

#[allow(dead_code, unused_variables, clippy::all)]
impl ViewLocator {
    /// The views of the view model types (`s_views`).
    fn s_views() -> &'static ::ferroui_static_view_locator::ViewTable {
        static TABLE: ::ferroui_static_view_locator::ViewTable = ::ferroui_static_view_locator::ViewTable::new(&[
            ::ferroui_static_view_locator::TypeEntry::new(::ferroui_static_view_locator::generated::TypeId::of::<crate::view_models::TestViewModel>, || ::ferroui_static_view_locator::upcast(crate::views::TestView::new())),
        ]);
        &TABLE
    }

    /// The messages of the view model types whose view is not found (`s_missingViews`).
    fn s_missing_views() -> &'static ::ferroui_static_view_locator::MissingViewTable {
        static TABLE: ::ferroui_static_view_locator::MissingViewTable = ::ferroui_static_view_locator::MissingViewTable::new(&[
            ::ferroui_static_view_locator::TypeEntry::new(::ferroui_static_view_locator::generated::TypeId::of::<crate::view_models::IgnoredViewModel>, "Not Found: test_app::views::IgnoredView"),
            ::ferroui_static_view_locator::TypeEntry::new(::ferroui_static_view_locator::generated::TypeId::of::<crate::view_models::MainWindowViewModel>, "Not Found: test_app::views::MainWindowView"),
        ]);
        &TABLE
    }

    /// The views of the open generic view model types, by the path of the definition (the `s_views` entries of generic type definitions).
    fn s_generic_views() -> &'static ::ferroui_static_view_locator::NameTable<::ferroui_static_view_locator::ViewFactory> {
        static TABLE: ::ferroui_static_view_locator::NameTable<::ferroui_static_view_locator::ViewFactory> = ::ferroui_static_view_locator::NameTable::<::ferroui_static_view_locator::ViewFactory>::new(&[]);
        &TABLE
    }

    /// The messages of the open generic view model types whose view is not found, by the path of the definition.
    fn s_generic_missing_views() -> &'static ::ferroui_static_view_locator::NameTable<&'static str> {
        static TABLE: ::ferroui_static_view_locator::NameTable<&'static str> = ::ferroui_static_view_locator::NameTable::<&'static str>::new(&[]);
        &TABLE
    }

    /// The views of the types whose nearest mapped base class has a view (the `BaseType` walk of `TryGetFactory`, flattened).
    fn s_base_views() -> &'static ::ferroui_static_view_locator::ViewTable {
        static TABLE: ::ferroui_static_view_locator::ViewTable = ::ferroui_static_view_locator::ViewTable::new(&[]);
        &TABLE
    }

    /// The views of the types whose last mapped trait has a view (the `GetInterfaces()` walk of `TryGetFactoryFromInterfaces`, flattened).
    fn s_interface_views() -> &'static ::ferroui_static_view_locator::ViewTable {
        static TABLE: ::ferroui_static_view_locator::ViewTable = ::ferroui_static_view_locator::ViewTable::new(&[]);
        &TABLE
    }

    /// The missing-view messages of the types by their nearest base class (the `BaseType` walk of `TryGetMissingView`, flattened).
    fn s_base_missing_views() -> &'static ::ferroui_static_view_locator::MissingViewTable {
        static TABLE: ::ferroui_static_view_locator::MissingViewTable = ::ferroui_static_view_locator::MissingViewTable::new(&[]);
        &TABLE
    }

    /// The missing-view messages of the types by their last trait (the `GetInterfaces()` walk of `TryGetMissingViewFromInterfaces`, flattened).
    fn s_interface_missing_views() -> &'static ::ferroui_static_view_locator::MissingViewTable {
        static TABLE: ::ferroui_static_view_locator::MissingViewTable = ::ferroui_static_view_locator::MissingViewTable::new(&[]);
        &TABLE
    }

    /// The view of `data` (`Build`): the view of its type, of its nearest base class or of its generic
    /// type definition, then of its traits; else the text of its missing view.
    ///
    /// # Panics
    /// Panics for a view model whose type has neither (`Unable to create view for type`).
    pub fn build(&self, data: ::ferroui_static_view_locator::generated::Option<&::ferroui_static_view_locator::generated::BoxedValue>) -> ::ferroui_static_view_locator::generated::Option<::ferroui_static_view_locator::generated::Ref<::ferroui_static_view_locator::generated::Control>> {
        let data = data?;
        let type_ = ::ferroui_static_view_locator::RuntimeType::of_value(data);
        if let Some(factory) = Self::try_get_factory(Some(&type_)).or_else(|| Self::try_get_factory_from_interfaces(&type_)) {
            return Some(factory());
        }
        if let Some(missing_view) = Self::try_get_missing_view(Some(&type_)).or_else(|| Self::try_get_missing_view_from_interfaces(&type_)) {
            return Some(::ferroui_static_view_locator::text_view(missing_view));
        }
        ::ferroui_static_view_locator::unable_to_create_view(&type_)
    }

    /// The factory of the view of `type_` or of its nearest mapped base class (`TryGetFactory`).
    fn try_get_factory(type_: ::ferroui_static_view_locator::generated::Option<&::ferroui_static_view_locator::RuntimeType>) -> ::ferroui_static_view_locator::generated::Option<::ferroui_static_view_locator::ViewFactory> {
        let type_ = type_?;
        Self::try_get_factory_for_type(type_).or_else(|| Self::s_base_views().get(type_.id()))
    }

    /// The factory of the view of the last mapped trait of `type_` (`TryGetFactoryFromInterfaces`).
    fn try_get_factory_from_interfaces(type_: &::ferroui_static_view_locator::RuntimeType) -> ::ferroui_static_view_locator::generated::Option<::ferroui_static_view_locator::ViewFactory> {
        Self::s_interface_views().get(type_.id())
    }

    /// The factory of the view of `type_`, or of its generic type definition (`TryGetFactoryForType`).
    fn try_get_factory_for_type(type_: &::ferroui_static_view_locator::RuntimeType) -> ::ferroui_static_view_locator::generated::Option<::ferroui_static_view_locator::ViewFactory> {
        Self::s_views()
            .get(type_.id())
            .or_else(|| type_.generic_definition().and_then(|definition| Self::s_generic_views().get(definition)))
    }

    /// The missing-view message of `type_` or of its nearest base class (`TryGetMissingView`).
    fn try_get_missing_view(type_: ::ferroui_static_view_locator::generated::Option<&::ferroui_static_view_locator::RuntimeType>) -> ::ferroui_static_view_locator::generated::Option<&'static str> {
        let type_ = type_?;
        Self::try_get_missing_view_for_type(type_).or_else(|| Self::s_base_missing_views().get(type_.id()))
    }

    /// The missing-view message of the last trait of `type_` (`TryGetMissingViewFromInterfaces`).
    fn try_get_missing_view_from_interfaces(type_: &::ferroui_static_view_locator::RuntimeType) -> ::ferroui_static_view_locator::generated::Option<&'static str> {
        Self::s_interface_missing_views().get(type_.id())
    }

    /// The missing-view message of `type_`, or of its generic type definition (`TryGetMissingViewForType`).
    fn try_get_missing_view_for_type(type_: &::ferroui_static_view_locator::RuntimeType) -> ::ferroui_static_view_locator::generated::Option<&'static str> {
        Self::s_missing_views()
            .get(type_.id())
            .or_else(|| type_.generic_definition().and_then(|definition| Self::s_generic_missing_views().get(definition)))
    }
}
"#;

#[test]
fn generates_mappings_for_multiple_locators() {
    let outcome = generated(&format!(
        "pub mod modules {{
    pub mod admin {{
        {VIEW_IMPORTS}
        #[ferroui_static_view_locator::static_view_locator]
        pub struct AdminViewLocator;

        pub struct AdminDashboardViewModel;

        {}
    }}

    pub mod client {{
        {VIEW_IMPORTS}
        #[ferroui_static_view_locator::static_view_locator]
        pub struct ClientViewLocator;

        pub struct ClientDashboardViewModel;

        {}
    }}

    pub mod shared {{
        pub struct ActivityLogViewModel;
    }}
}}
",
        view("AdminDashboardView"),
        view("ClientDashboardView")
    ));
    assert_eq!(outcome.files.len(), 2);
    for name in ["AdminViewLocator.rs", "ClientViewLocator.rs"] {
        let source = file(&outcome, name);
        assert!(source.contains(&view_entry("crate::modules::admin::AdminDashboardViewModel", "crate::modules::admin::AdminDashboardView::new")), "{source}");
        assert!(source.contains(&view_entry("crate::modules::client::ClientDashboardViewModel", "crate::modules::client::ClientDashboardView::new")), "{source}");
        assert!(source.contains(&missing_entry("crate::modules::shared::ActivityLogViewModel", "Not Found: test_app::modules::shared::ActivityLogView")), "{source}");
    }
    assert!(file(&outcome, "AdminViewLocator.rs").contains("impl AdminViewLocator {"));
    assert!(file(&outcome, "ClientViewLocator.rs").contains("impl ClientViewLocator {"));
}

#[test]
fn does_not_generate_build_method_when_already_present() {
    let outcome = generated(&format!(
        "pub mod view_models {{
    pub struct SampleViewModel;
}}

pub mod views {{
    {VIEW_IMPORTS}
    {}
}}

use ferroui_base::{{BoxedValue, Ref}};
use ferroui_controls::Control;

#[ferroui_static_view_locator::static_view_locator]
pub struct ViewLocator;

impl ViewLocator {{
    pub fn build(&self, data: Option<&BoxedValue>) -> Option<Ref<Control>> {{
        Some(views::SampleView::new().upcast())
    }}
}}
",
        view("SampleView")
    ));
    let source = file(&outcome, "ViewLocator.rs");
    assert!(!source.contains("pub fn build("), "{source}");
    assert!(source.contains(&view_entry("crate::view_models::SampleViewModel", "crate::views::SampleView::new")), "{source}");
    // The declared `build` keeps the fallback helpers it may call.
    assert!(source.contains("fn try_get_factory("), "{source}");
    assert!(source.contains("fn try_get_missing_view_from_interfaces("), "{source}");
}

#[test]
fn applies_empty_type_name_replacement() {
    let outcome = generated_with(
        &format!(
            "pub mod view_models {{
    pub struct SettingsWindowViewModel;
}}

pub mod windows {{
    {VIEW_IMPORTS}
    {}
}}

#[ferroui_static_view_locator::static_view_locator]
pub struct ViewLocator;
",
            view("SettingsWindow")
        ),
        |build| build.property("namespace_replacement_rules", "view_models=windows").property("type_name_replacement_rules", "ViewModel="),
    );
    let source = file(&outcome, "ViewLocator.rs");
    assert!(source.contains(&view_entry("crate::view_models::SettingsWindowViewModel", "crate::windows::SettingsWindow::new")), "{source}");
}

#[test]
fn applies_configured_namespace_and_type_replacement_rules() {
    let outcome = generated_with(
        &format!(
            "pub mod view_models {{
    pub mod pages {{
        pub struct DashboardViewModel;
    }}
}}

pub mod views {{
    pub mod pages {{
        {VIEW_IMPORTS}
        {}
    }}
}}

#[ferroui_static_view_locator::static_view_locator]
pub struct ViewLocator;
",
            view("DashboardScreen")
        ),
        |build| build.namespace_replacement_rules([("view_models", "views")]).type_name_replacement_rules([("ViewModel", "Screen")]),
    );
    let source = file(&outcome, "ViewLocator.rs");
    assert!(source.contains(&view_entry("crate::view_models::pages::DashboardViewModel", "crate::views::pages::DashboardScreen::new")), "{source}");
}

#[test]
fn parses_comma_separated_ms_build_configuration_values() {
    let outcome = generated_with(
        &format!(
            "pub mod view_models {{
    pub mod pages {{
        pub trait ISettingsViewModel {{}}
    }}
}}

pub mod views {{
    pub mod screens {{
        {VIEW_IMPORTS}
        {}
    }}
}}

#[ferroui_static_view_locator::static_view_locator]
pub struct ViewLocator;
",
            view("SettingsScreen")
        ),
        |build| {
            build
                .property("StaticViewLocatorViewModelNamespacePrefixes", "other::models,crate::view_models")
                .property("StaticViewLocatorNamespaceReplacementRules", "foo=bar,view_models=views,pages=screens")
                .property("StaticViewLocatorTypeNameReplacementRules", "Foo=Bar,ViewModel=View,View=Screen")
                .property("StaticViewLocatorInterfacePrefixesToStrip", "Vm,I")
        },
    );
    let source = file(&outcome, "ViewLocator.rs");
    assert!(
        source.contains(&view_entry("dyn crate::view_models::pages::ISettingsViewModel", "crate::views::screens::SettingsScreen::new")),
        "{source}"
    );
}

#[test]
fn strips_generic_arity_from_configured_view_names_by_default() {
    // A generic Rust type has no run-time definition: the legacy fallback finds the view of
    // `WidgetViewModel<i32>` through the path of the definition.
    let outcome = generated_with(
        &format!(
            "pub mod view_models {{
    pub struct WidgetViewModel<T>(T);
}}

pub mod views {{
    {VIEW_IMPORTS}
    {}
}}

#[ferroui_static_view_locator::static_view_locator]
pub struct ViewLocator;
",
            view("WidgetView")
        ),
        |build| build.property("type_name_replacement_rules", "ViewModel=View"),
    );
    let source = file(&outcome, "ViewLocator.rs");
    assert!(
        source.contains("(\"test_app::view_models::WidgetViewModel\", || ::ferroui_static_view_locator::upcast(crate::views::WidgetView::new())),"),
        "{source}"
    );
    assert!(!source.contains("TypeId::of::<crate::view_models::WidgetViewModel>"), "{source}");
}

#[test]
fn does_not_generate_invalid_factory_for_open_generic_view() {
    let outcome = generated_with(
        &format!(
            "pub mod view_models {{
    pub struct WidgetViewModel<T>(T);
}}

pub mod views {{
    {VIEW_IMPORTS}
    pub struct WidgetView<T> {{ base: UserControl, value: T }}
    impl<T> WidgetView<T> {{
        pub fn new() -> Ref<Self> {{
            unimplemented!()
        }}
    }}
}}

#[ferroui_static_view_locator::static_view_locator]
pub struct ViewLocator;
"
        ),
        |build| {
            build
                .property("namespace_replacement_rules", "view_models=views")
                .property("type_name_replacement_rules", "ViewModel=View")
                .property("strip_generic_arity_from_view_name", "false")
        },
    );
    let source = file(&outcome, "ViewLocator.rs");
    assert!(!source.contains("crate::views::WidgetView::new"), "{source}");
    assert!(!source.contains("\"test_app::view_models::WidgetViewModel\", ||"), "{source}");
}
