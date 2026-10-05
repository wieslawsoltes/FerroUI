//! Port of `Xaml/ThemeDictionariesTests.cs`.

use std::rc::Rc;

use ferroui_base::controls::{
    IResourceHost, IResourceNode, IResourceProvider, ResourceDictionary, ResourceKey, ResourceValue,
    ResourcesChangedEventArgs,
};
use ferroui_base::media::{Brushes, Color, Colors, GeometryDrawing, IBrush};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::{IThemeVariantHost, ThemeVariant};
use ferroui_base::{FerroLocator, Ref};
use ferroui_controls::documents::TextElement;
use ferroui_controls::primitives::Popup;
use ferroui_controls::{Application, Border, TextBlock, ThemeVariantScope, Window};
use ferroui_markup::markup::data::DelayedBinding;
use ferroui_markup_xaml::markup_extensions::DynamicResourceExtension;
use ferroui_markup_xaml::templates::Template;

use crate::support::app::{styled_window_application, xaml_test_base};
use crate::support::helpers::{assert_is_type, boxed, is_type, object_of, value_of};
use crate::support::loader::{document, document_without_uri, group_item, load_as, load_group};
use crate::support::xaml::theme_dictionaries_tests::ThemeDictionariesTests;

/// `((ISolidColorBrush)brush!).Color`.
#[track_caller]
fn color_of(brush: Option<Rc<dyn IBrush>>) -> Color {
    let brush = brush.expect("the brush is set");
    brush.as_solid_color_brush().expect("the brush is a solid color brush").color()
}

/// `(Border)themeVariantScope.Child!`.
#[track_caller]
fn border_of(scope: &Ref<ThemeVariantScope>) -> Ref<Border> {
    scope.child().expect("the scope has a child").cast::<Border>().expect("the child is a Border")
}

/// A resource dictionary with the brush `DemoBackground`.
fn demo_background(brush: Rc<dyn IBrush>) -> Ref<ResourceDictionary> {
    let dictionary = ResourceDictionary::new();
    dictionary.set("DemoBackground", Some(boxed(brush)));
    dictionary
}

/// The scope of the tests without a document: the light variant is
/// requested, the theme dictionaries hold a black (dark) and a white
/// (light) `DemoBackground`, and the child is a border.
fn scope_without_xaml() -> Ref<ThemeVariantScope> {
    let theme_variant_scope = ThemeVariantScope::new();
    theme_variant_scope.set_requested_theme_variant(Some(ThemeVariant::light()));
    let resources = ResourceDictionary::new();
    resources.set_theme_dictionary(ThemeVariant::dark(), demo_background(Brushes::black()));
    resources.set_theme_dictionary(ThemeVariant::light(), demo_background(Brushes::white()));
    theme_variant_scope.set_resources(resources);
    theme_variant_scope.set_child(Border::new());
    theme_variant_scope
}

/// The mock of the theme variant host of the application: its actual theme
/// variant is the dark one.
struct ApplicationThemeHost;

impl IResourceNode for ApplicationThemeHost {
    fn has_resources(&self) -> bool {
        false
    }

    fn try_get_resource(&self, _key: &ResourceKey, _theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        None
    }
}

impl IResourceHost for ApplicationThemeHost {
    fn resources_changed(&self, _handler: Rc<dyn Fn(&ResourcesChangedEventArgs)>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }

    fn notify_hosted_resources_changed(&self, _e: ResourcesChangedEventArgs) {}

    fn as_theme_variant_host(&self) -> Option<&dyn IThemeVariantHost> {
        Some(self)
    }
}

impl IThemeVariantHost for ApplicationThemeHost {
    fn actual_theme_variant(&self) -> Option<ThemeVariant> {
        Some(ThemeVariant::dark())
    }

