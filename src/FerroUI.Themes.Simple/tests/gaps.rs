//! Minimal reproductions of what keeps documents of the theme from
//! loading, or kept them from loading: one test per gap, each the smallest
//! markup that shows it. The test of a gap that is still open is ignored
//! with the gap it reproduces and passes once the gap is closed
//! (`cargo test -p ferroui-themes-simple -- --ignored gap_`); the others
//! stay as regression tests.

use super::support::*;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::Ref;
use ferroui_controls::Window;

const XMLNS: &str = "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";
const CONVERTERS: &str = "xmlns:converters='using:FerroUI.Controls.Converters'";

fn show(xaml: &str) -> Ref<Window> {
    let window = from_markup_value::<Ref<Window>>(&Some(load_text(xaml))).expect("a window");
    window.show();
    window
}

#[test]
fn gap_g01_border_gap_mask_converter() {
    let _app = start_application();
    load_text(&format!("<ResourceDictionary {XMLNS} {CONVERTERS}><converters:BorderGapMaskConverter x:Key='c' /></ResourceDictionary>"));
}

#[test]
fn gap_g01_string_format_converter() {
    let _app = start_application();
    load_text(&format!("<ResourceDictionary {XMLNS} {CONVERTERS}><converters:StringFormatConverter x:Key='c' /></ResourceDictionary>"));
}

#[test]
fn gap_g01_platform_key_gesture_converter() {
    let _app = start_application();
    load_text(&format!("<ResourceDictionary {XMLNS} {CONVERTERS}><converters:PlatformKeyGestureConverter x:Key='c' /></ResourceDictionary>"));
}

#[test]
fn gap_g01_margin_multiplier_converter() {
    let _app = start_application();
    load_text(&format!(
        "<ResourceDictionary {XMLNS} {CONVERTERS}><converters:MarginMultiplierConverter x:Key='c' Left='True' Top='True' Right='True' Bottom='True' Indent='1' /></ResourceDictionary>"
    ));
}

#[test]
fn gap_g01_corner_radius_filter_converter() {
    let _app = start_application();
    load_text(&format!(
        "<ResourceDictionary {XMLNS} {CONVERTERS}><converters:CornerRadiusFilterConverter x:Key='c' Filter='TopLeft, TopRight' /></ResourceDictionary>"
    ));
}

#[test]
fn gap_g01_menu_scrolling_visibility_converter_instance() {
    let _app = start_application();
    load_text(&format!(
        "<Border {XMLNS} {CONVERTERS}><Border.IsVisible><MultiBinding Converter='{{x:Static converters:MenuScrollingVisibilityConverter.Instance}}' ConverterParameter='0'><Binding Path='Tag' /></MultiBinding></Border.IsVisible></Border>"
    ));
}

#[test]
fn gap_g01_tree_view_item_indent_converter_instance() {
    let _app = start_application();
    load_text(&format!(
        "<Border {XMLNS} {CONVERTERS}><Border.Margin><MultiBinding Converter='{{x:Static converters:TreeViewItemIndentConverter.Instance}}'><Binding Path='Tag' /></MultiBinding></Border.Margin></Border>"
    ));
}

#[test]
fn gap_g02_automation_properties() {
    let _app = start_application();
    load_text(&format!("<Border {XMLNS} AutomationProperties.Name='Thumb' AutomationProperties.AutomationId='thumb' />"));
}

#[test]
fn gap_g02_automation_properties_setter() {
    let _app = start_application();
    load_text(&format!(
        "<Style {XMLNS} Selector='Border'><Setter Property='AutomationProperties.AutomationId' Value='PageUp' /></Style>"
    ));
}

#[test]
fn gap_g03_stream_geometry_from_text() {
    let _app = start_application();
    let value = load_text(&format!(
        "<ResourceDictionary {XMLNS}><StreamGeometry x:Key='g'>M 0,0 L 10,10 Z</StreamGeometry></ResourceDictionary>"
    ));
    assert_eq!(1, realise(&value).resources);
}

#[test]
fn gap_g04_flow_direction_attribute() {
    let _app = start_application();
    load_text(&format!("<Border {XMLNS} FlowDirection='LeftToRight' />"));
}

#[test]
fn gap_g05_cross_fade() {
    let _app = start_application();
    load_text(&format!(
        "<Style {XMLNS} Selector='Expander'><Setter Property='ContentTransition'><Setter.Value><CrossFade Duration='00:00:00.25' /></Setter.Value></Setter></Style>"
    ));
}

