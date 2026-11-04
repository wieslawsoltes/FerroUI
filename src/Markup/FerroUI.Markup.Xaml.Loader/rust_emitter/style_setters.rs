//! A typed dynamic Setter.Value site and the terminal run of such sites on a child Style.
//! Keep each original call available so any intervening user code is an ordering barrier.
use std::collections::HashMap;
use std::fmt::Write;

use super::emitter::{is_identifier, is_plain_borrow};

use super::selector_plans::SelectorPlans;

const STYLE: &str = "::ferroui_base::styling::Style";
const BASE: &str = "::ferroui_base::styling::StyleBase";

struct DynamicSetter {
    receiver: String,
    property: String,
    key: String,
}

#[derive(Default)]
pub(super) struct StyleSetters {
    sites: HashMap<String, DynamicSetter>,
    getters: HashMap<String, String>,
    pub(super) helpers: String,
}

impl StyleSetters {
    pub(super) fn program_getters(&self) -> impl Iterator<Item = (&str, &str)> {
        self.getters.iter().map(|(expression, name)| (expression.as_str(), name.as_str()))
    }

    pub(super) fn getter_name(&self, expression: &str) -> Option<&str> {
        self.getters.get(expression).map(String::as_str)
    }

    pub(super) fn dynamic(&mut self, receiver: &str, property: &str, key: &str) -> String {
        let call = format!("    rt::add_dynamic_resource_setter_with_parent(&context, {receiver}, {property}, {key});");
        // A getter invocation may initialize metadata. Keep it in the loop at its exact
        // original point. Only parameterless static paths can be hoisted into a thunk.
        if let Some(path) = property
            .strip_prefix("rt::property(")
            .and_then(|p| p.strip_suffix("())"))
        {
            if path.trim_start_matches("::").split("::").all(is_identifier) {
                self.sites
                    .entry(call.clone())
                    .or_insert_with(|| DynamicSetter {
                        receiver: receiver.to_owned(),
                        property: property.to_owned(),
                        key: key.to_owned(),
                    });
            }
        }
        call
    }

    /// Replace the exact framework Style.Add overload and, where present, the dynamic
    /// setter run immediately preceding its pop. Nested styles and mixed setters are
    /// boundaries. A singleton still folds its pop, fallible cast and Add into the call.
    pub(super) fn attach(
        &mut self,
        document: &str,
        selectors: &mut SelectorPlans,
        retain_child: bool,
        lines: &mut Vec<String>,
        start: usize,
        child: &str,
        call: &str,
        line: i32,
        column: i32,
    ) -> Option<()> {
        if !is_identifier(child) {
            return None;
        }
        let parent = call
            .strip_prefix(&format!("{BASE}::__markup_Add_1("))?
            .strip_suffix(&format!(", rt::cast({child}.clone(), {line}, {column})?)"))?;
        if !is_plain_borrow(parent) {
            return None;
        }
        let ctor = format!("    let {child} = {STYLE}::new();");
        let push = format!("    rt::push_parent(&context, &{child});");
        let first = (start..lines.len()).find(|&i| !lines[i].trim_start().starts_with("// "));
        if let Some(i) = first {
            if lines[i] == ctor && lines.get(i + 1) == Some(&push) {
                lines[i] = format!("    let {child} = rt::style_program::begin(&context);");
                lines.remove(i + 1);
            }
        }

        let receiver = format!("{child}.upcast_ref::<{BASE}>()");
        let mut rows = Vec::new();
        let mut boundary = lines.len();
        if lines
            .last()
            .is_some_and(|l| l == "    context.pop_parent();")
        {
            let mut cursor = boundary - 1;
            while cursor > start {
                let Some(site) = self.sites.get(&lines[cursor - 1]) else {
                    break;
                };
                if site.receiver != receiver {
                    break;
                }
                let index = self.getters.len();
                let getter = self
                    .getters
                    .entry(site.property.clone())
                    .or_insert_with(|| {
                        let name = format!("__{document}_style_property_{index}");
                        writeln!(
                            self.helpers,
                            "fn {name}() -> &'static ::ferroui_base::FerroProperty {{ {} }}",
                            site.property
                        )
                        .unwrap();
                        name
                    });
                let row = format!("            ({getter}, {}),", site.key);
                cursor -= 1;
                let end = cursor;
                while cursor > start && lines[cursor - 1].trim_start().starts_with("// ") {
                    cursor -= 1;
                }
                rows.push((cursor..end, row));
                boundary = cursor;
            }
        }
        if rows.is_empty() {
            lines.push(format!(
                "    rt::style_program::attach({parent}, &{child}, {line}, {column})?;"
            ));
        } else {
            let mut result = vec![
                format!("    rt::style_program::finish_dynamic(&context, {parent}, &{child}, {{"),
                "        &[".into(),
            ];
            for (markers, row) in rows.into_iter().rev() {
                result.extend(
                    lines[markers]
                        .iter()
                        .map(|m| format!("            {}", m.trim_start())),
                );
                result.push(row);
            }
            result.push("        ]".into());
            result.push(format!("    }}, {line}, {column})?;"));
            lines.truncate(boundary);
            lines.extend(result);
        }
        // Fuse a freshly created child's constant-operand selector. A prefix
        // selector, a custom statement or any other order is a hard boundary.
        if let Some(index) = first {
            Self::selected_child(selectors, lines, index, child, parent, retain_child);
        }
        Some(())
    }