    fn actual_theme_variant_changed(&self, _handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

#[test]
fn dynamic_resource_updated_when_control_theme_changed() {
    let _base = xaml_test_base();
    let theme_variant_scope = load_as::<Ref<ThemeVariantScope>>(
        "
<ThemeVariantScope xmlns='https://github.com/ferroui'
              xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
              RequestedThemeVariant='Light'>
    <ThemeVariantScope.Resources>
        <ResourceDictionary>
            <ResourceDictionary.ThemeDictionaries>
                <ResourceDictionary x:Key='Dark'>
                    <SolidColorBrush x:Key='DemoBackground'>Black</SolidColorBrush>
                </ResourceDictionary>
                <ResourceDictionary x:Key='Light'>
                    <SolidColorBrush x:Key='DemoBackground'>White</SolidColorBrush>
                </ResourceDictionary>
            </ResourceDictionary.ThemeDictionaries>
        </ResourceDictionary>
    </ThemeVariantScope.Resources>

    <Border Name='border' Background='{DynamicResource DemoBackground}'/>
</ThemeVariantScope>",
    );
    let border = border_of(&theme_variant_scope);

    assert_eq!(Colors::WHITE, color_of(border.background()));

    let theme_variant_key: String = ['D', 'a', 'r', 'k'].into_iter().collect(); // Ensure that a non-interned string works
    theme_variant_scope.set_requested_theme_variant(Some(ThemeVariant::new(theme_variant_key, None)));

    assert_eq!(Colors::BLACK, color_of(border.background()));
}

#[test]
fn dynamic_resource_updated_when_control_theme_changed_no_xaml() {
    let _base = xaml_test_base();
    let theme_variant_scope = scope_without_xaml();
    let border = border_of(&theme_variant_scope);
    let extension = DynamicResourceExtension::with_resource_key(Some(boxed("DemoBackground".to_string())));
    border.bind_indexer(&Border::background_property().bind(), &*extension);

    DelayedBinding::apply_bindings(&border);

    assert_eq!(Colors::WHITE, color_of(border.background()));

    theme_variant_scope.set_requested_theme_variant(Some(ThemeVariant::dark()));

    assert_eq!(Colors::BLACK, color_of(border.background()));
}

#[test]
fn intermediate_dynamic_resource_updated_when_control_theme_changed() {
    let _base = xaml_test_base();
    let theme_variant_scope = load_as::<Ref<ThemeVariantScope>>(
        "
<ThemeVariantScope xmlns='https://github.com/ferroui'
              xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
              RequestedThemeVariant='Light'>
    <ThemeVariantScope.Resources>
        <ResourceDictionary>
            <ResourceDictionary.ThemeDictionaries>
                <ResourceDictionary x:Key='Dark'>
                    <Color x:Key='TestColor'>Black</Color>
                </ResourceDictionary>
                <ResourceDictionary x:Key='Light'>
                    <Color x:Key='TestColor'>White</Color>
                </ResourceDictionary>
            </ResourceDictionary.ThemeDictionaries>
            <SolidColorBrush x:Key='DemoBackground' Color='{DynamicResource TestColor}' />
        </ResourceDictionary>
    </ThemeVariantScope.Resources>

    <Border Name='border' Background='{DynamicResource DemoBackground}'/>
</ThemeVariantScope>",
    );
    let border = border_of(&theme_variant_scope);

    assert_eq!(Colors::WHITE, color_of(border.background()));

    theme_variant_scope.set_requested_theme_variant(Some(ThemeVariant::dark()));

    assert_eq!(Colors::BLACK, color_of(border.background()));
}

#[test]
fn dynamic_resource_in_resource_provider_updated_when_control_theme_changed() {
    let _base = xaml_test_base();
    let theme_variant_scope = scope_without_xaml();

    let resources = load_as::<Ref<ResourceDictionary>>(
        "
<ResourceDictionary xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <GeometryDrawing x:Key='Geo' Brush='{DynamicResource DemoBackground}' />
</ResourceDictionary>",
    );

    let provider: Rc<dyn IResourceProvider> = resources.clone().into();
    theme_variant_scope.resources().merged_dictionaries().add(provider);
    let geo = object_of::<GeometryDrawing>(&theme_variant_scope.find_resource(&"Geo".into()));

    assert_eq!(Colors::WHITE, color_of(geo.brush()));

    theme_variant_scope.set_requested_theme_variant(Some(ThemeVariant::dark()));

    assert_eq!(Colors::BLACK, color_of(geo.brush()));
}

#[test]
fn intermediate_static_resource_can_be_reached_from_theme_dictionaries() {
    let _base = xaml_test_base();
    let theme_variant_scope = load_as::<Ref<ThemeVariantScope>>(
        "
<ThemeVariantScope xmlns='https://github.com/ferroui'
              xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
              RequestedThemeVariant='Light'>
    <ThemeVariantScope.Resources>
        <ResourceDictionary>
            <ResourceDictionary.ThemeDictionaries>
                <ResourceDictionary x:Key='Dark'>
                    <Color x:Key='TestColor'>Black</Color>
                    <StaticResource x:Key='DemoBackground' ResourceKey='TestColor' />
                </ResourceDictionary>
                <ResourceDictionary x:Key='Light'>
                    <Color x:Key='TestColor'>White</Color>
                    <StaticResource x:Key='DemoBackground' ResourceKey='TestColor' />
                </ResourceDictionary>
            </ResourceDictionary.ThemeDictionaries>
        </ResourceDictionary>
    </ThemeVariantScope.Resources>

    <Border Name='border' Background='{DynamicResource DemoBackground}'/>
</ThemeVariantScope>",
    );
    let border = border_of(&theme_variant_scope);

    assert_eq!(Colors::WHITE, color_of(border.background()));

    theme_variant_scope.set_requested_theme_variant(Some(ThemeVariant::dark()));

    assert_eq!(Colors::BLACK, color_of(border.background()));
}

#[test]
fn static_resource_inside_of_theme_dictionaries_should_use_same_theme_key() {
    let _base = xaml_test_base();
    let theme_variant_scope = load_as::<Ref<ThemeVariantScope>>(
        "
<ThemeVariantScope xmlns='https://github.com/ferroui'
              xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
              RequestedThemeVariant='Light'>
    <ThemeVariantScope.Resources>
        <ResourceDictionary>
            <ResourceDictionary.ThemeDictionaries>
                <ResourceDictionary x:Key='Dark'>
                    <Color x:Key='TestColor'>Black</Color>
                </ResourceDictionary>
                <ResourceDictionary x:Key='Light'>
                    <Color x:Key='TestColor'>White</Color>
                </ResourceDictionary>
            </ResourceDictionary.ThemeDictionaries>
        </ResourceDictionary>
    </ThemeVariantScope.Resources>

    <Border Name='border' Background='{DynamicResource DemoBackground}'>
        <Border.Resources>
            <ResourceDictionary>
                <ResourceDictionary.ThemeDictionaries>
                    <ResourceDictionary x:Key='Dark'>
                        <StaticResource x:Key='DemoBackground' ResourceKey='TestColor' />
                    </ResourceDictionary>
                    <ResourceDictionary x:Key='Light'>
                        <StaticResource x:Key='DemoBackground' ResourceKey='TestColor' />
                    </ResourceDictionary>
                </ResourceDictionary.ThemeDictionaries>
            </ResourceDictionary>
        </Border.Resources>
    </Border>
</ThemeVariantScope>",
    );
    let border = border_of(&theme_variant_scope);

    assert_eq!(Colors::WHITE, color_of(border.background()));

    theme_variant_scope.set_requested_theme_variant(Some(ThemeVariant::dark()));

    assert_eq!(Colors::BLACK, color_of(border.background()));
}

#[test]
fn static_resource_inside_of_theme_dictionaries_should_use_same_theme_key_from_inner_file() {
    let _base = xaml_test_base();
    let documents = vec![
        document(
            "ferres://Tests/Inner.xaml",
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <StaticResource x:Key='InnerKey' ResourceKey='OuterKey' />
</ResourceDictionary>",
        ),
        document_without_uri(
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.ThemeDictionaries>
        <ResourceDictionary x:Key='Default'>
            <Color x:Key='OuterKey'>Green</Color>
            <ResourceDictionary.MergedDictionaries>
                <ResourceInclude Source='ferres://Tests/Inner.xaml'/>
            </ResourceDictionary.MergedDictionaries>
        </ResourceDictionary>
        <ResourceDictionary x:Key='Dark'>
            <Color x:Key='OuterKey'>White</Color>
            <ResourceDictionary.MergedDictionaries>
                <ResourceInclude Source='ferres://Tests/Inner.xaml'/>
            </ResourceDictionary.MergedDictionaries>
        </ResourceDictionary>
    </ResourceDictionary.ThemeDictionaries>
</ResourceDictionary>",
        ),
    ];

    let parsed = load_group(documents, None);
    let dictionary = group_item::<Ref<ResourceDictionary>>(&parsed, 1);

    let resource = dictionary.try_get_resource(&"InnerKey".into(), Some(&ThemeVariant::dark())).flatten();
    assert!(is_type::<Color>(&resource));
    let color_resource = value_of::<Color>(&resource).expect("a Color");
    assert_eq!(Colors::WHITE, color_resource);

    let resource = dictionary.try_get_resource(&"InnerKey".into(), Some(&ThemeVariant::light())).flatten();
    assert!(is_type::<Color>(&resource));
    let color_resource = value_of::<Color>(&resource).expect("a Color");
    assert_eq!(Colors::GREEN, color_resource);
}

#[test]
fn dynamic_resource_inside_of_theme_dictionaries_should_use_same_theme_key_from_inner_file() {
    let _base = xaml_test_base();
    let documents = vec![
        document(
            "ferres://Tests/Inner.xaml",
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <SolidColorBrush x:Key='InnerKey' Color='{DynamicResource OuterKey}' />
</ResourceDictionary>",
        ),
        document_without_uri(
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.ThemeDictionaries>
        <ResourceDictionary x:Key='Default'>
            <Color x:Key='OuterKey'>Green</Color>
            <ResourceDictionary.MergedDictionaries>
                <ResourceInclude Source='ferres://Tests/Inner.xaml'/>
            </ResourceDictionary.MergedDictionaries>
        </ResourceDictionary>
        <ResourceDictionary x:Key='Dark'>
            <Color x:Key='OuterKey'>White</Color>
            <ResourceDictionary.MergedDictionaries>
                <ResourceInclude Source='ferres://Tests/Inner.xaml'/>
            </ResourceDictionary.MergedDictionaries>
        </ResourceDictionary>
    </ResourceDictionary.ThemeDictionaries>
</ResourceDictionary>",
        ),
    ];

    let parsed = load_group(documents, None);
    let dictionary1 = group_item::<Ref<ResourceDictionary>>(&parsed, 0);
    let dictionary2 = group_item::<Ref<ResourceDictionary>>(&parsed, 1);
    let owner_app = Application::new(); // DynamicResource needs an owner to work
    owner_app.set_requested_theme_variant(Some(ThemeVariant::new("FakeOne", None)));
    let provider1: Rc<dyn IResourceProvider> = dictionary1.clone().into();
    let provider2: Rc<dyn IResourceProvider> = dictionary2.clone().into();
    owner_app.resources().merged_dictionaries().add(provider1);
    owner_app.resources().merged_dictionaries().add(provider2);

    let resource = dictionary2.try_get_resource(&"InnerKey".into(), Some(&ThemeVariant::dark())).flatten();
    let brush = value_of::<Rc<dyn IBrush>>(&resource).expect("a brush");
    let color_resource = brush.as_solid_color_brush().expect("a solid color brush");
    assert_eq!(Colors::WHITE, color_resource.color());

    let resource = dictionary2.try_get_resource(&"InnerKey".into(), Some(&ThemeVariant::light())).flatten();
    let brush = value_of::<Rc<dyn IBrush>>(&resource).expect("a brush");
    let color_resource = brush.as_solid_color_brush().expect("a solid color brush");
    assert_eq!(Colors::GREEN, color_resource.color());
}

#[test]
fn dynamic_resource_inside_control_inside_of_theme_dictionaries_should_use_control_theme_variant() {
    let _base = xaml_test_base();
    let documents = vec![document_without_uri(
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.ThemeDictionaries>
        <ResourceDictionary x:Key='Light'>
            <Color x:Key='ResourceKey'>Green</Color>
            <Template x:Key='Template'>
                <ThemeVariantScope RequestedThemeVariant='Dark' TextElement.Foreground='{DynamicResource ResourceKey}' />
            </Template>
        </ResourceDictionary>
        <ResourceDictionary x:Key='Dark'>
            <Color x:Key='ResourceKey'>White</Color>
            <Template x:Key='Template'>
                <ThemeVariantScope RequestedThemeVariant='Light' TextElement.Foreground='{DynamicResource ResourceKey}' />
            </Template>
        </ResourceDictionary>
    </ResourceDictionary.ThemeDictionaries>
</ResourceDictionary>",
    )];

    let parsed = load_group(documents, None);
    let dictionary = group_item::<Ref<ResourceDictionary>>(&parsed, 0);
    let provider: Rc<dyn IResourceProvider> = dictionary.clone().into();

    let resource = dictionary.try_get_resource(&"Template".into(), Some(&ThemeVariant::dark())).flatten();
    let built = value_of::<Rc<Template>>(&resource).and_then(|template| template.build()).expect("a built control");
    assert_is_type::<ThemeVariantScope>(&built);
    let control = built.cast::<ThemeVariantScope>().expect("a ThemeVariantScope");
    control.resources().merged_dictionaries().add(provider.clone());
    assert_eq!(Colors::GREEN, color_of(control.get_value(TextElement::foreground_property())));
    control.resources().merged_dictionaries().remove(&provider);

    let resource = dictionary.try_get_resource(&"Template".into(), Some(&ThemeVariant::light())).flatten();
    let built = value_of::<Rc<Template>>(&resource).and_then(|template| template.build()).expect("a built control");
    assert_is_type::<ThemeVariantScope>(&built);
    let control = built.cast::<ThemeVariantScope>().expect("a ThemeVariantScope");
    control.resources().merged_dictionaries().add(provider);
    assert_eq!(Colors::WHITE, color_of(control.get_value(TextElement::foreground_property())));
}

#[test]
fn static_resource_outside_of_dictionaries_should_use_control_theme_variant() {
    let _base = xaml_test_base();
    let scope = FerroLocator::enter_scope();
    {
        let application_theme_host: Rc<dyn IThemeVariantHost> = Rc::new(ApplicationThemeHost);
        FerroLocator::current_mutable().bind::<dyn IThemeVariantHost>().to_constant(application_theme_host);

        let theme_variant_scope = load_as::<Ref<ThemeVariantScope>>(
            "
<ThemeVariantScope xmlns='https://github.com/ferroui'
              xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
              RequestedThemeVariant='Light'>
    <ThemeVariantScope.Resources>
        <ResourceDictionary>
            <ResourceDictionary.ThemeDictionaries>
                <ResourceDictionary x:Key='Dark'>
                    <SolidColorBrush x:Key='DemoBackground'>Black</SolidColorBrush>
                </ResourceDictionary>
                <ResourceDictionary x:Key='Light'>
                    <SolidColorBrush x:Key='DemoBackground'>White</SolidColorBrush>
                </ResourceDictionary>
            </ResourceDictionary.ThemeDictionaries>
        </ResourceDictionary>
    </ThemeVariantScope.Resources>

    <Border Name='border' Background='{StaticResource DemoBackground}'/>
</ThemeVariantScope>",
        );
        let border = border_of(&theme_variant_scope);

        theme_variant_scope.set_requested_theme_variant(Some(ThemeVariant::light()));
        assert_eq!(Colors::WHITE, color_of(border.background()));
    }
    scope.dispose();
}

#[test]
fn inner_theme_dictionaries_works_properly() {
    let _base = xaml_test_base();
    let theme_variant_scope = load_as::<Ref<ThemeVariantScope>>(
        "
<ThemeVariantScope xmlns='https://github.com/ferroui'
              xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
              RequestedThemeVariant='Light'>
    <Border Name='border' Background='{DynamicResource DemoBackground}'>
        <Border.Resources>
            <ResourceDictionary>
                <ResourceDictionary.ThemeDictionaries>
                    <ResourceDictionary x:Key='Dark'>
                        <SolidColorBrush x:Key='DemoBackground'>Black</SolidColorBrush>
                    </ResourceDictionary>
                    <ResourceDictionary x:Key='Light'>
                        <SolidColorBrush x:Key='DemoBackground'>White</SolidColorBrush>
                    </ResourceDictionary>
                </ResourceDictionary.ThemeDictionaries>
            </ResourceDictionary>
        </Border.Resources>
    </Border>
</ThemeVariantScope>",
    );
    let border = border_of(&theme_variant_scope);

    assert_eq!(Colors::WHITE, color_of(border.background()));

    theme_variant_scope.set_requested_theme_variant(Some(ThemeVariant::dark()));

    assert_eq!(Colors::BLACK, color_of(border.background()));
}

#[test]
fn inner_resource_can_reference_parent_theme_dictionaries() {
    let _base = xaml_test_base();
    let theme_variant_scope = load_as::<Ref<ThemeVariantScope>>(
        "
<ThemeVariantScope xmlns='https://github.com/ferroui'
              xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
              RequestedThemeVariant='Light'>
    <ThemeVariantScope.Resources>
        <ResourceDictionary>
            <ResourceDictionary.ThemeDictionaries>
                <ResourceDictionary x:Key='Dark'>
                    <Color x:Key='TestColor'>Black</Color>
                </ResourceDictionary>
                <ResourceDictionary x:Key='Light'>
                    <Color x:Key='TestColor'>White</Color>
                </ResourceDictionary>
            </ResourceDictionary.ThemeDictionaries>
        </ResourceDictionary>
    </ThemeVariantScope.Resources>

    <Border Name='border' Background='{DynamicResource DemoBackground}'>
        <Border.Resources>
            <ResourceDictionary>
                <SolidColorBrush x:Key='DemoBackground' Color='{DynamicResource TestColor}' />
            </ResourceDictionary>
        </Border.Resources>
    </Border>
</ThemeVariantScope>",
    );
    let border = border_of(&theme_variant_scope);

    assert_eq!(Colors::WHITE, color_of(border.background()));

    theme_variant_scope.set_requested_theme_variant(Some(ThemeVariant::dark()));

    assert_eq!(Colors::BLACK, color_of(border.background()));
}

#[test]
fn dynamic_resource_can_access_resources_outside_of_theme_dictionaries() {
    let _base = xaml_test_base();
    let theme_variant_scope = load_as::<Ref<ThemeVariantScope>>(
        "
<ThemeVariantScope xmlns='https://github.com/ferroui'
              xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
              RequestedThemeVariant='Light'>
    <ThemeVariantScope.Resources>
        <ResourceDictionary>
            <ResourceDictionary.ThemeDictionaries>
                <ResourceDictionary x:Key='Dark'>
                    <SolidColorBrush x:Key='DemoBackground' Color='{DynamicResource TestColor1}' />
                </ResourceDictionary>
                <ResourceDictionary x:Key='Light'>
                    <SolidColorBrush x:Key='DemoBackground' Color='{DynamicResource TestColor2}' />
                </ResourceDictionary>
            </ResourceDictionary.ThemeDictionaries>
            <Color x:Key='TestColor1'>Black</Color>
            <Color x:Key='TestColor2'>White</Color>
        </ResourceDictionary>
    </ThemeVariantScope.Resources>

    <Border Name='border' Background='{DynamicResource DemoBackground}' />
</ThemeVariantScope>",
    );
    let border = border_of(&theme_variant_scope);

    assert_eq!(Colors::WHITE, color_of(border.background()));

    theme_variant_scope.set_requested_theme_variant(Some(ThemeVariant::dark()));

    assert_eq!(Colors::BLACK, color_of(border.background()));
}

#[test]
fn inner_dictionary_does_not_affect_parent_resources() {
    // It might be a nice feature, but neither this framework nor UWP supports it.
    // Better to expect this limitation with a unit test.
    let _base = xaml_test_base();
    let theme_variant_scope = load_as::<Ref<ThemeVariantScope>>(
        "
<ThemeVariantScope xmlns='https://github.com/ferroui'
              xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
              RequestedThemeVariant='Light'>
    <ThemeVariantScope.Resources>
        <ResourceDictionary>
            <Color x:Key='TestColor'>Red</Color>
            <SolidColorBrush x:Key='DemoBackground' Color='{DynamicResource TestColor}' />
        </ResourceDictionary>
    </ThemeVariantScope.Resources>

    <Border Name='border' Background='{DynamicResource DemoBackground}'>
        <Border.Resources>
            <ResourceDictionary>
                <ResourceDictionary.ThemeDictionaries>
                    <ResourceDictionary x:Key='Dark'>
                        <Color x:Key='TestColor'>Black</Color>
                    </ResourceDictionary>
                    <ResourceDictionary x:Key='Light'>
                        <Color x:Key='TestColor'>White</Color>
                    </ResourceDictionary>
                </ResourceDictionary.ThemeDictionaries>
            </ResourceDictionary>
        </Border.Resources>
    </Border>
</ThemeVariantScope>",
    );
    let border = border_of(&theme_variant_scope);

    assert_eq!(Colors::RED, color_of(border.background()));

    theme_variant_scope.set_requested_theme_variant(Some(ThemeVariant::dark()));

    assert_eq!(Colors::RED, color_of(border.background()));
}

#[test]
fn custom_theme_can_be_defined_in_theme_dictionaries() {
    let _base = xaml_test_base();
    let theme_variant_scope = load_as::<Ref<ThemeVariantScope>>(
        "
<ThemeVariantScope xmlns='https://github.com/ferroui'
              xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
              xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'
              RequestedThemeVariant='Light'>
    <ThemeVariantScope.Resources>
        <ResourceDictionary>
            <ResourceDictionary.ThemeDictionaries>
                <ResourceDictionary x:Key='Dark'>
                    <SolidColorBrush x:Key='DemoBackground'>Black</SolidColorBrush>
                </ResourceDictionary>
                <ResourceDictionary x:Key='Light'>
                    <SolidColorBrush x:Key='DemoBackground'>White</SolidColorBrush>
                </ResourceDictionary>
                <ResourceDictionary x:Key='{x:Static local:ThemeDictionariesTests.Custom}'>
                    <SolidColorBrush x:Key='DemoBackground'>Pink</SolidColorBrush>
                </ResourceDictionary>
            </ResourceDictionary.ThemeDictionaries>
        </ResourceDictionary>
    </ThemeVariantScope.Resources>

    <Border Name='border' Background='{DynamicResource DemoBackground}'/>
</ThemeVariantScope>",
    );
    let border = border_of(&theme_variant_scope);

    theme_variant_scope.set_requested_theme_variant(Some(ThemeDictionariesTests::custom()));

    assert_eq!(Colors::PINK, color_of(border.background()));
}

#[test]
fn custom_theme_fallbacks_to_inherit_theme_dynamic_resource() {
    let _base = xaml_test_base();
    let theme_variant_scope = load_as::<Ref<ThemeVariantScope>>(
        "
<ThemeVariantScope xmlns='https://github.com/ferroui'
              xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
              RequestedThemeVariant='Light'>
   <ThemeVariantScope.Resources>
        <ResourceDictionary>
            <ResourceDictionary.ThemeDictionaries>
                <ResourceDictionary x:Key='Dark'>
                    <SolidColorBrush x:Key='DemoBackground'>Black</SolidColorBrush>
                </ResourceDictionary>
            </ResourceDictionary.ThemeDictionaries>
        </ResourceDictionary>
    </ThemeVariantScope.Resources>
    <Border Background='{DynamicResource DemoBackground}' />
</ThemeVariantScope>",
    );
    let border = border_of(&theme_variant_scope);

    theme_variant_scope.set_requested_theme_variant(Some(ThemeVariant::new("Custom", Some(ThemeVariant::dark()))));

    assert_eq!(Colors::BLACK, color_of(border.background()));
}

#[test]
fn custom_theme_fallbacks_to_inherit_theme_static_resource() {
    let _base = xaml_test_base();
    let theme_variant_scope = load_as::<Ref<ThemeVariantScope>>(
        "
<ThemeVariantScope xmlns='https://github.com/ferroui'
              xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ThemeVariantScope.RequestedThemeVariant>
        <ThemeVariant>
            <x:Arguments>
                <x:String>Custom</x:String>
                <ThemeVariant>Dark</ThemeVariant>
            </x:Arguments>
        </ThemeVariant>
    </ThemeVariantScope.RequestedThemeVariant>
   <ThemeVariantScope.Resources>
        <ResourceDictionary>
            <ResourceDictionary.ThemeDictionaries>
                <ResourceDictionary x:Key='Dark'>
                    <SolidColorBrush x:Key='DemoBackground'>Black</SolidColorBrush>
                </ResourceDictionary>
            </ResourceDictionary.ThemeDictionaries>
        </ResourceDictionary>
    </ThemeVariantScope.Resources>

    <Border Background='{StaticResource DemoBackground}' />
</ThemeVariantScope>",
    );
    let border = border_of(&theme_variant_scope);

    assert_eq!(Colors::BLACK, color_of(border.background()));
}

#[test]
#[ignore = "themes: needs the Simple theme (upstream runs this test under it)"]
fn theme_switch_works_in_nested_scope() {
    let _app = styled_window_application();
    let window = load_as::<Ref<Window>>(
        "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        RequestedThemeVariant='Dark'>
    <ThemeVariantScope Name='Scope'>
        <TextBlock Name='Text' />
    </ThemeVariantScope>
</Window>",
    );
    window.apply_template();

    let scope = window.find_control::<ThemeVariantScope>("Scope").expect("the scope is found by name");
    let text = window.find_control::<TextBlock>("Text").expect("the text block is found by name");

    assert!(Some(ThemeVariant::dark()) == text.actual_theme_variant());
    assert_eq!(Color::parse("#dedede").expect("a color"), color_of(text.foreground()));

    scope.set_requested_theme_variant(Some(ThemeVariant::light()));
    assert!(Some(ThemeVariant::light()) == text.actual_theme_variant());
    assert_eq!(Colors::BLACK, color_of(text.foreground()));
}

#[test]
#[ignore = "themes: needs the Simple theme (upstream runs this test under it)"]
fn theme_switch_works_in_with_popup() {
    let _outer_app = styled_window_application();
    let _app = styled_window_application();
    let window = load_as::<Ref<Window>>(
        "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ThemeVariantScope Name='Scope' RequestedThemeVariant='Dark'>
        <Popup Name='Popup'>
            <Border Width='100' Height='100'
                    Background='{DynamicResource ThemeBackgroundBrush}'>
            </Border>
        </Popup>
    </ThemeVariantScope>
</Window>",
    );
    window.show();

    let scope = window.find_control::<ThemeVariantScope>("Scope").expect("the scope is found by name");
    let popup = window.find_control::<Popup>("Popup").expect("the popup is found by name");

    popup.set_is_open(true);

    let border = popup.child().expect("the popup has a child").cast::<Border>().expect("the child is a Border");

    assert!(Some(ThemeVariant::dark()) == popup.actual_theme_variant());
    assert!(Some(ThemeVariant::dark()) == border.actual_theme_variant());
    assert_eq!(Color::parse("#282828").expect("a color"), color_of(border.background()));

    scope.set_requested_theme_variant(Some(ThemeVariant::light()));

    assert!(Some(ThemeVariant::light()) == popup.actual_theme_variant());
    assert!(Some(ThemeVariant::light()) == border.actual_theme_variant());
    assert_eq!(Colors::WHITE, color_of(border.background()));
}
