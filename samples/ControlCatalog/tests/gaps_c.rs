//! Minimal reproductions of gaps of the framework found while porting a
//! group of the sample (see `gaps.rs`).

#[allow(unused_imports)]
use super::support::*;

#[allow(dead_code)]
const XMLNS: &str = "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

use crate::markup::try_load_text;
use crate::pages::ToolTipPage;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::Ref;
use ferroui_controls::{AutoCompleteBox, Button, ComboBox, Control, Panel, SelectableTextBlock, Slider, TextBlock};

/// The first child of a loaded panel, as a `T`.
#[allow(dead_code)]
fn child<T: ferroui_base::ObjectType>(panel: &Ref<Panel>, index: usize) -> Ref<T> {
    panel.children().get(index).cast::<T>().expect("the child is of the expected class")
}

#[test]
#[ignore = "gap C301: a method name is not accepted for a property of a delegate type (CustomPopupPlacementCallback)"]
fn gap_c301_method_name_for_a_delegate_property() {
    let _app = start_application();
    let xaml = format!(
        "<ContentPage {XMLNS} x:Class='ControlCatalog.Pages.ToolTipPage'>\
           <Border ToolTip.Placement='Custom' ToolTip.CustomPopupPlacementCallback='CustomPlacementCallback' ToolTip.Tip='tip'/>\
         </ContentPage>"
    );
    let page = (ToolTipPage::XAML_CLASS.create_uninitialized)();
    let loaded = try_load_text(&xaml, None, Some(page.clone())).unwrap_or_else(|e| panic!("{}", crate::markup::describe(&e)));
    let page = from_markup_value::<Ref<ToolTipPage>>(&Some(loaded)).expect("the page");
    let border = Control::from_boxed(&page.content().expect("the content")).expect("a control");
    assert!(ferroui_controls::ToolTip::get_custom_popup_placement_callback(&border).is_some());
}

#[test]
fn gap_c302_data_validation_errors_error_property() {
    let _app = start_application();
    let button = from_markup_value::<Ref<Button>>(&Some(load_text(&format!(
        "<Button {XMLNS} xmlns:sys='using:System'>\
           <DataValidationErrors.Error><sys:Exception/></DataValidationErrors.Error>\
         </Button>"
    ))))
    .expect("a button");
    assert!(ferroui_controls::DataValidationErrors::get_has_errors(&button));
}

#[test]
fn gap_c303_system_exception_element() {
    let _app = start_application();
    let panel = from_markup_value::<Ref<Panel>>(&Some(load_text(&format!(
        "<Panel {XMLNS} xmlns:sys='using:System'><Panel.Resources><sys:Exception x:Key='e'/></Panel.Resources></Panel>"
    ))))
    .expect("a panel");
    assert!(panel.resources().contains_key(&"e".into()));
}

#[test]
fn gap_c304_resolve_by_name_extension_for_an_element_reference() {
    let _app = start_application();
    let panel = from_markup_value::<Ref<Panel>>(&Some(load_text(&format!(
        "<Panel {XMLNS}><Canvas Name='c'/><Popup PlacementTarget='{{ResolveByName c}}'/></Panel>"
    ))))
    .expect("a panel");
    let canvas = child::<Control>(&panel, 0);
    let popup = child::<ferroui_controls::primitives::Popup>(&panel, 1);
    assert!(popup.placement_target().is_some_and(|target| target == canvas));
}

#[test]
#[ignore = "gap C305: System.Collections.Generic.List`1 with x:TypeArguments is not a type of the markup type system"]
fn gap_c305_generic_list_element() {
    let _app = start_application();
    let combo_box = from_markup_value::<Ref<ComboBox>>(&Some(load_text(&format!(
        "<ComboBox {XMLNS} xmlns:generic='clr-namespace:System.Collections.Generic;assembly=netstandard'>\
           <ComboBox.ItemsSource>\
             <generic:List x:TypeArguments='XYFocusNavigationModes'>\
               <XYFocusNavigationModes>Enabled</XYFocusNavigationModes>\
               <XYFocusNavigationModes>Disabled</XYFocusNavigationModes>\
             </generic:List>\
           </ComboBox.ItemsSource>\
         </ComboBox>"
    ))))
    .expect("a combo box");
    assert_eq!(combo_box.items_source().map(|items| items.count()), Some(2));
}

