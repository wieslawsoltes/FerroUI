//! Scenario ports from WinUI InkCanvasTests.cs and InkToolbarTests.cs.
//! Tests use the existing FerroUI root, popup and automation services; Windows
//! COM aggregation and native pixels are separate platform boundaries.
use super::*;
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::automation::provider::{
    IExpandCollapseProvider, IInvokeProvider, ISelectionItemProvider, IToggleProvider,
    IValueProvider, ToggleState,
};
use ferroui_controls::automation::{AutomationProperties, ExpandCollapseState, ValuePatternIdentifiers};
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::primitives::PopupFlyoutBase;
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::{test_scope, TestRoot};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{
    Button, Canvas, CheckBox, ComboBox, Control, Flyout, Panel, PlacementMode, ScrollViewer,
    StackPanel, TextBlock, ToolTip, Window,
};
use ferroui_base::input::{IKeyboardDevice, InputElement, Key, KeyEventArgs, KeyboardDevice};
use ferroui_base::layout::Orientation;
use ferroui_base::media::{Geometry, RectangleGeometry};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{Rect, Ref, Size};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn canvas() -> Ref<InkCanvas> {
    let canvas = InkCanvas::new();
    canvas.set_width(400.0);
    canvas.set_height(300.0);
    canvas
}
fn attach(control: &Control, size: Size) -> Ref<TestRoot> {
    let root = TestRoot::with_child(control.to_ref());
    root.set_client_size(size);
    root.set_clip_to_bounds(true);
    root.measure(size);
    root.arrange(Rect::new(0.0, 0.0, size.width, size.height));
    root
}
fn peer(control: &Control) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
    ControlAutomationPeer::create_peer_for_element(control)
}
fn create_window_services() -> UnitTestApplicationScope {
    UnitTestApplication::start(
        TestServices::styled_window()
            .with_keyboard_device(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>)),
    )
}
fn shown_window(control: &Control) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(control.to_ref())));
    window.show();
    window
}
fn stencil_items(button: &InkToolbarStencilButton) -> Vec<Ref<InkToolbarFlyoutItem>> {
    let flyout = button.flyout().unwrap().cast::<Flyout>().unwrap();
    let content = flyout.content().unwrap();
    let panel = content
        .downcast_ref::<Ref<Control>>()
        .unwrap()
        .cast::<StackPanel>()
        .unwrap();
    panel
        .children()
        .snapshot()
        .iter()
        .filter_map(|child| child.cast())
        .collect()
}

#[test]
fn canvas_automation_reports_visible_bounds_and_collapsed_or_outside_root() {
    let _scope = test_scope();
    let ink = canvas();
    let root = attach(&ink, Size::new(500.0, 400.0));
    let automation = peer(&ink);
    let bounds = automation.get_bounding_rectangle();
    assert!(bounds.width > 0.0 && bounds.height > 0.0);
    assert!(!automation.is_offscreen());
    ink.set_is_visible(false);
    assert!(automation.is_offscreen());
    assert_eq!(automation.get_bounding_rectangle().size(), Size::default());
    root.set_child(None);
    let outside = canvas();
    let host = Canvas::new();
    host.children().add(&outside);
    Canvas::set_left(&outside, 20_000.0);
    root.set_child(&host);
    root.measure(Size::new(500.0, 400.0));
    root.arrange(Rect::new(0.0, 0.0, 500.0, 400.0));
    let outside_peer = peer(&outside);
    assert!(outside_peer.is_offscreen());
    assert_eq!(
        outside_peer.get_bounding_rectangle().size(),
        Size::default()
    );
}

#[test]
fn canvas_automation_respects_empty_ancestor_and_own_geometry_clips() {
    let _scope = test_scope();
    for self_clip in [false, true] {
        let ink = canvas();
        let host = Panel::new();
        host.children().add(&ink);
        let clip = RectangleGeometry::with_rect(Rect::default());
        if self_clip {
            ink.set_clip(Some(clip.upcast::<Geometry>()));
        } else {
            host.set_clip(Some(clip.upcast::<Geometry>()));
        }
        let _root = attach(&host, Size::new(400.0, 300.0));
        let automation = peer(&ink);
        assert!(automation.is_offscreen());
        assert_eq!(automation.get_bounding_rectangle().size(), Size::default());
    }
}

#[test]
fn canvas_automation_reduces_bounds_for_own_partial_clip() {
    let _scope = test_scope();
    let ink = canvas();
    let _root = attach(&ink, Size::new(400.0, 300.0));
    let automation = peer(&ink);
    let full = automation.get_bounding_rectangle();
    ink.set_clip(Some(
        RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 100.0, 75.0)).upcast::<Geometry>(),
    ));
    let clipped = automation.get_bounding_rectangle();
    assert!(!automation.is_offscreen());
    assert!(clipped.width > 0.0 && clipped.width < full.width);
    assert!(clipped.height > 0.0 && clipped.height < full.height);
    assert_eq!(clipped.size(), Size::new(100.0, 75.0));
}

