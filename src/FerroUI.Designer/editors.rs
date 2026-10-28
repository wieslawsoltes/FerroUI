//! Extensible native property editors. Factories create controls and send semantic values to the
//! host callback; they never mutate a runtime object behind the source document.
use ferroui_base::input::{InputElement, Key};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::utilities::Decimal;
use ferroui_base::{Ref, Thickness};
use ferroui_controls::{
    Button, CheckBox, ColumnDefinitions, Control, Grid, NumericUpDown, TextBox, ToolTip,
};
use ferroui_designer_core::NodeId;
use std::rc::Rc;

#[path = "property_editor_metadata.rs"]
mod metadata;
#[path = "property_object_draft.rs"]
mod object_draft;
#[path = "property_typed_editors.rs"]
mod typed;
#[path = "property_value_workbench.rs"]
mod workbench;
pub(crate) use metadata::{markup_type_for_node, resource_is_compatible};
pub use metadata::{
    PropertyEditorKind, PropertyEditorMetadata, PropertyLiteralValidator, PropertyResourceChoice,
    PropertyValueCommit, PropertyValueEdit, PropertyValueSource, resolve_property_editor_metadata,
};
pub use object_draft::PropertyObjectDraft;
pub use typed::PropertyVectorEditor;
pub use workbench::binding_expression;

#[cfg(test)]
#[path = "property_editor_system_tests.rs"]
mod tests;

#[derive(Clone)]
pub struct PropertyEditorContext {
    pub property: String,
    pub targets: Vec<NodeId>,
    pub value: Option<String>,
    pub mixed: bool,
    pub runtime_type: Option<String>,
    /// `None` removes the authored value; expressions are preserved as source strings.
    pub apply: Rc<dyn Fn(Option<String>)>,
}

/// Return a native control when this factory handles the property; otherwise let the next
/// factory try. Hosts register factories in priority order on [`crate::DesignerWorkspace`].
pub trait PropertyEditorFactory {
    fn create(&self, _context: &PropertyEditorContext) -> Option<Ref<Control>> {
        None
    }
    /// Metadata-aware factories can inspect real enum members, source origin, resource
    /// scope and an object draft. Existing factories continue to receive guarded values.
    fn create_with_metadata(
        &self,
        context: &PropertyEditorContext,
        _metadata: &PropertyEditorMetadata,
    ) -> Option<Ref<Control>> {
        self.create(context)
    }
}

/// Shared inspector/definition-editor pipeline. First matching registered factory wins;
/// built-ins and a source-safe fallback follow. Hosts supply a revision-bound commit.
pub fn create_property_editor(
    context: &PropertyEditorContext,
    metadata: &PropertyEditorMetadata,
    factories: &[Rc<dyn PropertyEditorFactory>],
) -> Ref<Control> {
    let inline = factories
        .iter()
        .find_map(|factory| factory.create_with_metadata(context, metadata))
        .unwrap_or_else(|| typed_builtin(context, metadata));
    workbench::attach(context, metadata, inline, factories)
}

pub(crate) fn built_in(context: &PropertyEditorContext) -> Ref<Control> {
    typed_builtin(context, &resolve_property_editor_metadata(context, None))
}

