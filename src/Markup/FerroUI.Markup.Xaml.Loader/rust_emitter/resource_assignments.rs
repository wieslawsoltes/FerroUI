//! Native StaticResource assignments. A factory owns the key, source position and
//! reference to its operation. The operation owns the resolved property and typed
//! overload plan. There is no dispatch table to retain unrelated properties.
use std::{collections::HashMap, fmt::Write};

use super::template_values::TEMPLATE_PRIORITY;

/// Only built from XamlX's verified binding/setter overloads. The ordinary
/// call and the operation body use the same plan, including exact branch and
/// property-getter evaluation order.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) enum SetterPlan {
    Priority {
        getter: String,
        expected: String,
        nullable: bool,
        member: String,
        priority: String,
    },
    Local {
        getter: String,
        expected: String,
        nullable: bool,
        member: String,
        name: String,
    },
    Null {
        getter: String,
        priority: String,
        member: String,
    },
}

impl SetterPlan {
    pub(super) fn call(&self, target: &str, value: &str, line: &str, column: &str) -> String {
        match self {
            Self::Priority { getter, expected, nullable, member, priority } => format!(
                "rt::bind_or_set_priority({target}, || {getter}, {value}, {priority}, || {expected}, {nullable}, {member}, {line}, {column})"),
            Self::Local { getter, expected, nullable, member, name } => format!(
                "rt::set_local_value({target}, || {getter}, {value}, || {expected}, {nullable}, {member}, {name}, {line}, {column})"),
            Self::Null { getter, priority, member } => format!(
                "rt::bind_or_set_null({target}, || {getter}, &{value}, {priority}, {member}, {line}, {column})"),
        }
    }

    pub(super) fn can_share(&self) -> bool {
        matches!(self, Self::Local { .. })
            || matches!(self, Self::Priority { priority, .. } if priority == TEMPLATE_PRIORITY)
    }
}

pub(super) struct StaticResource {
    pub node: usize,
    pub property: usize,
    pub result: String,
    pub key: String,
    pub descriptor: String,
    pub position: (i32, i32),
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Operation {
    descriptor: String,
    setter: SetterPlan,
}

#[derive(Default)]
pub(super) struct ResourceAssignments {
    operation_ids: HashMap<Operation, usize>,
    operations: Vec<Operation>,
    sites: HashMap<String, Vec<Site>>,
}

struct Site {
    resolve: String,
    convert: String,
    operation: usize,
    target: String,
    key: String,
    line: i32,
    column: i32,
}

const OBJECT: &str = "::ferroui_base::FerroObject";

impl ResourceAssignments {
    /// Record the instructions produced for this exact pair of AST nodes. The
    /// opaque text keys are used only to locate surviving sites after factory
    /// sharing and leaf-table lowering. No operation is recovered from Rust text.
    pub(super) fn record(
        &mut self,
        resource: StaticResource,
        setter: &SetterPlan,
        target: &str,
        emitted: &[String],
        statement: &str,
    ) {
        let [resolve, convert] = emitted else { return };
        let op = Operation {
            descriptor: resource.descriptor,
            setter: setter.clone(),
        };
        let operation = *self.operation_ids.entry(op.clone()).or_insert_with(|| {
            self.operations.push(op);
            self.operations.len() - 1
        });
        self.sites
            .entry(statement.to_owned())
            .or_default()
            .push(Site {
                resolve: resolve.trim_start().to_owned(),
                convert: convert.trim_start().to_owned(),
                operation,
                target: target.to_owned(),
                key: resource.key,
                line: resource.position.0,
                column: resource.position.1,
            });
    }

    fn site(&self, lines: &[&str]) -> Option<&Site> {
        let [resolve, convert, call] = *lines.get(..3)? else {
            return None;
        };
        self.sites.get(call.trim_start())?.iter().find(|site| {
            site.resolve == resolve.trim_start() && site.convert == convert.trim_start()
        })
    }

