//! Port of `Diagnostics/VisualTreeDebug.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::BindingPriority;
use ferroui_base::diagnostics::FerroObjectDiagnosticExtensions;
use ferroui_base::{BoxedValue, FerroPropertyRegistry, Visual};
use ferroui_controls::Control;
use std::fmt::Write;
use std::rc::Rc;

/// Prints a visual tree as text, for debugging.
pub struct VisualTreeDebug;

impl VisualTreeDebug {
    /// The visual tree of `visual` as text: one line per visual (the name of its type, and the
    /// classes of a control), the properties of a control that have a value, each with the
    /// priority of the value, and the children, indented.
    pub fn print_visual_tree(visual: &Visual) -> String {
        let mut result = String::new();
        Self::print_visual_tree_into(visual, &mut result, 0);
        result
    }

    fn print_visual_tree_into(visual: &Visual, builder: &mut String, indent: i32) {
        let control = visual.to_ref().cast::<Control>();

        builder.push_str(&Self::indent(indent - 1));

        if indent > 0 {
            builder.push_str(" +- ");
        }

        builder.push_str(visual.get_type().name());

        if let Some(control) = &control {
            builder.push(' ');
            // `control.Classes.ToString()`.
            let classes: BoxedValue = Rc::new(control.classes());
            builder.push_str(&ValueTypes::to_display_string(Some(&classes)));
            builder.push('\n');

            for &property in FerroPropertyRegistry::instance().get_registered(control.get_type()).iter() {
                let value = control.get_diagnostic(property);

                if value.priority() != BindingPriority::Unset {
                    builder.push_str(&Self::indent(indent));
                    builder.push_str(" |  ");
                    builder.push_str(value.property().name());
                    builder.push_str(" = ");
                    let shown = ValueTypes::normalize(value.value().clone());
                    builder.push_str(&ValueTypes::to_display_string(shown.as_ref()));
                    builder.push_str(" [");
                    let _ = write!(builder, "{:?}", value.priority());
                    builder.push_str("]\n");
                }
            }
        } else {
            builder.push('\n');
        }

        for child in visual.visual_children().snapshot().iter() {
            Self::print_visual_tree_into(child, builder, indent + 1);
        }
    }

    fn indent(indent: i32) -> String {
        " ".repeat(indent.max(0) as usize * 4)
    }
}
