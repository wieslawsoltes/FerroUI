//! Reusable native binding editor. Field changes are local until the host accepts value().
use super::binding_draft::{
    PropertyBindingDraft, PropertyBindingOption, PropertyBindingSource, value_source,
};
use ferroui_base::Ref;
use ferroui_base::media::TextWrapping;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{
    Button, ColumnDefinitions, ComboBox, Control, Expander, Grid, StackPanel, TextBlock, TextBox,
};
use ferroui_designer_core::MarkupValue;
use std::cell::RefCell;
use std::rc::Rc;

const MODES: &[&str] = &["Default", "OneWay", "TwoWay", "OneTime", "OneWayToSource"];
const TRIGGERS: &[&str] = &["Default", "PropertyChanged", "LostFocus", "Explicit"];
const SOURCES: &[&str] = &[
    "Keep current source",
    "DataContext",
    "Named element",
    "Self",
    "Templated parent",
    "Find ancestor",
    "Explicit source",
];
const OPTIONS: &[(PropertyBindingOption, &str)] = &[
    (PropertyBindingOption::Converter, "Converter"),
    (PropertyBindingOption::ConverterParameter, "Parameter"),
    (PropertyBindingOption::ConverterCulture, "Culture"),
    (PropertyBindingOption::StringFormat, "Format"),
    (PropertyBindingOption::FallbackValue, "Fallback"),
    (PropertyBindingOption::TargetNullValue, "Null value"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
struct OptionalValue {
    mode: i32,
    text: String,
}

struct OptionalField {
    mode: Ref<ComboBox>,
    text: Ref<TextBox>,
}
impl OptionalField {
    fn new(name: &str, help: &str) -> (Self, Ref<Control>) {
        let grid = Grid::new();
        grid.set_column_definitions(ColumnDefinitions::parse("92,6,*").unwrap());
        let mode = choice(
            &["Unset", "Text", "Expression"],
            &format!("{name}Mode"),
            &format!("{help} value kind"),
        );
        let text = text("", name, help);
        Grid::set_column(&text, 2);
        grid.children().add(mode.clone().upcast::<Control>());
        grid.children().add(text.clone().upcast::<Control>());
        let weak_text = text.downgrade();
        let weak_mode = mode.downgrade();
        mode.selection_changed(move |_, _| {
            if let Some((text, mode)) = weak_text.upgrade().zip(weak_mode.upgrade()) {
                text.set_is_enabled(mode.selected_index() > 0);
            }
        });
        text.set_is_enabled(false);
        (Self { mode, text }, grid.upcast::<Control>())
    }
    fn configure(&self, value: Option<&MarkupValue>) {
        let (mode, text) = match value {
            None => (0, String::new()),
            Some(MarkupValue::Literal(value)) => (1, value.clone()),
            Some(value) => (2, value_source(value)),
        };
        self.mode.set_selected_index(mode);
        self.text.set_text(Some(&text));
    }
    fn read(&self) -> OptionalValue {
        OptionalValue {
            mode: self.mode.selected_index(),
            text: self.text.text().unwrap_or_default(),
        }
    }
}
impl OptionalValue {
    fn value(&self, name: &str) -> Result<Option<MarkupValue>, String> {
        match self.mode {
            0 => Ok(None),
            1 => Ok(Some(MarkupValue::Literal(self.text.clone()))),
            2 => match MarkupValue::parse(self.text.trim())? {
                value @ MarkupValue::Extension(_) => Ok(Some(value)),
                _ => Err(format!(
                    "{name} requires a markup expression; choose Text for a literal"
                )),
            },
            _ => Err(format!("Choose a value kind for {name}")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Fields {
    path: String,
    source: i32,
    element: String,
    ancestor_type: String,
    ancestor_level: String,
    tree: i32,
    explicit: OptionalValue,
    mode: String,
    trigger: String,
    delay: String,
    options: Vec<OptionalValue>,
}

/// Native structured controls and an exact-expression fallback. `value()` accepts
/// either the fields or the expression. When both were edited, the user must reload
/// the fields explicitly, preventing a stale form from overwriting a newer expression.
pub struct PropertyBindingEditor {
    root: Ref<StackPanel>,
    expression: Ref<TextBox>,
    error: Ref<TextBlock>,
    path: Ref<TextBox>,
    source: Ref<ComboBox>,
    element: Ref<TextBox>,
    ancestor_type: Ref<TextBox>,
    ancestor_level: Ref<TextBox>,
    tree: Ref<ComboBox>,
    explicit: OptionalField,
    mode: Ref<ComboBox>,
    trigger: Ref<ComboBox>,
    delay: Ref<TextBox>,
    options: Vec<OptionalField>,
    source_rows: Vec<Ref<Control>>,
    extension: String,
    relative_source_extension: Option<String>,
    loaded_expression: RefCell<String>,
    loaded_fields: RefCell<Option<Fields>>,
}

impl PropertyBindingEditor {
    /// Create an unscoped editor. Relative-source creation requires the independently
    /// resolved extension supplied by `with_relative_source_extension`.
    pub fn new(
        source: Option<&str>,
        binding_extension: &str,
        named_elements: &[String],
    ) -> Rc<Self> {
        Self::with_relative_source_extension(source, binding_extension, None, named_elements)
    }

    /// Use independently resolved markup names when embedding in a XAML namespace scope.
    pub fn with_relative_source_extension(
        source: Option<&str>,
        binding_extension: &str,
        relative_source_extension: Option<&str>,
        named_elements: &[String],
    ) -> Rc<Self> {
        let root = StackPanel::new();
        root.set_spacing(6.0);
        let expression = text(
            source.unwrap_or_default(),
            "PropertyExpression",
            "Binding or markup expression",
        );
        expression.set_accepts_return(true);
        expression.set_min_height(58.0);
        expression.set_max_height(120.0);
        row(&root, "Expression", expression.clone().upcast::<Control>());
        let path = text("", "PropertyBindingPath", "Binding property path");
        row(&root, "Path", path.clone().upcast::<Control>());
        let source_choice = choice(SOURCES, "PropertyBindingSource", "Binding source");
        row(&root, "Source", source_choice.clone().upcast::<Control>());
        let element = text("", "PropertyBindingElement", "Binding source element name");
        let element_body = StackPanel::new();
        element_body.set_spacing(4.0);
        element_body
            .children()
            .add(element.clone().upcast::<Control>());
        if !named_elements.is_empty() {
            let mut names = vec!["Choose an element".to_owned()];
            names.extend_from_slice(named_elements);
            let picker = choice(
                &names.iter().map(String::as_str).collect::<Vec<_>>(),
                "PropertyBindingElementChoice",
                "Elements in this XAML namescope",
            );
            let weak_element = element.downgrade();
            let weak_picker = picker.downgrade();
            picker.selection_changed(move |_, _| {
                if let Some((element, picker)) = weak_element.upgrade().zip(weak_picker.upgrade()) {
                    if picker.selected_index() > 0 {
                        if let Some(name) = names.get(picker.selected_index() as usize) {
                            element.set_text(Some(name));
                        }
                    }
                }
            });
            element_body.children().add(picker.upcast::<Control>());
        }
        let element_row = row(&root, "Element", element_body.upcast::<Control>());
        let ancestor = StackPanel::new();
        ancestor.set_spacing(4.0);
        let ancestor_type = text(
            "",
            "PropertyBindingAncestorType",
            "Ancestor XAML type, including its namespace prefix",
        );
        let ancestor_level = text(
            "1",
            "PropertyBindingAncestorLevel",
            "Ancestor level, starting at one",
        );
        let tree = choice(
            &["Visual", "Logical"],
            "PropertyBindingAncestorTree",
            "Ancestor lookup tree",
        );
        row(&ancestor, "Type", ancestor_type.clone().upcast::<Control>());
        row(
            &ancestor,
            "Level",
            ancestor_level.clone().upcast::<Control>(),
        );
        row(&ancestor, "Tree", tree.clone().upcast::<Control>());
        let ancestor_row = row(&root, "Ancestor", ancestor.upcast::<Control>());
        let (explicit, explicit_control) =
            OptionalField::new("PropertyBindingExplicitSource", "Explicit binding source");
        let explicit_row = row(&root, "Value", explicit_control);
        let mode = choice(MODES, "PropertyBindingMode", "Binding mode");
        row(&root, "Mode", mode.clone().upcast::<Control>());
        let trigger = choice(
            TRIGGERS,
            "PropertyBindingUpdateTrigger",
            "Update source trigger",
        );
        row(&root, "Update", trigger.clone().upcast::<Control>());
        let delay = text(
            "",
            "PropertyBindingDelay",
            "Binding delay in milliseconds; blank uses the default",
        );
        row(&root, "Delay (ms)", delay.clone().upcast::<Control>());
        let advanced = StackPanel::new();
        advanced.set_spacing(6.0);
        let hint = label(
            "Unset removes the argument. Text may be empty. Expression accepts resources, x:Null and other XAML extensions.",
        );
        hint.set_text_wrapping(TextWrapping::Wrap);
        advanced.children().add(hint.upcast::<Control>());
        let options = OPTIONS
            .iter()
            .map(|(option, title)| {
                let (field, control) = OptionalField::new(
                    &format!("PropertyBinding{}", option.name()),
                    &format!("Binding {}", option.name()),
                );
                row(&advanced, title, control);
                field
            })
            .collect();
        let expander = Expander::new();
        expander.set_name(Some("PropertyBindingAdvanced".into()));
        expander.set_header(Some(Rc::new(
            "Conversion, formatting and fallback".to_owned(),
        )));
        expander.set_content(Some(Control::boxed(advanced)));
        root.children().add(expander.upcast::<Control>());
        let actions = StackPanel::new();
        actions.set_orientation(ferroui_base::layout::Orientation::Horizontal);
        actions.set_spacing(6.0);
        let reload = button(
            "Read expression",
            "PropertyBindingRead",
            "Reload structured fields from the expression, discarding field drafts",
        );
        let build = button(
            "Update binding expression",
            "PropertyBindingBuild",
            "Update the local expression from the structured fields",
        );
        actions.children().add(reload.clone().upcast::<Control>());
        actions.children().add(build.clone().upcast::<Control>());
        root.children().add(actions.upcast::<Control>());
        let error = label("");
        error.set_name(Some("PropertyBindingError".into()));
        error.set_text_wrapping(TextWrapping::Wrap);
        AutomationProperties::set_name(&error, Some("Binding editor status"));
        root.children().add(error.clone().upcast::<Control>());
        let editor = Rc::new(Self {
            root,
            expression,
            error,
            path,
            source: source_choice,
            element,
            ancestor_type,
            ancestor_level,
            tree,
            explicit,
            mode,
            trigger,
            delay,
            options,
            source_rows: vec![element_row, ancestor_row, explicit_row],
            extension: binding_extension.into(),
            relative_source_extension: relative_source_extension.map(str::to_owned),
            loaded_expression: RefCell::new(String::new()),
            loaded_fields: RefCell::new(None),
        });
        editor.load_fields();
        let weak = Rc::downgrade(&editor);
        editor.source.selection_changed(move |_, _| {
            if let Some(editor) = weak.upgrade() {
                editor.update_source_rows();
            }
        });
        let weak = Rc::downgrade(&editor);
        reload.click(move |_, _| {
            if let Some(editor) = weak.upgrade() {
                editor.load_fields();
            }
        });
        let weak = Rc::downgrade(&editor);
        build.click(move |_, _| {
            if let Some(editor) = weak.upgrade() {
                match editor.build_expression(true) {
                    Ok(value) => {
                        editor.expression.set_text(Some(&value));
                        editor.load_fields();
                    }
                    Err(message) => editor.error.set_text(Some(&message)),
                }
            }
        });
        editor
    }

    pub fn control(&self) -> Ref<Control> {
        self.root.clone().upcast::<Control>()
    }

    /// Return the staged expression. No runtime object or source document is mutated.
    pub fn value(&self) -> Result<String, String> {
        self.build_expression(false)
    }

    /// Replace the entire local draft and populate its structured fields.
    pub fn set_expression(&self, value: &str) {
        self.expression.set_text(Some(value));
        self.load_fields();
    }

    fn read_fields(&self) -> Fields {
        Fields {
            path: self.path.text().unwrap_or_default(),
            source: self.source.selected_index(),
            element: self.element.text().unwrap_or_default(),
            ancestor_type: self.ancestor_type.text().unwrap_or_default(),
            ancestor_level: self.ancestor_level.text().unwrap_or_default(),
            tree: self.tree.selected_index(),
            explicit: self.explicit.read(),
            mode: selected(&self.mode),
            trigger: selected(&self.trigger),
            delay: self.delay.text().unwrap_or_default(),
            options: self.options.iter().map(OptionalField::read).collect(),
        }
    }

    fn load_fields(&self) {
        let source = self.expression.text().unwrap_or_default();
        let draft = PropertyBindingDraft::new(&source, &self.extension)
            .ok()
            .map(|draft| {
                draft.with_relative_source_extension(self.relative_source_extension.as_deref())
            });
        self.path.set_text(Some(
            draft
                .as_ref()
                .and_then(|draft| draft.literal("Path"))
                .unwrap_or(""),
        ));
        configure_choice(
            &self.mode,
            MODES,
            draft
                .as_ref()
                .and_then(|draft| draft.literal("Mode"))
                .unwrap_or("Default"),
        );
        configure_choice(
            &self.trigger,
            TRIGGERS,
            draft
                .as_ref()
                .and_then(|draft| draft.literal("UpdateSourceTrigger"))
                .unwrap_or("Default"),
        );
        self.delay.set_text(Some(
            draft
                .as_ref()
                .and_then(|draft| draft.literal("Delay"))
                .unwrap_or(""),
        ));
        for ((option, _), field) in OPTIONS.iter().zip(&self.options) {
            field.configure(
                draft
                    .as_ref()
                    .and_then(|draft| draft.argument(option.name())),
            );
        }
        self.element.set_text(Some(""));
        self.ancestor_type.set_text(Some(""));
        self.ancestor_level.set_text(Some("1"));
        self.tree.set_selected_index(0);
        self.explicit.configure(None);
        let selected_source = match draft
            .as_ref()
            .and_then(PropertyBindingDraft::source_specification)
        {
            Some(PropertyBindingSource::DataContext) => 1,
            Some(PropertyBindingSource::ElementName(name)) => {
                self.element.set_text(Some(&name));
                2
            }
            Some(PropertyBindingSource::SelfValue) => 3,
            Some(PropertyBindingSource::TemplatedParent) => 4,
            Some(PropertyBindingSource::Ancestor {
                type_name,
                level,
                logical_tree,
            }) => {
                self.ancestor_type.set_text(Some(&type_name));
                self.ancestor_level.set_text(Some(&level.to_string()));
                self.tree.set_selected_index(i32::from(logical_tree));
                5
            }
            Some(PropertyBindingSource::Explicit(value)) => {
                self.explicit.configure(Some(&value));
                6
            }
            None => {
                if source.is_empty() {
                    1
                } else {
                    0
                }
            }
        };
        self.source.set_selected_index(selected_source);
        self.update_source_rows();
        self.error.set_text(Some(if !source.is_empty() && draft.is_none() { "This expression remains editable as source. Updating the structured fields explicitly creates a framework binding." } else { "" }));
        *self.loaded_expression.borrow_mut() = source;
        *self.loaded_fields.borrow_mut() = Some(self.read_fields());
    }

    fn update_source_rows(&self) {
        for (row, index) in self.source_rows.iter().zip([2, 5, 6]) {
            row.set_is_visible(self.source.selected_index() == index);
        }
    }

    fn build_expression(&self, force_binding: bool) -> Result<String, String> {
        let raw = self.expression.text().unwrap_or_default();
        let fields = self.read_fields();
        let loaded = self.loaded_fields.borrow();
        let initial = loaded.as_ref().ok_or("The binding editor is not ready")?;
        let changed = &fields != initial;
        if !changed && !force_binding {
            if !matches!(MarkupValue::parse(&raw)?, MarkupValue::Extension(_)) {
                return Err("Enter a XAML markup expression".into());
            }
            if let Ok(draft) = PropertyBindingDraft::new(&raw, &self.extension) {
                draft.validate()?;
            }
            return Ok(raw);
        }
        if raw != *self.loaded_expression.borrow() {
            return Err("The expression changed after the fields were loaded. Choose Read expression before updating the binding".into());
        }
        let existing = if raw.is_empty() {
            None
        } else {
            match MarkupValue::parse(&raw)? {
                MarkupValue::Extension(extension) if extension.name == self.extension => {
                    Some(PropertyBindingDraft::new(&raw, &self.extension)?)
                }
                MarkupValue::Extension(_) => None,
                _ => {
                    return Err(
                        "Enter a XAML markup expression before editing binding fields".into(),
                    );
                }
            }
        };
        let created = existing.is_none();
        let mut draft = existing
            .unwrap_or(PropertyBindingDraft::empty(&self.extension)?)
            .with_relative_source_extension(self.relative_source_extension.as_deref());
        if fields.path != initial.path || created {
            draft.set_path(&fields.path);
        }
        let source_changed = fields.source != initial.source
            || match fields.source {
                2 => fields.element != initial.element,
                5 => {
                    fields.ancestor_type != initial.ancestor_type
                        || fields.ancestor_level != initial.ancestor_level
                        || fields.tree != initial.tree
                }
                6 => fields.explicit != initial.explicit,
                _ => false,
            };
        if source_changed || created {
            let source = match fields.source {
                0 if !created => None,
                0 | 1 => Some(PropertyBindingSource::DataContext),
                2 => Some(PropertyBindingSource::ElementName(fields.element.clone())),
                3 => Some(PropertyBindingSource::SelfValue),
                4 => Some(PropertyBindingSource::TemplatedParent),
                5 => Some(PropertyBindingSource::Ancestor {
                    type_name: fields.ancestor_type.clone(),
                    level: fields
                        .ancestor_level
                        .trim()
                        .parse()
                        .map_err(|_| "Ancestor level must be an integer starting at 1")?,
                    logical_tree: fields.tree == 1,
                }),
                6 => Some(PropertyBindingSource::Explicit(
                    fields
                        .explicit
                        .value("Source")?
                        .ok_or("Choose Text or Expression for the explicit binding source")?,
                )),
                _ => return Err("Choose a binding source".into()),
            };
            if let Some(source) = source {
                draft.set_source(source)?;
            }
        }
        if fields.mode != initial.mode || created {
            draft.set_mode(&fields.mode)?;
        }
        if fields.trigger != initial.trigger || created {
            draft.set_update_source_trigger(&fields.trigger)?;
        }
        if fields.delay != initial.delay || created {
            draft.set_delay(if fields.delay.trim().is_empty() {
                None
            } else {
                Some(
                    fields.delay.trim().parse().map_err(
                        |_| "Binding delay must be a nonnegative integer in milliseconds",
                    )?,
                )
            })?;
        }
        for (((option, _), value), previous) in
            OPTIONS.iter().zip(&fields.options).zip(&initial.options)
        {
            if value != previous || created {
                draft.set_option(*option, value.value(option.name())?)?;
            }
        }
        draft.validate()?;
        Ok(draft.source())
    }
}

fn selected(control: &ComboBox) -> String {
    control
        .selected_item()
        .and_then(|item| item.downcast_ref::<String>().cloned())
        .unwrap_or_default()
}
fn configure_choice(control: &ComboBox, values: &[&str], value: &str) {
    control.items().clear();
    for item in values {
        control.items().add(Some(Rc::new((*item).to_owned())));
    }
    let index = values
        .iter()
        .position(|item| *item == value)
        .unwrap_or_else(|| {
            control.items().add(Some(Rc::new(value.to_owned())));
            values.len()
        });
    control.set_selected_index(index as i32);
}
fn choice(values: &[&str], name: &str, help: &str) -> Ref<ComboBox> {
    let field = ComboBox::new();
    field.set_name(Some(name.into()));
    field.classes().add("designer-property-field");
    field.set_min_width(0.0);
    configure_choice(&field, values, values[0]);
    AutomationProperties::set_name(&field, Some(help));
    field
}
fn text(value: &str, name: &str, help: &str) -> Ref<TextBox> {
    let field = TextBox::new();
    field.set_name(Some(name.into()));
    field.classes().add("designer-property-field");
    field.set_text(Some(value));
    field.set_min_height(28.0);
    field.set_font_size(12.0);
    AutomationProperties::set_name(&field, Some(help));
    field
}
fn label(value: &str) -> Ref<TextBlock> {
    let label = TextBlock::new();
    label.set_text(Some(value));
    label.set_font_size(11.0);
    label
}
fn row(body: &StackPanel, title: &str, control: Ref<Control>) -> Ref<Control> {
    let grid = Grid::new();
    grid.set_column_definitions(ColumnDefinitions::parse("85,*").unwrap());
    let title = label(title);
    title.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
    grid.children().add(title.upcast::<Control>());
    Grid::set_column(&control, 1);
    grid.children().add(control);
    body.children().add(grid.clone().upcast::<Control>());
    grid.upcast::<Control>()
}
fn button(value: &str, name: &str, help: &str) -> Ref<Button> {
    let button = Button::new();
    button.set_name(Some(name.into()));
    button.set_content(Some(Rc::new(value.to_owned())));
    button.set_min_height(26.0);
    AutomationProperties::set_name(&button, Some(help));
    button
}

#[cfg(test)]
#[path = "property_binding_editor_tests.rs"]
mod tests;