#[test]
#[ignore = "gap C306: Slider.Ticks is not converted from text"]
fn gap_c306_slider_ticks_from_text() {
    let _app = start_application();
    let slider = from_markup_value::<Ref<Slider>>(&Some(load_text(&format!(
        "<Slider {XMLNS} Ticks='0,20,25,40,75,100'/>"
    ))))
    .expect("a slider");
    assert_eq!(slider.ticks().map(|ticks| ticks.count()), Some(6));
}

#[test]
fn gap_c307_selectable_text_block_text_block_properties() {
    let _app = start_application();
    let text_block = from_markup_value::<Ref<SelectableTextBlock>>(&Some(load_text(&format!(
        "<SelectableTextBlock {XMLNS} TextWrapping='Wrap' TextAlignment='Right'/>"
    ))));
    assert!(text_block.is_some());
}

#[test]
fn gap_c308_text_element_properties_of_inlines() {
    let _app = start_application();
    let text_block = from_markup_value::<Ref<TextBlock>>(&Some(load_text(&format!(
        "<TextBlock {XMLNS}>\
           <Span FontWeight='Bold' FontStyle='Italic' Foreground='Maroon' TextDecorations='Underline'>a</Span>\
           <Run Text='b' FontSize='20' FontFamily='Arial' FontFeatures='+c2sc, +smcp'/>\
         </TextBlock>"
    ))))
    .expect("a text block");
    assert_eq!(text_block.inlines().map(|inlines| inlines.count()), Some(2));
}

#[test]
#[ignore = "gap C309: System.Collections.ArrayList is not a type of the markup type system"]
fn gap_c309_array_list_element() {
    let _app = start_application();
    let combo_box = from_markup_value::<Ref<ComboBox>>(&Some(load_text(&format!(
        "<ComboBox {XMLNS} xmlns:collections='using:System.Collections'>\
           <ComboBox.ItemsSource>\
             <collections:ArrayList><Stretch>Uniform</Stretch><Stretch>Fill</Stretch></collections:ArrayList>\
           </ComboBox.ItemsSource>\
         </ComboBox>"
    ))))
    .expect("a combo box");
    assert_eq!(combo_box.items_source().map(|items| items.count()), Some(2));
}

#[test]
#[ignore = "gap C310: AutoCompleteBox.MinimumPopulateDelay (TimeSpan) is not converted from text"]
fn gap_c310_time_span_property_from_text() {
    let _app = start_application();
    let auto_complete_box = from_markup_value::<Ref<AutoCompleteBox>>(&Some(load_text(&format!(
        "<AutoCompleteBox {XMLNS} MinimumPopulateDelay='00:00:01'/>"
    ))))
    .expect("an auto complete box");
    assert_eq!(auto_complete_box.minimum_populate_delay(), std::time::Duration::from_secs(1));
}

#[test]
#[ignore = "gap C312: the event FerroObject.PropertyChanged is not declared for markup"]
fn gap_c312_property_changed_event_in_markup() {
    let _app = start_application();
    // As `Pages/OpenGl/OpenGlInteropPage.xaml`: `<openGl:GlPageKnobs PropertyChanged="KnobsPropertyChanged"/>`.
    let markup = ferroui_base::FerroObject::TYPE.markup().expect("the markup metadata of the class");
    assert!(markup.find_event("PropertyChanged").is_some());
}

#[test]
fn gap_c311_binding_assigned_to_value_member_binding() {
    let _app = start_application();
    // As `Pages/AutoCompleteBoxPage.xaml`: a compiled binding needs the data type of its source.
    let auto_complete_box = from_markup_value::<Ref<AutoCompleteBox>>(&Some(load_text(&format!(
        "<AutoCompleteBox {XMLNS} xmlns:models='using:ControlCatalog.Models' \
           ValueMemberBinding='{{Binding Capital, x:DataType=models:StateData}}'/>"
    ))))
    .expect("an auto complete box");
    assert!(auto_complete_box.value_member_binding().is_some());
}
