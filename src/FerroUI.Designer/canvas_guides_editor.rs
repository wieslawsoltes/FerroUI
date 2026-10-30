//! Staged native editor for exact guide positions and local snapping preferences.
use crate::{CanvasGuide, CanvasGuideAxis, CanvasGuideSettings};
use ferroui_base::input::{InputElement, Key};
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{
    Button, CheckBox, ColumnDefinitions, Control, Grid, ScrollViewer, StackPanel, TextBlock,
    TextBox,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct GuideRow {
    guide: CanvasGuide,
    control: Ref<Grid>,
    position: Ref<TextBox>,
}
type Apply = Rc<dyn Fn(CanvasGuideSettings) -> Result<(), String>>;

pub struct CanvasGuidesEditor {
    root: Ref<StackPanel>,
    rows_host: Ref<StackPanel>,
    rows: RefCell<Vec<GuideRow>>,
    toggles: Vec<Ref<CheckBox>>,
    numbers: Vec<Ref<TextBox>>,
    error: Ref<TextBlock>,
    apply: RefCell<Option<Apply>>,
    cancel: RefCell<Option<Rc<dyn Fn()>>>,
    retired: Cell<bool>,
}

impl CanvasGuidesEditor {
    pub fn new(initial: CanvasGuideSettings) -> Rc<Self> {
        let root = StackPanel::new();
        root.set_width(350.0);
        root.set_spacing(8.0);
        let title = label("Guides and snapping");
        title.set_font_size(16.0);
        title.set_font_weight(FontWeight::SemiBold);
        root.children().add(title.upcast::<Control>());
        let hint = label(
            "Drag from a ruler to create a guide. Drag a guide back to its ruler to remove it. Positions use artboard logical units.",
        );
        hint.set_text_wrapping(TextWrapping::Wrap);
        root.children().add(hint.upcast::<Control>());
        let scroll = ScrollViewer::new();
        scroll.set_max_height(440.0);
        let body = StackPanel::new();
        body.set_spacing(6.0);
        scroll.set_content(Some(Control::boxed(&body)));
        root.children().add(scroll.upcast::<Control>());
        let options = Grid::new();
        options.set_column_definitions(ColumnDefinitions::parse("*,*").unwrap());
        options.set_row_definitions(
            ferroui_controls::RowDefinitions::parse("Auto,Auto,Auto").unwrap(),
        );
        body.children().add(options.clone().upcast::<Control>());
        let toggles = [
            ("Show rulers", initial.rulers_visible),
            ("Show guides", initial.guides_visible),
            ("Show grid", initial.grid_visible),
            ("Snap to grid", initial.snap_to_grid),
            ("Snap to guides", initial.snap_to_guides),
            ("Snap to objects", initial.snap_to_objects),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (name, value))| {
            let toggle = CheckBox::new();
            toggle.set_content(Some(Rc::new(name.to_owned())));
            toggle.set_is_checked(Some(value));
            AutomationProperties::set_name(&toggle, Some(name));
            Grid::set_row(&toggle, (index % 3) as i32);
            Grid::set_column(&toggle, (index / 3) as i32);
            options.children().add(toggle.clone().upcast::<Control>());
            toggle
        })
        .collect();
        let numbers = [
            ("Grid spacing", initial.grid_spacing),
            ("Snap distance · px", initial.snap_tolerance),
            ("Small nudge", initial.small_nudge),
            ("Large nudge", initial.large_nudge),
        ]
        .into_iter()
        .map(|(name, value)| {
            let field = number(&value.to_string(), name);
            row(&body, name, field.clone().upcast::<Control>());
            field
        })
        .collect();
        let heading = label("Guides in this canvas scope");
        heading.set_font_weight(FontWeight::SemiBold);
        heading.set_margin(Thickness::new(0.0, 6.0, 0.0, 0.0));
        body.children().add(heading.upcast::<Control>());
        let rows_host = StackPanel::new();
        rows_host.set_spacing(5.0);
        body.children().add(rows_host.clone().upcast::<Control>());
        let add_buttons = StackPanel::new();
        add_buttons.set_orientation(Orientation::Horizontal);
        add_buttons.set_spacing(6.0);
        let horizontal = button("Add horizontal", "GuideAddHorizontal");
        let vertical = button("Add vertical", "GuideAddVertical");
        let clear = button("Clear", "GuideClear");
        for action in [&horizontal, &vertical, &clear] {
            add_buttons
                .children()
                .add(action.clone().upcast::<Control>());
        }
        body.children().add(add_buttons.upcast::<Control>());
        let error = label("");
        error.classes().add("designer-error");
        error.set_text_wrapping(TextWrapping::Wrap);
        root.children().add(error.clone().upcast::<Control>());
        let actions = StackPanel::new();
        actions.set_orientation(Orientation::Horizontal);
        actions.set_horizontal_alignment(HorizontalAlignment::Right);
        actions.set_spacing(6.0);
        let cancel = button("Cancel", "GuidesCancel");
        let apply = button("Apply", "GuidesApply");
        apply.classes().add("primary");
        actions.children().add(cancel.clone().upcast::<Control>());
        actions.children().add(apply.clone().upcast::<Control>());
        root.children().add(actions.upcast::<Control>());
        let editor = Rc::new(Self {
            root,
            rows_host,
            rows: RefCell::new(Vec::new()),
            toggles,
            numbers,
            error,
            apply: RefCell::new(None),
            cancel: RefCell::new(None),
            retired: Cell::new(false),
        });
        for guide in initial.guides {
            editor.append_row(guide);
        }
        for (action, axis) in [
            (horizontal, CanvasGuideAxis::Horizontal),
            (vertical, CanvasGuideAxis::Vertical),
        ] {
            let weak = Rc::downgrade(&editor);
            action.click(move |_, _| {
                if let Some(editor) = weak.upgrade() {
                    let result = editor.add_guide(axis, 0.0);
                    editor.show_error(result.err().as_deref());
                }
            });
        }
        let weak = Rc::downgrade(&editor);
        clear.click(move |_, _| {
            if let Some(editor) = weak.upgrade() {
                editor.rows.borrow_mut().clear();
                editor.rows_host.children().clear();
            }
        });
        let weak = Rc::downgrade(&editor);
        apply.click(move |_, _| {
            if let Some(editor) = weak.upgrade() {
                let _ = editor.apply();
            }
        });
        let weak = Rc::downgrade(&editor);
        cancel.click(move |_, _| {
            if let Some(editor) = weak.upgrade() {
                editor.cancel();
            }
        });
        let weak = Rc::downgrade(&editor);
        editor
            .root
            .add_handler(InputElement::key_down_event(), move |_, event| {
                if event.key == Key::Escape {
                    if let Some(editor) = weak.upgrade() {
                        editor.cancel();
                    }
                    event.set_handled(true);
                }
            });
        editor
    }

    pub fn control(&self) -> &Ref<StackPanel> {
        &self.root
    }

    pub fn value(&self) -> Result<CanvasGuideSettings, String> {
        let number = |index: usize| {
            self.numbers[index]
                .text()
                .unwrap_or_default()
                .trim()
                .parse::<f64>()
                .map_err(|_| "Enter numeric grid, snap and nudge values".to_owned())
        };
        let enabled = |index: usize| self.toggles[index].is_checked() == Some(true);
        let guides = self
            .rows
            .borrow()
            .iter()
            .map(|row| {
                let position = row
                    .position
                    .text()
                    .unwrap_or_default()
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Enter a numeric guide position".to_owned())?;
                Ok(CanvasGuide {
                    position,
                    ..row.guide
                })
            })
            .collect::<Result<_, String>>()?;
        let value = CanvasGuideSettings {
            rulers_visible: enabled(0),
            guides_visible: enabled(1),
            grid_visible: enabled(2),
            snap_to_grid: enabled(3),
            snap_to_guides: enabled(4),
            snap_to_objects: enabled(5),
            grid_spacing: number(0)?,
            snap_tolerance: number(1)?,
            small_nudge: number(2)?,
            large_nudge: number(3)?,
            guides,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn add_guide(self: &Rc<Self>, axis: CanvasGuideAxis, position: f64) -> Result<u64, String> {
        self.ensure_current()?;
        let mut value = self.value()?;
        let id = value.add(axis, position)?;
        self.append_row(*value.guides.last().unwrap());
        Ok(id)
    }

    pub fn set_guide_position(&self, id: u64, draft: &str) -> Result<(), String> {
        self.ensure_current()?;
        let rows = self.rows.borrow();
        let row = rows
            .iter()
            .find(|row| row.guide.id == id)
            .ok_or("Unknown guide")?;
        row.position.set_text(Some(draft));
        Ok(())
    }

    pub fn remove_guide(&self, id: u64) -> Result<(), String> {
        self.ensure_current()?;
        let mut rows = self.rows.borrow_mut();
        let index = rows
            .iter()
            .position(|row| row.guide.id == id)
            .ok_or("Unknown guide")?;
        let row = rows.remove(index);
        self.rows_host
            .children()
            .remove(row.control.upcast::<Control>());
        Ok(())
    }

    pub fn on_apply(&self, callback: impl Fn(CanvasGuideSettings) -> Result<(), String> + 'static) {
        *self.apply.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn on_cancel(&self, callback: impl Fn() + 'static) {
        *self.cancel.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn apply(&self) -> Result<(), String> {
        let result = self
            .ensure_current()
            .and_then(|_| self.value())
            .and_then(|value| {
                self.apply
                    .borrow()
                    .clone()
                    .ok_or("No canvas guide host is attached")?(value)
            });
        self.show_error(result.as_ref().err().map(String::as_str));
        result
    }
    pub fn cancel(&self) {
        if self.retired.replace(true) {
            return;
        }
        let cancel = self.cancel.borrow().clone();
        if let Some(cancel) = cancel {
            cancel();
        }
    }
    pub fn retire(&self) {
        self.retired.set(true);
    }
    fn ensure_current(&self) -> Result<(), String> {
        if self.retired.get() {
            Err("This guide draft is closed".into())
        } else {
            Ok(())
        }
    }
    fn show_error(&self, error: Option<&str>) {
        self.error.set_text(Some(error.unwrap_or("")));
    }
    fn append_row(self: &Rc<Self>, guide: CanvasGuide) {
        let control = Grid::new();
        control.set_column_definitions(ColumnDefinitions::parse("96,*,Auto").unwrap());
        let title = label(if guide.axis == CanvasGuideAxis::Horizontal {
            "Horizontal · Y"
        } else {
            "Vertical · X"
        });
        title.set_vertical_alignment(VerticalAlignment::Center);
        control.children().add(title.upcast::<Control>());
        let position = number(
            &guide.position.to_string(),
            "Guide position in artboard units",
        );
        Grid::set_column(&position, 1);
        control.children().add(position.clone().upcast::<Control>());
        let delete = button("Remove", &format!("GuideRemove{}", guide.id));
        delete.set_margin(Thickness::new(5.0, 0.0, 0.0, 0.0));
        Grid::set_column(&delete, 2);
        control.children().add(delete.clone().upcast::<Control>());
        let weak = Rc::downgrade(self);
        delete.click(move |_, _| {
            if let Some(editor) = weak.upgrade() {
                let _ = editor.remove_guide(guide.id);
            }
        });
        self.rows_host
            .children()
            .add(control.clone().upcast::<Control>());
        self.rows.borrow_mut().push(GuideRow {
            guide,
            control,
            position,
        });
    }
}

fn label(value: &str) -> Ref<TextBlock> {
    let value_control = TextBlock::new();
    value_control.set_text(Some(value));
    value_control.set_font_size(12.0);
    value_control
}
fn number(value: &str, name: &str) -> Ref<TextBox> {
    let control = TextBox::new();
    control.set_text(Some(value));
    control.set_min_height(28.0);
    control.classes().add("designer-property-field");
    AutomationProperties::set_name(&control, Some(name));
    control
}
fn button(value: &str, name: &str) -> Ref<Button> {
    let control = Button::new();
    control.set_name(Some(name.into()));
    control.set_content(Some(Rc::new(value.to_owned())));
    control.set_padding(Thickness::new(8.0, 4.0, 8.0, 4.0));
    AutomationProperties::set_name(&control, Some(value));
    control
}
fn row(panel: &StackPanel, title: &str, field: Ref<Control>) {
    let control = Grid::new();
    control.set_column_definitions(ColumnDefinitions::parse("132,*").unwrap());
    let title = label(title);
    title.set_vertical_alignment(VerticalAlignment::Center);
    control.children().add(title.upcast::<Control>());
    Grid::set_column(&field, 1);
    control.children().add(field);
    panel.children().add(control.upcast::<Control>());
}
