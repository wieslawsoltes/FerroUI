//! Extensible native property editors. Factories create controls and send semantic values to the
//! host callback; they never mutate a runtime object behind the source document.
use ferroui_base::input::{InputElement, Key};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::utilities::Decimal;
use ferroui_base::{Ref, Thickness};
use ferroui_controls::{
    Button, CheckBox, ColumnDefinitions, ComboBox, Control, Grid, NumericUpDown, TextBox, ToolTip,
};
use ferroui_designer_core::NodeId;
use std::rc::Rc;

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
    fn create(&self, context: &PropertyEditorContext) -> Option<Ref<Control>>;
}

pub(crate) fn built_in(context: &PropertyEditorContext) -> Ref<Control> {
    let expression = context
        .value
        .as_ref()
        .is_some_and(|value| value.starts_with('{'));
    if !expression
        && context
            .runtime_type
            .as_ref()
            .is_some_and(|name| name == "bool" || name.ends_with("Option<bool>"))
    {
        let editor = CheckBox::new();
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
        if let Some(kind) = components_kind(context) {
            let values = match context.value.as_deref() {
                Some(value) => kind.parse(value).map(Some),
                None => Some(None),
            };
            if let Some(values) = values {
                let editor = crate::property_controls::PropertyComponentsEditor::new(
                    kind,
                    context.property != "Margin",
                );
                if editor.configure(values).is_ok() {
                    let apply = context.apply.clone();
                    editor.on_commit(move |values| {
                        apply(Some(crate::property_controls::components_text(values)))
                    });
                    return editor.control();
                }
            }
        }
        if let Some(editor) = numeric_editor(context) {
            return editor;
        }
        if let Some(editor) = alignment_editor(context) {
            return editor;
        }
        let values: &[&str] = match context.property.as_str() {
            "Orientation" => &["Horizontal", "Vertical"],
            "DockPanel.Dock" => &["Left", "Top", "Right", "Bottom"],
            "TextWrapping" => &["NoWrap", "Wrap", "WrapWithOverflow"],
            "Stretch" => &["None", "Fill", "Uniform", "UniformToFill"],
            "FlowDirection" => &["LeftToRight", "RightToLeft"],
            _ => &[],
        };
        if !values.is_empty() {
            let editor = ComboBox::new();
            let offset = if context.mixed {
                editor
                    .items()
                    .add(Some(Rc::new("(multiple values)".to_owned())));
                2
            } else {
                1
            };
            editor.items().add(Some(Rc::new("(inherited)".to_owned())));
            for value in values {
                editor.items().add(Some(Rc::new((*value).to_owned())));
            }
            editor.set_selected_index(
                context
                    .value
                    .as_deref()
                    .and_then(|current| {
                        values
                            .iter()
                            .position(|value| value.eq_ignore_ascii_case(current))
                    })
                    .map(|index| index as i32 + offset)
                    .unwrap_or(0),
            );
            let weak = editor.downgrade();
            let values = values
                .iter()
                .map(|value| (*value).to_owned())
                .collect::<Vec<_>>();
            let apply = context.apply.clone();
            editor.selection_changed(move |_, _| {
                if let Some(editor) = weak.upgrade() {
                    let index = editor.selected_index();
                    if offset == 2 && index == 0 {
                        return;
                    }
                    apply(if index < offset {
                        None
                    } else {
                        values.get((index - offset) as usize).cloned()
                    });
                }
            });
            return editor.upcast::<Control>();
        }
    }
    let editor = TextBox::new();
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

/// Native buttons provide focus, keyboard activation and accessibility for the frequently
/// used alignment choices. Local/expressed state and reset remain owned by the row.
fn alignment_editor(context: &PropertyEditorContext) -> Option<Ref<Control>> {
    use ferroui_controls::automation::AutomationProperties;
    let choices = match context.property.as_str() {
        "HorizontalAlignment" | "HorizontalContentAlignment" => [
            ("L", "Left"),
            ("C", "Center"),
            ("R", "Right"),
            ("↔", "Stretch"),
        ],
        "VerticalAlignment" | "VerticalContentAlignment" => [
            ("T", "Top"),
            ("C", "Center"),
            ("B", "Bottom"),
            ("↕", "Stretch"),
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
    for (index, (label, value)) in choices.into_iter().enumerate() {
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
        button.set_content(Some(Rc::new(label.to_owned())));
        button.set_min_width(0.0);
        button.set_min_height(26.0);
        button.set_padding(Thickness::new(2.0, 3.0, 2.0, 3.0));
        button.set_margin(Thickness::new(0.0, 0.0, 1.0, 0.0));
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

/// Numeric values retain native keyboard/spinner behavior. Special XAML values such as Auto,
/// NaN and expressions deliberately use the source editor, preserving their lexical meaning.
fn numeric_editor(context: &PropertyEditorContext) -> Option<Ref<Control>> {
    let options = numeric_options(context)?;
    let value = match context.value.as_deref() {
        Some(value) => Some(Decimal::parse(value).ok()?),
        None => None,
    };
    let editor = NumericUpDown::new();
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

pub(crate) fn components_kind(
    context: &PropertyEditorContext,
) -> Option<crate::property_controls::PropertyComponentsKind> {
    use crate::property_controls::PropertyComponentsKind;
    if context.property == "CornerRadius"
        || context
            .runtime_type
            .as_ref()
            .is_some_and(|name| name.ends_with("CornerRadius"))
    {
        Some(PropertyComponentsKind::CornerRadius)
    } else if matches!(
        context.property.as_str(),
        "Margin" | "Padding" | "BorderThickness"
    ) || context
        .runtime_type
        .as_ref()
        .is_some_and(|name| name.ends_with("Thickness"))
    {
        Some(PropertyComponentsKind::Thickness)
    } else {
        None
    }
}

pub(crate) fn numeric_options(
    context: &PropertyEditorContext,
) -> Option<crate::property_controls::PropertyNumberOptions> {
    let numeric = matches!(
        context.runtime_type.as_deref(),
        Some("f32" | "f64" | "i16" | "i32" | "i64" | "u16" | "u32" | "u64" | "usize")
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
            context.runtime_type.as_deref(),
            Some("i16" | "i32" | "i64" | "u16" | "u32" | "u64" | "usize")
        ) || matches!(
            context.property.as_str(),
            "Grid.Row" | "Grid.Column" | "Grid.RowSpan" | "Grid.ColumnSpan"
        ),
        ..Default::default()
    };
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
        context.runtime_type.as_deref(),
        Some("u16" | "u32" | "u64" | "usize")
    ) {
        options.minimum = 0.0;
    }
    Some(options)
}