#[test]
fn gap_g05_page_slide_with_spline_easing() {
    let _app = start_application();
    load_text(&format!(
        "<Style {XMLNS} Selector='TransitioningContentControl'><Setter Property='PageTransition'><Setter.Value><PageSlide Duration='0:0:0.3' Orientation='Horizontal' FillMode='Forward'><PageSlide.SlideInEasing><SplineEasing X1='0.16' Y1='1' X2='0.3' Y2='1' /></PageSlide.SlideInEasing></PageSlide></Setter.Value></Setter></Style>"
    ));
}

#[test]
fn gap_g06_shared_size_group() {
    let _app = start_application();
    load_text(&format!(
        "<Grid {XMLNS} Grid.IsSharedSizeScope='True'><Grid.ColumnDefinitions><ColumnDefinition Width='Auto' SharedSizeGroup='A' /><ColumnDefinition Width='*' /></Grid.ColumnDefinitions></Grid>"
    ));
}

#[test]
fn gap_g07_template_as_setter_value() {
    let _app = start_application();
    show(&format!(
        "<Window {XMLNS}><Window.Styles><Style Selector='GridSplitter'><Setter Property='PreviewContent'><Template><Rectangle /></Template></Setter></Style></Window.Styles><GridSplitter /></Window>"
    ));
}

// G8. A static resource that is not found, on a registered property of a control, is
// delayed upstream: the extension returns the unset value and repeats the lookup when the
// control is initialised (`StaticResourceExtension.ProvideValue`); the unset value setter of
// a registered property takes it as "leave unset". Nothing fails.
#[test]
fn gap_g08_missing_static_resource_on_a_control_is_delayed() {
    use ferroui_base::StyledElement;
    use ferroui_controls::ScrollViewer;

    let _app = start_application();
    let scroll_viewer = from_markup_value::<Ref<ScrollViewer>>(&Some(load_text(&format!(
        "<ScrollViewer {XMLNS} Theme='{{StaticResource Missing}}' />"
    ))))
    .expect("a scroll viewer");
    let element: &StyledElement = &scroll_viewer;
    assert!(!element.is_set(StyledElement::theme_property().as_property()));
}

// G8, inside a control template. Upstream's control template priority transformer keeps,
// for an assignment of a registered property inside a control template, only the setters
// that take a binding priority (the typed value and the binding): the unset value setter
// is not among them. The unset value of a delayed lookup therefore has no setter there
// and, as in the managed original, applying the template fails with the invalid cast of
// the dynamic setter. A theme must define the resources its templates name.
#[test]
#[should_panic(expected = "No setter of property Theme accepts a value of type 'FerroUI.UnsetValueType'")]
fn gap_g08_missing_static_resource_for_theme_in_template() {
    let _app = start_application();
    show(&format!(
        "<Window {XMLNS}><Window.Template><ControlTemplate><ScrollViewer Theme='{{StaticResource Missing}}' /></ControlTemplate></Window.Template></Window>"
    ));
}

#[test]
fn gap_g09_transform_none_as_setter_value() {
    let _app = start_application();
    load_text(&format!("<Style {XMLNS} Selector='Button'><Setter Property='RenderTransform' Value='none' /></Style>"));
}

#[test]
fn gap_g10_system_date_time() {
    let _app = start_application();
    load_text(&format!(
        "<ResourceDictionary {XMLNS} xmlns:sys='using:System'><DataTemplate x:Key='t' DataType='sys:DateTime'><TextBlock /></DataTemplate></ResourceDictionary>"
    ));
}

#[test]
#[ignore = "gap G20: the static member System.DateTime.Today is not known to markup"]
fn gap_g20_system_date_time_today() {
    let _app = start_application();
    // `Controls/CalendarDatePicker.xaml` (both themes): the day number of the calendar glyph.
    load_text(&format!(
        "<TextBlock {XMLNS} xmlns:sys='using:System' Text='{{Binding Source={{x:Static sys:DateTime.Today}}, Path=Day}}' />"
    ));
}

#[test]
#[ignore = "gap G21: the contract INotification has no markup metadata"]
fn gap_g21_notification_contract_as_data_type() {
    let _app = start_application();
    // `Controls/WindowNotificationManager.xaml` (both themes): the data template of the items panel of its template.
    load_text(&format!(
        "<ResourceDictionary {XMLNS}><DataTemplate x:Key='t' DataType='INotification'><TextBlock /></DataTemplate></ResourceDictionary>"
    ));
}
