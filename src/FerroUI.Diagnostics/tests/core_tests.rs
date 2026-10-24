//! Tests of the core of DevTools (not from upstream: the upstream project has no tests): the
//! options and the hot keys, the key gesture helpers, the converters, the type names, the
//! behaviors, the models and the visual tree helpers that run without a window.

use crate::diagnostics::behaviors::ColumnDefinition as ColumnDefinitionBehavior;
use crate::diagnostics::converters::{
    BoolToImageConverter, BoolToOpacityConverter, BrushSelectorConveter, EnumToCheckedConverter, GetTypeNameConverter,
};
use crate::diagnostics::models::EventChainLink;
use crate::diagnostics::{
    get_type_name_of_full_name, DevTools, DevToolsOptions, DevToolsViewKind, HotKeyConfiguration,
    KeyGestureExtesions, TypeExtesnions, VisualTreeDebug,
};
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingOperations;
use ferroui_base::input::raw::{IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType};
use ferroui_base::input::{
    FocusManager, IInputDevice, IInputRoot, InputElement, Key, KeyDeviceType, KeyGesture, KeyModifiers, PhysicalKey,
    RawInputModifiers,
};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::media::{Brushes, Colors, IBrush, SolidColorBrush};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{BoxedValue, Ref, Size, StaticType};
use ferroui_controls::{Border, ColumnDefinition, Control, GridLength, GridUnitType};
use std::any::Any;
use std::rc::Rc;

fn boxed<T: PartialEq + 'static>(value: T) -> BoxedValue {
    Rc::new(value)
}

fn convert(converter: &dyn IValueConverter, value: Option<BoxedValue>, parameter: Option<BoxedValue>) -> Option<BoxedValue> {
    converter
        .convert(value.as_ref(), ValueType::object(), parameter.as_ref(), &CultureInfo::invariant_culture())
        .expect("the conversion succeeds")
}

fn convert_back(
    converter: &dyn IValueConverter,
    value: Option<BoxedValue>,
    parameter: Option<BoxedValue>,
) -> Option<BoxedValue> {
    converter
        .convert_back(value.as_ref(), ValueType::object(), parameter.as_ref(), &CultureInfo::invariant_culture())
        .expect("the conversion back succeeds")
}

// --- options and hot keys -----------------------------------------------------------------

#[test]
fn the_options_have_the_defaults_of_upstream() {
    let options = DevToolsOptions::new();

    assert_eq!(KeyGesture::from_key(Key::F12), options.gesture());
    assert!(options.show_as_child_window());
    assert_eq!(Size::new(1280.0, 720.0), options.size());
    assert_eq!(None, options.startup_screen_index());
    assert!(options.show_implemented_interfaces());
    assert!(options.theme_variant().is_none());
    assert!(options.focus_highlighter_brush().is_none());
    // Deviation (DEVIATIONS.md, Developer tools): the combined tree, upstream the logical tree.
    assert_eq!(DevToolsViewKind::CombinedTree, options.launch_view());
    // The default screenshot handler is the one of the conventions, shared by all options.
    assert!(Rc::ptr_eq(&options.screenshot_handler(), &DevToolsOptions::new().screenshot_handler()));
}

#[test]
fn the_options_are_set_and_built_in_place() {
    let options = DevToolsOptions::new()
        .with_gesture(KeyGesture::new(Key::F11, KeyModifiers::CONTROL))
        .with_show_as_child_window(false)
        .with_startup_screen_index(Some(1))
        .with_launch_view(DevToolsViewKind::Events);
    options.set_size(Size::new(800.0, 600.0));

    assert_eq!(KeyGesture::new(Key::F11, KeyModifiers::CONTROL), options.gesture());
    assert!(!options.show_as_child_window());
    assert_eq!(Some(1), options.startup_screen_index());
    assert_eq!(DevToolsViewKind::Events, options.launch_view());
    assert_eq!(Size::new(800.0, 600.0), options.size());
}