pub(super) fn typed_builtin(
    context: &PropertyEditorContext,
    metadata: &PropertyEditorMetadata,
) -> Ref<Control> {
    if metadata.object_source.is_some() {
        let field = TextBox::new();
        field.classes().add("designer-property-field");
        field.set_is_read_only(true);
        let title = metadata
            .object_source
            .as_deref()
            .and_then(|source| PropertyObjectDraft::new(source).ok())
            .and_then(|draft| {
                draft
                    .nodes()
                    .first()
                    .and_then(|id| draft.document().ast().node(*id))
                    .map(|node| node.local_name().to_owned())
            })
            .unwrap_or_else(|| "Object / collection".into());
        field.set_text(Some(&title));
        return field.upcast();
    }
    if context
        .value
        .as_deref()
        .is_some_and(metadata::is_expression)
    {
        return typed::text(context, metadata);
    }
    match &metadata.kind {
        PropertyEditorKind::Components(shape)
            if context
                .value
                .as_deref()
                .is_none_or(|value| shape.parse_components(value).is_ok()) =>
        {
            return PropertyVectorEditor::new(context, metadata, *shape).control();
        }
        PropertyEditorKind::Enumeration {
            values,
            flags: false,
        } => {
            // Preserve the compact native alignment/flow affordances for standard controls.
            if !uses_compact_enum_editor(context, values) {
                return typed::enumeration(context, metadata, values);
            }
        }
        PropertyEditorKind::Enumeration { flags: true, .. } => {
            return typed::text(context, metadata);
        }
        PropertyEditorKind::GridLength => return typed::grid_length(context, metadata),
        PropertyEditorKind::FontFamily | PropertyEditorKind::FontWeight => {
            return typed::font(context, metadata);
        }
        PropertyEditorKind::TimeSpan
        | PropertyEditorKind::Duration
        | PropertyEditorKind::Object
        | PropertyEditorKind::Transform
        | PropertyEditorKind::Text => return typed::text(context, metadata),
        _ => {}
    }
    let kind = metadata.kind.clone();
    let feedback = Rc::new(std::cell::RefCell::new(
        None::<ferroui_base::WeakRef<Control>>,
    ));
    let mut guarded = context.clone();
    let original = context.clone();
    let metadata = metadata.clone();
    let errors = feedback.clone();
    guarded.apply = Rc::new(move |value| {
        let edit = match value {
            Some(value) if metadata::is_expression(&value) => PropertyValueEdit::Expression(value),
            Some(value) => PropertyValueEdit::Literal(value),
            None => PropertyValueEdit::Reset,
        };
        let result = metadata.apply(&original, edit);
        if let Some(control) = errors.borrow().as_ref().and_then(|weak| weak.upgrade()) {
            match result {
                Ok(()) => ferroui_controls::DataValidationErrors::clear_errors(&control),
                Err(message) => ferroui_controls::DataValidationErrors::set_errors(
                    &control,
                    Some(vec![Rc::new(message)]),
                ),
            }
        }
    });
    let editor = legacy_built_in(&guarded, &kind);
    *feedback.borrow_mut() = Some(editor.downgrade());
    editor
}

fn uses_compact_enum_editor(context: &PropertyEditorContext, values: &[String]) -> bool {
    let expected: &[&str] = match context.property.as_str() {
        "HorizontalAlignment" | "HorizontalContentAlignment" => {
            &["Left", "Center", "Right", "Stretch"]
        }
        "VerticalAlignment" | "VerticalContentAlignment" => &["Top", "Center", "Bottom", "Stretch"],
        "TextAlignment" => &["Left", "Center", "Right", "Justify"],
        "Orientation" => &["Horizontal", "Vertical"],
        _ => return false,
    };
    // Standard-looking property names can belong to a completely different enum in
    // application controls. Preserve its reflected members and unknown source values.
    values.len() == expected.len()
        && expected
            .iter()
            .all(|name| values.iter().any(|value| value == name))
        && context.value.as_deref().is_none_or(|current| {
            expected
                .iter()
                .any(|name| current.eq_ignore_ascii_case(name))
        })
}

