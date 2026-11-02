//! Share a repeated typed template setter without introducing a property table.
//! The getter runs at each assignment, so its registration and observers retain their order.
use std::collections::HashMap;
use std::fmt::Write;

use super::emitter::is_identifier;

pub(super) const TEMPLATE_PRIORITY: &str = "::ferroui_base::data::BindingPriority::Template";

#[derive(Default)]
pub(super) struct TemplateValues {
    sites: HashMap<String, (usize, String)>,
    operations: Vec<(String, i32, i32)>,
    ids: HashMap<String, usize>,
}

fn absolute_path(text: &str) -> bool {
    text.strip_prefix("::")
        .is_some_and(|path| path.split("::").all(is_identifier))
}

/// Restrict sharing to values whose emitted expression can neither capture a local
/// nor fail. Complex conversions, extension results and user calls stay in the caller.
fn closed_value(text: &str) -> bool {
    let value = text
        .strip_prefix("::core::clone::Clone::clone(&")
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(text);
    matches!(value, "true" | "false")
        || absolute_path(value)
        || value.rsplit_once('_').is_some_and(|(number, suffix)| {
            matches!(
                suffix,
                "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f32" | "f64"
            ) && number.parse::<f64>().is_ok()
        })
}

impl TemplateValues {
    pub(super) fn record(
        &mut self,
        statement: &str,
        receiver: &str,
        property: &str,
        value: &str,
        line: i32,
        column: i32,
    ) {
        if !is_identifier(receiver)
            || !property.strip_suffix("()").is_some_and(absolute_path)
            || !closed_value(value)
        {
            return;
        }
        let operation =
            format!("target.set_value_with_priority({property}, {value}, {TEMPLATE_PRIORITY});");
        let index = *self.ids.entry(operation.clone()).or_insert_with(|| {
            let index = self.operations.len();
            self.operations.push((operation, line, column));
            index
        });
        self.sites
            .entry(statement.to_owned())
            .or_insert_with(|| (index, receiver.to_owned()));
    }

    pub(super) fn lower(self, source: String, document: &str, display: &str) -> String {
        if self.sites.is_empty() {
            return source;
        }
        // Count only surviving sites, after intrinsic resource lowering and factory sharing.
        let mut uses = vec![0; self.operations.len()];
        for line in source.lines() {
            if let Some((index, _)) = self.sites.get(line.trim_start()) {
                uses[*index] += 1;
            }
        }
        let display = display.replace(['\r', '\n'], " ");
        let mut result = String::with_capacity(source.len());
        for line in source.lines() {
            let text = line.trim_start();
            if let Some((index, receiver)) =
                self.sites.get(text).filter(|(index, _)| uses[*index] >= 3)
            {
                let indent = &line[..line.len() - text.len()];
                writeln!(
                    result,
                    "{indent}__{document}_template_setter_{index}(&{receiver});"
                )
                .unwrap();
            } else {
                writeln!(result, "{line}").unwrap();
            }
        }
        for (index, (operation, line, column)) in self.operations.into_iter().enumerate() {
            if uses[index] >= 3 {
                writeln!(result, "\n#[inline(never)]\nfn __{document}_template_setter_{index}(target: &::ferroui_base::FerroObject) {{").unwrap();
                writeln!(
                    result,
                    "    // {display}({line},{column}) Template setter\n    {operation}\n}}"
                )
                .unwrap();
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_repeated_surviving_closed_setters_share_a_body() {
        let mut setters = TemplateValues::default();
        let call = format!(
            "control.set_value_with_priority(::ui::Panel::width(), 8.0_f64, {TEMPLATE_PRIORITY});"
        );
        let captured = call.replace("8.0_f64", "value_0");
        let absent = call.replace("::ui::Panel::width()", "::ui::Unused::width()");
        let rare = call.replace("8.0_f64", "4.0_f64");
        setters.record(&call, "control", "::ui::Panel::width()", "8.0_f64", 7, 9);
        setters.record(
            &captured,
            "control",
            "::ui::Panel::width()",
            "value_0",
            8,
            5,
        );
        setters.record(&absent, "control", "::ui::Unused::width()", "8.0_f64", 9, 5);
        setters.record(&rare, "control", "::ui::Panel::width()", "4.0_f64", 10, 5);
        let source = format!("    {call}\n    ordered_side_effect();\n    {captured}\n    {call}\n    {rare}\n    {call}\n");
        let result = setters.lower(source, "build_view", "view`a.xaml");
        assert_eq!(
            result.matches("fn __build_view_template_setter_").count(),
            1
        );
        assert_eq!(
            result
                .matches("__build_view_template_setter_0(&control);")
                .count(),
            3
        );
        assert!(result.contains(&format!("ordered_side_effect();\n    {captured}")));
        assert!(result.contains(&rare));
        assert!(!result.contains("Unused"));
        let map = super::super::compiled::position_map(
            &result,
            &[("build_view".into(), "view`a.xaml".into())],
        );
        assert!(map.contains("\"document\": \"view`a.xaml\", \"line\": 7, \"position\": 9"));
    }
}
