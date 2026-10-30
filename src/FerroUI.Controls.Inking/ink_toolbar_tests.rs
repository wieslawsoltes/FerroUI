use super::*;
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::Flyout;
use ferroui_controls::automation::provider::{
    IExpandCollapseProvider, IRangeValueProvider, ISelectionItemProvider, IValueProvider,
};
use crate::test_support::test_scope;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, IBrush};
use ferroui_base::Point;

fn toolbar_window_services() -> ferroui_controls::testing::UnitTestApplicationScope {
    use ferroui_controls::testing::{TestServices, UnitTestApplication};
    use ferroui_base::input::{IKeyboardDevice, KeyboardDevice};
    UnitTestApplication::start(
        TestServices::styled_window()
            .with_keyboard_device(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>)),
    )
}

fn toolbar_window(toolbar: &InkToolbar) -> Ref<ferroui_controls::Window> {
    let window = ferroui_controls::Window::new();
    window.set_content(Some(Control::boxed(toolbar.to_ref())));
    window.show();
    window
}

#[test]
fn localized_names_update_from_resources_without_overwriting_local_values() {
    use ferroui_controls::automation::AutomationProperties;
    use ferroui_base::controls::ResourceKey;
    use ferroui_base::utilities::CultureInfo;
    let _scope = test_scope();
    struct RestoreCulture(CultureInfo);
    impl Drop for RestoreCulture {
        fn drop(&mut self) {
            CultureInfo::set_current_ui_culture(self.0.clone());
        }
    }
    let _culture = RestoreCulture(CultureInfo::current_ui_culture());
    CultureInfo::set_current_ui_culture(CultureInfo::get_culture_info("pl-PL"));
    let pen = InkToolbarBallpointPenButton::new();
    assert_eq!(
        AutomationProperties::get_name(&pen).as_deref(),
        Some("Pióro kulkowe")
    );
    pen.resources()
        .add_value("InkToolbarBallpointPenButtonName", "Drawing pen".to_owned());
    assert_eq!(
        AutomationProperties::get_name(&pen).as_deref(),
        Some("Drawing pen")
    );
    pen.resources().set(
        "InkToolbarBallpointPenButtonName",
        Some(Rc::new("Updated pen".to_owned())),
    );
    assert_eq!(
        AutomationProperties::get_name(&pen).as_deref(),
        Some("Updated pen")
    );
    AutomationProperties::set_name(&pen, Some("Application name"));
    pen.resources()
        .remove(&ResourceKey::from("InkToolbarBallpointPenButtonName"));
    assert_eq!(
        AutomationProperties::get_name(&pen).as_deref(),
        Some("Application name")
    );
}

#[test]
fn configuration_headings_slider_and_palette_use_live_localized_resources() {
    use ferroui_controls::automation::AutomationProperties;
    use crate::test_support::TestRoot;
    use ferroui_controls::{TextBlock, ToolTip, WrapPanel};
    let _scope = test_scope();
    let host = StackPanel::new();
    for (key, text) in [
        ("InkToolbarPenConfigurationColorsLabel", "Ink colors"),
        ("InkToolbarPenConfigurationSizeLabel", "Stroke size"),
        ("InkToolbarPenConfigurationSizeSliderName", "Nib size"),
        ("InkToolbarColorNameBlack", "Night"),
        ("InkToolbarColorPaletteHelpText", "Choose an ink color"),
    ] {
        host.resources().add_value(key, text.to_owned());
    }
    let toolbar = InkToolbar::new();
    let pen = toolbar
        .active_tool()
        .unwrap()
        .cast::<InkToolbarPenButton>()
        .unwrap();
    let configuration = InkToolbarPenConfigurationControl::for_pen(&pen);
    host.children().add(&toolbar);
    host.children().add(&configuration);
    let _root = TestRoot::with_child(&host);
    let headings: Vec<_> = configuration
        .children()
        .snapshot()
        .iter()
        .filter_map(|child| child.cast::<TextBlock>())
        .collect();
    assert_eq!(headings[0].text().as_deref(), Some("Ink colors"));
    assert_eq!(headings[1].text().as_deref(), Some("Stroke size"));
    assert_eq!(AutomationProperties::get_heading_level(&headings[0]), 2);
    let slider = configuration.stroke_width_slider().unwrap();
    assert_eq!(
        AutomationProperties::get_name(&slider).as_deref(),
        Some("Nib size")
    );
    let palette = configuration
        .children()
        .snapshot()
        .iter()
        .find_map(|child| child.cast::<WrapPanel>())
        .unwrap();
    let first = palette.children().get(0);
    assert_eq!(
        AutomationProperties::get_name(&first).as_deref(),
        Some("Night")
    );
    assert_eq!(
        ToolTip::get_tip(&first)
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "Night"
    );
    let peer = ControlAutomationPeer::create_peer_for_element(&pen);
    assert!(peer.get_help_text().contains("Night"));
    assert!(peer.get_help_text().contains("Choose an ink color"));
    host.resources().set(
        "InkToolbarColorNameBlack",
        Some(Rc::new("Midnight".to_owned())),
    );
    assert_eq!(
        AutomationProperties::get_name(&palette.children().get(0)).as_deref(),
        Some("Midnight")
    );
    assert!(peer.get_help_text().contains("Midnight"));
}

#[test]
fn color_names_quantize_known_families_and_format_unknown_rgb() {
    use super::color_names::{color_name, color_resource_key};
    let _scope = test_scope();
    let control = Control::new();
    control
        .resources()
        .add_value("InkToolbarColorNameRed", "Localized red".to_owned());
    assert_eq!(
        color_resource_key(Color::from_rgb(0xe0, 0x18, 0x1f)),
        Some("InkToolbarColorNameRed")
    );
    assert_eq!(
        color_name(&control, Color::from_rgb(0xe0, 0x18, 0x1f)),
        "Localized red"
    );
    control.resources().add_value(
        "InkToolbarColorRgbFormat",
        "red=%1!u!; green=%2!u!; blue=%3!u!".to_owned(),
    );
    assert_eq!(
        color_name(&control, Color::from_rgb(17, 49, 123)),
        "red=17; green=49; blue=123"
    );
}

