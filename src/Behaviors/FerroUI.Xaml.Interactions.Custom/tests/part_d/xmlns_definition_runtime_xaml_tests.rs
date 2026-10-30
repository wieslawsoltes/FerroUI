//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Custom/XmlnsDefinitionRuntimeXamlTests.cs`
//! of Xaml.Behaviors.
//!
//! Adapted: the test assembly of the reference maps its namespace into the XML namespace
//! `https://unit.test/features` (`Properties/AssemblyInfo.cs` of the test project); here a markup
//! assembly of the tests does, for the namespace of this module. The reference names the behavior
//! assemblies with `clr-namespace:..;assembly=..`, as here. Its document uses the
//! `EventTriggerBehavior` of the interactions project, which this crate does not depend on; the
//! document here uses the loaded trigger of this crate, a trigger of the same event.

use super::support::start_app;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::{from_markup_value, MarkupAssembly, XmlnsDefinition};
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, FerroObject, FerroObjectImpl, Ref, StaticType, StyledElementImpl,
    TypeInfo, VisualImpl,
};
use ferroui_controls::{Border, ControlImpl};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

/// The namespace of the classes of the tests (the namespace of the reference's test file, with the
/// name of the framework).
const TEST_NAMESPACE: &str = "FerroUI.Xaml.Interactions.UnitTests.Custom";

/// The assembly of the tests: `[assembly: XmlnsDefinition("https://unit.test/features", ..)]`.
static TEST_ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Xaml.Interactions.UnitTests",
    crate_name: "ferroui_xaml_interactions_custom",
    xmlns_definitions: &[XmlnsDefinition { xml_namespace: "https://unit.test/features", namespace: TEST_NAMESPACE }],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// `public sealed class PreviewFeatureControl : Border;`
#[repr(C)]
pub(crate) struct PreviewFeatureControl {
    base: Border,
}

ferro_class!(PreviewFeatureControl: Border);
ferro_class_info!(PreviewFeatureControl { new: PreviewFeatureControl::new });
ferroui_base::ferro_impl_classes!(
    PreviewFeatureControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl PreviewFeatureControl {
    fn construct() -> Self {
        Self { base: Border::construct() }
    }

    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// Registers the namespace, the class and the assembly of the tests.
fn register_test_types() {
    TypeInfo::register_namespaces(&[(module_path!(), TEST_NAMESPACE)]);
    TypeInfo::register_all(&[<PreviewFeatureControl as StaticType>::TYPE]);
    MarkupAssembly::register(&TEST_ASSEMBLY);
}

#[test]
fn runtime_loader_resolves_explicit_behavior_assemblies_alongside_application_xmlns_definitions() {
    let _app = start_app();
    register_test_types();

    const XAML: &str = r#"
        <features:PreviewFeatureControl
            xmlns="https://github.com/ferroui"
            xmlns:features="https://unit.test/features"
            xmlns:i="clr-namespace:FerroUI.Xaml.Interactivity;assembly=FerroUI.Xaml.Interactivity"
            xmlns:custom="clr-namespace:FerroUI.Xaml.Interactions.Custom;assembly=FerroUI.Xaml.Interactions.Custom">
          <i:Interaction.Behaviors>
            <custom:LoadedTrigger />
          </i:Interaction.Behaviors>
        </features:PreviewFeatureControl>
        "#;

    let result = FerroRuntimeXamlLoader::load(XAML, Some(&TEST_ASSEMBLY), None, None, true)
        .unwrap_or_else(|error| panic!("the document failed to load: {}", error.message()));

    let result = from_markup_value::<Ref<FerroObject>>(&Some(result)).expect("an object");
    assert!(std::ptr::eq(result.get_type(), <PreviewFeatureControl as StaticType>::TYPE));
}
