//! Ported from the upstream `MarkupExtensions/ResourceIncludeTests`.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::controls::{
    IResourceProvider, ResourceDictionary, ResourceKey, ResourceProvider, ResourceProviderImpl,
    ResourceValue,
};
use ferroui_base::media::fonts::testing::TestAssetLoader;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref};
use ferroui_controls::testing::UnitTestApplicationScope;
use ferroui_controls::{Application, Border, UserControl};
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};

use crate::support::app::*;
use crate::support::helpers::*;
use crate::support::loader::*;
use crate::support::TypeModule;

// --- test types -------------------------------------------------------------

/// A resource provider that is a keyed collection of resource providers
/// (see issue 11172 of the upstream project).
#[repr(C)]
pub struct LocaleCollection {
    base: ResourceProvider,
    langs: RefCell<Vec<(Option<BoxedValue>, Rc<dyn IResourceProvider>)>>,
}

ferro_class!(LocaleCollection: ResourceProvider);
ferro_impl_classes!(LocaleCollection: FerroObjectImpl);
ferro_class_info!(LocaleCollection {
    new: LocaleCollection::new,
    interfaces: [Rc<dyn IResourceProvider>],
    markup: {
        methods: [
            // Allows the class to be used as a collection; requires x:Key on the IResourceProvider.
            fn Add(Option<BoxedValue>, Rc<dyn IResourceProvider>) =>
                |this: &Ref<LocaleCollection>, k: Option<BoxedValue>, v: Rc<dyn IResourceProvider>| this.add(k, v),
        ],
    },
});

impl ResourceProviderImpl for LocaleCollection {
    fn has_resources(_this: &Self) -> bool {
        true
    }

    fn try_get_resource(this: &Self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        let res = this
            .langs
            .borrow()
            .iter()
            .find(|(k, _)| string_of(k).as_deref() == Some("English"))
            .map(|(_, v)| v.clone());
        res.and_then(|res| res.try_get_resource(key, theme))
    }
}

impl LocaleCollection {
    pub fn construct() -> Self {
        Self { base: ResourceProvider::construct(), langs: RefCell::new(Vec::new()) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn add(&self, k: Option<BoxedValue>, v: Rc<dyn IResourceProvider>) {
        self.langs.borrow_mut().push((k, v));
    }
}

/// The test types of this file.
pub(crate) const MODULE: TypeModule =
    TypeModule { types: &[LocaleCollection::TYPE], markup_types: &[], value_types: || {} };

// --- helpers ----------------------------------------------------------------

fn start_with_resources() -> UnitTestApplicationScope {
    let asset_loader = Rc::new(TestAssetLoader::new());
    let services = TestServices::new().with_asset_loader(asset_loader);
    unit_test_application(services)
}

// --- tests ------------------------------------------------------------------

#[test]
fn resource_include_loads_resource_dictionary() {
    for create_source_info in [false, true] {
        let _base = xaml_test_base();
        let documents = vec![
            document(
                "ferres://Tests/Resource.xaml",
                r#"
<ResourceDictionary xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
<SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
</ResourceDictionary>"#,
            ),
            document_without_uri(
                r#"
<UserControl xmlns='https://github.com/ferroui'
         xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
<UserControl.Resources>
    <ResourceDictionary>
        <ResourceDictionary.MergedDictionaries>
            <ResourceInclude Source='ferres://Tests/Resource.xaml'/>
        </ResourceDictionary.MergedDictionaries>
    </ResourceDictionary>
</UserControl.Resources>

<Border Name='border' Background='{StaticResource brush}'/>
</UserControl>"#,
            ),
        ];

        let mut config = RuntimeXamlLoaderConfiguration::new();
        config.set_create_source_info(create_source_info);

        let _app = start_with_resources();
        let compiled = load_group(documents, Some(config));
        let user_control: Ref<UserControl> = group_item(&compiled, 1);
        assert_is_type::<UserControl>(&user_control);
        let border = user_control.get_control::<Border>("border");

        let background = border.background().expect("the background is null");
        let brush = background.as_solid_color_brush().expect("the brush is not an ISolidColorBrush");
        assert_eq!(0xff506070, brush.color().to_uint32(), "createSourceInfo: {create_source_info}");
    }
}

#[test]
fn missing_resource_key_in_resource_include_does_not_cause_stack_overflow() {
    let _base = xaml_test_base();
    let app = Application::current().map(boxed);
    let documents = vec![
        document(
            "ferres://Tests/Resource.xaml",
            r#"
<ResourceDictionary xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
<StaticResource x:Key='brush' ResourceKey='missing' />
</ResourceDictionary>"#,
        ),
        RuntimeXamlLoaderDocument::with_root_instance(
            app,
            r#"
<Application xmlns='https://github.com/ferroui'
         xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
<Application.Resources>
    <ResourceDictionary>
        <ResourceDictionary.MergedDictionaries>
            <ResourceInclude Source='ferres://Tests/Resource.xaml'/>
        </ResourceDictionary.MergedDictionaries>
    </ResourceDictionary>
</Application.Resources>
</Application>"#,
        ),
    ];

    let _app = start_with_resources();
    // Only the failure of the lookup of the missing key is expected (the
    // key-not-found error of the static resource); anything else fails.
    if let Err(error) = try_load_group(documents, None) {
        let description = describe(&error);
        assert!(
            description.contains("Static resource 'missing' not found."),
            "the group failed to load: {description}"
        );
    }
}

#[test]
fn resource_include_should_be_allowed_to_have_key_in_custom_container() {
    let _base = xaml_test_base();
    let app = Application::current().map(boxed);
    let documents = vec![
        document(
            "ferres://Demo/en-us.xaml",
            r#"
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <x:String x:Key='OkButton'>OK</x:String>
</ResourceDictionary>"#,
        ),
        RuntimeXamlLoaderDocument::with_root_instance(
            app,
            r#"
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
                    xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <ResourceDictionary.MergedDictionaries>
        <local:LocaleCollection>
            <ResourceInclude Source='ferres://Demo/en-us.xaml' x:Key='English' />
        </local:LocaleCollection>
    </ResourceDictionary.MergedDictionaries>
</ResourceDictionary>"#,
        ),
    ];

    let _app = start_with_resources();
    let groups = load_group(documents, None);
    let res: Ref<ResourceDictionary> = group_item(&groups, 1);
    assert_is_type::<ResourceDictionary>(&res);

    let val = res.try_get_resource(&ResourceKey::from("OkButton"), None);
    assert!(val.is_some());
    assert_string("OK", &val.unwrap());
}