#[test]
fn palette_descriptions_cover_known_nearby_rgb_and_gradient_brushes() {
    use ferroui_controls::automation::AutomationProperties;
    use ferroui_controls::{ToolTip, WrapPanel};
    use ferroui_base::collections::FerroList;
    use ferroui_base::media::immutable::ImmutableLinearGradientBrush;
    let _app = toolbar_window_services();
    let toolbar = InkToolbar::new();
    toolbar.set_initial_controls(InkToolbarInitialControls::PensOnly);
    let window = toolbar_window(&toolbar);
    let pen = toolbar
        .get_tool_button(InkToolbarTool::BallpointPen)
        .unwrap()
        .cast::<InkToolbarPenButton>()
        .unwrap();
    let mut brushes: Vec<Rc<dyn IBrush>> = [
        (0, 0, 0),
        (255, 255, 255),
        (1, 2, 3),
        (18, 52, 86),
        (68, 200, 245),
        (236, 0, 140),
    ]
    .into_iter()
    .map(|(r, g, b)| {
        Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(r, g, b))) as Rc<dyn IBrush>
    })
    .collect();
    brushes.push(Rc::new(ImmutableLinearGradientBrush::from_stops(&[])));
    pen.set_palette(FerroList::from_items(brushes));
    pen.set_selected_brush_index(0);
    ControlAutomationPeer::create_peer_for_element(&pen)
        .get_provider::<dyn IExpandCollapseProvider>()
        .unwrap()
        .expand()
        .unwrap();
    let flyout = pen.flyout().unwrap().cast::<Flyout>().unwrap();
    assert!(flyout.is_open());
    let content = flyout.content().unwrap();
    let configuration = content
        .downcast_ref::<Ref<Control>>()
        .unwrap()
        .cast::<InkToolbarPenConfigurationControl>()
        .unwrap();
    let palette = configuration
        .children()
        .snapshot()
        .iter()
        .find_map(|child| child.cast::<WrapPanel>())
        .unwrap();
    let expected = [
        "Black",
        "White",
        "Black",
        "RGB 18, 52, 86",
        "Light blue",
        "Pink",
        "Custom color",
    ];
    assert_eq!(palette.children().count(), expected.len());
    for (index, expected) in expected.iter().enumerate() {
        let item = palette.children().get(index);
        assert_eq!(
            AutomationProperties::get_name(&item).as_deref(),
            Some(*expected)
        );
        assert_eq!(
            ToolTip::get_tip(&item)
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            expected,
        );
    }
    pen.set_selected_brush_index(1);
    assert_eq!(
        AutomationProperties::get_help_text(&pen).as_deref(),
        Some("White")
    );
    pen.set_selected_brush_index(3);
    assert!(AutomationProperties::get_help_text(&pen).is_none_or(|text| text.is_empty()));
    pen.set_selected_brush_index(0);
    pen.set_selected_brush_index(6);
    assert!(AutomationProperties::get_help_text(&pen).is_none_or(|text| text.is_empty()));
    flyout.hide();
    window.close();
}

#[test]
fn high_contrast_adjustment_obeys_original_system_and_seven_to_one_policy() {
    use super::color_names::{adjust_high_contrast_color, contrast_ratio};
    let black = Color::from_rgb(0, 0, 0);
    let white = Color::from_rgb(255, 255, 255);
    let gray = Color::from_rgb(128, 128, 128);
    assert!((contrast_ratio(black, white) - 21.0).abs() < 1e-12);
    assert!((contrast_ratio(gray, gray) - 1.0).abs() < 1e-12);
    assert_eq!(
        adjust_high_contrast_color(
            gray,
            white,
            black,
            InkHighContrastAdjustment::UseOriginalColors
        ),
        gray
    );
    assert_eq!(
        adjust_high_contrast_color(
            gray,
            white,
            black,
            InkHighContrastAdjustment::UseSystemColors
        ),
        black
    );
    assert_eq!(
        adjust_high_contrast_color(
            gray,
            white,
            black,
            InkHighContrastAdjustment::UseSystemColorsWhenNecessary
        ),
        black
    );
    assert_eq!(
        adjust_high_contrast_color(
            black,
            white,
            gray,
            InkHighContrastAdjustment::UseSystemColorsWhenNecessary
        ),
        black
    );
}

#[test]
fn contrast_palette_switch_restores_normal_selection_and_tracks_presenter_policy() {
    use ferroui_controls::ThemeVariantScope;
    use ferroui_base::styling::ThemeVariant;
    let _scope = test_scope();
    let scope = ThemeVariantScope::new();
    let toolbar = InkToolbar::new();
    let presenter = InkPresenter::new();
    toolbar.set_target_ink_presenter(Some(presenter.clone()));
    scope.set_child(&toolbar);
    let pen = toolbar
        .active_tool()
        .unwrap()
        .cast::<InkToolbarPenButton>()
        .unwrap();
    pen.set_selected_brush_index(7);
    let normal_color = pen.create_ink_drawing_attributes().unwrap().color;
    scope
        .resources()
        .add_value("SystemColorWindowColor", Color::from_rgb(0, 0, 0));
    scope
        .resources()
        .add_value("SystemColorWindowTextColor", Color::from_rgb(255, 255, 255));
    scope.set_requested_theme_variant(Some(ThemeVariant::new(
        "HighContrast",
        Some(ThemeVariant::dark()),
    )));
    assert!(pen.effective_palette().len() < pen.palette().count());
    assert!(pen
        .effective_palette()
        .iter()
        .all(|brush| super::color_names::contrast_ratio(
            brush.as_solid_color_brush().unwrap().color(),
            Color::from_rgb(0, 0, 0)
        ) >= 7.0));
    presenter.set_high_contrast_adjustment(InkHighContrastAdjustment::UseSystemColors);
    assert_eq!(pen.effective_palette().len(), 1);
    presenter.set_high_contrast_adjustment(InkHighContrastAdjustment::UseOriginalColors);
    assert_eq!(pen.effective_palette().len(), 30);
    scope.set_requested_theme_variant(Some(ThemeVariant::light()));
    assert_eq!(pen.effective_palette().len(), 30);
    assert_eq!(pen.selected_brush_index(), 7);
    assert_eq!(
        pen.create_ink_drawing_attributes().unwrap().color,
        normal_color
    );
}

