//! The shared helpers and test types of the suites: the loader calls, the
//! application scopes and the classes, view models, converters and markup
//! extensions the documents of the tests name.
//!
//! Types are declared per upstream file: the shared ones of the test
//! project in their own modules, the ones a test file declares in the
//! module named after that test file (under the folder of its namespace).
//! Every module with types lists them in a [`TypeModule`].

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

pub mod app;
pub mod assets;
pub mod helpers;
pub mod loader;

pub mod emitter;
pub mod sample_ferro_object;
pub mod setter_tests;
pub mod style_tests;
pub mod test_view_model;
pub mod xaml_il_tests;

pub mod converters;
pub mod templates;
pub mod xaml;

pub use sample_ferro_object::SampleFerroObject;
pub use test_view_model::TestViewModel;

/// The types a support module declares.
pub(crate) struct TypeModule {
    /// The classes and static types of the object model (`X::TYPE`,
    /// `<X as StaticType>::TYPE`).
    pub types: &'static [&'static TypeInfo],
    /// The types declared with `ferro_markup_type!` / `ferro_markup_enum!`
    /// (`<X as MarkupTyped>::MARKUP`).
    pub markup_types: &'static [&'static MarkupType],
    /// What the untyped value conversions must know about the types
    /// (`ValueTypes::register_reference::<X>()`, casts to contracts,
    /// nullable forms).
    pub value_types: fn(),
}

impl TypeModule {
    /// A module without types.
    pub(crate) const EMPTY: TypeModule = TypeModule { types: &[], markup_types: &[], value_types: || {} };
}

/// Every support module with types.
const MODULES: &[&TypeModule] = &[
    // FerroUI.Markup.Xaml.UnitTests
    &ROOT,
    &emitter::MODULE,
    &setter_tests::MODULE,
    &style_tests::MODULE,
    &xaml_il_tests::MODULE,
    // FerroUI.Markup.Xaml.UnitTests.Xaml
    &xaml::MODULE,
    &xaml::basic_tests::MODULE,
    &xaml::control_template_tests::MODULE,
    &xaml::control_theme_tests::MODULE,
    &xaml::data_template_tests::MODULE,
    &xaml::design_mode_tests::MODULE,
    &xaml::event_tests::MODULE,
    &xaml::ferro_intrinsics_tests::MODULE,
    &xaml::generic_template_tests::MODULE,
    &xaml::items_panel_template_tests::MODULE,
    &xaml::merge_resource_include_tests::MODULE,
    &xaml::parent_stack_provider_tests::MODULE,
    &xaml::provide_value_target_tests::MODULE,
    &xaml::resource_dictionary_tests::MODULE,
    &xaml::style_include_tests::MODULE,
    &xaml::style_tests::MODULE,
    &xaml::theme_dictionaries_tests::MODULE,
    &xaml::tree_data_template_tests::MODULE,
    &xaml::xaml_source_info_tests::MODULE,
    // FerroUI.Markup.Xaml.UnitTests.Converters
    &converters::ferro_property_converter_test::MODULE,
    &converters::converter_tests::MODULE,
    &converters::geometry_type_converter_tests::MODULE,
    &converters::multi_value_converter_tests::MODULE,
    &converters::nullable_converter_tests::MODULE,
    &converters::points_list_type_converter_tests::MODULE,
    &converters::value_converter_tests::MODULE,
    // FerroUI.Markup.Xaml.UnitTests.Templates
    &templates::data_template_tests::MODULE,
];

/// The shared types of the root of the test project.
const ROOT: TypeModule = TypeModule {
    types: &[SampleFerroObject::TYPE],
    markup_types: &[<TestViewModel as ferroui_base::metadata::MarkupTyped>::MARKUP],
    value_types: || ValueTypes::register_reference::<TestViewModel>(),
};

/// The dotted namespaces of the support modules.
pub(crate) const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_markup_xaml_tests::support", "FerroUI.Markup.Xaml.UnitTests"),
    ("ferroui_markup_xaml_tests::support::xaml", "FerroUI.Markup.Xaml.UnitTests.Xaml"),
    ("ferroui_markup_xaml_tests::support::converters", "FerroUI.Markup.Xaml.UnitTests.Converters"),
    ("ferroui_markup_xaml_tests::support::templates", "FerroUI.Markup.Xaml.UnitTests.Templates"),
];

/// Registers the namespaces, the types, the value knowledge and the assets
/// of the support modules. Called once by `register_types()` of the crate.
pub(crate) fn register() {
    TypeInfo::register_namespaces(NAMESPACES);
    for module in MODULES {
        TypeInfo::register_all(module.types);
        MarkupType::register_all(module.markup_types);
    }
    TypeInfo::register_rust_paths(emitter::RUST_PATHS);
    ValueTypes::register_global(register_value_types);
    assets::register();
}

fn register_value_types() {
    for module in MODULES {
        (module.value_types)();
    }
}
