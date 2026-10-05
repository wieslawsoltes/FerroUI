//! Ported from the upstream `MarkupExtensions/DynamicResourceExtensionTests`.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::controls::{
    IResourceDictionary, IResourceProvider, ResourceDictionary, ResourceKey, ResourceProvider, ResourceProviderImpl,
    ResourceValue,
};
use ferroui_base::media::{Color, Colors, IBrush, SolidColorBrush};
use ferroui_base::reactive::ObservableExt;
use ferroui_base::styling::{IStyle, Selectors, Setter, Style, Styles, ThemeVariant};
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, FerroObject, FerroObjectExtensions, FerroObjectImpl, ObjectType, Ref,
    Visual,
};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use ferroui_controls::testing::UnitTestApplicationScope;
use ferroui_controls::{Application, Border, Button, ContentControl, Control, Grid, ListBox, UserControl, Window};
use ferroui_markup::markup::data::DelayedBinding;

use crate::support::app::*;
use crate::support::helpers::*;
use crate::support::loader::*;
use crate::support::TypeModule;
use crate::support_bindings::*;

// --- test types -------------------------------------------------------------

/// A resource provider that resolves every key to the key itself and
/// records the keys it was asked for.
#[repr(C)]
pub struct TrackingResourceProvider {
    base: ResourceProvider,
    requested_resources: RefCell<Vec<ResourceKey>>,
}

ferro_class!(TrackingResourceProvider: ResourceProvider);
ferro_class_info!(TrackingResourceProvider {
    new: TrackingResourceProvider::new,
    markup: { namespace: "FerroUI.Markup.Xaml.UnitTests.MarkupExtensions" },
});

impl FerroObjectImpl for TrackingResourceProvider {}

impl ResourceProviderImpl for TrackingResourceProvider {
    fn has_resources(_this: &Self) -> bool {
        true
    }

    fn try_get_resource(this: &Self, key: &ResourceKey, _theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        this.requested_resources.borrow_mut().push(key.clone());
        // `value = key`: the resource is the key object.
        Some(match key {
            ResourceKey::String(text) => obj(text.to_string()),
            ResourceKey::Type(type_) => obj(*type_),
            ResourceKey::Object(_) => obj(key.clone()),
        })
    }
}

impl TrackingResourceProvider {
    pub fn construct() -> Self {
        Self { base: ResourceProvider::construct(), requested_resources: RefCell::new(Vec::new()) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The keys the provider was asked for, in order.
    pub fn requested_resources(&self) -> Vec<ResourceKey> {
        self.requested_resources.borrow().clone()
    }
}

/// The test types of this file.
pub(crate) const MODULE: TypeModule =
    TypeModule { types: &[TrackingResourceProvider::TYPE], markup_types: &[], value_types: || {} };

// --- helpers ----------------------------------------------------------------

/// `Assert.IsAssignableFrom<ISolidColorBrush>(brush).Color`.
#[track_caller]
fn solid_color(brush: &Option<Rc<dyn IBrush>>) -> Color {
    let brush = brush.as_ref().expect("the brush is null");
    brush.as_solid_color_brush().expect("the brush is not a solid color brush").color()
}

/// `Assert.IsAssignableFrom<SolidColorBrush>(brush)`.
#[track_caller]
fn solid_color_brush(brush: &Option<Rc<dyn IBrush>>) -> Ref<SolidColorBrush> {
    let brush = brush.as_ref().expect("the brush is null");
    brush
        .as_object()
        .and_then(|object| object.to_ref().cast::<SolidColorBrush>())
        .expect("the brush is not a SolidColorBrush")
}

/// `(Style)styles[index]`.
#[track_caller]
fn style_at(styles: &Styles, index: usize) -> Ref<Style> {
    let style: Rc<dyn IStyle> = styles.get(index);
    style.as_object().and_then(|object| object.to_ref().cast::<Style>()).expect("the style is not a Style")
}

/// `(T)dictionary.MergedDictionaries[index]`.
#[track_caller]
fn merged_dictionary_at<T: ObjectType>(dictionary: &ResourceDictionary, index: usize) -> Ref<T> {
    let provider: Rc<dyn IResourceProvider> = dictionary.merged_dictionaries().get(index);
    provider
        .as_object()
        .and_then(|object| object.to_ref().cast::<T>())
        .unwrap_or_else(|| panic!("the merged dictionary is not a '{}'", T::TYPE.name()))
}

/// `button.GetVisualChildren().Single()`.
#[track_caller]
fn single_visual_child(visual: &Visual) -> Ref<Visual> {
    let children = visual.get_visual_children();
    assert_eq!(children.len(), 1, "the sequence does not contain exactly one element");
    children[0].clone()
}

/// `new ResourceDictionary { { "brush", new SolidColorBrush(0xff506070) } }`.
fn brush_dictionary() -> Ref<ResourceDictionary> {
    let dictionary = ResourceDictionary::new();
    dictionary.add("brush", obj(SolidColorBrush::from_uint32(0xff506070)));
    dictionary
}

/// `new Window { Resources = { { "color", Colors.Red } }, Content = userControl }`.
fn window_with_color_and_content(user_control: &Ref<UserControl>) -> Ref<Window> {
    let window = Window::new();
    window.resources().add("color", obj(Colors::RED));
    window.set_content(Some(Control::boxed(user_control.clone())));
    window
}

fn styled_window() -> UnitTestApplicationScope {
    let services = TestServices::styled_window().with_theme(|| {
        let styles = Styles::new();
        styles.add(window_style());
        styles.into()
    });

    unit_test_application(services)
}

fn window_style() -> Ref<Style> {
    let template: Rc<dyn IControlTemplate> = FuncControlTemplate::for_type::<Window>(|x, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.bind_indexer(
            &ContentPresenter::content_property().bind(),
            &x.indexer(&ContentControl::content_property().bind()),
        );
        presenter.register_in_name_scope(&**scope).upcast()
    });

    Style::with_setters(
        Selectors::of_type::<Window>(),
        [Setter::new(TemplatedControl::template_property(), Some(template))],
    )
}

// --- tests ------------------------------------------------------------------

#[test]
fn dynamic_resource_can_be_assigned_to_property() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
    </UserControl.Resources>