#[test]
fn pen_preview_uses_source_path_and_follows_attributes() {
    use ferroui_base::media::PolylineGeometry;
    let _scope = test_scope();
    let pen = InkToolbarBallpointPenButton::new();
    let config = InkToolbarPenConfigurationControl::for_pen(&pen);
    let preview = config.stroke_preview().unwrap();
    let path = preview.data().unwrap().cast::<PolylineGeometry>().unwrap();
    assert_eq!(path.points().len(), 103);
    pen.set_selected_stroke_width(9.0);
    pen.set_selected_brush_index(7);
    assert_eq!(preview.stroke_thickness(), 9.0);
    assert_eq!(
        preview
            .stroke()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color(),
        pen.create_ink_drawing_attributes().unwrap().color
    );
    assert_eq!(preview.opacity(), 1.0);
    let highlighter = InkToolbarHighlighterButton::new();
    let highlighter_config = InkToolbarPenConfigurationControl::for_pen(&highlighter);
    let preview = highlighter_config.stroke_preview().unwrap();
    assert_eq!(preview.opacity(), 0.5);
    assert_eq!(preview.stroke_thickness(), 16.0 / 3.0);
}

#[test]
fn toolbar_style_key_tracks_available_theme_and_returns_to_native_fallback() {
    use ferroui_controls::RadioButton;
    use ferroui_base::controls::ResourceKey;
    use ferroui_base::styling::ControlTheme;
    let _scope = test_scope();
    let pen = InkToolbarBallpointPenButton::new();
    let _root = crate::test_support::TestRoot::with_child(&pen);
    assert!(std::ptr::eq(pen.style_key(), RadioButton::TYPE));
    let inherited = ControlTheme::with_target_type(InkToolbarPenButton::TYPE);
    pen.resources()
        .add_value(ResourceKey::Type(InkToolbarPenButton::TYPE), inherited);
    assert!(std::ptr::eq(pen.style_key(), InkToolbarPenButton::TYPE));
    let specific = ControlTheme::with_target_type(InkToolbarBallpointPenButton::TYPE);
    pen.resources().add_value(
        ResourceKey::Type(InkToolbarBallpointPenButton::TYPE),
        specific,
    );
    assert!(std::ptr::eq(
        pen.style_key(),
        InkToolbarBallpointPenButton::TYPE
    ));
    pen.resources()
        .remove(&ResourceKey::Type(InkToolbarBallpointPenButton::TYPE));
    pen.resources()
        .remove(&ResourceKey::Type(InkToolbarPenButton::TYPE));
    assert!(std::ptr::eq(pen.style_key(), RadioButton::TYPE));
}

#[test]
fn palette_arrow_keys_follow_six_column_layout_and_tab_keeps_selection() {
    use crate::test_support::TestRoot;
    use ferroui_controls::WrapPanel;
    use ferroui_base::input::{InputElement, Key, KeyEventArgs};
    let _app = toolbar_window_services();
    let pen = InkToolbarBallpointPenButton::new();
    let config = InkToolbarPenConfigurationControl::for_pen(&pen);
    let _root = TestRoot::with_child(&config);
    let panel = config
        .children()
        .snapshot()
        .iter()
        .find_map(|child| child.cast::<WrapPanel>())
        .unwrap();
    let first = panel.children().get(0);
    assert!(first.focus());
    let key = |key: Key| {
        let mut event = KeyEventArgs::new();
        event.key = key;
        event.set_routed_event(Some(InputElement::key_down_event()));
        let focused = panel
            .children()
            .snapshot()
            .iter()
            .find(|child| child.is_focused())
            .unwrap()
            .clone();
        focused.raise_event(&event);
        assert!(event.handled());
    };
    key(Key::Down);
    assert_eq!(pen.selected_brush_index(), 6);
    assert!(panel.children().get(6).is_focused());
    key(Key::Right);
    assert_eq!(pen.selected_brush_index(), 7);
    assert_eq!(
        KeyboardNavigation::get_tab_navigation(&panel),
        KeyboardNavigationMode::Once
    );
    assert_eq!(
        KeyboardNavigation::get_tab_once_active_element(&panel),
        Some(panel.children().get(7).upcast::<InputElement>())
    );
    key(Key::End);
    assert_eq!(pen.selected_brush_index(), 29);
    key(Key::Right);
    assert_eq!(pen.selected_brush_index(), 29);
    key(Key::Home);
    assert_eq!(pen.selected_brush_index(), 0);
}

#[test]
fn normal_and_secondary_activation_follow_source_flyout_sequence() {
    use ferroui_base::input::{
        KeyModifiers, Pointer, PointerEventArgs, PointerPointProperties, PointerType,
        TappedEventArgs,
    };
    let _app = toolbar_window_services();
    let toolbar = InkToolbar::new();
    let window = toolbar_window(&toolbar);
    let stencil = toolbar
        .get_menu_button(InkToolbarMenuKind::Stencil)
        .unwrap()
        .cast::<InkToolbarStencilButton>()
        .unwrap();
    let flyout = stencil.flyout().unwrap();
    assert_ne!(stencil.is_checked(), Some(true));
    stencil.on_click();
    assert_eq!(stencil.is_checked(), Some(true));
    assert!(!flyout.is_open(), "First activation selects the stencil");
    stencil.on_click();
    assert_eq!(stencil.is_checked(), Some(true));
    assert!(
        flyout.is_open(),
        "An active stencil opens its configuration"
    );
    flyout.hide();
    stencil.set_is_checked(Some(false));

    let right_tap = |button: &Control| {
        let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
        let pointer_event = PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            button.to_ref(),
            pointer,
            Some(&window),
            Point::new(5.0, 5.0),
            1,
            PointerPointProperties::default(),
            KeyModifiers::NONE,
        );
        let event = TappedEventArgs::new(Some(InputElement::right_tapped_event()), &pointer_event);
        button.raise_event(&event);
        assert!(event.handled());
    };
    right_tap(&stencil);
    assert_eq!(stencil.is_checked(), Some(true));
    assert!(
        flyout.is_open(),
        "Secondary activation selects and opens together"
    );
    flyout.hide();
    stencil.set_is_protractor_item_visible(false);
    stencil.on_click();
    assert_eq!(stencil.is_checked(), Some(false));
    assert!(!flyout.is_open());
    stencil.on_click();
    assert_eq!(stencil.is_checked(), Some(true));
    assert!(!flyout.is_open(), "A single stencil is only a toggle");

    let pencil = toolbar.get_tool_button(InkToolbarTool::Pencil).unwrap();
    assert_ne!(toolbar.active_tool(), Some(pencil.clone()));
    right_tap(&pencil);
    assert_eq!(toolbar.active_tool(), Some(pencil.clone()));
    assert!(pencil.flyout().unwrap().is_open());
    assert!(InputElement::get_is_holding_enabled(&pencil));
    assert!(InputElement::get_is_holding_enabled(&stencil));
    pencil.flyout().unwrap().hide();
    window.close();
}

