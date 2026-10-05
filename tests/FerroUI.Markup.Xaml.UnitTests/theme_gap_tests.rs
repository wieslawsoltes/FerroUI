//! The gaps the port of the Simple theme found (G1 to G10 of its gap list),
//! each as the smallest document that shows it. A test stays ignored with
//! the owner and the blocker of its gap until the gap is closed.

use ferroui_base::controls::ResourceDictionary;
use ferroui_base::styling::Style;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Border, Window};

use crate::support::app::{styled_window_application, xaml_test_base};
use crate::support::loader::{describe, load, load_as, try_load};

const NS: &str = "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";
const CONVERTERS: &str = "xmlns:converters='using:FerroUI.Controls.Converters'";

/// The resource of a loaded dictionary (realised if it is deferred).
fn resource(dictionary: &Ref<ResourceDictionary>, key: &str) -> BoxedValue {
    dictionary.get(&key.into()).unwrap_or_else(|| panic!("the dictionary has no resource '{key}'"))
}

fn dictionary(body: &str, namespaces: &str) -> Ref<ResourceDictionary> {
    load_as::<Ref<ResourceDictionary>>(&format!("<ResourceDictionary {NS} {namespaces}>{body}</ResourceDictionary>"))
}

// --- G1: the converters of the controls ---------------------------------------

#[test]
fn g01_border_gap_mask_converter() {
    let _base = xaml_test_base();
    resource(&dictionary("<converters:BorderGapMaskConverter x:Key='c' />", CONVERTERS), "c");
}

#[test]
fn g01_string_format_converter() {
    let _base = xaml_test_base();
    resource(&dictionary("<converters:StringFormatConverter x:Key='c' />", CONVERTERS), "c");
}

#[test]
fn g01_platform_key_gesture_converter() {
    let _base = xaml_test_base();
    resource(&dictionary("<converters:PlatformKeyGestureConverter x:Key='c' />", CONVERTERS), "c");
}

#[test]
fn g01_margin_multiplier_converter() {
    let _base = xaml_test_base();
    resource(
        &dictionary(
            "<converters:MarginMultiplierConverter x:Key='c' Left='True' Top='True' Right='True' Bottom='True' Indent='16' />",
            CONVERTERS,
        ),
        "c",
    );
}

#[test]
fn g01_corner_radius_filter_converter() {
    let _base = xaml_test_base();
    resource(
        &dictionary("<converters:CornerRadiusFilterConverter x:Key='c' Filter='TopLeft, TopRight' />", CONVERTERS),
        "c",
    );
}

#[test]
fn g01_menu_scrolling_visibility_converter_instance() {
    let _base = xaml_test_base();
    load(&format!(
        "<Border {NS} {CONVERTERS}><Border.IsVisible><MultiBinding Converter='{{x:Static converters:MenuScrollingVisibilityConverter.Instance}}' ConverterParameter='0'><Binding Path='Tag' /></MultiBinding></Border.IsVisible></Border>"
    ));
}

#[test]
fn g01_tree_view_item_indent_converter_instance() {
    let _base = xaml_test_base();
    load(&format!(
        "<Border {NS} {CONVERTERS}><Border.Margin><MultiBinding Converter='{{x:Static converters:TreeViewItemIndentConverter.Instance}}'><Binding Path='Tag' /></MultiBinding></Border.Margin></Border>"
    ));
}

#[test]
fn g01_converter_resource_is_used_by_a_binding() {
    let _base = xaml_test_base();
    // The three forms the theme documents use a converter in: a resource with
    // properties, `{StaticResource}` as the converter of a binding.
    let border = load_as::<Ref<Border>>(&format!(
        "<Border {NS} {CONVERTERS} Tag='2'>
           <Border.Resources>
             <converters:MarginMultiplierConverter x:Key='conv' Indent='16' Left='True' />
           </Border.Resources>
           <Border Margin='{{Binding $parent[Border].Tag, Converter={{StaticResource conv}}}}' />
         </Border>"
    ));
    assert!(border.child().is_some());
}