    <Border Name='border' Background='{DynamicResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    DelayedBinding::apply_bindings(&border);

    assert_eq!(0xff506070, solid_color(&border.background()).to_uint32());
}

#[test]
fn dynamic_resource_can_be_assigned_to_attached_property() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <x:Int32 x:Key='col'>5</x:Int32>
    </UserControl.Resources>

    <Border Name='border' Grid.Column='{DynamicResource col}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    DelayedBinding::apply_bindings(&border);

    assert_eq!(5, Grid::get_column(&border));
}

#[test]
fn dynamic_resource_from_style_can_be_assigned_to_property() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Styles>
        <Style>
            <Style.Resources>
                <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
            </Style.Resources>
        </Style>
    </UserControl.Styles>

    <Border Name='border' Background='{DynamicResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    DelayedBinding::apply_bindings(&border);

    assert_eq!(0xff506070, solid_color(&border.background()).to_uint32());
}

#[test]
fn dynamic_resource_from_merged_dictionary_can_be_assigned_to_property() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <ResourceDictionary>
            <ResourceDictionary.MergedDictionaries>
                <ResourceDictionary>
                    <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
                </ResourceDictionary>
            </ResourceDictionary.MergedDictionaries>
        </ResourceDictionary>
    </UserControl.Resources>

    <Border Name='border' Background='{DynamicResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    DelayedBinding::apply_bindings(&border);

    assert_eq!(0xff506070, solid_color(&border.background()).to_uint32());
}

#[test]
fn dynamic_resource_from_merged_dictionary_in_style_can_be_assigned_to_property() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Styles>
        <Style>
            <Style.Resources>
                <ResourceDictionary>
                    <ResourceDictionary.MergedDictionaries>
                        <ResourceDictionary>
                            <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
                        </ResourceDictionary>
                    </ResourceDictionary.MergedDictionaries>
                </ResourceDictionary>
            </Style.Resources>
        </Style>
    </UserControl.Styles>

    <Border Name='border' Background='{DynamicResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    DelayedBinding::apply_bindings(&border);

    assert_eq!(0xff506070, solid_color(&border.background()).to_uint32());
}

#[test]
fn dynamic_resource_from_application_can_be_assigned_to_property_in_window() {
    let _app = styled_window();
    Application::current().unwrap().resources().add("brush", obj(SolidColorBrush::from_uint32(0xff506070)));

    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Border Name='border' Background='{DynamicResource brush}'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let border = window.get_control::<Border>("border");

    let brush = solid_color_brush(&border.background());
    assert_eq!(0xff506070, brush.color().to_uint32());
}