#[test]
fn custom_configuration_requires_content_and_refreshes_it_on_each_open() {
    let _app = toolbar_window_services();
    let toolbar = InkToolbar::new();
    toolbar.set_initial_controls(InkToolbarInitialControls::None);
    let pen = InkToolbarCustomPenButton::new();
    let tool = InkToolbarCustomToolButton::new();
    toolbar.children().add(&pen);
    toolbar.children().add(&tool);
    let window = toolbar_window(&toolbar);
    for button in [
        pen.clone().upcast::<InkToolbarToolButton>(),
        tool.clone().upcast(),
    ] {
        toolbar.set_active_tool(Some(button.clone()));
        button.on_click();
        assert!(!button.flyout().unwrap().is_open());
        for _ in 0..2 {
            let content = Control::new();
            if let Some(pen) = button.cast::<InkToolbarCustomPenButton>() {
                pen.set_configuration_content(Some(content.clone()));
            } else {
                tool.set_configuration_content(Some(content.clone()));
            }
            button.on_click();
            let flyout = button.flyout().unwrap().cast::<Flyout>().unwrap();
            assert!(flyout.is_open());
            assert_eq!(
                flyout.content().unwrap().downcast_ref::<Ref<Control>>(),
                Some(&content)
            );
            flyout.hide();
        }
        if let Some(pen) = button.cast::<InkToolbarCustomPenButton>() {
            pen.set_configuration_content(None);
        } else {
            tool.set_configuration_content(None);
        }
        button.on_click();
        assert!(
            !button.flyout().unwrap().is_open(),
            "Removing content suppresses the popup"
        );
    }
    window.close();
}

#[test]
fn panel_adaptation_includes_native_border_and_padding_in_measure_and_arrange() {
    use ferroui_controls::Border;
    use ferroui_base::{CornerRadius, Rect, Size, Thickness};
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    toolbar.set_initial_controls(InkToolbarInitialControls::None);
    let child = InkToolbarCustomToolButton::new();
    child.set_width(20.0);
    child.set_height(10.0);
    toolbar.children().add(&child);
    toolbar.set_padding(Thickness::new(3.0, 5.0, 7.0, 11.0));
    toolbar.set_border_thickness(Thickness::new(1.0, 2.0, 3.0, 4.0));
    toolbar.set_corner_radius(CornerRadius::uniform(7.0));
    assert_eq!(
        toolbar.get_value(Border::corner_radius_property()),
        CornerRadius::uniform(7.0)
    );
    toolbar.measure(Size::new(1000.0, 1000.0));
    assert_eq!(toolbar.desired_size(), Size::new(34.0, 32.0));
    toolbar.arrange(Rect::from_size(toolbar.desired_size()));
    assert_eq!(child.bounds(), Rect::new(4.0, 7.0, 20.0, 10.0));

    let config = InkToolbarPenConfigurationControl::new();
    config.measure(Size::new(1000.0, 1000.0));
    let unpadded = config.desired_size();
    config.arrange(Rect::from_size(unpadded));
    let first = config.children().get(0);
    let original_bounds = first.bounds();
    config.set_padding(toolbar.padding());
    config.set_border_thickness(toolbar.border_thickness());
    assert!(!config.is_measure_valid());
    config.measure(Size::new(1000.0, 1000.0));
    assert_eq!(
        config.desired_size(),
        Size::new(unpadded.width + 14.0, unpadded.height + 22.0)
    );
    config.arrange(Rect::from_size(config.desired_size()));
    assert_eq!(first.bounds().x, original_bounds.x + 4.0);
    assert_eq!(first.bounds().y, original_bounds.y + 7.0);
}