#[test]
fn the_hot_keys_have_the_defaults_of_upstream() {
    let hot_keys = HotKeyConfiguration::new();

    assert_eq!(KeyGesture::new(Key::S, KeyModifiers::ALT), hot_keys.value_frames_freeze());
    assert_eq!(KeyGesture::new(Key::D, KeyModifiers::ALT), hot_keys.value_frames_unfreeze());
    assert_eq!(
        KeyGesture::new(Key::None, KeyModifiers::SHIFT | KeyModifiers::CONTROL),
        hot_keys.inspect_hovered_control()
    );
    assert_eq!(KeyGesture::new(Key::F, KeyModifiers::ALT | KeyModifiers::CONTROL), hot_keys.toggle_popup_freeze());
    assert_eq!(KeyGesture::from_key(Key::F8), hot_keys.screenshot_selected_control());

    let options = DevToolsOptions::new().with_hot_keys(
        HotKeyConfiguration::new().with_screenshot_selected_control(KeyGesture::from_key(Key::F9)),
    );
    assert_eq!(KeyGesture::from_key(Key::F9), options.hot_keys().screenshot_selected_control());
    assert_eq!(KeyGesture::new(Key::S, KeyModifiers::ALT), options.hot_keys().value_frames_freeze());
}

// --- key gestures -------------------------------------------------------------------------

struct TestDevice;

impl IInputDevice for TestDevice {
    fn process_raw_event(&self, _ev: &dyn IRawInputEventArgs) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct TestRoot;

impl IInputRoot for TestRoot {
    fn focus_manager(&self) -> Option<Rc<FocusManager>> {
        None
    }

    fn pointer_over_element(&self) -> Option<Ref<InputElement>> {
        None
    }

    fn set_pointer_over_element(&self, _value: Option<Ref<InputElement>>) {}

    fn cursor_element(&self) -> Option<Ref<InputElement>> {
        None
    }

    fn set_cursor_element(&self, _value: Option<Ref<InputElement>>) {}

    fn root_element(&self) -> Ref<InputElement> {
        unreachable!("the key gestures do not ask for the root element")
    }

    fn focus_root(&self) -> Ref<InputElement> {
        unreachable!("the key gestures do not ask for the focus root")
    }