#[test]
fn dynamic_resource_from_application_can_be_assigned_to_property_in_user_control() {
    let _app = styled_window_application();
    Application::current().unwrap().resources().add("brush", obj(SolidColorBrush::from_uint32(0xff506070)));

    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Border Name='border' Background='{DynamicResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    // We don't actually know where the global styles are until we attach the control
    // to a window, as Window has StylingParent set to Application.
    let window = Window::new();
    window.set_content(Some(Control::boxed(user_control.clone())));
    window.show();

    let brush = solid_color_brush(&border.background());
    assert_eq!(0xff506070, brush.color().to_uint32());
}

#[test]
fn dynamic_resource_can_be_assigned_to_setter() {
    let _app = styled_window();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Resources>
        <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
    </Window.Resources>
    <Window.Styles>
        <Style Selector='Button'>
            <Setter Property='Background' Value='{DynamicResource brush}'/>
        </Style>
    </Window.Styles>
    <Button Name='button'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");
    let color = solid_color(&button.background());

    assert_eq!(0xff506070, color.to_uint32());
}

#[test]
fn dynamic_resource_from_style_can_be_assigned_to_setter() {
    let _app = styled_window();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <Style>
            <Style.Resources>
                <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
            </Style.Resources>
        </Style>
        <Style Selector='Button'>
            <Setter Property='Background' Value='{DynamicResource brush}'/>
        </Style>
    </Window.Styles>
    <Button Name='button'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");
    let color = solid_color(&button.background());

    assert_eq!(0xff506070, color.to_uint32());
}

#[test]
fn dynamic_resource_can_be_assigned_to_setter_in_styles_file() {
    let documents = vec![
        document("ferres://Tests/Style.xaml", r#"
<Styles xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Styles.Resources>
        <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
    </Styles.Resources>
    <Style Selector='Border'>
        <Setter Property='Background' Value='{DynamicResource brush}'/>
    </Style>
</Styles>"#),
        document_without_uri(r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <StyleInclude Source='ferres://Tests/Style.xaml'/>
    </Window.Styles>
    <Border Name='border'/>
</Window>"#),
    ];

    let _app = styled_window();
    let compiled = load_group(documents, None);
    let window: Ref<Window> = group_item(&compiled, 1);
    assert_is_type::<Window>(&window);
    let border = window.get_control::<Border>("border");
    let color = solid_color(&border.background());

    assert_eq!(0xff506070, color.to_uint32());
}

#[test]
fn dynamic_resource_can_be_assigned_to_property_in_control_template_in_styles_file() {
    let documents = vec![
        document("ferres://Tests/Style.xaml", r#"
<Styles xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Styles.Resources>
        <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
    </Styles.Resources>
    <Style Selector='Button'>
        <Setter Property='Template'>
            <ControlTemplate>
                <Border Name='border' Background='{DynamicResource brush}'/>
            </ControlTemplate>
        </Setter>
    </Style>
</Styles>"#),
        document_without_uri(r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <StyleInclude Source='ferres://Tests/Style.xaml'/>
    </Window.Styles>
    <Button Name='button'/>
</Window>"#),
    ];

    let _app = styled_window();
    let compiled = load_group(documents, None);
    let window: Ref<Window> = group_item(&compiled, 1);
    assert_is_type::<Window>(&window);
    let button = window.get_control::<Button>("button");

    window.show();

    let border = single_visual_child(&button).cast::<Border>().expect("the visual child is not a Border");
    let color = solid_color(&border.background());

    assert_eq!(0xff506070, color.to_uint32());
}

#[test]
fn dynamic_resource_can_be_assigned_to_resource_property() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <Color x:Key='color'>#ff506070</Color>
        <SolidColorBrush x:Key='brush' Color='{DynamicResource color}'/>
    </UserControl.Resources>

    <Border Name='border' Background='{DynamicResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    DelayedBinding::apply_bindings(&border);

    let brush = solid_color_brush(&border.background());
    assert_eq!(0xff506070, brush.color().to_uint32());
}

#[test]
fn dynamic_resource_can_be_assigned_to_resource_property_in_application() {
    let _base = xaml_test_base();
    let xaml = r#"
<Application xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Application.Resources>
        <Color x:Key='color'>#ff506070</Color>
        <SolidColorBrush x:Key='brush' Color='{DynamicResource color}'/>
    </Application.Resources>
</Application>"#;

    let application: Ref<Application> = load_as(xaml);
    let brush = object_of::<SolidColorBrush>(&application.resources().get(&"brush".into()));

    assert_eq!(0xff506070, brush.color().to_uint32());
}

#[test]
fn dynamic_resource_can_be_assigned_to_item_template_property() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <DataTemplate x:Key='PurpleData'>
          <TextBlock Text='{Binding Name}' Background='Purple'/>
        </DataTemplate>
    </UserControl.Resources>

    <ListBox Name='listBox' ItemTemplate='{DynamicResource PurpleData}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let list_box = user_control.get_control::<ListBox>("listBox");

    DelayedBinding::apply_bindings(&list_box);

    assert!(list_box.item_template().is_some());
}

#[test]
fn dynamic_resource_tracks_added_resource() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Border Name='border' Background='{DynamicResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    DelayedBinding::apply_bindings(&border);

    assert!(border.background().is_none());

    user_control.resources().add("brush", obj(SolidColorBrush::from_uint32(0xff506070)));

    let brush = solid_color_brush(&border.background());
    assert_eq!(0xff506070, brush.color().to_uint32());
}

#[test]
fn dynamic_resource_tracks_added_style_resource() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Border Name='border' Background='{DynamicResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    DelayedBinding::apply_bindings(&border);

    assert!(border.background().is_none());

    user_control.styles().resources().add("brush", obj(SolidColorBrush::from_uint32(0xff506070)));

    let brush = solid_color_brush(&border.background());
    assert_eq!(0xff506070, brush.color().to_uint32());
}

