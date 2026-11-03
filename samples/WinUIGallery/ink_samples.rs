//! The handlers of WinUI Gallery's `InkCanvasPage.xaml.cs` (14061eea; see NOTICE.md) in Rust, on
//! the native InkCanvas, InkPresenter and InkToolbar of `ferroui-controls-inking`. The controls own
//! input, drying, erasing and the toolbar tools; the sample owns only the behaviour of its options.
//!
//! The source saves the strokes with `InkStrokeContainer.SaveAsync` into an
//! `InMemoryRandomAccessStream`; the port keeps the saved strokes in memory as the bytes of
//! `InkStrokeContainer::save` (the portable format of the crate) and restores them with
//! `InkStrokeContainer::load`.

use ferroui_base::media::Color;
use ferroui_base::{ObjectType, Ref, Size, WeakRef};
use ferroui_controls::primitives::{RangeBase, SelectingItemsControl, ToggleButton};
use ferroui_controls::{Button, CheckBox, ComboBox, Control, Slider, TextBlock, ToggleSwitch};
use ferroui_controls_inking::{
    CoreInputDeviceTypes, InkCanvas, InkToolbar, InkToolbarButtonFlyoutPlacement, InkToolbarTool,
    InkToolbarToolButton,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `CoreInputDeviceTypes.Pen | CoreInputDeviceTypes.Mouse | CoreInputDeviceTypes.Touch`.
const ALL_INPUT_DEVICES: CoreInputDeviceTypes = CoreInputDeviceTypes::PEN
    .union(CoreInputDeviceTypes::MOUSE)
    .union(CoreInputDeviceTypes::TOUCH);

pub(crate) fn wire(root: &Ref<Control>) {
    let Some(drawing) = find::<InkCanvas>(root, "drawingCanvas") else {
        return;
    };
    wire_drawing(root, &drawing);
    wire_toolbar(root);
    wire_strokes(root);
}

fn find<T: ObjectType>(root: &Ref<Control>, name: &str) -> Option<Ref<T>> {
    root.find_control::<Control>(name)
        .and_then(|control| control.cast())
}

fn is_checked_change(change: &ferroui_base::FerroPropertyChangedEventArgs<'_>) -> bool {
    change.property() == ToggleButton::is_checked_property().as_property()
}

fn is_selection_change(change: &ferroui_base::FerroPropertyChangedEventArgs<'_>) -> bool {
    change.property() == SelectingItemsControl::selected_index_property().as_property()
}

// --- Example 1: draw with an InkCanvas ----------------------------------------------------------

fn wire_drawing(root: &Ref<Control>, canvas: &Ref<InkCanvas>) {
    let pen = root.get_control::<CheckBox>("penInput");
    let mouse = root.get_control::<CheckBox>("mouseInput");
    let touch = root.get_control::<CheckBox>("touchInput");
    let color = root.get_control::<ComboBox>("inkColor");
    let width = root.get_control::<Slider>("strokeWidth");
    let enabled = root.get_control::<ToggleSwitch>("inkingEnabled");

    // ApplyInputDevices.
    let apply_devices: Rc<dyn Fn()> = {
        let canvas = canvas.downgrade();
        let (pen, mouse, touch) = (pen.downgrade(), mouse.downgrade(), touch.downgrade());
        Rc::new(move || {
            let (Some(canvas), Some(pen), Some(mouse), Some(touch)) =
                (canvas.upgrade(), pen.upgrade(), mouse.upgrade(), touch.upgrade())
            else {
                return;
            };
            let mut types = CoreInputDeviceTypes::NONE;
            if pen.is_checked() == Some(true) {
                types |= CoreInputDeviceTypes::PEN;
            }
            if mouse.is_checked() == Some(true) {
                types |= CoreInputDeviceTypes::MOUSE;
            }
            if touch.is_checked() == Some(true) {
                types |= CoreInputDeviceTypes::TOUCH;
            }
            canvas.ink_presenter().set_input_device_types(types);
        })
    };
    for check in [&pen, &mouse, &touch] {
        let apply = apply_devices.clone();
        check.property_changed(move |change| {
            if is_checked_change(change) {
                apply();
            }
        });
    }

    // ApplyDrawingAttributes.
    let apply_attributes: Rc<dyn Fn()> = {
        let canvas = canvas.downgrade();
        let (color, width) = (color.downgrade(), width.downgrade());
        Rc::new(move || {
            let (Some(canvas), Some(color), Some(width)) = (canvas.upgrade(), color.upgrade(), width.upgrade())
            else {
                return;
            };
            let color = match color.selected_index() {
                0 => Color::from_rgb(0, 0, 0),
                1 => Color::from_rgb(255, 0, 0),
                2 => Color::from_rgb(0, 0, 255),
                // The source throws on an unexpected selection; the port keeps the attributes.
                _ => return,
            };
            let presenter = canvas.ink_presenter();
            let mut attributes = presenter.copy_default_drawing_attributes();
            attributes.color = color;
            attributes.size = Size::new(width.value(), width.value());
            // The slider keeps the width in 1..12, which the attributes accept.
            let _ = presenter.update_default_drawing_attributes(&attributes);
        })
    };
    {
        let apply = apply_attributes.clone();
        color.property_changed(move |change| {
            if is_selection_change(change) {
                apply();
            }
        });
    }
    {
        let apply = apply_attributes.clone();
        width.property_changed(move |change| {
            if change.property() == RangeBase::value_property().as_property() {
                apply();
            }
        });
    }

    // OnInkingEnabledChanged.
    {
        let canvas = canvas.downgrade();
        let weak_enabled = enabled.downgrade();
        enabled.property_changed(move |change| {
            if !is_checked_change(change) {
                return;
            }
            if let (Some(canvas), Some(enabled)) = (canvas.upgrade(), weak_enabled.upgrade()) {
                canvas.ink_presenter().set_is_input_enabled(enabled.is_checked() == Some(true));
            }
        });
    }

    // OnClearDrawingClick.
    {
        let canvas = canvas.downgrade();
        root.get_control::<Button>("clearDrawing").click(move |_, _| {
            if let Some(canvas) = canvas.upgrade() {
                canvas.ink_presenter().stroke_container().clear();
            }
        });
    }

    // The end of the constructor of the source page.
    apply_devices();
    apply_attributes();
    canvas.ink_presenter().set_is_input_enabled(enabled.is_checked() == Some(true));
}

// --- Example 2: customize an InkToolbar ---------------------------------------------------------

/// The state of the toolbar example: the source's `_updatingToolbarOptions` and the named
/// elements, held weakly (the handlers of the elements hold this state).
struct ToolbarSample {
    toolbar: WeakRef<InkToolbar>,
    ballpoint: WeakRef<InkToolbarToolButton>,
    pencil: WeakRef<InkToolbarToolButton>,
    highlighter: WeakRef<InkToolbarToolButton>,
    eraser: WeakRef<InkToolbarToolButton>,
    show_pencil: WeakRef<CheckBox>,
    show_highlighter: WeakRef<CheckBox>,
    show_eraser: WeakRef<CheckBox>,
    active_tool: WeakRef<ComboBox>,
    updating: Cell<bool>,
}

impl ToolbarSample {
    fn is_shown(check: &WeakRef<CheckBox>) -> bool {
        check
            .upgrade()
            .is_some_and(|check| check.is_checked() == Some(true))
    }

    /// `UpdateToolbarButtons`: shows the checked tools, selects the requested tool (the ballpoint
    /// pen when it is hidden), then hides the unchecked tools.
    fn update_toolbar_buttons(&self, requested: InkToolbarTool) {
        let Some(toolbar) = self.toolbar.upgrade() else {
            return;
        };
        let pencil = Self::is_shown(&self.show_pencil);
        let highlighter = Self::is_shown(&self.show_highlighter);
        let eraser = Self::is_shown(&self.show_eraser);
        let selected = match requested {
            InkToolbarTool::Pencil if pencil => &self.pencil,
            InkToolbarTool::Highlighter if highlighter => &self.highlighter,
            InkToolbarTool::Eraser if eraser => &self.eraser,
            _ => &self.ballpoint,
        };
        let Some(selected) = selected.upgrade() else {
            return;
        };
        let buttons = [
            (&self.pencil, pencil),
            (&self.highlighter, highlighter),
            (&self.eraser, eraser),
        ];
        for (button, shown) in buttons {
            if let (true, Some(button)) = (shown, button.upgrade()) {
                button.set_is_visible(true);
            }
        }
        if toolbar.active_tool().as_ref() != Some(&selected) {
            toolbar.set_active_tool(Some(selected));
        }
        for (button, shown) in buttons {
            if let (false, Some(button)) = (shown, button.upgrade()) {
                button.set_is_visible(false);
            }
        }
    }

    /// `SyncActiveToolSelection`: the active-tool option follows the active tool of the toolbar.
    fn sync_active_tool_selection(&self) {
        let (Some(toolbar), Some(active_tool)) = (self.toolbar.upgrade(), self.active_tool.upgrade()) else {
            return;
        };
        let index = match toolbar.active_tool().map(|tool| tool.tool_kind()) {
            Some(InkToolbarTool::BallpointPen) => 0,
            Some(InkToolbarTool::Pencil) => 1,
            Some(InkToolbarTool::Highlighter) => 2,
            Some(InkToolbarTool::Eraser) => 3,
            _ => -1,
        };
        if index >= 0 && active_tool.selected_index() != index {
            self.updating.set(true);
            active_tool.set_selected_index(index);
            self.updating.set(false);
        }
    }

    /// `OnToolbarToolsChanged`.
    fn tools_changed(&self) {
        if self.updating.get() {
            return;
        }
        let Some(toolbar) = self.toolbar.upgrade() else {
            return;
        };
        let previously_active = toolbar
            .active_tool()
            .map_or(InkToolbarTool::BallpointPen, |tool| tool.tool_kind());
        self.updating.set(true);
        self.update_toolbar_buttons(previously_active);
        self.updating.set(false);
        self.sync_active_tool_selection();
    }

    /// `OnActiveToolSelectionChanged`: the requested tool is shown, then selected.
    fn active_tool_selection_changed(&self) {
        let Some(active_tool) = self.active_tool.upgrade() else {
            return;
        };
        if self.updating.get() || active_tool.selected_index() < 0 {
            return;
        }
        let requested = match active_tool.selected_index() {
            0 => InkToolbarTool::BallpointPen,
            1 => InkToolbarTool::Pencil,
            2 => InkToolbarTool::Highlighter,
            3 => InkToolbarTool::Eraser,
            _ => return,
        };
        self.updating.set(true);
        let show = match requested {
            InkToolbarTool::Pencil => Some(&self.show_pencil),
            InkToolbarTool::Highlighter => Some(&self.show_highlighter),
            InkToolbarTool::Eraser => Some(&self.show_eraser),
            _ => None,
        };
        if let Some(show) = show.and_then(WeakRef::upgrade) {
            show.set_is_checked(Some(true));
        }
        self.update_toolbar_buttons(requested);
        self.updating.set(false);
        self.sync_active_tool_selection();
    }

    /// `OnToolbarLoaded`.
    fn toolbar_loaded(&self) {
        let (Some(toolbar), Some(ballpoint)) = (self.toolbar.upgrade(), self.ballpoint.upgrade()) else {
            return;
        };
        if toolbar.active_tool().is_none() {
            toolbar.set_active_tool(Some(ballpoint));
        }
        self.sync_active_tool_selection();
    }
}

fn wire_toolbar(root: &Ref<Control>) {
    let toolbar = root.get_control::<InkToolbar>("sampleToolbar");
    let canvas = root.get_control::<InkCanvas>("toolbarCanvas");
    canvas.ink_presenter().set_input_device_types(ALL_INPUT_DEVICES);
    // TargetInkCanvas="{x:Bind toolbarCanvas}".
    toolbar.set_target_ink_canvas(Some(canvas.clone()));
    let show_pencil = root.get_control::<CheckBox>("showPencil");
    let show_highlighter = root.get_control::<CheckBox>("showHighlighter");
    let show_eraser = root.get_control::<CheckBox>("showEraser");
    let active_tool = root.get_control::<ComboBox>("activeTool");
    let sample = Rc::new(ToolbarSample {
        toolbar: toolbar.downgrade(),
        ballpoint: root.get_control::<InkToolbarToolButton>("ballpointButton").downgrade(),
        pencil: root.get_control::<InkToolbarToolButton>("pencilButton").downgrade(),
        highlighter: root.get_control::<InkToolbarToolButton>("highlighterButton").downgrade(),
        eraser: root.get_control::<InkToolbarToolButton>("eraserButton").downgrade(),
        show_pencil: show_pencil.downgrade(),
        show_highlighter: show_highlighter.downgrade(),
        show_eraser: show_eraser.downgrade(),
        active_tool: active_tool.downgrade(),
        updating: Cell::new(false),
    });
    for check in [&show_pencil, &show_highlighter, &show_eraser] {
        let sample = sample.clone();
        check.property_changed(move |change| {
            if is_checked_change(change) {
                sample.tools_changed();
            }
        });
    }
    {
        let sample = sample.clone();
        active_tool.property_changed(move |change| {
            if is_selection_change(change) {
                sample.active_tool_selection_changed();
            }
        });
    }
    {
        let sample = sample.clone();
        // The subscription lives with the toolbar, as the source's event handler does.
        let _ = toolbar.active_tool_changed(move |_| {
            if !sample.updating.get() {
                sample.sync_active_tool_selection();
            }
        });
    }
    {
        let sample = sample.clone();
        toolbar.loaded(move |_, _| sample.toolbar_loaded());
    }
    // OnButtonFlyoutPlacementChanged.
    {
        let placement = root.get_control::<ComboBox>("buttonFlyoutPlacement");
        let weak_toolbar = toolbar.downgrade();
        let weak_placement = placement.downgrade();
        placement.property_changed(move |change| {
            if !is_selection_change(change) {
                return;
            }
            let (Some(toolbar), Some(placement)) = (weak_toolbar.upgrade(), weak_placement.upgrade()) else {
                return;
            };
            let value = match placement.selected_index() {
                0 => InkToolbarButtonFlyoutPlacement::Auto,
                1 => InkToolbarButtonFlyoutPlacement::Top,
                2 => InkToolbarButtonFlyoutPlacement::Bottom,
                3 => InkToolbarButtonFlyoutPlacement::Left,
                4 => InkToolbarButtonFlyoutPlacement::Right,
                _ => return,
            };
            toolbar.set_button_flyout_placement(value);
        });
    }
    // OnClearToolbarClick.
    {
        let canvas = canvas.downgrade();
        root.get_control::<Button>("clearToolbar").click(move |_, _| {
            if let Some(canvas) = canvas.upgrade() {
                canvas.ink_presenter().stroke_container().clear();
            }
        });
    }
    // The toolbar selects a tool when it is loaded; wiring follows it at once as well, so that
    // the options and the toolbar agree before the page is shown.
    sample.toolbar_loaded();
}

// --- Example 3: save and restore ink strokes ----------------------------------------------------

/// The state of the strokes example: the source's `_savedInk` and `_savedStrokeCount`.
struct StrokesSample {
    canvas: WeakRef<InkCanvas>,
    count: WeakRef<TextBlock>,
    status: WeakRef<TextBlock>,
    restore: WeakRef<Button>,
    saved: RefCell<Option<Vec<u8>>>,
    saved_count: Cell<usize>,
}

impl StrokesSample {
    fn set_status(&self, text: &str) {
        if let Some(status) = self.status.upgrade() {
            status.set_text(Some(text));
        }
    }

    fn stroke_count(&self) -> usize {
        self.canvas
            .upgrade()
            .map_or(0, |canvas| canvas.ink_presenter().stroke_container().get_strokes().len())
    }

    /// `UpdateSavedStrokeCount`.
    fn update_saved_stroke_count(&self) {
        let count = self.stroke_count();
        if let Some(label) = self.count.upgrade() {
            label.set_text(Some(&format!("{count} stroke(s) on canvas")));
        }
        if let Some(restore) = self.restore.upgrade() {
            restore.set_is_enabled(self.saved.borrow().is_some() && count == 0);
        }
    }

    /// `OnSaveDrawingClick`.
    fn save(&self) {
        let Some(canvas) = self.canvas.upgrade() else {
            return;
        };
        let container = canvas.ink_presenter().stroke_container();
        let count = container.get_strokes().len();
        if count == 0 {
            self.set_status("Draw something before saving.");
            return;
        }
        match container.save() {
            Ok(data) => {
                *self.saved.borrow_mut() = Some(data);
                self.saved_count.set(count);
                self.update_saved_stroke_count();
                self.set_status(&format!("Saved {count} stroke(s). Clear the canvas to restore them."));
            }
            Err(error) => self.set_status(&format!("The drawing could not be saved: {error}")),
        }
    }

    /// `OnClearStrokesClick`.
    fn clear(&self) {
        if let Some(canvas) = self.canvas.upgrade() {
            canvas.ink_presenter().stroke_container().clear();
        }
        self.update_saved_stroke_count();
        let text = if self.saved.borrow().is_none() {
            "Canvas cleared. Draw and save something to restore it.".to_owned()
        } else {
            format!(
                "Canvas cleared. {} saved stroke(s) are ready to restore.",
                self.saved_count.get()
            )
        };
        self.set_status(&text);
    }

    /// `OnRestoreDrawingClick`.
    fn restore(&self) {
        let Some(saved) = self.saved.borrow().clone() else {
            self.set_status("Save a drawing before restoring it.");
            return;
        };
        let Some(canvas) = self.canvas.upgrade() else {
            return;
        };
        let container = canvas.ink_presenter().stroke_container();
        if !container.get_strokes().is_empty() {
            self.set_status("Clear the canvas before restoring saved ink.");
            return;
        }
        let result = container.load(&saved);
        self.update_saved_stroke_count();
        match result {
            Ok(()) => self.set_status(&format!("Restored {} stroke(s).", container.get_strokes().len())),
            Err(error) => self.set_status(&format!("The saved drawing could not be restored: {error}")),
        }
    }
}

fn wire_strokes(root: &Ref<Control>) {
    let canvas = root.get_control::<InkCanvas>("strokesCanvas");
    let presenter = canvas.ink_presenter();
    presenter.set_input_device_types(ALL_INPUT_DEVICES);
    let sample = Rc::new(StrokesSample {
        canvas: canvas.downgrade(),
        count: root.get_control::<TextBlock>("strokeCount").downgrade(),
        status: root.get_control::<TextBlock>("strokesStatus").downgrade(),
        restore: root.get_control::<Button>("restoreButton").downgrade(),
        saved: RefCell::new(None),
        saved_count: Cell::new(0),
    });
    // The subscriptions live with the presenter of the canvas, as the source's handlers do.
    {
        let sample = sample.clone();
        let _ = presenter.strokes_collected(move |_| sample.update_saved_stroke_count());
    }
    {
        let sample = sample.clone();
        let _ = presenter.strokes_erased(move |_| sample.update_saved_stroke_count());
    }
    for (name, action) in [
        ("saveDrawing", StrokesSample::save as fn(&StrokesSample)),
        ("clearStrokes", StrokesSample::clear),
        ("restoreButton", StrokesSample::restore),
    ] {
        let sample = sample.clone();
        root.get_control::<Button>(name).click(move |_, _| action(&sample));
    }
    sample.update_saved_stroke_count();
}