    /// Run after binary-resource lowering, factory sharing and pruning. A
    /// unique surviving use stays inline. A rewritten factory keeps its own
    /// marker immediately before the new fallible call.
    pub(super) fn lower(self, source: String, document: &str, display: &str) -> String {
        if self.sites.is_empty() {
            return source;
        }
        let lines: Vec<_> = source.lines().collect();
        let mut uses = vec![0; self.operations.len()];
        for triple in lines.windows(3) {
            if let Some(site) = self.site(triple) {
                uses[site.operation] += 1;
            }
        }
        if !uses.iter().any(|&count| count >= 2) {
            return source;
        }
        let display = display.replace(['\r', '\n'], " ");
        let mut result = String::with_capacity(source.len());
        let mut first = vec![None; self.operations.len()];
        let mut i = 0;
        while i < lines.len() {
            if let Some(site) = self.site(&lines[i..]).filter(|s| uses[s.operation] >= 2) {
                let indent = &lines[i][..lines[i].len() - lines[i].trim_start().len()];
                writeln!(
                    result,
                    "{indent}__{document}_resource_assignment_{}(&context, {}, {}, {}, {})?;",
                    site.operation, site.target, site.key, site.line, site.column
                )
                .unwrap();
                first[site.operation].get_or_insert((site.line, site.column));
                i += 3;
            } else {
                writeln!(result, "{}", lines[i]).unwrap();
                i += 1;
            }
        }
        for (id, (operation, location)) in self.operations.iter().zip(first).enumerate() {
            if let Some((line, column)) = location {
                let descriptor = &operation.descriptor;
                let setter = operation.setter.call("target", "value", "line", "column");
                writeln!(result, "\n#[inline(never)]\nfn __{document}_resource_assignment_{id}(\n    context: &::std::rc::Rc<::ferroui_markup_xaml::xaml_il::runtime::XamlIlContext>,\n    target: &::ferroui_base::Ref<{OBJECT}>, key: &str, line: i32, column: i32,\n) -> ::core::result::Result<(), ::ferroui_markup_xaml::XamlLoadError> {{\n    // {display}({line},{column}) StaticResource assignment\n    let provided = rt::provide_static_resource(context, key, || {descriptor}, line, column)?;\n    let value = rt::to_value(provided);\n    {setter}\n}}").unwrap();
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setter() -> SetterPlan {
        SetterPlan::Priority {
            getter: "::ui::Panel::width_property()".into(),
            expected: "::ui::value_type()".into(),
            nullable: false,
            member: "\"Panel.Width\"".into(),
            priority: TEMPLATE_PRIORITY.into(),
        }
    }

    fn site(ops: &mut ResourceAssignments, key: &str, line: i32) -> String {
        let plan = setter();
        let descriptor = "rt::property_value(::ui::Panel::width_property())";
        let lines = [
            format!("    let supplied_{line} = rt::provide_static_resource(&context, {key}, || {descriptor}, {line}, 8)?;"),
            format!("    let local_{line} = rt::to_value(supplied_{line});"),
        ];
        let target = "panel.upcast_ref::<::ferroui_base::FerroObject>()";
        let call = format!(
            "{}?;",
            plan.call(target, &format!("local_{line}"), &line.to_string(), "8")
        );
        ops.record(
            StaticResource {
                node: 1,
                property: 2,
                result: format!("supplied_{line}"),
                key: key.into(),
                descriptor: descriptor.into(),
                position: (line, 8),
            },
            &plan,
            target,
            &lines,
            &call,
        );
        format!(
            "    // theme({line},8) Width\n{}\n    {call}\n",
            lines.join("\n")
        )
    }

    #[test]
    fn repeated_plans_share_a_native_operation_with_per_site_key_and_diagnostics() {
        let mut ops = ResourceAssignments::default();
        let a = site(&mut ops, "\"A\"", 12);
        let b = site(&mut ops, "\"B\"", 64);
        let out = ops.lower(a + &b, "theme", "WinUI.xaml");
        assert_eq!(out.matches("fn __theme_resource_assignment_").count(), 1);
        assert!(out.contains("\"A\", 12, 8)?;\n    // theme(64,8) Width"));
        assert!(out.contains("\"B\", 64, 8)?;"));
        let body = out.split("fn __theme_resource_assignment_").nth(1).unwrap();
        assert!(
            body.find("provide_static_resource").unwrap()
                < body.find("to_value(provided)").unwrap()
        );
        assert!(body
            .contains("rt::bind_or_set_priority(target, || ::ui::Panel::width_property(), value,"));
        assert!(body.contains("false, \"Panel.Width\", line, column)"));
    }

    #[test]
    fn pruning_and_intervening_side_effects_keep_the_original_instructions() {
        let mut ops = ResourceAssignments::default();
        let a = site(&mut ops, "\"A\"", 12);
        let b = site(&mut ops, "\"B\"", 64);
        let source = a + &b.replace("    let local_64 =", "    inspect();\n    let local_64 =");
        assert_eq!(ops.lower(source.clone(), "theme", "WinUI.xaml"), source);
    }

    #[test]
    fn captured_priority_or_borrowed_markup_value_stays_on_the_typed_inline_path() {
        let mut dynamic = setter();
        if let SetterPlan::Priority {
            ref mut priority, ..
        } = dynamic
        {
            *priority = "priority_local".into();
        }
        assert!(!dynamic.can_share());
        assert!(!SetterPlan::Null {
            getter: "get()".into(),
            priority: TEMPLATE_PRIORITY.into(),
            member: "\"Member\"".into()
        }
        .can_share());
    }
}