#[test]
fn canvas_automation_clips_to_actual_scroll_viewer_viewport() {
    let _scope = test_scope();
    let ink = canvas();
    ink.set_height(2000.0);
    let scroll = ScrollViewer::new();
    scroll.set_width(300.0);
    scroll.set_height(200.0);
    scroll.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(
        |_, scope| {
            let presenter = ScrollContentPresenter::new();
            presenter.set_name(Some("PART_ContentPresenter".into()));
            presenter.register_in_name_scope(&**scope).upcast()
        },
    )));
    scroll.set_content(Some(Control::boxed(ink.clone())));
    let _root = attach(&scroll, Size::new(300.0, 200.0));
    let canvas_peer = peer(&ink);
    let scroll_bounds = peer(&scroll).get_bounding_rectangle();
    let bounds = canvas_peer.get_bounding_rectangle();
    assert!(bounds.width > 0.0 && bounds.height > 0.0);
    assert!(bounds.width <= scroll_bounds.width + 1.0);
    assert!(bounds.height <= scroll_bounds.height + 1.0);
    assert!(bounds.height < ink.bounds().height);
    assert!(!canvas_peer.is_offscreen());
}

#[test]
fn canvas_load_unload_revokes_callbacks_and_can_be_reloaded() {
    let _scope = test_scope();
    let ink = canvas();
    let loaded = Rc::new(Cell::new(0));
    let unloaded = Rc::new(Cell::new(0));
    let count = loaded.clone();
    ink.loaded(move |_, _| count.set(count.get() + 1));
    let count = unloaded.clone();
    ink.unloaded(move |_, _| count.set(count.get() + 1));
    let root = attach(&ink, Size::new(400.0, 300.0));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(ink.is_loaded());
    root.set_child(None);
    assert!(!ink.is_loaded());
    assert_eq!((loaded.get(), unloaded.get()), (1, 1));
    root.set_child(&ink);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(ink.is_loaded());
    assert_eq!(loaded.get(), 2);
    root.set_child(None);
}

#[test]
fn dropping_canvas_before_queued_presenter_change_does_not_keep_it_alive() {
    let _scope = test_scope();
    for _ in 0..5 {
        let ink = canvas();
        let weak = ink.downgrade();
        let presenter = ink.ink_presenter();
        let root = attach(&ink, Size::new(400.0, 300.0));
        // The retained presenter invokes the canvas's actual weak invalidation
        // subscription after its owner has been detached and dropped.
        Dispatcher::ui_thread().post_local(
            move || {
                presenter.set_ruler_visible(true);
            },
            DispatcherPriority::BACKGROUND,
        );
        root.set_child(None);
        drop(ink);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak.upgrade().is_none());
    }
}

#[test]
fn toolbar_orientation_and_every_flyout_placement_round_trip_and_reach_flyouts() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    assert_eq!(toolbar.orientation(), Orientation::Horizontal);
    assert_eq!(
        toolbar.button_flyout_placement(),
        InkToolbarButtonFlyoutPlacement::Auto
    );
    toolbar.set_orientation(Orientation::Vertical);
    assert_eq!(toolbar.orientation(), Orientation::Vertical);
    toolbar.set_orientation(Orientation::Horizontal);
    assert_eq!(toolbar.orientation(), Orientation::Horizontal);
    for (ink, expected) in [
        (
            InkToolbarButtonFlyoutPlacement::Top,
            PlacementMode::TopEdgeAlignedLeft,
        ),
        (
            InkToolbarButtonFlyoutPlacement::Bottom,
            PlacementMode::BottomEdgeAlignedLeft,
        ),
        (InkToolbarButtonFlyoutPlacement::Left, PlacementMode::Left),
        (InkToolbarButtonFlyoutPlacement::Right, PlacementMode::Right),
    ] {
        toolbar.set_button_flyout_placement(ink);
        assert_eq!(toolbar.button_flyout_placement(), ink);
        for child in toolbar.children().snapshot().iter() {
            if let Some(flyout) = child
                .cast::<Button>()
                .and_then(|button| button.flyout())
                .and_then(|flyout| flyout.cast::<PopupFlyoutBase>())
            {
                assert_eq!(flyout.placement(), expected);
            }
        }
    }
    toolbar.set_orientation(Orientation::Vertical);
    toolbar.set_button_flyout_placement(InkToolbarButtonFlyoutPlacement::Auto);
    let pen = toolbar
        .get_tool_button(InkToolbarTool::BallpointPen)
        .unwrap();
    assert_eq!(
        pen.flyout()
            .unwrap()
            .cast::<PopupFlyoutBase>()
            .unwrap()
            .placement(),
        PlacementMode::BottomEdgeAlignedLeft
    );
}