// --- G3: geometry from the text of an element -------------------------------

#[test]
fn g03_stream_geometry_from_text() {
    use ferroui_base::media::StreamGeometry;
    let _app = styled_window_application();
    let dictionary = dictionary("<StreamGeometry x:Key='g'>M 0,0 L 10,10 Z</StreamGeometry>", "");
    let geometry = resource(&dictionary, "g");
    assert!(crate::support::loader::try_cast::<Ref<StreamGeometry>>(&geometry).is_some());
}

// --- G4: FlowDirection ---------------------------------------------------------

#[test]
fn g04_flow_direction_attribute() {
    let _base = xaml_test_base();
    load(&format!("<Border {NS} FlowDirection='LeftToRight' />"));
}

// --- G5: page transitions and easings ----------------------------------------

#[test]
fn g05_cross_fade() {
    let _base = xaml_test_base();
    load(&format!(
        "<Style {NS} Selector='Expander'><Setter Property='ContentTransition'><Setter.Value><CrossFade Duration='00:00:00.25' /></Setter.Value></Setter></Style>"
    ));
}

#[test]
fn g05_page_slide_with_spline_easing() {
    let _base = xaml_test_base();
    load(&format!(
        "<Style {NS} Selector='TransitioningContentControl'><Setter Property='PageTransition'><Setter.Value><PageSlide Duration='0:0:0.3' Orientation='Horizontal' FillMode='Forward'><PageSlide.SlideInEasing><SplineEasing X1='0.16' Y1='1' X2='0.3' Y2='1' /></PageSlide.SlideInEasing></PageSlide></Setter.Value></Setter></Style>"
    ));
}

// --- G7: a template as the value of a setter --------------------------------

#[test]
fn g07_template_as_setter_value() {
    let _app = styled_window_application();
    let window = load_as::<Ref<Window>>(&format!(
        "<Window {NS}><Window.Styles><Style Selector='GridSplitter'><Setter Property='PreviewContent'><Template><Rectangle /></Template></Setter></Style></Window.Styles><GridSplitter /></Window>"
    ));
    window.show();
    let splitter = window
        .content()
        .and_then(|content| ferroui_controls::Control::from_boxed(&content))
        .and_then(|control| control.cast::<ferroui_controls::GridSplitter>())
        .expect("the content is the splitter");
    assert!(splitter.preview_content().is_some());
}

// --- G8: a static resource that is not found --------------------------------

#[test]
fn g08_missing_static_resource_on_a_control_is_delayed_and_elsewhere_an_error() {
    use ferroui_controls::ScrollViewer;
    let _base = xaml_test_base();
    // The target is a control and the target is a property (registered or not): upstream
    // delays the lookup until the control is initialised and assigns the unset value
    // now, which the unset value setter of a registered property takes.
    let border = load_as::<Ref<Border>>(&format!(
        "<Border {NS}>\n  <ScrollViewer Theme='{{StaticResource Missing}}' />\n</Border>"
    ));
    let scroll_viewer = border.child().and_then(|child| child.cast::<ScrollViewer>()).expect("the scroll viewer");
    assert!(scroll_viewer.theme().is_none());

    // The target is not a control: the lookup is not delayed and the load fails with
    // the error of the extension and the position of the element.
    let error = try_load(&format!(
        "<Border {NS}>\n  <Border.Background>\n    <SolidColorBrush Color='{{StaticResource Missing}}' />\n  </Border.Background>\n</Border>"
    ))
    .err()
    .expect("the load fails");
    let text = describe(&error);
    assert!(text.contains("Static resource 'Missing' not found."), "{text}");
    assert!(text.contains("line 3"), "{text}");
}

// --- G9: a transform from text in a setter ----------------------------------

#[test]
fn g09_transform_none_as_setter_value() {
    let _base = xaml_test_base();
    let style = load_as::<Ref<Style>>(&format!(
        "<Style {NS} Selector='Button'><Setter Property='RenderTransform' Value='none' /></Style>"
    ));
    assert_eq!(style.setters().count(), 1);
}