#[test]
fn dynamic_resource_tracks_added_nested_style_resource() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Styles>
        <Style>
        </Style>
    </UserControl.Styles>
    <Border Name='border' Background='{DynamicResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    DelayedBinding::apply_bindings(&border);

    assert!(border.background().is_none());

    style_at(&user_control.styles(), 0).resources().add("brush", obj(SolidColorBrush::from_uint32(0xff506070)));

    let brush = solid_color_brush(&border.background());
    assert_eq!(0xff506070, brush.color().to_uint32());
}

#[test]
fn dynamic_resource_tracks_added_merged_resource() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <ResourceDictionary>
            <ResourceDictionary.MergedDictionaries>
                <ResourceDictionary/>
            </ResourceDictionary.MergedDictionaries>
        </ResourceDictionary>
    </UserControl.Resources>
    <Border Name='border' Background='{DynamicResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    DelayedBinding::apply_bindings(&border);

    assert!(border.background().is_none());

    let merged: Rc<dyn IResourceDictionary> =
        merged_dictionary_at::<ResourceDictionary>(&user_control.resources(), 0).into();
    merged.add("brush".into(), obj(SolidColorBrush::from_uint32(0xff506070)));

    let brush = solid_color_brush(&border.background());
    assert_eq!(0xff506070, brush.color().to_uint32());
}

#[test]
fn dynamic_resource_tracks_added_merged_resource_dictionary() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Border Name='border' Background='{DynamicResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    DelayedBinding::apply_bindings(&border);

    assert!(border.background().is_none());

    let dictionary = brush_dictionary();

    user_control.resources().merged_dictionaries().add(Rc::<dyn IResourceProvider>::from(dictionary));

    let brush = solid_color_brush(&border.background());
    assert_eq!(0xff506070, brush.color().to_uint32());
}

#[test]
fn dynamic_resource_tracks_added_style_merged_resource_dictionary() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Styles>
        <Style>
        </Style>
    </UserControl.Styles>
    <Border Name='border' Background='{DynamicResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    DelayedBinding::apply_bindings(&border);

    assert!(border.background().is_none());

    let dictionary = brush_dictionary();

    style_at(&user_control.styles(), 0).resources().merged_dictionaries().add(Rc::<dyn IResourceProvider>::from(dictionary));

    let brush = solid_color_brush(&border.background());
    assert_eq!(0xff506070, brush.color().to_uint32());
}

