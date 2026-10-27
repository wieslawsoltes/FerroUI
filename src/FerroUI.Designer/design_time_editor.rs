//! Reusable staged design-data editor. The host owns acceptance, history and scope.
use super::{DesignTimeEdit, DesignTimeProperty};
use ferroui_base::input::{InputElement, Key};
use ferroui_base::layout::Orientation;
use ferroui_base::media::{FontWeight, SolidColorBrush, TextWrapping};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{Button, CheckBox, Control, ScrollViewer, StackPanel, TextBlock, TextBox};
use ferroui_designer_core::NodeId;
use std::rc::Rc;

pub struct DesignTimeEditorModel {
    pub node: NodeId,
    pub title: String,
    pub properties: Vec<DesignTimeProperty>,
    pub enabled: bool,
    pub writable: bool,
    pub sample_count: Option<usize>,
}

pub struct DesignTimeEditor {
    root: Ref<StackPanel>,
}
impl DesignTimeEditor {
    /// Dismissing the containing flyout discards drafts. Acceptance occurs once, after
    /// the host validates the captured document/selection identity.
    pub fn new(
        model: DesignTimeEditorModel,
        apply: impl Fn(bool, Vec<DesignTimeEdit>) -> Result<(), String> + 'static,
        cancel: impl Fn() + 'static,
    ) -> Rc<Self> {
        let root = StackPanel::new();
        root.set_width(410.0);
        root.set_spacing(10.0);
        root.set_margin(Thickness::uniform(8.0));
        root.set_name(Some("DesignTimeEditor".into()));
        let heading = label(&format!("Design data · {}", model.title));
        heading.set_font_size(15.0);
        heading.set_font_weight(FontWeight::SemiBold);
        root.children().add(heading.upcast::<Control>());
        let help = label(
            "Preview-only values use the d: namespace. Runtime bindings and values stay in source. Uncheck an override to reset it.",
        );
        help.set_text_wrapping(TextWrapping::Wrap);
        root.children().add(help.upcast::<Control>());
        let show = toggle("Show design-time values", model.enabled);
        show.set_name(Some("DesignTimeEnabled".into()));
        root.children().add(show.clone().upcast::<Control>());
        let scroll = ScrollViewer::new();
        scroll.set_max_height(410.0);
        let fields = StackPanel::new();
        fields.set_spacing(8.0);
        let mut properties = model.properties;
        let authored = properties
            .iter()
            .map(|property| property.name.clone())
            .collect::<Vec<_>>();
        for name in ["DesignWidth", "DesignHeight"] {
            if !properties.iter().any(|property| property.name == name) {
                properties.push(DesignTimeProperty {
                    name: name.into(),
                    value: String::new(),
                    object: false,
                });
            }
        }
        let mut rows = Vec::new();
        for property in properties {
            if property.name == "ItemsSource" && model.sample_count.is_some() {
                continue;
            }
            let active = authored.contains(&property.name);
            let checked = toggle(
                &format!(
                    "{}{}",
                    property.name,
                    if property.object {
                        " · object XAML"
                    } else {
                        ""
                    }
                ),
                active,
            );
            checked.set_name(Some(format!(
                "DesignOverride{}",
                property.name.replace('.', "_")
            )));
            checked.set_is_enabled(model.writable);
            let value = text(&property.value, &format!("Design-time {}", property.name));
            value.set_name(Some(format!(
                "DesignValue{}",
                property.name.replace('.', "_")
            )));
            value.set_accepts_return(property.object);
            value.set_min_height(if property.object { 76.0 } else { 30.0 });
            value.set_is_enabled(model.writable && active);
            let weak = value.downgrade();
            let checked_weak = checked.downgrade();
            checked.is_checked_changed(move |_, _| {
                if let (Some(value), Some(checked)) = (weak.upgrade(), checked_weak.upgrade()) {
                    value.set_is_enabled(checked.is_checked() == Some(true));
                }
            });
            fields.children().add(checked.clone().upcast::<Control>());
            fields.children().add(value.clone().upcast::<Control>());
            rows.push((property.name, property.object, checked, value));
        }
        let add = label("Add or replace an override");
        add.set_font_weight(FontWeight::SemiBold);
        fields.children().add(add.upcast::<Control>());
        let property = text("", "Design property name");
        property.set_name(Some("DesignNewProperty".into()));
        property.set_placeholder_text(Some("Text, Content, DataContext, Background…"));
        property.set_is_enabled(model.writable);
        fields.children().add(property.clone().upcast::<Control>());
        let value = text("", "New design property value");
        value.set_name(Some("DesignNewValue".into()));
        value.set_accepts_return(true);
        value.set_min_height(65.0);
        value.set_is_enabled(model.writable);
        fields.children().add(value.clone().upcast::<Control>());
        let object = toggle("Value contains object XAML", false);
        object.set_name(Some("DesignNewObject".into()));
        object.set_is_enabled(model.writable);
        fields.children().add(object.clone().upcast::<Control>());
        let sample = toggle(
            "Generate sample items from template bindings",
            model.sample_count.is_some(),
        );
        sample.set_name(Some("DesignSampleItems".into()));
        sample.set_is_enabled(model.writable);
        fields.children().add(sample.clone().upcast::<Control>());
        let count = text(
            &model.sample_count.unwrap_or(5).to_string(),
            "Number of sample items, from 0 to 200",
        );
        count.set_name(Some("DesignSampleCount".into()));
        count.set_is_enabled(model.writable);
        fields.children().add(count.clone().upcast::<Control>());
        let sample_help = label(
            "Sample records support simple and nested binding paths, text, numeric and Boolean values. Use explicit overrides for complex paths or converter-specific data.",
        );
        sample_help.set_text_wrapping(TextWrapping::Wrap);
        fields.children().add(sample_help.upcast::<Control>());
        scroll.set_content(Some(Control::boxed(&fields)));
        root.children().add(scroll.upcast::<Control>());
        let error = label("");
        error.set_name(Some("DesignTimeError".into()));
        error.set_text_wrapping(TextWrapping::Wrap);
        error.set_foreground(Some(SolidColorBrush::from_uint32(0xffc64b3c).into()));
        root.children().add(error.clone().upcast::<Control>());
        let buttons = StackPanel::new();
        buttons.set_orientation(Orientation::Horizontal);
        buttons.set_spacing(8.0);
        let accept = button("Apply design data", "Apply design-time values");
        accept.set_name(Some("DesignTimeApply".into()));
        let dismiss = button("Cancel", "Discard design-time draft");
        dismiss.set_name(Some("DesignTimeCancel".into()));
        buttons.children().add(accept.clone().upcast::<Control>());
        buttons.children().add(dismiss.clone().upcast::<Control>());
        root.children().add(buttons.upcast::<Control>());
        let node = model.node;
        accept.click(move |_, _| {
            let result = (|| {
                let mut edits = Vec::new();
                if model.writable {
                    for (name, object, checked, value) in &rows {
                        let value = (checked.is_checked() == Some(true))
                            .then(|| value.text().unwrap_or_default());
                        edits.push(if *object {
                            DesignTimeEdit::SetObject {
                                node,
                                property: name.clone(),
                                source: value,
                            }
                        } else {
                            DesignTimeEdit::SetProperty {
                                node,
                                property: name.clone(),
                                value,
                            }
                        });
                    }
                    let name = property.text().unwrap_or_default().trim().to_owned();
                    if !name.is_empty() {
                        let source = Some(value.text().unwrap_or_default());
                        edits.push(if object.is_checked() == Some(true) {
                            DesignTimeEdit::SetObject {
                                node,
                                property: name,
                                source,
                            }
                        } else {
                            DesignTimeEdit::SetProperty {
                                node,
                                property: name,
                                value: source,
                            }
                        });
                    }
                    if sample.is_checked() == Some(true) {
                        let count = count
                            .text()
                            .unwrap_or_default()
                            .trim()
                            .parse::<usize>()
                            .map_err(|_| {
                                "Sample count must be an integer from 0 to 200.".to_owned()
                            })?;
                        edits.push(DesignTimeEdit::SampleItems {
                            node,
                            count: Some(count),
                        });
                    } else if model.sample_count.is_some() {
                        edits.push(DesignTimeEdit::SampleItems { node, count: None });
                    }
                }
                apply(show.is_checked() == Some(true), edits)
            })();
            if let Err(message) = result {
                error.set_text(Some(&message));
            }
        });
        let cancel: Rc<dyn Fn()> = Rc::new(cancel);
        let on_cancel = cancel.clone();
        dismiss.click(move |_, _| on_cancel());
        root.add_handler(InputElement::key_down_event(), move |_, event| {
            if event.key == Key::Escape {
                cancel();
                event.set_handled(true);
            }
        });
        Rc::new(Self { root })
    }
    pub fn control(&self) -> Ref<Control> {
        self.root.clone().upcast()
    }
}
fn label(value: &str) -> Ref<TextBlock> {
    let label = TextBlock::new();
    label.set_text(Some(value));
    label.set_font_size(12.0);
    label
}
fn text(value: &str, name: &str) -> Ref<TextBox> {
    let text = TextBox::new();
    text.set_text(Some(value));
    text.classes().add("designer-property-field");
    AutomationProperties::set_name(&text, Some(name));
    text
}
fn toggle(label: &str, checked: bool) -> Ref<CheckBox> {
    let checkbox = CheckBox::new();
    checkbox.set_content(Some(Rc::new(label.to_owned())));
    checkbox.set_is_checked(Some(checked));
    AutomationProperties::set_name(&checkbox, Some(label));
    checkbox
}
fn button(label: &str, name: &str) -> Ref<Button> {
    let button = Button::new();
    button.set_content(Some(Rc::new(label.to_owned())));
    AutomationProperties::set_name(&button, Some(name));
    button
}