#[test]
fn toolbar_in_visual_tree_drives_canvas_and_legacy_ruler_toggle() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let ink = canvas();
    toolbar.set_target_ink_canvas(Some(ink.clone()));
    let panel = StackPanel::new();
    panel.children().add(&toolbar);
    panel.children().add(&ink);
    let _root = attach(&panel, Size::new(600.0, 500.0));
    assert!(toolbar.visual_root().is_some());
    assert_eq!(toolbar.target_ink_canvas(), Some(ink.clone()));
    assert_eq!(ink.width(), 400.0);
    assert_eq!(ink.height(), 300.0);
    assert!(!toolbar.is_ruler_button_checked());
    assert!(!toolbar.is_stencil_button_checked());
    toolbar.set_is_stencil_button_checked(true);
    assert!(toolbar.is_stencil_button_checked());
    assert!(ink.ink_presenter().ruler_visible());
    toolbar.set_is_stencil_button_checked(false);
    assert!(!ink.ink_presenter().ruler_visible());
    // As WinUI, `IsRulerButtonChecked` shows the ruler through a ruler button only.
    toolbar.set_is_ruler_button_checked(true);
    assert!(toolbar.is_ruler_button_checked());
    assert!(!ink.ink_presenter().ruler_visible());
    toolbar.set_is_ruler_button_checked(false);
    let ruler = InkToolbarRulerButton::new();
    toolbar.children().add(&ruler);
    toolbar.set_is_ruler_button_checked(true);
    assert_eq!(ruler.is_checked(), Some(true));
    assert!(ink.ink_presenter().ruler_visible());
    toolbar.set_is_ruler_button_checked(false);
    assert!(!toolbar.is_ruler_button_checked());
    assert!(!ink.ink_presenter().ruler_visible());
}

/// Ports the seven source interaction-page actions through actual FerroUI
/// selectors, checkbox providers and routed handlers. The source harness's
/// out-of-process Windows UIA transport is replaced by framework test services.
#[test]
fn interaction_page_selectors_and_checkboxes_drive_the_live_toolbar() {
    fn selector(labels: &[&str], changed: impl Fn(usize) + 'static) -> Ref<ComboBox> {
        let selector = ComboBox::new();
        for label in labels {
            selector.items().add(Some(Rc::new((*label).to_owned())));
        }
        selector.selection_changed(move |sender, _| {
            let selector = sender.to_ref().cast::<ComboBox>().unwrap();
            if selector.selected_index() >= 0 {
                changed(selector.selected_index() as usize);
            }
        });
        selector
    }
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let ink = canvas();
    toolbar.set_target_ink_canvas(Some(ink.clone()));
    let active_text = TextBlock::new();
    let active_label = active_text.clone();
    toolbar.active_tool_changed(move |toolbar| {
        active_label.set_text(Some(
            &toolbar
                .active_tool()
                .map(|tool| format!("{:?}", tool.tool_kind()))
                .unwrap_or_else(|| "None".into()),
        ));
    });
    let target = toolbar.clone();
    let initial = selector(
        &["None", "PensOnly", "AllExceptPens", "All"],
        move |index| {
            target.set_initial_controls(
                [
                    InkToolbarInitialControls::None,
                    InkToolbarInitialControls::PensOnly,
                    InkToolbarInitialControls::AllExceptPens,
                    InkToolbarInitialControls::All,
                ][index],
            );
        },
    );
    let target = toolbar.clone();
    let orientation = selector(&["Vertical", "Horizontal"], move |index| {
        target.set_orientation([Orientation::Vertical, Orientation::Horizontal][index]);
    });
    let target = toolbar.clone();
    let placement = selector(&["Top", "Bottom", "Left", "Right", "Auto"], move |index| {
        target.set_button_flyout_placement(
            [
                InkToolbarButtonFlyoutPlacement::Top,
                InkToolbarButtonFlyoutPlacement::Bottom,
                InkToolbarButtonFlyoutPlacement::Left,
                InkToolbarButtonFlyoutPlacement::Right,
                InkToolbarButtonFlyoutPlacement::Auto,
            ][index],
        );
    });
    let ruler = CheckBox::new();
    let target = toolbar.clone();
    ruler.is_checked_changed(move |sender, _| {
        target.set_is_ruler_button_checked(
            sender.to_ref().cast::<CheckBox>().unwrap().is_checked() == Some(true),
        );
    });
    let stencil = CheckBox::new();
    let target = toolbar.clone();
    stencil.is_checked_changed(move |sender, _| {
        target.set_is_stencil_button_checked(
            sender.to_ref().cast::<CheckBox>().unwrap().is_checked() == Some(true),
        );
    });
    let panel = StackPanel::new();
    let controls: [Ref<Control>; 8] = [
        toolbar.clone().upcast(),
        ink.clone().upcast(),
        initial.clone().upcast(),
        orientation.clone().upcast(),
        placement.clone().upcast(),
        ruler.clone().upcast(),
        stencil.clone().upcast(),
        active_text.clone().upcast(),
    ];
    for control in &controls {
        panel.children().add(control);
    }
    let root = attach(&panel, Size::new(600.0, 800.0));
    for control in controls {
        assert!(control.visual_root().is_some());
    }
    for (index, expected_count) in [0, 3, 2, 5].into_iter().enumerate() {
        initial.set_selected_index(index as i32);
        root.measure(Size::new(600.0, 800.0));
        root.arrange(Rect::new(0.0, 0.0, 600.0, 800.0));
        assert_eq!(toolbar.children().count(), expected_count);
    }
    assert_eq!(active_text.text().as_deref(), Some("BallpointPen"));
    for (index, expected) in [Orientation::Vertical, Orientation::Horizontal]
        .into_iter()
        .enumerate()
    {
        orientation.set_selected_index(index as i32);
        assert_eq!(toolbar.orientation(), expected);
    }
    for (index, expected) in [
        InkToolbarButtonFlyoutPlacement::Top,
        InkToolbarButtonFlyoutPlacement::Bottom,
        InkToolbarButtonFlyoutPlacement::Left,
        InkToolbarButtonFlyoutPlacement::Right,
        InkToolbarButtonFlyoutPlacement::Auto,
    ]
    .into_iter()
    .enumerate()
    {
        placement.set_selected_index(index as i32);
        assert_eq!(toolbar.button_flyout_placement(), expected);
    }
    // The default controls have the stencil button and no ruler button, so, as WinUI, the
    // ruler check box sets `IsRulerButtonChecked` without showing the ruler, and the stencil
    // check box shows it.
    for (checkbox, shows_ruler) in [(&ruler, false), (&stencil, true)] {
        let toggle = peer(checkbox)
            .get_provider::<dyn IToggleProvider>()
            .unwrap();
        toggle.toggle().unwrap();
        assert_eq!(checkbox.is_checked(), Some(true));
        assert_eq!(ink.ink_presenter().ruler_visible(), shows_ruler);
        toggle.toggle().unwrap();
        assert_eq!(checkbox.is_checked(), Some(false));
        assert!(!ink.ink_presenter().ruler_visible());
    }
}