fn legacy_built_in(context: &PropertyEditorContext, kind: &PropertyEditorKind) -> Ref<Control> {
    let expression = context
        .value
        .as_ref()
        .is_some_and(|value| metadata::is_expression(value));
    if !expression && *kind == PropertyEditorKind::Boolean {
        let editor = CheckBox::new();
        editor.classes().add("property-boolean");
        editor.set_is_three_state(true);
        editor.set_is_checked(if context.mixed {
            None
        } else {
            context
                .value
                .as_deref()
                .and_then(|value| match value.to_ascii_lowercase().as_str() {
                    "true" => Some(true),
                    "false" => Some(false),
                    _ => None,
                })
        });
        let weak = editor.downgrade();
        let apply = context.apply.clone();
        editor.is_checked_changed(move |_, _| {
            if let Some(editor) = weak.upgrade() {
                apply(
                    editor
                        .is_checked()
                        .map(|value| if value { "True".into() } else { "False".into() }),
                );
            }
        });
        return editor.upcast::<Control>();
    }
    if !expression {
        if matches!(kind, PropertyEditorKind::Color | PropertyEditorKind::Brush) {
            if let Some(editor) = color_editor(context) {
                return editor;
            }
        }
        if let Some(kind) = (*kind == PropertyEditorKind::Thickness)
            .then_some(crate::property_controls::PropertyComponentsKind::Thickness)
        {
            let values = match context.value.as_deref() {
                Some(value) => kind.parse(value).map(Some),
                None => Some(None),
            };
            if let Some(values) = values {
                let editor = crate::property_controls::PropertyComponentsEditor::new(
                    kind,
                    matches!(context.property.as_str(), "Padding" | "BorderThickness"),
                );
                if editor.configure(values).is_ok() {
                    // Uniform authored/effective values occupy one field. Expanding the
                    // chain reveals individual sides without creating a source edit.
                    if values.is_some_and(|values| values.iter().all(|value| *value == values[0])) {
                        editor.set_linked(true);
                    }
                    let apply = context.apply.clone();
                    editor.on_commit(move |values| {
                        apply(Some(crate::property_controls::components_text(values)))
                    });
                    return editor.control();
                }
            }
        }
        if *kind == PropertyEditorKind::Number {
            if let Some(editor) = numeric_editor(context) {
                return editor;
            }
        }
        if matches!(kind, PropertyEditorKind::Enumeration { .. }) {
            if let Some(editor) = alignment_editor(context) {
                return editor;
            }
            if let Some(editor) = orientation_editor(context) {
                return editor;
            }
        }
    }
    let editor = TextBox::new();
    editor.classes().add("designer-property-field");
    editor.classes().set("property-expression", expression);
    editor.set_text(context.value.as_deref());
    editor.set_font_size(12.0);
    editor.set_placeholder_text(Some(if context.mixed {
        "Multiple values"
    } else {
        "Default / inherited"
    }));
    let weak = editor.downgrade();
    let apply = context.apply.clone();
    let original = context.value.clone();
    editor.add_handler(InputElement::key_down_event(), move |_, event| {
        if let Some(editor) = weak.upgrade() {
            match event.key {
                Key::Enter => {
                    let value = editor.text().unwrap_or_default();
                    // Empty text is an authored empty value. Reset is a separate action.
                    apply(Some(value));
                    event.set_handled(true);
                }
                Key::Escape => {
                    editor.set_text(original.as_deref());
                    event.set_handled(true);
                }
                _ => {}
            }
        }
    });
    editor.upcast::<Control>()
}

pub(crate) fn is_color_property(context: &PropertyEditorContext) -> bool {
    matches!(
        context.property.as_str(),
        "Background"
            | "Foreground"
            | "BorderBrush"
            | "Fill"
            | "Stroke"
            | "Color"
            | "CaretBrush"
            | "SelectionBrush"
            | "SelectionForegroundBrush"
    ) || context
        .runtime_type
        .as_deref()
        .is_some_and(|name| name.ends_with("::Color"))
}

fn color_editor(context: &PropertyEditorContext) -> Option<Ref<Control>> {
    if context
        .value
        .as_deref()
        .is_some_and(|value| ferroui_base::media::Color::parse(value.trim()).is_err())
    {
        return None;
    }
    let editor = crate::PropertyColorEditor::new().ok()?;
    editor
        .configure(context.value.as_deref(), context.mixed)
        .ok()?;
    let apply = context.apply.clone();
    editor.on_commit(move |value| apply(Some(value)));
    Some(editor.control())
}