    fn pointer_over_invalidated(&self) {}
}

fn raw_key(key: Key, modifiers: RawInputModifiers) -> RawKeyEventArgs {
    RawKeyEventArgs::new(
        Rc::new(TestDevice),
        0,
        Rc::new(TestRoot),
        RawKeyEventType::KeyUp,
        key,
        modifiers,
        PhysicalKey::None,
        None,
        KeyDeviceType::Keyboard,
    )
}

#[test]
fn a_gesture_matches_a_raw_key_with_the_same_key_and_keyboard_modifiers() {
    let gesture = KeyGesture::from_key(Key::F12);

    assert!(gesture.matches_raw(&raw_key(Key::F12, RawInputModifiers::empty())));
    // The mouse buttons are not keyboard modifiers.
    assert!(gesture.matches_raw(&raw_key(Key::F12, RawInputModifiers::LEFT_MOUSE_BUTTON)));
    assert!(!gesture.matches_raw(&raw_key(Key::F12, RawInputModifiers::SHIFT)));
    assert!(!gesture.matches_raw(&raw_key(Key::F11, RawInputModifiers::empty())));

    let with_modifiers = KeyGesture::new(Key::F, KeyModifiers::ALT | KeyModifiers::CONTROL);
    assert!(with_modifiers.matches_raw(&raw_key(Key::F, RawInputModifiers::ALT | RawInputModifiers::CONTROL)));
    assert!(!with_modifiers.matches_raw(&raw_key(Key::F, RawInputModifiers::ALT)));
}

#[test]
fn the_numpad_operation_keys_match_their_regular_counterparts() {
    assert!(KeyGesture::from_key(Key::OemPlus).matches_raw(&raw_key(Key::Add, RawInputModifiers::empty())));
    assert!(KeyGesture::from_key(Key::Subtract).matches_raw(&raw_key(Key::OemMinus, RawInputModifiers::empty())));
    assert!(KeyGesture::from_key(Key::Decimal).matches_raw(&raw_key(Key::OemPeriod, RawInputModifiers::empty())));
    assert!(!KeyGesture::from_key(Key::Add).matches_raw(&raw_key(Key::OemMinus, RawInputModifiers::empty())));
}

// --- converters ---------------------------------------------------------------------------

#[test]
fn bool_to_opacity_converts_true_to_one_and_anything_else_to_the_opacity() {
    let converter = BoolToOpacityConverter::new();
    converter.set_opacity(0.6);

    let opacity = |value: Option<BoxedValue>| *convert(&*converter, value, None).unwrap().downcast_ref::<f64>().unwrap();
    assert_eq!(1.0, opacity(Some(boxed(true))));
    assert_eq!(0.6, opacity(Some(boxed(false))));
    assert_eq!(0.6, opacity(None));
    assert_eq!(0.6, opacity(Some(boxed(1i32))));

    assert!(converter
        .convert_back(Some(&boxed(1.0f64)), ValueType::of::<bool>(), None, &CultureInfo::invariant_culture())
        .is_err());
}

#[test]
fn bool_to_image_converts_a_boolean_to_its_image_and_anything_else_to_null() {
    let converter = BoolToImageConverter::new();

    assert!(convert(&*converter, Some(boxed(true)), None).is_none());
    assert!(convert(&*converter, Some(boxed(false)), None).is_none());
    assert!(convert(&*converter, Some(boxed("true".to_string())), None).is_none());
    assert!(convert(&*converter, None, None).is_none());
    assert!(BindingOperations::is_do_nothing(convert_back(&*converter, Some(boxed(true)), None).as_ref()));
}

#[test]
fn enum_to_checked_compares_the_value_with_the_parameter() {
    let converter = EnumToCheckedConverter::new();
    let checked = |value: Option<BoxedValue>, parameter: Option<BoxedValue>| {
        *convert(&*converter, value, parameter).unwrap().downcast_ref::<bool>().unwrap()
    };

    assert!(checked(Some(boxed(HorizontalAlignment::Left)), Some(boxed(HorizontalAlignment::Left))));
    assert!(!checked(Some(boxed(HorizontalAlignment::Left)), Some(boxed(HorizontalAlignment::Right))));
    assert!(!checked(Some(boxed(HorizontalAlignment::Left)), None));
    assert!(checked(None, None));

    let parameter = boxed(HorizontalAlignment::Center);
    let back = convert_back(&*converter, Some(boxed(true)), Some(parameter.clone())).unwrap();
    assert!(Rc::ptr_eq(&parameter, &back));
    assert!(BindingOperations::is_do_nothing(
        convert_back(&*converter, Some(boxed(false)), Some(parameter)).as_ref()
    ));
}

#[test]
fn get_type_name_converts_a_type_to_its_name() {
    let converter = GetTypeNameConverter::new();

    let name = convert(&*converter, Some(boxed(ValueType::of::<f64>())), None).unwrap();
    assert_eq!(Some(&"Double".to_string()), name.downcast_ref::<String>());

    let name = convert(&*converter, Some(boxed(<Border as StaticType>::TYPE)), None).unwrap();
    assert_eq!(Some(&"Border".to_string()), name.downcast_ref::<String>());

    assert!(BindingOperations::is_do_nothing(convert(&*converter, Some(boxed(1i32)), None).as_ref()));
    assert!(BindingOperations::is_do_nothing(convert(&*converter, None, None).as_ref()));
}

#[test]
fn the_brush_selector_selects_the_same_brush_or_a_solid_brush_of_the_same_color() {
    let highlight: Rc<dyn IBrush> = Brushes::green();
    let converter = BrushSelectorConveter::new();
    converter.set_brush(Some(highlight.clone()));
    let contract = BrushSelectorConveter::as_value_converter(converter.clone());

    let selected = |value: Option<BoxedValue>, parameter: Option<BoxedValue>| {
        convert(&*contract, value, parameter)
            .and_then(|brush| brush.downcast_ref::<Rc<dyn IBrush>>().cloned())
            .is_some_and(|brush| brush.reference_id() == highlight.reference_id())
    };

    let red: Rc<dyn IBrush> = Brushes::red();
    let blue: Rc<dyn IBrush> = Brushes::blue();
    let other_red: Rc<dyn IBrush> = SolidColorBrush::with_color(Colors::RED).into();

    // Two nulls are the same object.
    assert!(selected(None, None));
    assert!(selected(Some(boxed(red.clone())), Some(boxed(red.clone()))));
    assert!(selected(Some(boxed(other_red)), Some(boxed(red.clone()))));
    assert!(!selected(Some(boxed(blue)), Some(boxed(red.clone()))));
    assert!(!selected(Some(boxed(red)), None));

    assert!(BindingOperations::is_do_nothing(convert_back(&*contract, None, None).as_ref()));
}

// --- type names ---------------------------------------------------------------------------

#[test]
fn the_name_of_a_type_is_its_short_name() {
    assert_eq!("Double", ValueType::of::<f64>().get_type_name());
    assert_eq!("String", ValueType::of::<String>().get_type_name());
    assert_eq!("Boolean", ValueType::of::<bool>().get_type_name());

    ValueTypes::register_nullable::<f64>();
    assert_eq!("Double?", ValueType::of::<Option<f64>>().get_type_name());
}

#[test]
fn the_name_of_a_nullable_or_generic_type_is_built_from_its_full_name() {
    assert_eq!("Int32?", get_type_name_of_full_name("System.Nullable`1[System.Int32]"));
    assert_eq!("List<String>", get_type_name_of_full_name("System.Collections.Generic.List`1[System.String]"));
    assert_eq!(
        "Dictionary<String,Int32>",
        get_type_name_of_full_name(
            "System.Collections.Generic.Dictionary`2[[System.String, System.Private.CoreLib],[System.Int32, System.Private.CoreLib]]"
        )
    );
    assert_eq!(
        "List<Int32?>",
        get_type_name_of_full_name("System.Collections.Generic.List`1[System.Nullable`1[System.Int32]]")
    );
    assert_eq!("Thickness", get_type_name_of_full_name("FerroUI.Thickness"));
    assert_eq!("String?", get_type_name_of_full_name("core::option::Option<alloc::string::String>"));
    assert_eq!("Rc<dyn IBrush>", get_type_name_of_full_name("alloc::rc::Rc<dyn ferroui_base::media::i_brush::IBrush>"));
    assert_eq!("Vec<i32,u8>", get_type_name_of_full_name("alloc::vec::Vec<i32, u8>"));
}

// --- behaviors ----------------------------------------------------------------------------

#[test]
fn a_hidden_column_has_no_width_and_gets_it_back_when_it_is_shown() {
    let column = ColumnDefinition::with_width(GridLength::new(100.0, GridUnitType::Pixel));
    assert!(ColumnDefinitionBehavior::get_is_visible(&column));

    ColumnDefinitionBehavior::set_is_visible(&column, false);
    assert!(!ColumnDefinitionBehavior::get_is_visible(&column));
    assert_eq!(GridLength::new(0.0, GridUnitType::Pixel), column.width());

    ColumnDefinitionBehavior::set_is_visible(&column, true);
    assert!(ColumnDefinitionBehavior::get_is_visible(&column));
    assert_eq!(GridLength::new(100.0, GridUnitType::Pixel), column.width());
}

// --- models -------------------------------------------------------------------------------

#[test]
fn a_link_of_an_event_chain_names_its_handler() {
    let border = Border::new();
    let link = EventChainLink::new(Control::boxed(border.clone()), true, RoutingStrategies::BUBBLE);

    assert_eq!("Border", link.handler_name());
    assert!(link.handled());
    assert!(!link.begins_new_route());
    assert_eq!(RoutingStrategies::BUBBLE, link.route());

    border.set_name(Some("target".to_string()));
    assert_eq!("target (Border)", link.handler_name());

    link.set_handled(false);
    link.set_begins_new_route(true);
    assert!(!link.handled());
    assert!(link.begins_new_route());
}

// --- visual tree helpers ------------------------------------------------------------------

#[test]
fn the_visual_tree_is_printed_with_the_values_of_the_properties() {
    let parent = Border::new();
    let child = Border::new();
    child.set_width(10.0);
    parent.set_child(child.clone());

    let text = VisualTreeDebug::print_visual_tree(&parent);
    let lines: Vec<&str> = text.lines().collect();

    assert!(lines[0].starts_with("Border "), "{text}");
    assert!(lines.iter().any(|line| line.starts_with(" +- Border ")), "{text}");
    assert!(
        lines.iter().any(|line| line.starts_with("     |  Width = ") && line.ends_with(" [LocalValue]")),
        "{text}"
    );
    // A property without a value is not printed.
    assert!(!lines.iter().any(|line| line.contains("|  Height = ")), "{text}");
}

#[test]
fn a_visual_in_no_top_level_belongs_to_dev_tools_as_upstream() {
    let border = Border::new();
    assert!(DevTools::does_belong_to_dev_tool(&border));
}
