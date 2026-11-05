//! The InkCanvas page: the three source examples on the native ink controls, driven through their
//! options and through ink input, as a user does. No WinUI visual-parity claim.

use crate::{examples, tests::start};
use ferroui_base::input::PointerType;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::media::Color;
use ferroui_base::{Point, Ref};
use ferroui_controls::{Button, CheckBox, ComboBox, Control, Grid, Slider, TextBlock, ToggleSwitch};
use ferroui_controls_inking::{
    CoreInputDeviceTypes, InkCanvas, InkInputProcessingMode, InkPoint, InkPointerEventArgs,
    InkPointerPhase, InkToolbar, InkToolbarButtonFlyoutPlacement, InkToolbarTool,
    InkToolbarToolButton,
};

fn page() -> Ref<Control> {
    examples::create(examples::find("InkCanvas").unwrap()).unwrap()
}

fn click(root: &Ref<Control>, name: &str) {
    root.get_control::<Button>(name)
        .raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
}

/// A stroke drawn with `device` through the presenter of `canvas`, as the canvas forwards input.
/// Returns whether the presenter consumed the press.
fn draw(canvas: &InkCanvas, id: i32, device: PointerType, y: f64) -> bool {
    let presenter = canvas.ink_presenter();
    let event = |x: f64| InkPointerEventArgs::new(id, device, InkPoint::new(Point::new(x, y), 0.5).unwrap());
    let consumed = presenter
        .process_pointer(InkPointerPhase::Pressed, &event(10.0))
        .unwrap();
    presenter
        .process_pointer(InkPointerPhase::Moved, &event(40.0))
        .unwrap();
    presenter
        .process_pointer(InkPointerPhase::Released, &event(80.0))
        .unwrap();
    consumed
}

fn all_devices() -> CoreInputDeviceTypes {
    CoreInputDeviceTypes::PEN | CoreInputDeviceTypes::MOUSE | CoreInputDeviceTypes::TOUCH
}

#[test]
fn ink_canvas_page_loads_three_examples_on_native_ink_controls() {
    let _application = start();
    let root = page();
    for index in 1..=3 {
        assert!(root
            .find_control::<Grid>(&format!("InkCanvasExample{index}"))
            .is_some());
    }
    // The constructor of the source page: every option applied once.
    let drawing = root.get_control::<InkCanvas>("drawingCanvas").ink_presenter();
    assert_eq!(drawing.input_device_types(), all_devices());
    let attributes = drawing.copy_default_drawing_attributes();
    assert_eq!(attributes.color, Color::from_rgb(0, 0, 0));
    assert_eq!((attributes.size.width, attributes.size.height), (4.0, 4.0));
    assert!(drawing.is_input_enabled());
    for name in ["toolbarCanvas", "strokesCanvas"] {
        assert_eq!(
            root.get_control::<InkCanvas>(name).ink_presenter().input_device_types(),
            all_devices()
        );
    }
    // The toolbar targets its canvas and starts with the ballpoint pen.
    let toolbar = root.get_control::<InkToolbar>("sampleToolbar");
    assert!(toolbar.target_ink_canvas() == Some(root.get_control::<InkCanvas>("toolbarCanvas")));
    assert_eq!(
        toolbar.active_tool().map(|tool| tool.tool_kind()),
        Some(InkToolbarTool::BallpointPen)
    );
    assert_eq!(root.get_control::<ComboBox>("activeTool").selected_index(), 0);
    assert_eq!(
        root.get_control::<TextBlock>("strokeCount").text().as_deref(),
        Some("0 stroke(s) on canvas")
    );
    assert!(!root.get_control::<Button>("restoreButton").is_enabled());
}

