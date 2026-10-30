//! Reusable staged appearance-property picker. All mutations go through its host's
//! reviewed source-plan callback; checkbox changes never touch the document.
use crate::{PropertyClipboard, PropertyPastePreview};
use ferroui_base::input::{InputElement, Key};
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{
    Button, CheckBox, Control, Grid, RowDefinitions, ScrollViewer, StackPanel, TextBlock,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type ApplyHandler = Rc<dyn Fn(Vec<String>) -> Result<(), String>>;
type CloseHandler = Rc<dyn Fn()>;

struct PropertyChoice {
    name: String,
    check: Ref<CheckBox>,
    changed: usize,
}

/// A native picker with fixed Apply/Cancel controls and an independently scrolling body.
/// Compatible authored values are selected initially; every skipped value is explained.
pub struct PropertyTransferEditor {
    root: Ref<Grid>,
    choices: Vec<PropertyChoice>,
    summary: Ref<TextBlock>,
    error: Ref<TextBlock>,
    apply_button: Ref<Button>,
    writable: bool,
    active: Cell<bool>,
    apply: RefCell<Option<ApplyHandler>>,
    close: RefCell<Option<CloseHandler>>,
}

impl PropertyTransferEditor {
    pub fn new(
        clipboard: &PropertyClipboard,
        preview: &PropertyPastePreview,
        writable: bool,
    ) -> Rc<Self> {
        let root = Grid::new();
        root.set_max_width(450.0);
        root.set_max_height(590.0);
        root.set_row_definitions(RowDefinitions::parse("Auto,*,Auto").unwrap());
        root.classes().add("designer-chrome");
        let heading = StackPanel::new();
        heading.set_spacing(6.0);
        heading.set_margin(Thickness::new(0.0, 0.0, 0.0, 10.0));
        let title = text("Paste appearance properties", 16.0);
        title.set_font_weight(FontWeight::SemiBold);
        heading.children().add(title.upcast::<Control>());
        heading.children().add(
            text(&format!("Copied from {}", clipboard.source_label()), 12.0).upcast::<Control>(),
        );
        heading.children().add(text("Choose the authored values to apply. Each compatible destination keeps its identity, content and layout.", 11.0).upcast::<Control>());
        root.children().add(heading.upcast::<Control>());

        let scroll = ScrollViewer::new();
        scroll.set_name(Some("PropertyTransferScroll".into()));
        scroll.set_max_height(370.0);
        Grid::set_row(&scroll, 1);
        let body = StackPanel::new();
        body.set_spacing(8.0);
        let mut choices = Vec::new();
        let mut category = "";
        for entry in clipboard.entries() {
            if category != entry.category {
                category = &entry.category;
                let group = text(category, 12.0);
                group.set_font_weight(FontWeight::SemiBold);
                group.set_margin(Thickness::new(0.0, 4.0, 0.0, 1.0));
                body.children().add(group.upcast::<Control>());
            }
            let compatible = preview
                .changes()
                .iter()
                .filter(|change| change.property == entry.name)
                .count();
            let changed = preview
                .changes()
                .iter()
                .filter(|change| change.property == entry.name && change.changed)
                .count();
            let skipped = preview
                .skipped()
                .iter()
                .filter(|issue| issue.property == entry.name)
                .count();
            let check = CheckBox::new();
            check.set_name(Some(format!("PropertyPaste{}", entry.name)));
            check.set_content(Some(Rc::new(entry.name.clone())));
            check.set_is_checked(Some(changed > 0));
            check.set_is_enabled(writable && compatible > 0);
            AutomationProperties::set_name(&check, Some(&format!("Paste {}", entry.name)));
            body.children().add(check.clone().upcast::<Control>());
            let value = entry.summary.chars().take(160).collect::<String>();
            let detail = text(
                &format!(
                    "{value}\n{compatible} compatible · {changed} changes · {skipped} skipped"
                ),
                11.0,
            );
            detail.set_margin(Thickness::new(24.0, -4.0, 0.0, 2.0));
            body.children().add(detail.upcast::<Control>());
            choices.push(PropertyChoice {
                name: entry.name.clone(),
                check,
                changed,
            });
        }
        if !clipboard.skipped().is_empty() || !preview.skipped().is_empty() {
            let title = text("Values kept unchanged", 12.0);
            title.set_font_weight(FontWeight::SemiBold);
            body.children().add(title.upcast::<Control>());
            for issue in clipboard.skipped() {
                body.children().add(
                    text(
                        &format!("Not copied · {}: {}", issue.property, issue.reason),
                        11.0,
                    )
                    .upcast::<Control>(),
                );
            }
            for issue in preview.skipped() {
                let target = issue
                    .target_label
                    .as_ref()
                    .map(|label| format!("{label} · "))
                    .unwrap_or_default();
                body.children().add(
                    text(
                        &format!("{target}{}: {}", issue.property, issue.reason),
                        11.0,
                    )
                    .upcast::<Control>(),
                );
            }
        }
        if clipboard.entries().is_empty() {
            body.children().add(
                text(
                    "The copied control has no transferable authored appearance values.",
                    12.0,
                )
                .upcast::<Control>(),
            );
        }
        scroll.set_content(Some(Control::boxed(body.upcast::<Control>())));
        root.children().add(scroll.upcast::<Control>());

        let footer = StackPanel::new();
        footer.set_name(Some("PropertyTransferFooter".into()));
        footer.set_spacing(7.0);
        footer.set_margin(Thickness::new(0.0, 10.0, 0.0, 0.0));
        Grid::set_row(&footer, 2);
        let summary = text("", 11.0);
        summary.set_name(Some("PropertyTransferSummary".into()));
        footer.children().add(summary.clone().upcast::<Control>());
        let error = text("", 11.0);
        error.set_name(Some("PropertyTransferError".into()));
        footer.children().add(error.clone().upcast::<Control>());
        let buttons = StackPanel::new();
        buttons.set_orientation(Orientation::Horizontal);
        buttons.set_horizontal_alignment(HorizontalAlignment::Right);
        buttons.set_spacing(8.0);
        let cancel = button("Cancel", "PropertyTransferCancel");
        let apply_button = button("Apply", "PropertyTransferApply");
        buttons.children().add(cancel.clone().upcast::<Control>());
        buttons
            .children()
            .add(apply_button.clone().upcast::<Control>());
        footer.children().add(buttons.upcast::<Control>());
        root.children().add(footer.upcast::<Control>());
        let editor = Rc::new(Self {
            root,
            choices,
            summary,
            error,
            apply_button: apply_button.clone(),
            writable,
            active: Cell::new(true),
            apply: RefCell::new(None),
            close: RefCell::new(None),
        });
        for choice in &editor.choices {
            let weak = Rc::downgrade(&editor);
            choice.check.is_checked_changed(move |_, _| {
                if let Some(editor) = weak.upgrade() {
                    editor.refresh_summary();
                }
            });
        }
        let weak = Rc::downgrade(&editor);
        apply_button.click(move |_, _| {
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
        editor.refresh_summary();
        editor
    }

    pub fn control(&self) -> Ref<Control> {
        self.root.clone().upcast()
    }
    pub fn selected_properties(&self) -> Vec<String> {
        self.choices
            .iter()
            .filter(|choice| choice.check.is_checked() == Some(true))
            .map(|choice| choice.name.clone())
            .collect()
    }
    pub fn select_property(&self, name: &str, selected: bool) -> Result<(), String> {
        if !self.active.get() {
            return Err("This appearance draft is closed".into());
        }
        let choice = self
            .choices
            .iter()
            .find(|choice| choice.name == name)
            .ok_or("The clipboard does not contain that property")?;
        if !choice.check.is_enabled() {
            return Err("The property is unavailable for these destinations".into());
        }
        choice.check.set_is_checked(Some(selected));
        Ok(())
    }
    pub fn on_apply(&self, callback: impl Fn(Vec<String>) -> Result<(), String> + 'static) {
        *self.apply.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn on_close(&self, callback: impl Fn() + 'static) {
        *self.close.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn apply(&self) -> Result<(), String> {
        let result = (|| {
            if !self.active.get() {
                return Err("This appearance draft is closed".into());
            }
            if !self.writable {
                return Err("This collaboration role has read-only access".into());
            }
            let selected = self.selected_properties();
            if selected.is_empty() {
                return Err("Choose at least one appearance property".into());
            }
            let apply = self
                .apply
                .borrow()
                .clone()
                .ok_or("No appearance-paste host is attached")?;
            apply(selected)
        })();
        self.show_error(result.as_ref().err().map(String::as_str).unwrap_or(""));
        if result.is_ok() {
            self.cancel();
        }
        result
    }
    pub fn show_error(&self, message: &str) {
        self.error.set_text(Some(message));
    }
    pub fn cancel(&self) {
        self.retire();
        let close = self.close.borrow().clone();
        if let Some(close) = close {
            close();
        }
    }
    pub fn retire(&self) {
        self.active.set(false);
        self.apply_button.set_is_enabled(false);
        for choice in &self.choices {
            choice.check.set_is_enabled(false);
        }
    }
    fn refresh_summary(&self) {
        let selected = self
            .choices
            .iter()
            .filter(|choice| choice.check.is_checked() == Some(true))
            .collect::<Vec<_>>();
        let changes: usize = selected.iter().map(|choice| choice.changed).sum();
        self.summary.set_text(Some(&format!(
            "{} properties selected · {changes} destination changes · one undo step",
            selected.len()
        )));
        self.apply_button
            .set_is_enabled(self.active.get() && self.writable && changes > 0);
    }
}

fn text(value: &str, size: f64) -> Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_text(Some(value));
    text.set_font_size(size);
    text.set_text_wrapping(TextWrapping::Wrap);
    text
}
fn button(label: &str, name: &str) -> Ref<Button> {
    let button = Button::new();
    button.set_name(Some(name.into()));
    button.set_content(Some(Rc::new(label.to_owned())));
    button.set_min_width(72.0);
    AutomationProperties::set_name(&button, Some(label));
    button
}