#[test]
fn toolbar_instances_controls_toggle_lookup_and_custom_configuration_are_independent() {
    let _scope = test_scope();
    let first = InkToolbar::new();
    let second = InkToolbar::new();
    first.set_initial_controls(InkToolbarInitialControls::PensOnly);
    second.set_initial_controls(InkToolbarInitialControls::None);
    first.set_orientation(Orientation::Vertical);
    assert_eq!(
        first.initial_controls(),
        InkToolbarInitialControls::PensOnly
    );
    assert_eq!(second.initial_controls(), InkToolbarInitialControls::None);
    assert_eq!(second.orientation(), Orientation::Horizontal);
    assert_eq!(first.orientation(), Orientation::Vertical);
    assert!(second.get_toggle_button(InkToolbarToggle::Custom).is_none());
    assert!(second.get_toggle_button(InkToolbarToggle::Ruler).is_none());
    let custom = InkToolbarCustomToggleButton::new();
    let ruler = InkToolbarRulerButton::new();
    second.children().add(&custom);
    second.children().add(&ruler);
    assert_eq!(custom.toggle_kind(), InkToolbarToggle::Custom);
    assert_eq!(ruler.toggle_kind(), InkToolbarToggle::Ruler);
    assert_eq!(
        second.get_toggle_button(InkToolbarToggle::Custom),
        Some(custom.upcast())
    );
    assert_eq!(
        second.get_toggle_button(InkToolbarToggle::Ruler),
        Some(ruler.upcast())
    );
    let tool = InkToolbarCustomToolButton::new();
    assert_eq!(tool.tool_kind(), InkToolbarTool::CustomTool);
    assert!(tool.configuration_content().is_none());
    let content = TextBlock::new();
    content.set_text(Some("Custom configuration".into()));
    tool.set_configuration_content(Some(content.clone().upcast()));
    assert_eq!(tool.configuration_content(), Some(content.upcast()));
    let custom_pen = InkToolbarCustomPenButton::new();
    assert_eq!(custom_pen.tool_kind(), InkToolbarTool::CustomPen);
    assert!(custom_pen.custom_pen().is_none());
    assert!(custom_pen.configuration_content().is_none());
    let content = TextBlock::new();
    content.set_text(Some("Custom pen configuration".into()));
    custom_pen.set_configuration_content(Some(content.clone().upcast()));
    assert_eq!(custom_pen.configuration_content(), Some(content.upcast()));
}

#[test]
fn toolbar_all_except_pens_and_button_defaults_match_source() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    toolbar.set_initial_controls(InkToolbarInitialControls::AllExceptPens);
    let _root = attach(&toolbar, Size::new(500.0, 100.0));
    assert!(toolbar
        .get_tool_button(InkToolbarTool::BallpointPen)
        .is_none());
    assert!(toolbar.get_tool_button(InkToolbarTool::Pencil).is_none());
    assert!(toolbar
        .get_tool_button(InkToolbarTool::Highlighter)
        .is_none());
    assert!(toolbar.get_tool_button(InkToolbarTool::Eraser).is_some());
    assert_eq!(
        InkToolbarBallpointPenButton::new().tool_kind(),
        InkToolbarTool::BallpointPen
    );
    assert_eq!(
        InkToolbarPencilButton::new().tool_kind(),
        InkToolbarTool::Pencil
    );
    assert_eq!(
        InkToolbarHighlighterButton::new().tool_kind(),
        InkToolbarTool::Highlighter
    );
    let eraser = InkToolbarEraserButton::new();
    assert_eq!(eraser.tool_kind(), InkToolbarTool::Eraser);
    assert!(eraser.is_clear_all_visible());
    eraser.set_is_clear_all_visible(false);
    assert!(!eraser.is_clear_all_visible());
    eraser.set_is_clear_all_visible(true);
    assert!(eraser.is_clear_all_visible());
}