// --- G10: the types of the runtime library ----------------------------------

#[test]
fn g10_system_date_time() {
    let _base = xaml_test_base();
    resource(
        &dictionary("<DataTemplate x:Key='t' DataType='sys:DateTime'><TextBlock /></DataTemplate>", "xmlns:sys='using:System'"),
        "t",
    );
}

// --- G2: the automation attached properties ------------------------------------

#[test]
fn g02_automation_properties_as_attributes() {
    let _base = xaml_test_base();
    load(&format!(
        "<StackPanel {NS}>
           <TextBlock Name='other' Text='Label'/>
           <Button AutomationProperties.Name='x' AutomationProperties.AutomationId='id' AutomationProperties.LabeledBy='{{Binding #other}}'/>
         </StackPanel>"
    ));
}

#[test]
fn g02_automation_properties_in_a_setter() {
    let _base = xaml_test_base();
    let style = load_as::<Ref<Style>>(&format!(
        "<Style {NS} Selector='Border'><Setter Property='AutomationProperties.AutomationId' Value='PageUp' /></Style>"
    ));
    assert_eq!(style.setters().count(), 1);
}

// --- G6: shared size groups ----------------------------------------------------

#[test]
fn g06_shared_size_group_of_a_definition() {
    let _base = xaml_test_base();
    load(&format!(
        "<Grid {NS} Grid.IsSharedSizeScope='True'><Grid.ColumnDefinitions><ColumnDefinition Width='Auto' SharedSizeGroup='A' /><ColumnDefinition Width='*' /></Grid.ColumnDefinitions></Grid>"
    ));
}

#[test]
fn g06_shared_size_scope_on_an_items_presenter() {
    let _base = xaml_test_base();
    load(&format!("<ItemsPresenter {NS} Grid.IsSharedSizeScope='True' />"));
}

// --- G13: transform operations from the text of an element --------------------

#[test]
fn g13_transform_operations_from_text() {
    let _base = xaml_test_base();
    resource(&dictionary("<TransformOperations x:Key='t'>scaleX(0.125) translateX(-2px)</TransformOperations>", ""), "t");
}

// --- G14: a property selector for an attached property the type registers -------

#[test]
fn g14_property_selector_for_an_attached_property_of_the_target_type() {
    let _base = xaml_test_base();
    // Upstream finds the property among the instance properties of the target type
    // (`type.GetAllProperties()`): `ScrollViewer.AllowAutoHide` is an instance property
    // over the attached property the class registers.
    let style = load_as::<Ref<Style>>(&format!(
        "<Style {NS} Selector='ScrollViewer'><Style Selector='^[AllowAutoHide=True]'><Setter Property='Padding' Value='1' /></Style></Style>"
    ));
    assert_eq!(style.children().count(), 1);
}

#[test]
fn g14_property_selectors_for_inherited_and_attached_properties() {
    let _base = xaml_test_base();
    // A registered property of a base class, and the explicit attached form.
    let style = load_as::<Ref<Style>>(&format!(
        "<Style {NS} Selector='ScrollViewer'>
           <Style Selector='^[IsEnabled=False]'><Setter Property='Padding' Value='1' /></Style>
           <Style Selector='^[(ScrollViewer.AllowAutoHide)=True]'><Setter Property='Padding' Value='2' /></Style>
           <Style Selector='^[(Grid.Row)=1]'><Setter Property='Padding' Value='3' /></Style>
         </Style>"
    ));
    assert_eq!(style.children().count(), 3);
}

// --- G15: a property that names an element --------------------------------------

#[test]
fn g15_popup_placement_target_is_resolved_by_name() {
    use ferroui_controls::primitives::Popup;
    use ferroui_controls::Grid;
    let _base = xaml_test_base();
    let grid = load_as::<Ref<Grid>>(&format!(
        "<Grid {NS}><Border Name='Background' /><Popup Name='PART_Popup' PlacementTarget='Background' /></Grid>"
    ));
    let popup = grid.find_control::<Popup>("PART_Popup").expect("the popup");
    let target = popup.placement_target().expect("the placement target");
    assert_eq!(target.name().as_deref(), Some("Background"));
}

