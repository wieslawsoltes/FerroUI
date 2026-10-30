//! Reusable, staged preview-environment controls. Dimensions are logical layout units;
//! changing this model never changes a document or pretends to emulate device hardware.
use ferroui_base::input::{InputElement, Key};
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{
    Button, ColumnDefinitions, ComboBox, Control, Grid, StackPanel, TextBlock, TextBox,
};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewTheme {
    #[default]
    Light,
    Dark,
    Inherit,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewFlowDirection {
    #[default]
    Inherit,
    LeftToRight,
    RightToLeft,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewEnvironment {
    pub width: f64,
    pub height: f64,
    pub theme: PreviewTheme,
    pub flow_direction: PreviewFlowDirection,
}

impl Default for PreviewEnvironment {
    fn default() -> Self {
        Self {
            width: 800.0,
            height: 520.0,
            theme: PreviewTheme::Light,
            flow_direction: PreviewFlowDirection::Inherit,
        }
    }
}

impl PreviewEnvironment {
    pub fn validate(&self) -> Result<(), String> {
        for (name, value) in [("Width", self.width), ("Height", self.height)] {
            if !value.is_finite() || !(1.0..=16384.0).contains(&value) {
                return Err(format!("{name} must be between 1 and 16384 logical units"));
            }
        }
        Ok(())
    }

    pub fn rotated(self) -> Self {
        Self {
            width: self.height,
            height: self.width,
            ..self
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreviewSizePreset {
    pub name: &'static str,
    pub width: f64,
    pub height: f64,
}

/// Generic logical sizes deliberately avoid claiming a particular device's scale factor.
pub const PREVIEW_SIZE_PRESETS: &[PreviewSizePreset] = &[
    PreviewSizePreset {
        name: "Compact phone · 360 × 640",
        width: 360.0,
        height: 640.0,
    },
    PreviewSizePreset {
        name: "Phone · 390 × 844",
        width: 390.0,
        height: 844.0,
    },
    PreviewSizePreset {
        name: "Tablet · 768 × 1024",
        width: 768.0,
        height: 1024.0,
    },
    PreviewSizePreset {
        name: "Desktop · 1280 × 800",
        width: 1280.0,
        height: 800.0,
    },
    PreviewSizePreset {
        name: "Wide desktop · 1920 × 1080",
        width: 1920.0,
        height: 1080.0,
    },
    PreviewSizePreset {
        name: "Designer default · 800 × 520",
        width: 800.0,
        height: 520.0,
    },
];

type Apply = Rc<dyn Fn(PreviewEnvironment) -> Result<(), String>>;

/// The host retains this object while its control is visible. Callbacks are weak and do not
/// make control/host cycles; applying validates the complete draft before notifying the host.
pub struct PreviewEnvironmentEditor {
    root: Ref<StackPanel>,
    width: Ref<TextBox>,
    height: Ref<TextBox>,
    theme: Ref<ComboBox>,
    flow: Ref<ComboBox>,
    error: Ref<TextBlock>,
    design_size: Cell<(Option<f64>, Option<f64>)>,
    use_design_size: Ref<Button>,
    apply: RefCell<Option<Apply>>,
    cancel: RefCell<Option<Rc<dyn Fn()>>>,
    retired: Cell<bool>,
}

impl PreviewEnvironmentEditor {
    pub fn new(initial: PreviewEnvironment) -> Rc<Self> {
        let root = StackPanel::new();
        root.set_width(340.0);
        root.set_spacing(10.0);
        let title = label("Preview environment");
        title.set_font_size(16.0);
        title.set_font_weight(FontWeight::SemiBold);
        root.children().add(title.upcast::<Control>());
        let description =
            label("Test available space, theme and reading direction. Sizes use logical units.");
        description.set_text_wrapping(TextWrapping::Wrap);
        root.children().add(description.upcast::<Control>());

        let mut names = vec!["Custom size"];
        names.extend(PREVIEW_SIZE_PRESETS.iter().map(|preset| preset.name));
        let preset = choices(&names, "Preview size preset");
        row(&root, "Preset", preset.clone().upcast::<Control>());
        let width = field(&initial.width.to_string(), "Preview width");
        let height = field(&initial.height.to_string(), "Preview height");
        row(&root, "Width", width.clone().upcast::<Control>());
        row(&root, "Height", height.clone().upcast::<Control>());
        let rotate = action("Swap width and height", "Rotate the preview size");
        row(&root, "Orientation", rotate.clone().upcast::<Control>());
        let use_design_size = action(
            "Use design size",
            "Use the document's d:DesignWidth and d:DesignHeight",
        );
        use_design_size.set_is_enabled(false);
        row(
            &root,
            "From XAML",
            use_design_size.clone().upcast::<Control>(),
        );
        let theme = choices(&["Light", "Dark", "Application theme"], "Preview theme");
        theme.set_selected_index(match initial.theme {
            PreviewTheme::Light => 0,
            PreviewTheme::Dark => 1,
            PreviewTheme::Inherit => 2,
        });
        row(&root, "Theme", theme.clone().upcast::<Control>());
        let flow = choices(
            &["Inherited", "Left to right", "Right to left"],
            "Preview reading direction",
        );
        flow.set_selected_index(match initial.flow_direction {
            PreviewFlowDirection::Inherit => 0,
            PreviewFlowDirection::LeftToRight => 1,
            PreviewFlowDirection::RightToLeft => 2,
        });
        row(&root, "Direction", flow.clone().upcast::<Control>());
        let error = label("");
        error.set_text_wrapping(TextWrapping::Wrap);
        error.classes().add("designer-error");
        root.children().add(error.clone().upcast::<Control>());
        let buttons = StackPanel::new();
        buttons.set_orientation(Orientation::Horizontal);
        buttons.set_horizontal_alignment(HorizontalAlignment::Right);
        buttons.set_spacing(8.0);
        let cancel = action("Cancel", "Discard preview environment changes");
        let apply = action("Apply preview", "Apply preview environment");
        apply.classes().add("primary");
        buttons.children().add(cancel.clone().upcast::<Control>());
        buttons.children().add(apply.clone().upcast::<Control>());
        root.children().add(buttons.upcast::<Control>());
        let editor = Rc::new(Self {
            root,
            width,
            height,
            theme,
            flow,
            error,
            design_size: Cell::new((None, None)),
            use_design_size: use_design_size.clone(),
            apply: RefCell::new(None),
            cancel: RefCell::new(None),
            retired: Cell::new(false),
        });
        let weak = Rc::downgrade(&editor);
        let weak_preset = preset.downgrade();
        preset.selection_changed(move |_, _| {
            let (Some(editor), Some(picker)) = (weak.upgrade(), weak_preset.upgrade()) else {
                return;
            };
            if let Some(preset) = picker
                .selected_index()
                .checked_sub(1)
                .and_then(|index| usize::try_from(index).ok())
                .and_then(|index| PREVIEW_SIZE_PRESETS.get(index))
            {
                editor.width.set_text(Some(&preset.width.to_string()));
                editor.height.set_text(Some(&preset.height.to_string()));
            }
        });
        let weak = Rc::downgrade(&editor);
        let weak_preset = preset.downgrade();
        rotate.click(move |_, _| {
            let Some(editor) = weak.upgrade() else {
                return;
            };
            let width = editor.width.text();
            let height = editor.height.text();
            editor.width.set_text(height.as_deref());
            editor.height.set_text(width.as_deref());
            if let Some(picker) = weak_preset.upgrade() {
                picker.set_selected_index(0);
            }
        });
        let weak = Rc::downgrade(&editor);
        let weak_preset = preset.downgrade();
        use_design_size.click(move |_, _| {
            let Some(editor) = weak.upgrade() else {
                return;
            };
            let (width, height) = editor.design_size.get();
            if let Some(width) = width {
                editor.width.set_text(Some(&width.to_string()));
            }
            if let Some(height) = height {
                editor.height.set_text(Some(&height.to_string()));
            }
            if let Some(picker) = weak_preset.upgrade() {
                picker.set_selected_index(0);
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

    pub fn control(&self) -> Ref<Control> {
        self.root.clone().upcast()
    }
    pub fn set_design_size_hint(&self, width: Option<f64>, height: Option<f64>) {
        let valid = |value: &f64| value.is_finite() && (1.0..=16384.0).contains(value);
        let width = width.filter(valid);
        let height = height.filter(valid);
        self.design_size.set((width, height));
        self.use_design_size
            .set_is_enabled(width.is_some() || height.is_some());
    }
    pub fn value(&self) -> Result<PreviewEnvironment, String> {
        let value = PreviewEnvironment {
            width: self
                .width
                .text()
                .unwrap_or_default()
                .trim()
                .parse()
                .map_err(|_| "Enter a numeric preview width")?,
            height: self
                .height
                .text()
                .unwrap_or_default()
                .trim()
                .parse()
                .map_err(|_| "Enter a numeric preview height")?,
            theme: match self.theme.selected_index() {
                0 => PreviewTheme::Light,
                1 => PreviewTheme::Dark,
                _ => PreviewTheme::Inherit,
            },
            flow_direction: match self.flow.selected_index() {
                1 => PreviewFlowDirection::LeftToRight,
                2 => PreviewFlowDirection::RightToLeft,
                _ => PreviewFlowDirection::Inherit,
            },
        };
        value.validate()?;
        Ok(value)
    }
    pub fn on_apply(&self, callback: impl Fn(PreviewEnvironment) -> Result<(), String> + 'static) {
        *self.apply.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn on_cancel(&self, callback: impl Fn() + 'static) {
        *self.cancel.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn apply(&self) -> Result<(), String> {
        if self.retired.get() {
            return Err("This preview environment draft is closed".into());
        }
        let result = self.value().and_then(|value| {
            let callback = self
                .apply
                .borrow()
                .clone()
                .ok_or("No preview environment host is attached")?;
            callback(value)
        });
        self.error.set_text(Some(
            result.as_ref().err().map(String::as_str).unwrap_or(""),
        ));
        result
    }
    pub fn cancel(&self) {
        self.retire();
        let callback = self.cancel.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
    }
    pub fn retire(&self) {
        self.retired.set(true);
    }
}

fn label(value: &str) -> Ref<TextBlock> {
    let control = TextBlock::new();
    control.set_text(Some(value));
    control.set_font_size(12.0);
    control
}
fn field(value: &str, name: &str) -> Ref<TextBox> {
    let control = TextBox::new();
    control.set_text(Some(value));
    control.set_min_height(28.0);
    control.classes().add("designer-property-field");
    AutomationProperties::set_name(&control, Some(name));
    control
}
fn choices(values: &[&str], name: &str) -> Ref<ComboBox> {
    let control = ComboBox::new();
    control.set_horizontal_alignment(HorizontalAlignment::Stretch);
    for value in values {
        control.items().add(Some(Rc::new((*value).to_owned())));
    }
    control.set_selected_index(0);
    AutomationProperties::set_name(&control, Some(name));
    control
}
fn action(value: &str, name: &str) -> Ref<Button> {
    let control = Button::new();
    control.set_content(Some(Rc::new(value.to_owned())));
    control.set_padding(Thickness::new(10.0, 5.0, 10.0, 5.0));
    AutomationProperties::set_name(&control, Some(name));
    control
}
fn row(panel: &StackPanel, name: &str, control: Ref<Control>) {
    let row = Grid::new();
    row.set_column_definitions(ColumnDefinitions::parse("86,*").unwrap());
    let title = label(name);
    title.set_vertical_alignment(VerticalAlignment::Center);
    row.children().add(title.upcast::<Control>());
    Grid::set_column(&control, 1);
    row.children().add(control);
    panel.children().add(row.upcast::<Control>());
}

#[cfg(test)]
#[path = "preview_environment_tests.rs"]
mod tests;