/// Native buttons provide focus, keyboard activation and accessibility for the frequently
/// used alignment choices. Local/expressed state and reset remain owned by the row.
fn alignment_editor(context: &PropertyEditorContext) -> Option<Ref<Control>> {
    use ferroui_controls::automation::AutomationProperties;
    let choices = match context.property.as_str() {
        "HorizontalAlignment" | "HorizontalContentAlignment" => [
            (
                "F1 M2,1 L3.5,1 3.5,15 2,15 Z M5,4 L14,4 14,7 5,7 Z M5,9 L11,9 11,12 5,12 Z",
                "Left",
            ),
            (
                "F1 M7.25,1 L8.75,1 8.75,3 7.25,3 Z M7.25,7.5 L8.75,7.5 8.75,8.5 7.25,8.5 Z M7.25,13 L8.75,13 8.75,15 7.25,15 Z M2,4 L14,4 14,6.5 2,6.5 Z M4,9.5 L12,9.5 12,12 4,12 Z",
                "Center",
            ),
            (
                "F1 M12.5,1 L14,1 14,15 12.5,15 Z M2,4 L11,4 11,7 2,7 Z M5,9 L11,9 11,12 5,12 Z",
                "Right",
            ),
            (
                "F1 M1,1 L2.5,1 2.5,15 1,15 Z M13.5,1 L15,1 15,15 13.5,15 Z M4,4 L12,4 12,7 4,7 Z M4,9 L12,9 12,12 4,12 Z",
                "Stretch",
            ),
        ],
        "VerticalAlignment" | "VerticalContentAlignment" => [
            (
                "F1 M1,2 L15,2 15,3.5 1,3.5 Z M4,5 L7,5 7,14 4,14 Z M9,5 L12,5 12,11 9,11 Z",
                "Top",
            ),
            (
                "F1 M1,7.25 L3,7.25 3,8.75 1,8.75 Z M7.5,7.25 L8.5,7.25 8.5,8.75 7.5,8.75 Z M13,7.25 L15,7.25 15,8.75 13,8.75 Z M4,2 L6.5,2 6.5,14 4,14 Z M9.5,4 L12,4 12,12 9.5,12 Z",
                "Center",
            ),
            (
                "F1 M1,12.5 L15,12.5 15,14 1,14 Z M4,2 L7,2 7,11 4,11 Z M9,5 L12,5 12,11 9,11 Z",
                "Bottom",
            ),
            (
                "F1 M1,1 L15,1 15,2.5 1,2.5 Z M1,13.5 L15,13.5 15,15 1,15 Z M4,4 L7,4 7,12 4,12 Z M9,4 L12,4 12,12 9,12 Z",
                "Stretch",
            ),
        ],
        "TextAlignment" => [
            (
                "F1 M2,2 L14,2 14,3.5 2,3.5 Z M2,5.5 L10,5.5 10,7 2,7 Z M2,9 L14,9 14,10.5 2,10.5 Z M2,12.5 L10,12.5 10,14 2,14 Z",
                "Left",
            ),
            (
                "F1 M2,2 L14,2 14,3.5 2,3.5 Z M4,5.5 L12,5.5 12,7 4,7 Z M2,9 L14,9 14,10.5 2,10.5 Z M4,12.5 L12,12.5 12,14 4,14 Z",
                "Center",
            ),
            (
                "F1 M2,2 L14,2 14,3.5 2,3.5 Z M6,5.5 L14,5.5 14,7 6,7 Z M2,9 L14,9 14,10.5 2,10.5 Z M6,12.5 L14,12.5 14,14 6,14 Z",
                "Right",
            ),
            (
                "F1 M2,2 L14,2 14,3.5 2,3.5 Z M2,5.5 L14,5.5 14,7 2,7 Z M2,9 L14,9 14,10.5 2,10.5 Z M2,12.5 L14,12.5 14,14 2,14 Z",
                "Justify",
            ),
        ],
        _ => return None,
    };
    let root = Grid::new();
    root.classes().add("property-choice");
    root.set_column_definitions(ColumnDefinitions::parse("*,*,*,*").unwrap());
    ToolTip::set_tip(
        &root,
        Some(Rc::new(
            if context.mixed {
                "Multiple values"
            } else if context.value.is_none() {
                "Inherited alignment"
            } else {
                "Local alignment"
            }
            .to_owned(),
        )),
    );
    for (index, (path, value)) in choices.into_iter().enumerate() {
        let button = Button::new();
        button.classes().add("property-choice-item");
        button.classes().set(
            "active",
            !context.mixed
                && context
                    .value
                    .as_ref()
                    .is_some_and(|current| current.eq_ignore_ascii_case(value)),
        );
        let icon = crate::property_controls::property_icon(path, 14.0);
        button.set_content(Some(Control::boxed(&icon)));
        button.set_min_width(0.0);
        button.set_min_height(26.0);
        button.set_padding(Thickness::new(2.0, 3.0, 2.0, 3.0));
        button.set_margin(Thickness::new(0.0, 1.0, 2.0, 1.0));
        button.set_font_size(11.0);
        button.set_horizontal_alignment(ferroui_base::layout::HorizontalAlignment::Stretch);
        button.set_horizontal_content_alignment(ferroui_base::layout::HorizontalAlignment::Center);
        ToolTip::set_tip(
            &button,
            Some(Rc::new(format!("{}: {value}", context.property))),
        );
        AutomationProperties::set_name(&button, Some(&format!("{}: {value}", context.property)));
        let apply = context.apply.clone();
        button.click(move |_, _| apply(Some(value.to_owned())));
        Grid::set_column(&button, index as i32);
        root.children().add(button.upcast::<Control>());
    }
    Some(root.upcast::<Control>())
}