// --- G16: a static resource of a theme dictionary, from a merged dictionary -----

#[test]
fn g16_static_resource_in_a_theme_dictionary_uses_the_variant_of_the_dictionary() {
    use ferroui_base::media::Color;
    use ferroui_base::styling::ThemeVariant;
    let _app = styled_window_application();
    let window = load_as::<Ref<Window>>(&format!(
        "<Window {NS}><Window.Resources><ResourceDictionary>
           <ResourceDictionary.MergedDictionaries><ResourceDictionary><ResourceDictionary.ThemeDictionaries>
             <ResourceDictionary x:Key='Default'><Color x:Key='C'>#FFFFFFFF</Color></ResourceDictionary>
             <ResourceDictionary x:Key='Dark'><Color x:Key='C'>#FF000000</Color></ResourceDictionary>
           </ResourceDictionary.ThemeDictionaries></ResourceDictionary></ResourceDictionary.MergedDictionaries>
           <ResourceDictionary.ThemeDictionaries>
             <ResourceDictionary x:Key='Default'><SolidColorBrush x:Key='B' Color='{{StaticResource C}}' /></ResourceDictionary>
             <ResourceDictionary x:Key='Dark'><SolidColorBrush x:Key='B' Color='{{StaticResource C}}' /></ResourceDictionary>
           </ResourceDictionary.ThemeDictionaries>
         </ResourceDictionary></Window.Resources><Border Background='{{DynamicResource B}}' /></Window>"
    ));
    window.show();
    let border = window
        .content()
        .and_then(|content| ferroui_controls::Control::from_boxed(&content))
        .and_then(|control| control.cast::<Border>())
        .expect("the border");
    let color = |border: &Ref<Border>| {
        border.background().and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color()))
    };
    assert_eq!(color(&border), Some(Color::parse("#FFFFFFFF").expect("a color")));
    window.set_requested_theme_variant(Some(ThemeVariant::dark()));
    assert_eq!(color(&border), Some(Color::parse("#FF000000").expect("a color")));
}

// Inside a control template the assignments of registered properties use the setters
// with a priority (the control template priority transformer keeps only the binding and
// the typed value setter), so the unset value of a delayed lookup has no setter there: as
// in the managed original, applying the template fails with the invalid cast of the
// dynamic setter. The failure of a template is raised where the template is applied.
#[test]
#[should_panic(expected = "No setter of property Theme accepts a value of type 'FerroUI.UnsetValueType'")]
fn g08_missing_static_resource_inside_a_control_template_has_no_setter_for_the_unset_value() {
    let _app = styled_window_application();
    let window = load_as::<Ref<Window>>(&format!(
        "<Window {NS}><Window.Template><ControlTemplate><ScrollViewer Theme='{{StaticResource Missing}}' /></ControlTemplate></Window.Template></Window>"
    ));
    window.show();
}

// --- G20: a static property of a type of the runtime library -------------------

#[test]
fn g20_static_date_time_today() {
    use ferroui_base::utilities::DateTime;
    let _base = xaml_test_base();
    let border = load_as::<Ref<Border>>(&format!(
        "<Border {NS} xmlns:sys='using:System' Tag='{{x:Static sys:DateTime.Today}}' />"
    ));
    let tag = border.tag().expect("the tag");
    assert_eq!(tag.downcast_ref::<DateTime>(), Some(&DateTime::today()));
}

// --- G21: a contract of the notifications as the data type of a template --------

#[test]
fn g21_notification_contract_as_data_type() {
    let _base = xaml_test_base();
    resource(&dictionary("<DataTemplate x:Key='t' DataType='INotification'><TextBlock /></DataTemplate>", ""), "t");
}