#[test]
fn toolbar_contract_properties_are_registered_with_their_source_names() {
    let _scope = test_scope();
    let properties = [
        InkToolbar::initial_controls_property().as_property(),
        InkToolbar::children_property().as_property(),
        InkToolbar::active_tool_property().as_property(),
        InkToolbar::ink_drawing_attributes_property().as_property(),
        InkToolbar::is_ruler_button_checked_property().as_property(),
        InkToolbar::target_ink_canvas_property().as_property(),
        InkToolbar::is_stencil_button_checked_property().as_property(),
        InkToolbar::button_flyout_placement_property().as_property(),
        InkToolbar::orientation_property().as_property(),
        InkToolbar::target_ink_presenter_property().as_property(),
    ];
    for (property, name) in properties.into_iter().zip([
        "InitialControls",
        "Children",
        "ActiveTool",
        "InkDrawingAttributes",
        "IsRulerButtonChecked",
        "TargetInkCanvas",
        "IsStencilButtonChecked",
        "ButtonFlyoutPlacement",
        "Orientation",
        "TargetInkPresenter",
    ]) {
        assert_eq!(property.name(), name);
    }
}

#[test]
fn toolbar_attribute_and_stencil_events_expose_updated_state_and_dispose() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    toolbar.set_target_ink_presenter(Some(InkPresenter::new()));
    let attributes = Rc::new(RefCell::new(Vec::new()));
    let copy = attributes.clone();
    let attributes_subscription = toolbar.ink_drawing_attributes_changed(move |toolbar| {
        copy.borrow_mut()
            .push(toolbar.ink_drawing_attributes().unwrap());
    });
    let pencil = toolbar.get_tool_button(InkToolbarTool::Pencil).unwrap();
    toolbar.set_active_tool(Some(pencil));
    assert_eq!(
        attributes.borrow().last().unwrap().kind,
        InkDrawingAttributesKind::Pencil
    );
    let count = attributes.borrow().len();
    attributes_subscription.dispose();
    toolbar.set_active_tool(toolbar.get_tool_button(InkToolbarTool::Highlighter));
    assert_eq!(attributes.borrow().len(), count);
    let states = Rc::new(RefCell::new(Vec::new()));
    let copy = states.clone();
    let subscription = toolbar.is_stencil_button_checked_changed(move |args| {
        copy.borrow_mut()
            .push((args.stencil_kind, args.stencil_button.is_checked()));
    });
    toolbar.set_is_stencil_button_checked(true);
    toolbar.set_is_stencil_button_checked(false);
    assert_eq!(
        *states.borrow(),
        vec![
            (InkToolbarStencilKind::Ruler, Some(true)),
            (InkToolbarStencilKind::Ruler, Some(false))
        ]
    );
    subscription.dispose();
    toolbar.set_is_stencil_button_checked(true);
    assert_eq!(states.borrow().len(), 2);
}

#[test]
fn stencil_and_flyout_item_property_transitions_match_source() {
    let _scope = test_scope();
    let stencil = InkToolbarStencilButton::new();
    assert!(stencil.is_ruler_item_visible());
    assert!(stencil.is_protractor_item_visible());
    for kind in [
        InkToolbarStencilKind::Ruler,
        InkToolbarStencilKind::Protractor,
    ] {
        stencil.set_selected_stencil(kind);
        assert_eq!(stencil.selected_stencil(), kind);
    }
    stencil.set_is_ruler_item_visible(false);
    assert!(!stencil.is_ruler_item_visible());
    stencil.set_is_ruler_item_visible(true);
    stencil.set_is_protractor_item_visible(false);
    assert!(!stencil.is_protractor_item_visible());
    stencil.set_is_protractor_item_visible(true);
    let item = InkToolbarFlyoutItem::new();
    for kind in [
        InkToolbarFlyoutItemKind::Simple,
        InkToolbarFlyoutItemKind::Radio,
        InkToolbarFlyoutItemKind::Check,
        InkToolbarFlyoutItemKind::RadioCheck,
    ] {
        item.set_kind(kind);
        assert_eq!(item.kind(), kind);
    }
    item.set_is_checked(Some(true));
    assert_eq!(item.is_checked(), Some(true));
    item.set_is_checked(Some(false));
    assert_eq!(item.is_checked(), Some(false));
}