#[test]
fn ink_drawing_options_set_devices_attributes_enabled_state_and_clear() {
    let _application = start();
    let root = page();
    let canvas = root.get_control::<InkCanvas>("drawingCanvas");
    let presenter = canvas.ink_presenter();

    root.get_control::<CheckBox>("penInput").set_is_checked(Some(false));
    assert_eq!(
        presenter.input_device_types(),
        CoreInputDeviceTypes::MOUSE | CoreInputDeviceTypes::TOUCH
    );
    // The pen is no longer an input device of the canvas; the mouse still is.
    assert!(!draw(&canvas, 1, PointerType::Pen, 20.0));
    assert!(presenter.stroke_container().is_empty());
    assert!(draw(&canvas, 2, PointerType::Mouse, 20.0));
    assert_eq!(presenter.stroke_container().len(), 1);
    root.get_control::<CheckBox>("touchInput").set_is_checked(Some(false));
    assert_eq!(presenter.input_device_types(), CoreInputDeviceTypes::MOUSE);
    root.get_control::<CheckBox>("penInput").set_is_checked(Some(true));
    assert_eq!(
        presenter.input_device_types(),
        CoreInputDeviceTypes::PEN | CoreInputDeviceTypes::MOUSE
    );

    root.get_control::<ComboBox>("inkColor").set_selected_index(1);
    root.get_control::<Slider>("strokeWidth").set_range_value(9.0);
    let attributes = presenter.copy_default_drawing_attributes();
    assert_eq!(attributes.color, Color::from_rgb(255, 0, 0));
    assert_eq!((attributes.size.width, attributes.size.height), (9.0, 9.0));
    root.get_control::<ComboBox>("inkColor").set_selected_index(2);
    assert_eq!(
        presenter.copy_default_drawing_attributes().color,
        Color::from_rgb(0, 0, 255)
    );
    // A stroke takes the attributes of the presenter when it is drawn.
    assert!(draw(&canvas, 3, PointerType::Mouse, 60.0));
    let strokes = presenter.stroke_container().get_strokes();
    assert_eq!(strokes.last().unwrap().drawing_attributes().color, Color::from_rgb(0, 0, 255));

    root.get_control::<ToggleSwitch>("inkingEnabled").set_is_checked(Some(false));
    assert!(!presenter.is_input_enabled());
    assert!(!draw(&canvas, 4, PointerType::Mouse, 100.0));
    assert_eq!(presenter.stroke_container().len(), 2);
    root.get_control::<ToggleSwitch>("inkingEnabled").set_is_checked(Some(true));
    assert!(presenter.is_input_enabled());

    click(&root, "clearDrawing");
    assert!(presenter.stroke_container().is_empty());
}

#[test]
fn ink_toolbar_options_show_hide_and_select_tools_both_ways() {
    let _application = start();
    let root = page();
    let toolbar = root.get_control::<InkToolbar>("sampleToolbar");
    let active_tool = root.get_control::<ComboBox>("activeTool");
    let kind = || toolbar.active_tool().map(|tool| tool.tool_kind());
    let highlighter = root.get_control::<InkToolbarToolButton>("highlighterButton");
    let eraser = root.get_control::<InkToolbarToolButton>("eraserButton");

    // The active-tool option selects the tool of the toolbar.
    active_tool.set_selected_index(2);
    assert_eq!(kind(), Some(InkToolbarTool::Highlighter));
    // Hiding the active tool falls back to the ballpoint pen, and the option follows.
    root.get_control::<CheckBox>("showHighlighter").set_is_checked(Some(false));
    assert!(!highlighter.is_visible());
    assert_eq!(kind(), Some(InkToolbarTool::BallpointPen));
    assert_eq!(active_tool.selected_index(), 0);

    // Requesting a hidden tool shows it again and selects it; the eraser switches the target
    // canvas to erasing.
    root.get_control::<CheckBox>("showEraser").set_is_checked(Some(false));
    assert!(!eraser.is_visible());
    active_tool.set_selected_index(3);
    assert_eq!(root.get_control::<CheckBox>("showEraser").is_checked(), Some(true));
    assert!(eraser.is_visible());
    assert_eq!(kind(), Some(InkToolbarTool::Eraser));
    assert_eq!(
        root.get_control::<InkCanvas>("toolbarCanvas")
            .ink_presenter()
            .input_processing_configuration()
            .mode(),
        InkInputProcessingMode::Erasing
    );

    // A tool the user picks on the toolbar updates the option (ActiveToolChanged).
    toolbar.set_active_tool(Some(root.get_control::<InkToolbarToolButton>("pencilButton")));
    assert_eq!(active_tool.selected_index(), 1);
    assert!(!highlighter.is_visible());

    for (index, placement) in [
        (1, InkToolbarButtonFlyoutPlacement::Top),
        (2, InkToolbarButtonFlyoutPlacement::Bottom),
        (3, InkToolbarButtonFlyoutPlacement::Left),
        (4, InkToolbarButtonFlyoutPlacement::Right),
        (0, InkToolbarButtonFlyoutPlacement::Auto),
    ] {
        root.get_control::<ComboBox>("buttonFlyoutPlacement").set_selected_index(index);
        assert_eq!(toolbar.button_flyout_placement(), placement);
    }
}

