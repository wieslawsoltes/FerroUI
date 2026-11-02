//! Typed structural interning of linear selector builders. The shape names only the
//! operations the compiler resolved. Strings, types and nth-child operands belong to
//! the use site. Each call builds a fresh chain (selector nodes may be mutable).
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Op {
    Nesting,
    Class,
    Name,
    OfType,
    IsType,
    Template,
    Child,
    Descendant,
    NthChild,
    NthLastChild,
}

impl Op {
    fn method(self) -> &'static str {
        match self {
            Self::Nesting => "nesting",
            Self::Class => "class",
            Self::Name => "name",
            Self::OfType => "of_type_info",
            Self::IsType => "is_type_info",
            Self::Template => "template",
            Self::Child => "child",
            Self::Descendant => "descendant",
            Self::NthChild => "nth_child",
            Self::NthLastChild => "nth_last_child",
        }
    }

    fn parameters(self) -> &'static [&'static str] {
        match self {
            Self::Class | Self::Name => &["&str"],
            Self::OfType | Self::IsType => &["&'static ::ferroui_base::TypeInfo"],
            Self::NthChild | Self::NthLastChild => &["i32", "i32"],
            _ => &[],
        }
    }

    fn call(self, previous: &str, operands: &[String]) -> String {
        let mut call = format!(
            "::ferroui_base::styling::Selectors::{}({previous}",
            self.method()
        );
        for operand in operands {
            write!(call, ", {operand}").unwrap();
        }
        call.push(')');
        call
    }
}

#[derive(Default)]
pub(super) struct SelectorPlans {
    /// A seed is the result of a non-linear or property-value selector.
    shapes: HashMap<(bool, Vec<Op>), String>,
    style_builders: HashSet<String>,
    pub(super) helpers: String,
}

impl SelectorPlans {
    /// Fold the setup of a child Style into its already-interned selector shape.
    /// Only rootless linear plans qualify: all their operands are literals or
    /// static type references, so no user execution moves across Style creation.
    pub(super) fn start_style(&mut self, call: &str, parent: Option<&str>) -> Option<String> {
        let (name, operands) = call.strip_suffix(')')?.split_once('(')?;
        let ((_, shape), _) = self
            .shapes
            .iter()
            .find(|((seed, _), candidate)| !seed && candidate.as_str() == name)?;
        let complete = parent.is_some();
        let helper = format!(
            "{name}_{}",
            if complete { "add_style" } else { "begin_style" }
        );
        if self.style_builders.insert(helper.clone()) {
            let mut params = vec![
                "context: &::std::rc::Rc<::ferroui_markup_xaml::xaml_il::runtime::XamlIlContext>"
                    .to_owned(),
            ];
            if complete {
                params.push(
                    "parent: &::ferroui_base::Ref<::ferroui_base::styling::StyleBase>".into(),
                );
            }
            let mut args = Vec::new();
            for ty in shape.iter().flat_map(|op| op.parameters()) {
                let arg = format!("arg{}", args.len());
                params.push(format!("{arg}: {ty}"));
                args.push(arg);
            }
            let (result, tail) = if complete {
                params.extend(
                    [
                        "setters: &[rt::style_program::DynamicSetter]",
                        "line: i32",
                        "column: i32",
                    ]
                    .map(str::to_owned),
                );
                ("::core::result::Result<(), ::ferroui_markup_xaml::XamlLoadError>",
                 "rt::style_program::finish_dynamic(context, parent, &style, setters, line, column)")
            } else {
                (
                    "::ferroui_base::Ref<::ferroui_base::styling::Style>",
                    "style",
                )
            };
            writeln!(
                self.helpers,
                "#[inline(never)]\nfn {helper}({}) -> {result} {{",
                params.join(", ")
            )
            .unwrap();
            writeln!(
                self.helpers,
                "    let style = rt::style_program::begin(context);"
            )
            .unwrap();
            writeln!(self.helpers, "    ::ferroui_base::styling::Style::__markup_set_Selector(&style, ::core::option::Option::Some({name}({})));", args.join(", ")).unwrap();
            writeln!(self.helpers, "    {tail}\n}}\n").unwrap();
        }
        let parent = parent.map_or_else(String::new, |parent| format!(", {parent}"));
        let separator = if operands.is_empty() { "" } else { ", " };
        Some(format!("{helper}(&context{parent}{separator}{operands}"))
    }