#[test]
fn tool_peers_report_pen_roles_selected_names_and_palette_help() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    for tool in [
        InkToolbarTool::BallpointPen,
        InkToolbarTool::Pencil,
        InkToolbarTool::Highlighter,
    ] {
        let button = toolbar.get_tool_button(tool).unwrap();
        let automation = peer(&button);
        assert_eq!(
            automation.get_automation_control_type(),
            AutomationControlType::Button
        );
        assert_eq!(automation.get_localized_control_type(), "drop down button");
        let help = AutomationProperties::get_help_text(&button).unwrap();
        assert!(!help.is_empty());
        assert_eq!(
            automation.get_help_text(),
            format!("{help}, Open to choose a color from the palette")
        );
        AutomationProperties::set_help_text(&button, Some("Custom tool help"));
        assert_eq!(
            automation.get_help_text(),
            "Custom tool help, Open to choose a color from the palette"
        );
        button.clear_value(AutomationProperties::help_text_property());
        assert_eq!(
            automation.get_help_text(),
            "Open to choose a color from the palette"
        );
        assert_eq!(
            automation
                .get_provider::<dyn IExpandCollapseProvider>()
                .unwrap()
                .expand_collapse_state(),
            ExpandCollapseState::Collapsed
        );
    }
    let ballpoint = toolbar
        .get_tool_button(InkToolbarTool::BallpointPen)
        .unwrap();
    let pencil = toolbar.get_tool_button(InkToolbarTool::Pencil).unwrap();
    toolbar.set_active_tool(Some(ballpoint.clone()));
    assert!(peer(&ballpoint).get_name().contains("selected"));
    assert!(!peer(&pencil).get_name().contains("selected"));
    let selection = peer(&pencil)
        .get_provider::<dyn ISelectionItemProvider>()
        .unwrap();
    selection.select().unwrap();
    assert!(selection.is_selected());
    assert_eq!(toolbar.active_tool(), Some(pencil));
    assert!(!peer(&ballpoint).get_name().contains("selected"));
}

#[test]
fn provider_expand_selects_owner_and_respects_live_input_and_configuration_availability() {
    use ferroui_base::input::PointerType;
    use ferroui_base::Point;
    let _app = create_window_services();
    let toolbar = InkToolbar::new();
    let presenter = InkPresenter::new();
    toolbar.set_target_ink_presenter(Some(presenter.clone()));
    let window = shown_window(&toolbar);
    let pencil = toolbar.get_tool_button(InkToolbarTool::Pencil).unwrap();
    let provider = peer(&pencil)
        .get_provider::<dyn IExpandCollapseProvider>()
        .unwrap();
    assert_ne!(toolbar.active_tool(), Some(pencil.clone()));
    let sample = InkPointerEventArgs::new(
        77,
        PointerType::Pen,
        InkPoint::new(Point::new(1000.0, 1000.0), 0.5).unwrap(),
    );
    presenter
        .process_pointer(InkPointerPhase::Pressed, &sample)
        .unwrap();
    assert!(presenter.is_inking());
    provider.expand().unwrap();
    assert_eq!(toolbar.active_tool(), Some(pencil.clone()));
    assert!(presenter.is_inking());
    assert!(!pencil.flyout().unwrap().is_open());
    let stencil = toolbar
        .get_menu_button(InkToolbarMenuKind::Stencil)
        .unwrap();
    peer(&stencil)
        .get_provider::<dyn IExpandCollapseProvider>()
        .unwrap()
        .expand()
        .unwrap();
    assert_eq!(stencil.is_checked(), Some(true));
    assert!(!stencil.flyout().unwrap().is_open());
    presenter
        .process_pointer(InkPointerPhase::Lost, &sample)
        .unwrap();
    provider.expand().unwrap();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(pencil.flyout().unwrap().is_open());
    provider.collapse().unwrap();
    let custom_tool = InkToolbarCustomToolButton::new();
    let custom_pen = InkToolbarCustomPenButton::new();
    toolbar.children().add(&custom_tool);
    toolbar.children().add(&custom_pen);
    let eraser = toolbar
        .get_tool_button(InkToolbarTool::Eraser)
        .unwrap()
        .cast::<InkToolbarEraserButton>()
        .unwrap();
    eraser.set_is_clear_all_visible(false);
    for tool in [
        custom_tool.upcast::<InkToolbarToolButton>(),
        custom_pen.upcast::<InkToolbarToolButton>(),
        eraser.upcast::<InkToolbarToolButton>(),
    ] {
        peer(&tool)
            .get_provider::<dyn IExpandCollapseProvider>()
            .unwrap()
            .expand()
            .unwrap();
        assert_eq!(toolbar.active_tool(), Some(tool.clone()));
        assert!(tool.flyout().is_none_or(|flyout| !flyout.is_open()));
    }
    window.close();
}

#[test]
fn builtin_tool_names_and_tooltips_are_available_after_loading() {
    let _scope = test_scope();
    let toolbar = InkToolbar::new();
    let _root = attach(&toolbar, Size::new(500.0, 100.0));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(toolbar.is_loaded());
    for (tool, expected) in [
        (InkToolbarTool::BallpointPen, "Ballpoint pen"),
        (InkToolbarTool::Pencil, "Pencil"),
        (InkToolbarTool::Highlighter, "Highlighter"),
        (InkToolbarTool::Eraser, "Eraser"),
    ] {
        let button = toolbar.get_tool_button(tool).unwrap();
        button.measure(Size::new(1000.0, 1000.0));
        assert_eq!(
            AutomationProperties::get_name(&button).as_deref(),
            Some(expected)
        );
        let tooltip = ToolTip::get_tip(&button).unwrap();
        assert_eq!(
            tooltip.downcast_ref::<String>().map(String::as_str),
            Some(expected)
        );
    }
}