#[test]
fn default_controls_and_selection_match_upstream() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    assert_eq!(toolbar.children().count(), 5);
    assert_eq!(
        toolbar.active_tool().unwrap().tool_kind(),
        InkToolbarTool::BallpointPen
    );
    assert_eq!(toolbar.orientation(), Orientation::Horizontal);
    assert!(toolbar
        .get_menu_button(InkToolbarMenuKind::Stencil)
        .is_some());
}
#[test]
fn changing_initial_controls_preserves_application_children() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let custom = InkToolbarCustomToolButton::new();
    toolbar.children().add(&custom);
    toolbar.set_initial_controls(InkToolbarInitialControls::PensOnly);
    assert_eq!(toolbar.children().count(), 4);
    toolbar.set_initial_controls(InkToolbarInitialControls::None);
    assert_eq!(toolbar.children().count(), 1);
    assert_eq!(toolbar.children().get(0), custom.upcast::<Control>());
}
#[test]
fn registration_ignores_duplicate_system_kinds_and_illegal_content_without_removing_it() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    toolbar.set_initial_controls(InkToolbarInitialControls::None);
    let first: Vec<Ref<Control>> = vec![
        InkToolbarBallpointPenButton::new().upcast(),
        InkToolbarPencilButton::new().upcast(),
        InkToolbarHighlighterButton::new().upcast(),
        InkToolbarEraserButton::new().upcast(),
        InkToolbarRulerButton::new().upcast(),
        InkToolbarStencilButton::new().upcast(),
    ];
    let duplicate: Vec<Ref<Control>> = vec![
        InkToolbarBallpointPenButton::new().upcast(),
        InkToolbarPencilButton::new().upcast(),
        InkToolbarHighlighterButton::new().upcast(),
        InkToolbarEraserButton::new().upcast(),
        InkToolbarRulerButton::new().upcast(),
        InkToolbarStencilButton::new().upcast(),
        Control::new(),
        Button::new().upcast(),
        InkToolbarFlyoutItem::new().upcast(),
    ];
    toolbar.children().add_range(first.iter().cloned());
    toolbar.children().add_range(duplicate.iter().cloned());
    assert_eq!(toolbar.children().count(), 15);
    assert_eq!(toolbar.visual_children().count(), 6);
    assert_eq!(
        toolbar.get_direct_value(InkToolbar::children_property()),
        toolbar.children()
    );
    for child in &duplicate {
        assert!(
            child.is_visible(),
            "Ignoring registration must preserve caller visibility"
        );
        assert!(child.visual_parent().is_none());
        assert!(child.parent().is_none());
    }
    for (index, kind) in [
        InkToolbarTool::BallpointPen,
        InkToolbarTool::Pencil,
        InkToolbarTool::Highlighter,
        InkToolbarTool::Eraser,
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(
            toolbar.get_tool_button(kind).unwrap().upcast::<Control>(),
            first[index]
        );
    }
    assert_eq!(
        toolbar
            .get_toggle_button(InkToolbarToggle::Ruler)
            .unwrap()
            .upcast::<Control>(),
        first[4]
    );
    assert_eq!(
        toolbar
            .get_menu_button(InkToolbarMenuKind::Stencil)
            .unwrap()
            .upcast::<Control>(),
        first[5]
    );
}

#[test]
fn registration_accepts_distinct_custom_buttons_once_per_identity() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    toolbar.set_initial_controls(InkToolbarInitialControls::None);
    let buttons: Vec<Ref<Control>> = vec![
        InkToolbarCustomPenButton::new().upcast(),
        InkToolbarCustomPenButton::new().upcast(),
        InkToolbarCustomToolButton::new().upcast(),
        InkToolbarCustomToolButton::new().upcast(),
        InkToolbarCustomToggleButton::new().upcast(),
        InkToolbarCustomToggleButton::new().upcast(),
    ];
    toolbar.children().add_range(buttons.iter().cloned());
    toolbar
        .children()
        .add_range([buttons[0].clone(), buttons[2].clone(), buttons[4].clone()]);
    assert_eq!(toolbar.children().count(), 9);
    assert_eq!(toolbar.visual_children().count(), 6);
    assert!(buttons
        .iter()
        .all(|button| button.visual_parent().is_some()));
    assert_eq!(
        toolbar
            .get_tool_button(InkToolbarTool::CustomPen)
            .unwrap()
            .upcast::<Control>(),
        buttons[0]
    );
    assert_eq!(
        toolbar
            .get_tool_button(InkToolbarTool::CustomTool)
            .unwrap()
            .upcast::<Control>(),
        buttons[2]
    );
    assert_eq!(
        toolbar
            .get_toggle_button(InkToolbarToggle::Custom)
            .unwrap()
            .upcast::<Control>(),
        buttons[4]
    );
}

#[test]
fn ignored_stencil_is_not_wired_or_promoted_when_the_registered_stencil_is_removed() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    toolbar.set_initial_controls(InkToolbarInitialControls::None);
    let presenter = InkPresenter::new();
    toolbar.set_target_ink_presenter(Some(presenter.clone()));
    let first = InkToolbarStencilButton::new();
    let ignored = InkToolbarStencilButton::new();
    toolbar.children().add(&first);
    toolbar.children().add(&ignored);
    first.set_is_checked(Some(true));
    assert!(presenter.ruler_visible());
    assert!(first.ruler().is_some());
    assert!(ignored.ruler().is_none());
    ignored.set_selected_stencil(InkToolbarStencilKind::Protractor);
    ignored.set_is_checked(Some(true));
    assert!(presenter.ruler_visible());
    assert!(!presenter.protractor_visible());
    toolbar.children().remove(&first);
    assert!(toolbar
        .get_menu_button(InkToolbarMenuKind::Stencil)
        .is_none());
    assert_eq!(toolbar.children().count(), 1);
    assert!(ignored.visual_parent().is_none());
    // Source AddButton rejects the second registration; removal does not
    // retroactively admit it. An explicit re-add can now claim the slot.
    toolbar.children().remove(&ignored);
    toolbar.children().add(&ignored);
    assert_eq!(
        toolbar.get_menu_button(InkToolbarMenuKind::Stencil),
        Some(ignored.clone().upcast())
    );
    assert!(ignored.ruler().is_some());
}