    pub(super) fn call(
        &mut self,
        document: &str,
        seed: Option<&str>,
        steps: &[(Op, Vec<String>)],
    ) -> String {
        let previous = seed.map_or_else(
            || "::core::option::Option::None".into(),
            |s| format!("::core::option::Option::Some({s})"),
        );
        if let [(op, args)] = steps {
            return op.call(&previous, args);
        }
        debug_assert!(!steps.is_empty());
        let shape = (
            seed.is_some(),
            steps.iter().map(|(op, _)| *op).collect::<Vec<_>>(),
        );
        let next = self.shapes.len();
        let name = self.shapes.entry(shape).or_insert_with(|| {
            let name = format!("__{document}_selector_shape_{next}");
            let mut parameters = Vec::new();
            let mut body = String::new();
            let mut previous = match seed {
                Some(_) => {
                    parameters.push("seed: ::ferroui_base::styling::Selector".into());
                    "::core::option::Option::Some(seed)"
                }
                None => "::core::option::Option::None",
            };
            for (op, _) in steps {
                let mut args = Vec::new();
                for ty in op.parameters() {
                    let operand = format!("arg{}", parameters.len());
                    parameters.push(format!("{operand}: {ty}"));
                    args.push(operand);
                }
                writeln!(body, "    let selector = {};", op.call(previous, &args)).unwrap();
                previous = "::core::option::Option::Some(selector)";
            }
            writeln!(self.helpers, "#[inline(never)]\nfn {name}({}) -> ::ferroui_base::styling::Selector {{\n{body}    selector\n}}\n", parameters.join(", ")).unwrap();
            name
        });
        let operands = seed
            .into_iter()
            .chain(
                steps
                    .iter()
                    .flat_map(|(_, values)| values.iter().map(String::as_str)),
            )
            .collect::<Vec<_>>();
        format!("{name}({})", operands.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_shares_code_without_sharing_instances_or_rooting_other_control_types() {
        let mut plans = SelectorPlans::default();
        let steps = |ty: &str, name: &str| {
            vec![
                (Op::Nesting, vec![]),
                (Op::Template, vec![]),
                (Op::OfType, vec![ty.into()]),
                (Op::Name, vec![name.into()]),
            ]
        };
        assert_eq!(
            plans.call("build", None, &steps("Border::TYPE", "\"part\"")),
            "__build_selector_shape_0(Border::TYPE, \"part\")"
        );
        assert_eq!(
            plans.call("build", None, &steps("Grid::TYPE", "\"other\"")),
            "__build_selector_shape_0(Grid::TYPE, \"other\")"
        );
        assert_eq!(plans.helpers.matches("fn ").count(), 1);
        let style = plans
            .start_style(
                "__build_selector_shape_0(Border::TYPE, \"part\")",
                Some("parent"),
            )
            .unwrap();
        assert_eq!(
            style,
            "__build_selector_shape_0_add_style(&context, parent, Border::TYPE, \"part\""
        );
        plans
            .start_style(
                "__build_selector_shape_0(Grid::TYPE, \"next\")",
                Some("parent"),
            )
            .unwrap();
        assert_eq!(plans.helpers.matches("_add_style(").count(), 1);
        let helper = plans
            .helpers
            .split("fn __build_selector_shape_0_add_style")
            .nth(1)
            .unwrap();
        let begin = helper.find("style_program::begin(").unwrap();
        let select = helper.find("__markup_set_Selector").unwrap();
        let attach = helper.find("style_program::finish_dynamic").unwrap();
        assert!(begin < select && select < attach);

        assert!(!plans.helpers.contains("Border") && !plans.helpers.contains("Grid"));
        assert_eq!(
            plans.call("build", Some("prefix"), &steps("T", "s")),
            "__build_selector_shape_1(prefix, T, s)"
        );
        assert!(plans
            .start_style("__build_selector_shape_1(prefix, T, s)", None)
            .is_none());
        assert_eq!(
            plans.call("build", None, &[(Op::Class, vec!["s".into()])]),
            "::ferroui_base::styling::Selectors::class(::core::option::Option::None, s)"
        );
    }
}