#[test]
fn dynamic_resource_can_be_found_across_xaml_style_files() {
    let documents = vec![
        document("ferres://Tests/Style1.xaml", r#"
<Style xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
  <Style.Resources>
    <Color x:Key='Red'>Red</Color>
  </Style.Resources>
</Style>"#),
        document("ferres://Tests/Style2.xaml", r#"
<Style xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
  <Style.Resources>
    <SolidColorBrush x:Key='RedBrush' Color='{DynamicResource Red}'/>
  </Style.Resources>
</Style>"#),
        document_without_uri(r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <StyleInclude Source='ferres://Tests/Style1.xaml'/>
        <StyleInclude Source='ferres://Tests/Style2.xaml'/>
    </Window.Styles>
    <Border Name='border' Background='{DynamicResource RedBrush}'/>
</Window>"#),
    ];

    let _app = styled_window();
    let compiled = load_group(documents, None);
    let window: Ref<Window> = group_item(&compiled, 2);
    assert_is_type::<Window>(&window);
    let border = window.get_control::<Border>("border");
    let border_brush_color = solid_color(&border.background());

    assert_eq!(0xffff0000, border_brush_color.to_uint32());
}

#[test]
fn dynamic_resource_can_be_found_in_nested_style_file() {
    let documents = vec![
        document("ferres://Tests/Style1.xaml", r#"
<Styles xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
  <StyleInclude Source='ferres://Tests/Style2.xaml'/>
</Styles>"#),
        document("ferres://Tests/Style2.xaml", r#"
<Style xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
  <Style.Resources>
    <Color x:Key='Red'>Red</Color>
    <SolidColorBrush x:Key='RedBrush' Color='{DynamicResource Red}'/>
  </Style.Resources>
</Style>"#),
        document_without_uri(r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <StyleInclude Source='ferres://Tests/Style1.xaml'/>
    </Window.Styles>
    <Border Name='border' Background='{DynamicResource RedBrush}'/>
</Window>"#),
    ];

    let _app = styled_window();
    let compiled = load_group(documents, None);
    let window: Ref<Window> = group_item(&compiled, 2);
    assert_is_type::<Window>(&window);
    let border = window.get_control::<Border>("border");
    let border_brush_color = solid_color(&border.background());

    assert_eq!(0xffff0000, border_brush_color.to_uint32());
}

#[test]
fn control_property_is_updated_when_parent_is_changed() {
    let _app = styled_window();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Resources>
        <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
    </Window.Resources>

    <Border Name='border' Background='{DynamicResource brush}'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let border = window.get_control::<Border>("border");

    DelayedBinding::apply_bindings(&border);

    assert_eq!(0xff506070, solid_color(&border.background()).to_uint32());

    window.set_content(None);

    assert!(border.background().is_none());

    window.set_content(Some(Control::boxed(border.clone())));

    assert_eq!(0xff506070, solid_color(&border.background()).to_uint32());
}

#[test]
fn resource_with_dynamic_resource_is_updated_when_added_to_parent() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <SolidColorBrush x:Key='brush' Color='{DynamicResource color}'/>
    </UserControl.Resources>

    <Border Name='border' Background='{DynamicResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    DelayedBinding::apply_bindings(&border);

    let brush = solid_color_brush(&border.background());
    assert_eq!(0u32, brush.color().to_uint32());

    let brush_object: &FerroObject = &brush;
    let _subscription =
        FerroObjectExtensions::get_observable(brush_object, SolidColorBrush::color_property()).subscribe_fn(|_: Color| {});

    let _app = styled_window_application();
    let window = window_with_color_and_content(&user_control);

    window.show();

    assert_eq!(Colors::RED, brush.color());
}

#[test]
fn merged_dictionary_resource_with_dynamic_resource_is_updated_when_added_to_parent() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <ResourceDictionary>
            <ResourceDictionary.MergedDictionaries>
                <ResourceDictionary>
                    <SolidColorBrush x:Key='brush' Color='{DynamicResource color}'/>
                </ResourceDictionary>
            </ResourceDictionary.MergedDictionaries>
        </ResourceDictionary>
    </UserControl.Resources>

    <Border Name='border' Background='{DynamicResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    DelayedBinding::apply_bindings(&border);

    let brush = solid_color_brush(&border.background());
    assert_eq!(0u32, brush.color().to_uint32());

    let brush_object: &FerroObject = &brush;
    let _subscription =
        FerroObjectExtensions::get_observable(brush_object, SolidColorBrush::color_property()).subscribe_fn(|_: Color| {});

    let _app = styled_window_application();
    let window = window_with_color_and_content(&user_control);

    window.show();

    assert_eq!(Colors::RED, brush.color());
}

#[test]
fn style_resource_with_dynamic_resource_is_updated_when_added_to_parent() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Styles>
        <Style>
            <Style.Resources>
                <SolidColorBrush x:Key='brush' Color='{DynamicResource color}'/>
            </Style.Resources>
        </Style>
    </UserControl.Styles>

    <Border Name='border' Background='{DynamicResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    DelayedBinding::apply_bindings(&border);

    let brush = solid_color_brush(&border.background());
    assert_eq!(0u32, brush.color().to_uint32());

    let _app = styled_window_application();
    let window = window_with_color_and_content(&user_control);

    window.show();

    assert_eq!(Colors::RED, brush.color());
}

#[test]
fn automatically_converts_color_to_solid_color_brush() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <Color x:Key='color'>#ff506070</Color>
    </UserControl.Resources>

    <Border Name='border' Background='{DynamicResource color}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    assert_eq!(0xff506070, solid_color(&border.background()).to_uint32());
}

#[test]
fn resource_in_non_matching_style_is_not_resolved() {
    let _app = styled_window();

    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Window.Resources>
        <ResourceDictionary>
            <ResourceDictionary.MergedDictionaries>
                <local:TrackingResourceProvider/>
            </ResourceDictionary.MergedDictionaries>
        </ResourceDictionary>
    </Window.Resources>

    <Window.Styles>
        <Style Selector='Border.nomatch'>
            <Setter Property='Tag' Value='{DynamicResource foo}'/>
        </Style>
        <Style Selector='Border'>
            <Setter Property='Tag' Value='{DynamicResource bar}'/>
        </Style>
    </Window.Styles>

    <Border Name='border'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let border = window.get_control::<Border>("border");

    assert_string("bar", &border.tag());

    let resource_provider = merged_dictionary_at::<TrackingResourceProvider>(&window.resources(), 0);
    assert!(resource_provider.requested_resources().contains(&"bar".into()));
    assert!(!resource_provider.requested_resources().contains(&"foo".into()));
}

#[test]
fn resource_in_non_active_style_is_not_resolved() {
    let _app = styled_window();

    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Window.Resources>
        <ResourceDictionary>
            <ResourceDictionary.MergedDictionaries>
                <local:TrackingResourceProvider/>
            </ResourceDictionary.MergedDictionaries>
        </ResourceDictionary>
    </Window.Resources>

    <Window.Styles>
        <Style Selector='Border'>
            <Setter Property='Tag' Value='{DynamicResource foo}'/>
        </Style>
        <Style Selector='Border'>
            <Setter Property='Tag' Value='{DynamicResource bar}'/>
        </Style>
    </Window.Styles>

    <Border Name='border'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let border = window.get_control::<Border>("border");

    assert_string("bar", &border.tag());

    let resource_provider = merged_dictionary_at::<TrackingResourceProvider>(&window.resources(), 0);
    assert!(resource_provider.requested_resources().contains(&"bar".into()));
    assert!(!resource_provider.requested_resources().contains(&"foo".into()));
}

#[test]
fn can_detach_control_with_dynamic_resource_control_theme_that_contains_dynamic_resource() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
    RequestedThemeVariant='Light'>
    <Window.Resources>
        <SolidColorBrush x:Key='Blue'>Blue</SolidColorBrush>
        <ControlTheme x:Key='MyTheme' TargetType='Button'>
            <Setter Property='Background' Value='{DynamicResource Blue}'/>
        </ControlTheme>
    </Window.Resources>

    <Button Theme='{DynamicResource MyTheme}'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let target = object_of::<Button>(&window.content());
    assert_is_type::<Button>(&target);

    window.show();

    assert_eq!(Colors::BLUE, solid_color(&target.background()));

    window.set_content(None);
}

#[test]
fn handles_clearing_resources_with_dynamic_theme_in_dynamic_template() {
    // Issue #14753
    let _app = styled_window_application();
    let xaml = r#"<Window xmlns="https://github.com/ferroui"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
    <Window.Resources>
        <SolidColorBrush x:Key='Blue'>Blue</SolidColorBrush>
        <ControlTheme x:Key="MyBorder" TargetType="Border">
            <Setter Property="Background" Value="{DynamicResource Blue}"/>
        </ControlTheme>
        <ControlTheme x:Key="MyButton" TargetType="Button">
            <Setter Property="Template">
                <ControlTemplate>
                    <Border Theme="{DynamicResource MyBorder}"/>
                </ControlTemplate>
            </Setter>
        </ControlTheme>
    </Window.Resources>
    <Button Theme="{DynamicResource MyButton}" Background="{DynamicResource Blue}"/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    window.show();

    let button = object_of::<Button>(&window.content());
    assert_is_type::<Button>(&button);
    let border = single_visual_child(&button).cast::<Border>().expect("the visual child is not a Border");
    assert_is_type::<Border>(&border);
    assert_eq!(Colors::BLUE, solid_color(&border.background()));

    window.resources().clear();
}