#[test]
fn moving_registered_children_keeps_selection_flyouts_and_handlers() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    toolbar.set_initial_controls(InkToolbarInitialControls::None);
    let first = InkToolbarCustomToolButton::new();
    let second = InkToolbarCustomToolButton::new();
    toolbar.children().add(&first);
    toolbar.children().add(&second);
    toolbar.set_active_tool(Some(first.clone().upcast()));
    let flyout = first.flyout().unwrap();
    toolbar.children().move_item(0, 1);
    assert_eq!(
        toolbar.visual_children().get(1),
        first.clone().upcast::<Visual>()
    );
    assert_eq!(first.flyout(), Some(flyout));
    assert_eq!(toolbar.active_tool(), Some(first.clone().upcast()));
    second.set_is_checked(Some(true));
    assert_eq!(toolbar.active_tool(), Some(second.clone().upcast()));
    toolbar.children().clear();
    assert!(toolbar.visual_children().is_empty());
    first.set_is_checked(Some(true));
    assert!(toolbar.active_tool().is_none());
}
#[test]
fn presenter_target_wins_then_falls_back_to_canvas() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let canvas = InkCanvas::new();
    let explicit = InkPresenter::new();
    toolbar.set_target_ink_canvas(Some(canvas.clone()));
    toolbar.set_target_ink_presenter(Some(explicit.clone()));
    assert!(Rc::ptr_eq(
        &toolbar.effective_ink_presenter().unwrap(),
        &explicit
    ));
    let highlighter = toolbar
        .get_tool_button(InkToolbarTool::Highlighter)
        .unwrap();
    toolbar.set_active_tool(Some(highlighter));
    assert!(
        explicit
            .copy_default_drawing_attributes()
            .draw_as_highlighter
    );
    assert!(
        !canvas
            .ink_presenter()
            .copy_default_drawing_attributes()
            .draw_as_highlighter
    );
    toolbar.set_target_ink_presenter(None);
    assert!(Rc::ptr_eq(
        &toolbar.effective_ink_presenter().unwrap(),
        &canvas.ink_presenter()
    ));
    assert!(
        canvas
            .ink_presenter()
            .copy_default_drawing_attributes()
            .draw_as_highlighter
    );
}
#[test]
fn highlighter_uses_exact_palette_and_chisel_nib() {
    let _scope = test_scope();
    let highlighter = InkToolbarHighlighterButton::new();
    let attrs = highlighter.create_ink_drawing_attributes().unwrap();
    assert_eq!(highlighter.palette().count(), 6);
    assert_eq!(attrs.color, Color::from_uint32(0xffffe600));
    assert_eq!(attrs.size, ferroui_base::Size::new(16.0 / 3.0, 16.0));
    assert_eq!(attrs.pen_tip, PenTipShape::Rectangle);
    assert!(attrs.draw_as_highlighter);
}
#[test]
fn pen_palettes_and_widths_are_independent() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let a = toolbar
        .get_tool_button(InkToolbarTool::BallpointPen)
        .unwrap()
        .cast::<InkToolbarPenButton>()
        .unwrap();
    let b = toolbar
        .get_tool_button(InkToolbarTool::Pencil)
        .unwrap()
        .cast::<InkToolbarPenButton>()
        .unwrap();
    a.set_selected_stroke_width(8.0);
    a.palette().clear();
    assert_eq!(b.palette().count(), 30);
    assert_eq!(b.selected_stroke_width(), 4.0);
}
#[test]
fn pen_stroke_widths_default_to_the_values_of_winui() {
    let _scope = test_scope();
    let widths = |pen: &InkToolbarPenButton| {
        (
            pen.min_stroke_width(),
            pen.max_stroke_width(),
            pen.selected_stroke_width(),
        )
    };
    assert_eq!((1.0, 24.0, 2.0), widths(&InkToolbarBallpointPenButton::new()));
    assert_eq!((1.0, 24.0, 4.0), widths(&InkToolbarPencilButton::new()));
    assert_eq!((12.0, 64.0, 16.0), widths(&InkToolbarHighlighterButton::new()));
    assert_eq!((1.0, 10.0, 5.0), widths(&InkToolbarCustomPenButton::new()));
}
#[test]
fn editing_palette_updates_active_presenter_without_reselecting() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let presenter = InkPresenter::new();
    toolbar.set_target_ink_presenter(Some(presenter.clone()));
    let pen = toolbar
        .active_tool()
        .unwrap()
        .cast::<InkToolbarPenButton>()
        .unwrap();
    let red = Color::from_uint32(0xffe61b1b);
    pen.palette().set(
        0,
        Rc::new(ImmutableSolidColorBrush::new(red)) as Rc<dyn IBrush>,
    );
    assert_eq!(presenter.copy_default_drawing_attributes().color, red);
}
#[test]
fn invalid_widths_are_coerced_to_valid_bounds() {
    let _scope = test_scope();
    let pen = InkToolbarBallpointPenButton::new();
    pen.set_selected_stroke_width(f64::NAN);
    assert!(pen.selected_stroke_width().is_finite());
    pen.set_min_stroke_width(5.0);
    pen.set_max_stroke_width(10.0);
    pen.set_selected_stroke_width(100.0);
    assert_eq!(pen.selected_stroke_width(), 10.0);
    pen.set_selected_stroke_width(-1.0);
    assert_eq!(pen.selected_stroke_width(), 5.0);
}
#[test]
fn tool_selection_changes_processing_mode() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let presenter = InkPresenter::new();
    toolbar.set_target_ink_presenter(Some(presenter.clone()));
    toolbar.set_active_tool(toolbar.get_tool_button(InkToolbarTool::Eraser));
    assert_eq!(
        presenter.input_processing_configuration().mode(),
        InkInputProcessingMode::Erasing
    );
    let custom = InkToolbarCustomToolButton::new();
    toolbar.children().add(&custom);
    toolbar.set_active_tool(Some(custom.upcast()));
    assert_eq!(
        presenter.input_processing_configuration().mode(),
        InkInputProcessingMode::None
    );
    toolbar.set_active_tool(toolbar.get_tool_button(InkToolbarTool::BallpointPen));
    assert_eq!(
        presenter.input_processing_configuration().mode(),
        InkInputProcessingMode::Inking
    );
}
#[test]
fn erase_all_restores_the_previous_tool_then_raises_the_event_then_clears() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let presenter = InkPresenter::new();
    toolbar.set_target_ink_presenter(Some(presenter.clone()));
    presenter
        .stroke_container()
        .add_stroke(
            InkStrokeBuilder::new()
                .create_stroke([Point::new(2.0, 3.0)])
                .unwrap(),
        )
        .unwrap();
    toolbar.set_active_tool(toolbar.get_tool_button(InkToolbarTool::Pencil));
    toolbar.set_active_tool(toolbar.get_tool_button(InkToolbarTool::Eraser));
    let seen = Rc::new(Cell::new(0));
    let seen_copy = seen.clone();
    let tool_seen = Rc::new(Cell::new(None));
    let tool_seen_copy = tool_seen.clone();
    toolbar.erase_all_clicked(move |toolbar| {
        seen_copy.set(
            toolbar
                .effective_ink_presenter()
                .unwrap()
                .stroke_container()
                .len(),
        );
        // As WinUI, the previous tool is restored before the event is raised.
        tool_seen_copy.set(toolbar.active_tool().map(|tool| tool.tool_kind()));
    });
    toolbar.erase_all();
    assert_eq!(seen.get(), 1);
    assert_eq!(tool_seen.get(), Some(InkToolbarTool::Pencil));
    assert!(presenter.stroke_container().is_empty());
    assert_eq!(
        toolbar.active_tool().unwrap().tool_kind(),
        InkToolbarTool::Pencil
    );
}
#[test]
fn stencil_switch_is_exclusive_and_target_change_hides_old_stencil() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let a = InkPresenter::new();
    let b = InkPresenter::new();
    toolbar.set_target_ink_presenter(Some(a.clone()));
    toolbar.set_is_stencil_button_checked(true);
    assert!(a.ruler_visible());
    let stencil = toolbar
        .get_menu_button(InkToolbarMenuKind::Stencil)
        .unwrap()
        .cast::<InkToolbarStencilButton>()
        .unwrap();
    stencil.set_selected_stencil(InkToolbarStencilKind::Protractor);
    assert!(!a.ruler_visible());
    assert!(a.protractor_visible());
    toolbar.set_target_ink_presenter(Some(b.clone()));
    assert!(!a.protractor_visible());
    assert!(b.protractor_visible());
    assert!(Rc::ptr_eq(&stencil.protractor().unwrap(), &b.protractor()));
}
#[test]
fn configuration_slider_and_palette_stay_synchronized() {
    let _scope = test_scope();
    let pen = InkToolbarBallpointPenButton::new();
    let config = InkToolbarPenConfigurationControl::for_pen(&pen);
    let slider = config.stroke_width_slider().unwrap();
    slider.set_range_value(7.0);
    assert_eq!(pen.selected_stroke_width(), 7.0);
    pen.set_selected_stroke_width(11.0);
    assert_eq!(slider.value(), 11.0);
    pen.set_selected_brush_index(7);
    let palette = config
        .children()
        .snapshot()
        .iter()
        .find_map(|child| child.cast::<ferroui_controls::WrapPanel>())
        .unwrap();
    assert_eq!(
        palette
            .children()
            .get(7)
            .cast::<InkToolbarFlyoutItem>()
            .unwrap()
            .is_checked(),
        Some(true)
    );
}
#[test]
fn default_pen_configuration_is_created_lazily() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let pen = toolbar.active_tool().unwrap();
    let flyout = pen.flyout().unwrap().cast::<Flyout>().unwrap();
    assert!(flyout.content().is_none());
}
#[test]
fn removed_tool_no_longer_updates_presenter() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let presenter = InkPresenter::new();
    toolbar.set_target_ink_presenter(Some(presenter.clone()));
    let pen = toolbar
        .active_tool()
        .unwrap()
        .cast::<InkToolbarPenButton>()
        .unwrap();
    toolbar.children().remove(&pen);
    let previous = presenter.copy_default_drawing_attributes();
    pen.set_selected_stroke_width(15.0);
    assert_eq!(presenter.copy_default_drawing_attributes(), previous);
}
#[test]
fn event_subscriptions_can_be_disposed_and_handlers_reenter() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let count = Rc::new(Cell::new(0));
    let copy = count.clone();
    let subscription = toolbar.active_tool_changed(move |toolbar| {
        let _ = toolbar.children().snapshot();
        copy.set(copy.get() + 1);
    });
    toolbar.set_active_tool(toolbar.get_tool_button(InkToolbarTool::Pencil));
    assert_eq!(count.get(), 1);
    subscription.dispose();
    toolbar.set_active_tool(toolbar.get_tool_button(InkToolbarTool::Highlighter));
    assert_eq!(count.get(), 1);
}
#[test]
fn named_automation_peers_expose_expected_patterns() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let tool = toolbar.active_tool().unwrap();
    let peer = ControlAutomationPeer::create_peer_for_element(&tool);
    assert!(peer.get_provider::<dyn ISelectionItemProvider>().is_some());
    assert!(peer.get_provider::<dyn IExpandCollapseProvider>().is_some());
    let slider = InkToolbarStrokeWidthSlider::new();
    let peer = ControlAutomationPeer::create_peer_for_element(&slider);
    assert!(peer.get_provider::<dyn IRangeValueProvider>().is_some());
    assert!(peer.get_provider::<dyn IValueProvider>().is_some());
    assert_eq!(
        ControlAutomationPeer::create_peer_for_element(&toolbar).get_automation_control_type(),
        AutomationControlType::Pane
    );
}
#[test]
fn slider_value_automation_validates_text_and_disabled_state() {
    let _scope = test_scope();
    let slider = InkToolbarStrokeWidthSlider::new();
    slider.set_minimum(1.0);
    slider.set_maximum(20.0);
    let peer = ControlAutomationPeer::create_peer_for_element(&slider);
    let provider = peer.get_provider::<dyn IValueProvider>().unwrap();
    provider.set_value(Some("8.5")).unwrap();
    assert_eq!(slider.value(), 8.5);
    assert!(provider.set_value(Some("NaN")).is_err());
    provider.set_value(Some("40")).unwrap();
    assert_eq!(slider.value(), 20.0);
    slider.set_is_enabled(false);
    assert!(provider.set_value(Some("2")).is_err());
}