fn orientation_editor(context: &PropertyEditorContext) -> Option<Ref<Control>> {
    if context.property != "Orientation" {
        return None;
    }
    let root = Grid::new();
    root.classes().add("property-choice");
    root.set_column_definitions(ColumnDefinitions::parse("*,*").unwrap());
    for (index, (path, value)) in [
        (
            "F1 M1,3 L5,3 5,13 1,13 Z M6,3 L10,3 10,13 6,13 Z M11,3 L15,3 15,13 11,13 Z",
            "Horizontal",
        ),
        (
            "F1 M3,1 L13,1 13,5 3,5 Z M3,6 L13,6 13,10 3,10 Z M3,11 L13,11 13,15 3,15 Z",
            "Vertical",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let button = Button::new();
        button.classes().add("property-choice-item");
        button.classes().set(
            "active",
            !context.mixed
                && context
                    .value
                    .as_ref()
                    .is_some_and(|current| current.eq_ignore_ascii_case(value)),
        );
        let icon = crate::property_controls::property_icon(path, 14.0);
        button.set_content(Some(Control::boxed(&icon)));
        button.set_min_width(0.0);
        button.set_min_height(26.0);
        button.set_padding(Thickness::uniform(3.0));
        button.set_margin(Thickness::new(0.0, 1.0, 2.0, 1.0));
        button.set_horizontal_alignment(ferroui_base::layout::HorizontalAlignment::Stretch);
        button.set_horizontal_content_alignment(ferroui_base::layout::HorizontalAlignment::Center);
        let help = format!("{value} layout flow");
        ToolTip::set_tip(&button, Some(Rc::new(help.clone())));
        ferroui_controls::automation::AutomationProperties::set_name(&button, Some(&help));
        let apply = context.apply.clone();
        button.click(move |_, _| apply(Some(value.to_owned())));
        Grid::set_column(&button, index as i32);
        root.children().add(button.upcast::<Control>());
    }
    Some(root.upcast::<Control>())
}

/// Numeric values retain native keyboard/spinner behavior. Special XAML values such as Auto,
/// NaN and expressions deliberately use the source editor, preserving their lexical meaning.
fn numeric_editor(context: &PropertyEditorContext) -> Option<Ref<Control>> {
    let options = numeric_options(context)?;
    let value = match context.value.as_deref() {
        Some(value) => Some(Decimal::parse(value).ok()?),
        None => None,
    };
    let editor = NumericUpDown::new();
    editor.classes().add("designer-property-field");
    editor.set_show_button_spinner(false);
    editor.set_min_height(26.0);
    editor.set_padding(Thickness::new(7.0, 3.0, 5.0, 3.0));
    if options.minimum != -f64::MAX {
        editor.set_minimum(Decimal::parse(&options.minimum.to_string()).ok()?);
    }
    if options.maximum != f64::MAX {
        editor.set_maximum(Decimal::parse(&options.maximum.to_string()).ok()?);
    }
    editor.set_numeric_value(value);
    editor.set_increment(Decimal::parse(&options.step.to_string()).ok()?);
    editor.set_placeholder_text(Some(if context.mixed {
        "Multiple values"
    } else {
        "Inherited"
    }));
    // Typing is committed with Enter; scrubbing the label and modifier arrow keys each
    // request one transaction. Draft text never refreshes or replaces the focused row.
    let weak = editor.downgrade();
    let apply = context.apply.clone();
    editor.add_handler_with(
        InputElement::key_down_event(),
        move |_, event| {
            if event.key == Key::Enter {
                if let Some(editor) = weak.upgrade() {
                    apply(Some(editor.text().unwrap_or_default()));
                    event.set_handled(true);
                }
            }
        },
        RoutingStrategies::BUBBLE,
        true,
    );
    let weak = editor.downgrade();
    let original = context.value.clone();
    let apply = context.apply.clone();
    editor.add_handler_with(
        InputElement::key_down_event(),
        move |_, event| {
            let Some(editor) = weak.upgrade() else {
                return;
            };
            if event.key == Key::Escape {
                editor.set_text(original.as_deref());
                event.set_handled(true);
            } else if matches!(event.key, Key::Up | Key::Down) {
                let Some(value) = editor
                    .text()
                    .and_then(|text| text.parse::<f64>().ok())
                    .filter(|value| value.is_finite())
                else {
                    return;
                };
                let step = if event.key == Key::Up { 1.0 } else { -1.0 };
                let value = options.adjusted(value, step, event.key_modifiers);
                apply(Some(crate::property_controls::number_text(value)));
                event.set_handled(true);
            }
        },
        RoutingStrategies::TUNNEL,
        false,
    );
    let weak = editor.downgrade();
    let apply = context.apply.clone();
    editor.spinned(move |_| {
        if let Some(editor) = weak.upgrade() {
            if let Some(value) = editor.value() {
                apply(Some(value.to_string()));
            }
        }
    });
    Some(editor.upcast::<Control>())
}

pub(crate) fn numeric_options(
    context: &PropertyEditorContext,
) -> Option<crate::property_controls::PropertyNumberOptions> {
    let numeric = matches!(
        context.runtime_type.as_deref().map(metadata::type_leaf),
        Some(
            "f32"
                | "f64"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "isize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "usize"
        )
    ) || matches!(
        context.property.as_str(),
        "Canvas.Left"
            | "Canvas.Top"
            | "Canvas.Right"
            | "Canvas.Bottom"
            | "Grid.Row"
            | "Grid.Column"
            | "Grid.RowSpan"
            | "Grid.ColumnSpan"
    );
    if !numeric {
        return None;
    }
    let mut options = crate::property_controls::PropertyNumberOptions {
        integer: matches!(
            context.runtime_type.as_deref().map(metadata::type_leaf),
            Some("i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize")
        ) || matches!(
            context.property.as_str(),
            "Grid.Row" | "Grid.Column" | "Grid.RowSpan" | "Grid.ColumnSpan"
        ),
        ..Default::default()
    };
    // NumericUpDown and scrubbing currently use f64 for relative movement. Values
    // outside its exact integer range stay in a validated source field.
    if options.integer
        && context
            .value
            .as_deref()
            .and_then(|value| value.parse::<i128>().ok())
            .is_some_and(|value| value.unsigned_abs() > 9_007_199_254_740_991)
    {
        return None;
    }
    match context.property.as_str() {
        "Opacity" => {
            options.minimum = 0.0;
            options.maximum = 1.0;
            options.step = 0.05;
        }
        "Grid.RowSpan" | "Grid.ColumnSpan" => options.minimum = 1.0,
        "Width" | "Height" | "MinWidth" | "MinHeight" | "MaxWidth" | "MaxHeight" | "FontSize"
        | "Grid.Row" | "Grid.Column" | "Rows" | "Columns" | "FirstColumn" => {
            options.minimum = 0.0;
        }
        _ => {}
    }
    if matches!(
        context.runtime_type.as_deref().map(metadata::type_leaf),
        Some("u8" | "u16" | "u32" | "u64" | "usize")
    ) {
        options.minimum = 0.0;
    }
    Some(options)
}