#[test]
fn menu_peer_opens_and_closes_real_framework_flyout_and_reports_item_selection() {
    let _app = create_window_services();
    let toolbar = InkToolbar::new();
    let window = shown_window(&toolbar);
    let button = toolbar
        .get_menu_button(InkToolbarMenuKind::Stencil)
        .unwrap()
        .cast::<InkToolbarStencilButton>()
        .unwrap();
    let automation = peer(&button);
    assert_eq!(
        automation.get_automation_control_type(),
        AutomationControlType::Button
    );
    assert_eq!(automation.get_localized_control_type(), "dropdown");
    assert!(automation.get_provider::<dyn IToggleProvider>().is_none());
    assert_eq!(automation.get_name(), "Measuring tools, Ruler");
    button.set_selected_stencil(InkToolbarStencilKind::Protractor);
    assert_eq!(automation.get_name(), "Measuring tools, Protractor");
    button.set_selected_stencil(InkToolbarStencilKind::Ruler);
    let provider = automation
        .get_provider::<dyn IExpandCollapseProvider>()
        .unwrap();
    assert_eq!(
        provider.expand_collapse_state(),
        ExpandCollapseState::Collapsed
    );
    provider.expand().unwrap();
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(
        provider.expand_collapse_state(),
        ExpandCollapseState::Expanded
    );
    assert_eq!(automation.get_name(), "Measuring tools, Ruler");
    assert!(automation.get_provider::<dyn IToggleProvider>().is_none());
    let items = stencil_items(&button);
    assert_eq!(items.len(), 2);
    for (index, item) in items.iter().enumerate() {
        assert_eq!(
            peer(item).get_automation_control_type(),
            AutomationControlType::MenuItem
        );
        assert_eq!(
            AutomationProperties::get_position_in_set(item),
            index as i32 + 1
        );
        assert_eq!(AutomationProperties::get_size_of_set(item), 2);
    }
    assert!(peer(&items[0])
        .get_provider::<dyn ISelectionItemProvider>()
        .unwrap()
        .is_selected());
    items[1].set_is_checked(Some(true));
    assert!(!peer(&items[0])
        .get_provider::<dyn ISelectionItemProvider>()
        .unwrap()
        .is_selected());
    assert!(peer(&items[1])
        .get_provider::<dyn ISelectionItemProvider>()
        .unwrap()
        .is_selected());
    provider.collapse().unwrap();
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(
        provider.expand_collapse_state(),
        ExpandCollapseState::Collapsed
    );
    window.close();
}

#[test]
fn single_stencil_peer_switches_patterns_and_cached_expand_becomes_noop() {
    let _scope = test_scope();
    let button = InkToolbarStencilButton::new();
    let _root = attach(&button, Size::new(200.0, 100.0));
    let automation = peer(&button);
    let cached = automation
        .get_provider::<dyn IExpandCollapseProvider>()
        .unwrap();
    let items = stencil_items(&button);
    for kind in [
        InkToolbarStencilKind::Ruler,
        InkToolbarStencilKind::Protractor,
    ] {
        button.set_selected_stencil(kind);
        button.set_is_ruler_item_visible(kind == InkToolbarStencilKind::Ruler);
        button.set_is_protractor_item_visible(kind == InkToolbarStencilKind::Protractor);
        assert_eq!(
            automation.get_automation_control_type(),
            AutomationControlType::Button
        );
        assert_eq!(automation.get_localized_control_type(), "button");
        assert_eq!(automation.get_name(), format!("Measuring tools, {kind:?}"));
        assert!(automation
            .get_provider::<dyn IExpandCollapseProvider>()
            .is_none());
        assert_eq!(
            cached.expand_collapse_state(),
            ExpandCollapseState::LeafNode
        );
        let old = button.is_checked();
        cached.expand().unwrap();
        assert_eq!(button.is_checked(), old);
        assert_eq!(
            cached.expand_collapse_state(),
            ExpandCollapseState::LeafNode
        );
        let toggle = automation.get_provider::<dyn IToggleProvider>().unwrap();
        button.set_is_checked(Some(true));
        assert_eq!(toggle.toggle_state(), ToggleState::On);
        button.set_is_checked(Some(false));
        assert_eq!(toggle.toggle_state(), ToggleState::Off);
        for (index, item) in items.iter().enumerate() {
            let visible = index
                == if kind == InkToolbarStencilKind::Ruler {
                    0
                } else {
                    1
                };
            assert_eq!(item.is_visible(), visible);
            if visible {
                assert_eq!(AutomationProperties::get_position_in_set(item), 1);
                assert_eq!(AutomationProperties::get_size_of_set(item), 1);
            }
        }
    }
    button.set_is_ruler_item_visible(true);
    button.set_is_protractor_item_visible(true);
    assert_eq!(automation.get_localized_control_type(), "dropdown");
    assert!(automation.get_provider::<dyn IToggleProvider>().is_none());
    assert_eq!(
        automation
            .get_provider::<dyn IExpandCollapseProvider>()
            .unwrap()
            .expand_collapse_state(),
        ExpandCollapseState::Collapsed
    );
}