#[test]
fn invalid_palette_index_preserves_previous_selection() {
    let _scope = test_scope();
    let pen = InkToolbarBallpointPenButton::new();
    let previous = pen.selected_brush();
    pen.set_selected_brush_index(-1);
    assert!(pen.selected_brush() == previous);
    pen.set_selected_brush_index(999);
    assert!(pen.selected_brush() == previous);
}
#[test]
fn readonly_contract_properties_are_registered_readonly() {
    assert!(InkToolbar::children_property().is_read_only());
    assert!(InkToolbar::ink_drawing_attributes_property().is_read_only());
    assert!(InkToolbarPenButton::selected_brush_property().is_read_only());
    assert!(InkToolbarStencilButton::ruler_property().is_read_only());
    assert!(InkToolbarStencilButton::protractor_property().is_read_only());
    assert!(InkToolbarPenConfigurationControl::pen_button_property().is_read_only());
}
#[test]
fn target_changes_do_not_raise_active_tool_changed() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let count = Rc::new(Cell::new(0));
    let copy = count.clone();
    toolbar.active_tool_changed(move |_| copy.set(copy.get() + 1));
    toolbar.set_target_ink_presenter(Some(InkPresenter::new()));
    toolbar.set_target_ink_presenter(Some(InkPresenter::new()));
    assert_eq!(count.get(), 0);
}
#[test]
fn eraser_retains_last_pen_drawing_attributes() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    toolbar.set_active_tool(toolbar.get_tool_button(InkToolbarTool::Highlighter));
    let attrs = toolbar.ink_drawing_attributes();
    toolbar.set_active_tool(toolbar.get_tool_button(InkToolbarTool::Eraser));
    assert_eq!(toolbar.ink_drawing_attributes(), attrs);
}

