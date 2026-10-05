//! Minimal reproductions of what keeps documents of the Fluent theme from
//! loading, beyond the gaps the Simple theme found (`G1`..`G12`, reproduced
//! in the tests of that crate): one test per gap, each the smallest markup
//! that shows it. A test is ignored with the gap it reproduces and passes
//! once the gap is closed (`cargo test -p ferroui-themes-fluent -- --ignored gap_`).

use super::support::*;

const XMLNS: &str = "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

#[test]
fn gap_g13_transform_operations_from_text() {
    let _app = start_application();
    let value = load_text(&format!(
        "<ResourceDictionary {XMLNS}><TransformOperations x:Key='t'>scaleX(0.125) translateX(-2px)</TransformOperations></ResourceDictionary>"
    ));
    assert_eq!(1, realise(&value).resources);
}

#[test]
fn gap_g14_attached_property_of_the_target_type_in_a_property_selector() {
    let _app = start_application();
    load_text(&format!(
        "<Style {XMLNS} Selector='ScrollViewer'><Style Selector='^[AllowAutoHide=True]'><Setter Property='Padding' Value='1' /></Style></Style>"
    ));
}

#[test]
fn gap_g15_popup_placement_target_by_name() {
    let _app = start_application();
    load_text(&format!(
        "<Grid {XMLNS}><Border Name='Background' /><Popup Name='PART_Popup' PlacementTarget='Background' /></Grid>"
    ));
}

#[test]
fn gap_g16_static_resource_in_a_theme_dictionary_uses_the_variant_of_the_dictionary() {
    use ferroui_base::metadata::from_markup_value;
    use ferroui_base::styling::ThemeVariant;
    use ferroui_base::Ref;
    use ferroui_controls::{Border, Control, Window};

    let _app = start_application();
    // The colours are in the theme dictionaries of a merged dictionary, the brushes that name
    // them in the theme dictionaries of the including one (the layout of the Fluent theme:
    // `Accents/BaseColorsPalette.xaml` and `Accents/FluentControlResources.xaml`).
    let window = load_text(&format!(
        "<Window {XMLNS}><Window.Resources><ResourceDictionary>
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
    let window = from_markup_value::<Ref<Window>>(&Some(window)).expect("a window");
    window.show();
    let border = window.content().and_then(|content| Control::from_boxed(&content)).and_then(|c| c.cast::<Border>());
    let border = border.expect("the border");
    let background = || border.background().and_then(|b| b.as_solid_color_brush().map(|s| s.color())).expect("a brush");

    assert_eq!((255, 255, 255), (background().r, background().g, background().b));
    window.set_requested_theme_variant(Some(ThemeVariant::dark()));
    assert_eq!((0, 0, 0), (background().r, background().g, background().b));
}

#[test]
fn gap_g19_null_as_setter_value() {
    use ferroui_base::metadata::from_markup_value;
    use ferroui_base::Ref;
    use ferroui_controls::{Border, Control, Window};

    let _app = start_application();
    // Upstream: null is a valid value of a property of a reference type; the setter sets it.
    let window = load_text(&format!(
        "<Window {XMLNS}><Window.Styles><Style Selector='Border'><Setter Property='Background' Value='{{x:Null}}' /></Style></Window.Styles><Border Background='Red' /></Window>"
    ));
    let window = from_markup_value::<Ref<Window>>(&Some(window)).expect("a window");
    window.show();
    let border = window.content().and_then(|content| Control::from_boxed(&content)).and_then(|c| c.cast::<Border>());
    // The local value wins over the style; the point is that applying the style does not fail.
    assert!(border.expect("the border").background().is_some());
}
