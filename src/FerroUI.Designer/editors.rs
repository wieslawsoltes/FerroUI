//! Extensible native property editors. Factories create controls and send semantic values to the
//! host callback; they never mutate a runtime object behind the source document.
use ferroui_base::input::{InputElement, Key};
use ferroui_base::Ref;
use ferroui_controls::{CheckBox, ComboBox, Control, TextBox};
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
        return editor.upcast();
    }
    if !expression {
        let values: &[&str] = match context.property.as_str() {
            "HorizontalAlignment" | "HorizontalContentAlignment" => {
                &["Left", "Center", "Right", "Stretch"]
            }
            "VerticalAlignment" | "VerticalContentAlignment" => {
                &["Top", "Center", "Bottom", "Stretch"]
            }
            "Orientation" => &["Horizontal", "Vertical"],
            "DockPanel.Dock" => &["Left", "Top", "Right", "Bottom"],
            "TextWrapping" => &["NoWrap", "Wrap", "WrapWithOverflow"],
            "Stretch" => &["None", "Fill", "Uniform", "UniformToFill"],
            "FlowDirection" => &["LeftToRight", "RightToLeft"],
            _ => &[],
        };
        if !values.is_empty() {
            let editor = ComboBox::new();
            editor.items().add(Some(Rc::new("(inherited)".to_owned())));
            for value in values {
                editor.items().add(Some(Rc::new((*value).to_owned())));
            }
            editor.set_selected_index(
                context
                    .value
                    .as_deref()
                    .and_then(|current| values.iter().position(|value| *value == current))
                    .map(|index| index as i32 + 1)
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
                    apply(if index <= 0 {
                        None
                    } else {
                        values.get(index as usize - 1).cloned()
                    });
                }
            });
            return editor.upcast();
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
    editor.add_handler(InputElement::key_down_event(), move |_, event| {
        if event.key != Key::Enter {
            return;
        }
        if let Some(editor) = weak.upgrade() {
            let value = editor.text().unwrap_or_default();
            // Empty text is an authored empty value. Reset is a separate action.
            apply(Some(value));
            event.set_handled(true);
        }
    });
    editor.upcast()
}