    fn selected_child(
        selectors: &mut SelectorPlans,
        lines: &mut Vec<String>,
        index: usize,
        child: &str,
        parent: &str,
        retain_child: bool,
    ) -> Option<()> {
        let [begin, marker, build, assign] = lines.get(index..index + 4)? else {
            return None;
        };
        if begin != &format!("    let {child} = rt::style_program::begin(&context);")
            || !marker.trim_start().starts_with("// ")
        {
            return None;
        }
        let (selector, call) = build
            .strip_prefix("    let ")?
            .strip_suffix(';')?
            .split_once(" = ")?;
        if !is_identifier(selector) || assign != &format!("    {STYLE}::__markup_set_Selector(&{child}, ::core::option::Option::Some({selector}.clone()));") {
            return None;
        }
        let complete = !retain_child
            && lines.get(index + 4).is_some_and(|line| {
                line == &format!(
                    "    rt::style_program::finish_dynamic(&context, {parent}, &{child}, {{"
                )
            });
        let call = selectors.start_style(call, complete.then_some(parent))?;
        let (end, replacement) = if complete {
            (index + 5, format!("    {call}, {{"))
        } else {
            (index + 4, format!("    let {child} = {call});"))
        };
        lines.splice(index..end, [marker.clone(), replacement]);
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_setters_share_getters_and_keep_order_and_locations() {
        let mut setters = StyleSetters::default();
        let receiver = format!("style_2.upcast_ref::<{BASE}>()");
        let parent = format!("style_1.upcast_ref::<{BASE}>()");
        let call = format!("{BASE}::__markup_Add_1({parent}, rt::cast(style_2.clone(), 3, 7)?)");
        let property = "rt::property(::controls::Button::foreground_property())";
        let first = setters.dynamic(&receiver, property, "\"first\"");
        let second = setters.dynamic(&receiver, property, "\"second\"");
        let mut lines = vec![
            first.clone(),
            "    custom();".into(),
            first,
            "    // doc(2,9) Value".into(),
            second,
            "    context.pop_parent();".into(),
        ];
        setters
            .attach(
                "doc",
                &mut SelectorPlans::default(),
                false,
                &mut lines,
                0,
                "style_2",
                &call,
                3,
                7,
            )
            .unwrap();
        assert!(lines[0].contains("add_dynamic_resource_setter_with_parent"));
        let generated = lines[2..].join("\n");
        assert!(generated.find("\"first\"").unwrap() < generated.find("doc(2,9)").unwrap());
        assert!(generated.find("doc(2,9)").unwrap() < generated.find("\"second\"").unwrap());
        assert!(generated.ends_with("}, 3, 7)?;"));
        assert_eq!(setters.helpers.matches("fn ").count(), 1);
    }
}