#[repr(C)]
struct CustomNibPen {
    base: InkToolbarCustomPen,
}
ferroui_base::ferro_class!(CustomNibPen:InkToolbarCustomPen);
ferroui_base::ferro_impl_classes!(CustomNibPen:FerroObjectImpl);
impl InkToolbarCustomPenImpl for CustomNibPen {
    fn create_ink_drawing_attributes_core(
        _this: &Self,
        _brush: Option<Rc<dyn IBrush>>,
        stroke_width: f64,
    ) -> Result<InkDrawingAttributes, InkError> {
        Ok(InkDrawingAttributes {
            color: Color::from_uint32(0xff123456),
            size: ferroui_base::Size::new(stroke_width, stroke_width * 2.0),
            pen_tip: PenTipShape::Rectangle,
            ..InkDrawingAttributes::default()
        })
    }
}
#[test]
fn custom_pen_base_accepts_null_brush_and_preserves_requested_width() {
    let _scope = test_scope();
    let pen = InkToolbarCustomPen::new();
    let attributes = pen.create_ink_drawing_attributes(None, 3.0).unwrap();
    assert_eq!(attributes.color, InkDrawingAttributes::default().color);
    assert_eq!(attributes.size, ferroui_base::Size::new(3.0, 3.0));
    let red = Color::from_rgb(255, 0, 0);
    let brush = Rc::new(ImmutableSolidColorBrush::new(red)) as Rc<dyn IBrush>;
    let attributes = pen.create_ink_drawing_attributes(Some(brush), 5.0).unwrap();
    assert_eq!(attributes.color, red);
    assert_eq!(attributes.size, ferroui_base::Size::new(5.0, 5.0));
}
#[test]
fn custom_pen_factory_is_dispatched_virtually_by_toolbar() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let presenter = InkPresenter::new();
    toolbar.set_target_ink_presenter(Some(presenter.clone()));
    let factory = ferroui_base::instantiate(CustomNibPen {
        base: InkToolbarCustomPen::construct(),
    });
    let button = InkToolbarCustomPenButton::new();
    button.set_custom_pen(Some(factory.upcast()));
    button.set_selected_stroke_width(5.0);
    toolbar.children().add(&button);
    toolbar.set_active_tool(Some(button.upcast()));
    let attrs = presenter.copy_default_drawing_attributes();
    assert_eq!(attrs.color, Color::from_uint32(0xff123456));
    assert_eq!(attrs.size, ferroui_base::Size::new(5.0, 10.0));
}
#[test]
fn selecting_a_hidden_stencil_shows_its_item() {
    let _scope = test_scope();
    let stencil = InkToolbarStencilButton::new();
    stencil.set_is_protractor_item_visible(false);
    assert_eq!(stencil.selected_stencil(), InkToolbarStencilKind::Ruler);
    stencil.set_selected_stencil(InkToolbarStencilKind::Protractor);
    assert_eq!(stencil.selected_stencil(), InkToolbarStencilKind::Protractor);
    assert!(stencil.is_protractor_item_visible());
    assert!(stencil.is_ruler_item_visible());
}
#[test]
fn application_buttons_replace_the_built_in_buttons_of_their_kind() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    assert_eq!(toolbar.initial_controls(), InkToolbarInitialControls::All);
    let ballpoint = InkToolbarBallpointPenButton::new();
    ballpoint.set_selected_brush_index(7);
    let eraser = InkToolbarEraserButton::new();
    eraser.set_is_clear_all_visible(false);
    toolbar.children().add(&ballpoint);
    toolbar.children().add(&eraser);
    let registered = |kind| toolbar.get_tool_button(kind).unwrap();
    assert_eq!(
        registered(InkToolbarTool::BallpointPen),
        ballpoint.clone().upcast::<InkToolbarToolButton>()
    );
    assert_eq!(
        registered(InkToolbarTool::Eraser),
        eraser.clone().upcast::<InkToolbarToolButton>()
    );
    // One button of each kind, and the application's ballpoint is the active tool, as the
    // built-in one it replaced was.
    let ballpoints = toolbar
        .children()
        .snapshot()
        .iter()
        .filter(|child| child.cast::<InkToolbarBallpointPenButton>().is_some())
        .count();
    assert_eq!(ballpoints, 1);
    assert_eq!(
        toolbar.active_tool(),
        Some(ballpoint.clone().upcast::<InkToolbarToolButton>())
    );
    assert_eq!(ballpoint.selected_brush_index(), 7);
}
#[test]
fn a_toolbar_without_an_active_tool_leaves_inking_on() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    toolbar.set_initial_controls(InkToolbarInitialControls::None);
    toolbar.set_active_tool(None);
    let presenter = InkPresenter::new();
    presenter
        .input_processing_configuration()
        .set_mode(InkInputProcessingMode::Inking);
    toolbar.set_target_ink_presenter(Some(presenter.clone()));
    assert!(toolbar.active_tool().is_none());
    assert_eq!(
        presenter.input_processing_configuration().mode(),
        InkInputProcessingMode::Inking
    );
}
#[test]
fn the_stencil_event_is_raised_for_a_change_of_is_stencil_button_checked_alone() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let raised = Rc::new(Cell::new(0));
    let count = raised.clone();
    let _subscription = toolbar.is_stencil_button_checked_changed(move |_| count.set(count.get() + 1));
    toolbar.set_target_ink_presenter(Some(InkPresenter::new()));
    toolbar.set_is_ruler_button_checked(true);
    assert_eq!(raised.get(), 0);
    toolbar.set_is_stencil_button_checked(true);
    assert_eq!(raised.get(), 1);
}