#[test]
fn flyout_peer_invocation_and_radio_selection_preserve_source_kind_rules() {
    let _scope = test_scope();
    for kind in [
        InkToolbarFlyoutItemKind::Radio,
        InkToolbarFlyoutItemKind::RadioCheck,
    ] {
        let item = InkToolbarFlyoutItem::new();
        item.set_kind(kind);
        item.set_content(Some(Rc::new("Ruler".to_owned())));
        let automation = peer(&item);
        assert_eq!(
            automation.get_automation_control_type(),
            AutomationControlType::MenuItem
        );
        assert_eq!(automation.get_name(), "Ruler");
        let selection = automation
            .get_provider::<dyn ISelectionItemProvider>()
            .unwrap();
        assert!(!selection.is_selected());
        item.set_is_checked(Some(true));
        assert!(selection.is_selected());
        item.set_is_checked(Some(false));
        assert!(!selection.is_selected());
        selection.select().unwrap();
        selection.select().unwrap();
        assert!(selection.is_selected());
        item.set_is_checked(Some(false));
        let invoke = automation.get_provider::<dyn IInvokeProvider>().unwrap();
        invoke.invoke().unwrap();
        assert!(selection.is_selected());
        invoke.invoke().unwrap();
        assert_eq!(
            selection.is_selected(),
            kind == InkToolbarFlyoutItemKind::Radio
        );
        assert_eq!(automation.get_name(), "Ruler");
        item.set_is_enabled(false);
        assert!(invoke.invoke().is_err());
        assert!(selection.select().is_err());
    }
    for kind in [
        InkToolbarFlyoutItemKind::Simple,
        InkToolbarFlyoutItemKind::Check,
    ] {
        let item = InkToolbarFlyoutItem::new();
        item.set_kind(kind);
        let automation = peer(&item);
        assert!(automation
            .get_provider::<dyn ISelectionItemProvider>()
            .is_none());
        automation
            .get_provider::<dyn IInvokeProvider>()
            .unwrap()
            .invoke()
            .unwrap();
        assert_eq!(
            item.is_checked() == Some(true),
            kind == InkToolbarFlyoutItemKind::Check
        );
    }
}

#[test]
fn eraser_and_custom_tool_peers_keep_their_non_palette_roles_and_help() {
    let _scope = test_scope();
    for button in [
        InkToolbarEraserButton::new().upcast::<InkToolbarToolButton>(),
        InkToolbarCustomToolButton::new().upcast(),
    ] {
        let automation = peer(&button);
        assert_eq!(
            automation.get_automation_control_type(),
            AutomationControlType::Custom
        );
        assert_eq!(automation.get_localized_control_type(), "button");
        AutomationProperties::set_help_text(&button, Some("Custom tool help"));
        assert_eq!(automation.get_help_text(), "Custom tool help");
        assert_eq!(
            automation
                .get_provider::<dyn IExpandCollapseProvider>()
                .unwrap()
                .expand_collapse_state(),
            ExpandCollapseState::Collapsed
        );
    }
}

#[test]
fn width_value_provider_clamps_and_announces_absolute_range_and_value_changes() {
    let _scope = test_scope();
    let slider = InkToolbarStrokeWidthSlider::new();
    slider.set_minimum(2.0);
    slider.set_maximum(9.0);
    slider.set_range_value(5.0);
    let automation = peer(&slider);
    let value = automation.get_provider::<dyn IValueProvider>().unwrap();
    assert!(!value.is_read_only());
    assert_eq!(value.value().as_deref(), Some("5"));
    let events = Rc::new(Cell::new(0));
    let count = events.clone();
    automation.property_changed(move |event| {
        if event.property() == ValuePatternIdentifiers::value_property() {
            count.set(count.get() + 1);
        }
    });
    value.set_value(Some("7")).unwrap();
    assert_eq!(slider.value(), 7.0);
    assert_eq!(value.value().as_deref(), Some("7"));
    value.set_value(Some("100")).unwrap();
    assert_eq!(slider.value(), 9.0);
    value.set_value(Some("-5")).unwrap();
    assert_eq!(slider.value(), 2.0);
    assert_eq!(events.get(), 3);
    assert!(automation.get_name().contains('2'));
    assert!(automation.get_name().contains('9'));
    assert!(value.set_value(Some("invalid")).is_err());
}

#[test]
fn toolbar_arrow_focus_wraps_skips_hidden_disabled_without_selecting_tools() {
    let _app = create_window_services();
    let toolbar = InkToolbar::new();
    let window = shown_window(&toolbar);
    let children = toolbar.children().snapshot();
    let first = &children[0];
    let last = children.last().unwrap();
    children[1].set_is_visible(false);
    children[2].set_is_enabled(false);
    assert!(first.focus());
    let active = toolbar.active_tool();
    let mut key = KeyEventArgs::new();
    key.key = Key::Right;
    key.set_routed_event(Some(InputElement::key_down_event()));
    first.raise_event(&key);
    assert!(key.handled());
    assert!(children[3].is_focused());
    assert_eq!(toolbar.active_tool(), active);
    assert!(last.focus());
    let mut key = KeyEventArgs::new();
    key.key = Key::Down;
    key.set_routed_event(Some(InputElement::key_down_event()));
    last.raise_event(&key);
    assert!(first.is_focused());
    assert_eq!(toolbar.active_tool(), active);
    let mut key = KeyEventArgs::new();
    key.key = Key::Left;
    key.set_routed_event(Some(InputElement::key_down_event()));
    first.raise_event(&key);
    assert!(last.is_focused());
    window.close();
}