#[test]
fn ink_toolbar_clear_button_erases_the_strokes_of_its_canvas() {
    let _application = start();
    let root = page();
    let canvas = root.get_control::<InkCanvas>("toolbarCanvas");
    assert!(draw(&canvas, 1, PointerType::Touch, 20.0));
    assert_eq!(canvas.ink_presenter().stroke_container().len(), 1);
    click(&root, "clearToolbar");
    assert!(canvas.ink_presenter().stroke_container().is_empty());
}

#[test]
fn ink_strokes_save_clear_and_restore_keep_the_source_messages() {
    let _application = start();
    let root = page();
    let canvas = root.get_control::<InkCanvas>("strokesCanvas");
    let count = root.get_control::<TextBlock>("strokeCount");
    let status = root.get_control::<TextBlock>("strokesStatus");
    let restore = root.get_control::<Button>("restoreButton");
    let text = |block: &Ref<TextBlock>| block.text().unwrap_or_default();

    click(&root, "saveDrawing");
    assert_eq!(text(&status), "Draw something before saving.");
    click(&root, "restoreButton");
    assert_eq!(text(&status), "Save a drawing before restoring it.");

    // StrokesCollected updates the count.
    assert!(draw(&canvas, 1, PointerType::Mouse, 20.0));
    assert!(draw(&canvas, 2, PointerType::Pen, 60.0));
    assert_eq!(text(&count), "2 stroke(s) on canvas");

    click(&root, "saveDrawing");
    assert_eq!(text(&status), "Saved 2 stroke(s). Clear the canvas to restore them.");
    assert!(!restore.is_enabled());
    click(&root, "restoreButton");
    assert_eq!(text(&status), "Clear the canvas before restoring saved ink.");

    click(&root, "clearStrokes");
    assert_eq!(text(&count), "0 stroke(s) on canvas");
    assert_eq!(text(&status), "Canvas cleared. 2 saved stroke(s) are ready to restore.");
    assert!(restore.is_enabled());

    click(&root, "restoreButton");
    assert_eq!(text(&status), "Restored 2 stroke(s).");
    assert_eq!(text(&count), "2 stroke(s) on canvas");
    assert_eq!(canvas.ink_presenter().stroke_container().len(), 2);
    assert!(!restore.is_enabled());
}

#[test]
fn ink_page_releases_its_controls_with_the_page() {
    let _application = start();
    let root = page();
    let weak_canvas = root.get_control::<InkCanvas>("strokesCanvas").downgrade();
    let weak_toolbar = root.get_control::<InkToolbar>("sampleToolbar").downgrade();
    drop(root);
    assert!(weak_canvas.upgrade().is_none());
    assert!(weak_toolbar.upgrade().is_none());
}
