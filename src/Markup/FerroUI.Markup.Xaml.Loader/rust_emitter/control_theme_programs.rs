//! A deliberately closed lowering of complete deferred ControlTheme factories.
//!
//! This pass recognizes the emitter's statements, not arbitrary Rust. One unknown
//! statement, local role, scope, move or parent-stack operation leaves the entire
//! original function untouched. Each accepted resource gets its own import table;
//! imports are lazy calls, never cached values or eagerly initialized metadata.
//!
//! Kept independent of the AST/type-system crates so the grammar and fallback
//! boundary can be exercised without compiling the framework runtime.

use std::collections::HashMap;
use std::fmt::Write;

const THEME: &str = "::ferroui_base::styling::ControlTheme";
const STYLE: &str = "::ferroui_base::styling::Style";
const BASE: &str = "::ferroui_base::styling::StyleBase";
const SELECTORS: &str = "::ferroui_base::styling::Selectors";
const TEMPLATE: &str = "::ferroui_markup_xaml::templates::ControlTemplate";
const SOME: &str = "::core::option::Option::Some";
const NONE: &str = "::core::option::Option::None";
const CLONE: &str = "::core::clone::Clone::clone";
const RUNTIME: &str = "rt::control_theme_program";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Theme,
    Style,
    Selector,
    Setter,
    Template,
    Deferred,
}

struct Local {
    role: Role,
    live: bool,
}

#[derive(Clone, Copy)]
enum Operand {
    Local(usize),
    Import(u16),
    Unused,
}

struct Step {
    operation: u16,
    operands: [Operand; 4],
    comments: Vec<String>,
}

#[derive(Default)]
struct Pool<T>(Vec<T>);

impl<T: PartialEq> Pool<T> {
    fn add(&mut self, value: T) -> Option<u16> {
        if let Some(index) = self.0.iter().position(|existing| *existing == value) {
            return u16::try_from(index).ok();
        }
        let index = u16::try_from(self.0.len()).ok()?;
        self.0.push(value);
        Some(index)
    }
}

struct Plan<'a> {
    selector_builders: HashMap<&'a str, usize>,
    getter_expressions: HashMap<&'a str, &'a str>,
    getter_names: HashMap<&'a str, &'a str>,
    names: HashMap<String, usize>,
    locals: Vec<Local>,
    parents: Vec<usize>,
    operations: Pool<&'static str>,
    steps: Vec<Step>,
    properties: Pool<String>,
    values: Pool<String>,
    selectors: Pool<String>,
    types: Pool<String>,
    texts: Pool<String>,
    positions: Pool<(i32, i32)>,
    deferred: Pool<String>,
    dynamic_runs: Pool<Vec<[u16; 2]>>,
}

/// Return a replacement only after validating the complete function. The caller
/// retains `source` verbatim when this returns `None`.
pub(super) fn lower<'a>(
    function: &str,
    source: &str,
    selector_builders: impl IntoIterator<Item = (&'a str, usize)>,
    property_getters: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Option<String> {
    if !identifier(function) {
        return None;
    }
    let header = format!(
        "fn {function}(\n    service_provider: &::std::rc::Rc<dyn ::ferroui_base::metadata::IServiceProvider>,\n) -> ::core::result::Result<::ferroui_base::metadata::MarkupValue, ::ferroui_markup_xaml::XamlLoadError> {{\n"
    );
    let offset = source.find(&header)?;
    if !source[..offset].lines().all(|line| line.starts_with("/// ")) {
        return None;
    }
    let body = source[offset + header.len()..].strip_suffix("}\n")?;
    // Comments are source locations only. A Rust block, declaration or explicit
    // drop remains a statement and must pass the closed grammar below.
    let lines: Vec<_> = body.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
    let first = lines.iter().position(|line| !line.starts_with("// "))?;
    let last = lines.iter().rposition(|line| !line.starts_with("// "))?;
    // The emitter currently puts source markers on body instructions. Do not
    // silently discard a new marker outside that region if its format changes.
    if first != 0 || last != lines.len() - 1 {
        return None;
    }
    let context = lines[first];
    if !context.starts_with("let context = rt::deferred_context(service_provider, &") || !context.ends_with(");") {
        return None;
    }
    let returned = lines[last].strip_prefix("::core::result::Result::Ok(rt::to_value(")?.strip_suffix(".clone()))")?;
    if !identifier(returned) || last <= first + 1 {
        return None;
    }
    let getters: Vec<_> = property_getters.into_iter().collect();
    let mut plan = Plan {
        selector_builders: selector_builders.into_iter().collect(),
        getter_expressions: getters.iter().map(|&(expression, name)| (name, expression)).collect(),
        getter_names: getters.into_iter().collect(),
        names: HashMap::new(),
        locals: Vec::new(),
        parents: Vec::new(),
        operations: Pool::default(),
        steps: Vec::new(),
        properties: Pool::default(),
        values: Pool::default(),
        selectors: Pool::default(),
        types: Pool::default(),
        texts: Pool::default(),
        positions: Pool::default(),
        deferred: Pool::default(),
        dynamic_runs: Pool::default(),
    };
    for (statement, comments) in statements_in(&lines[first + 1..last])? {
        plan.statement(&statement)?;
        plan.steps.last_mut()?.comments = comments;
    }
    let returned = plan.local(returned, Role::Theme)?;
    if !plan.parents.is_empty() || plan.locals.len() > u16::MAX as usize {
        return None;
    }
    // The shared Local drop glue mentions all six types. Require an original
    // declaration of every role in *this* resource, not merely in the document.
    // Fused helper temporaries do not satisfy this conservative requirement.
    for role in [Role::Theme, Role::Style, Role::Selector, Role::Setter, Role::Template, Role::Deferred] {
        if !plan.locals.iter().any(|local| local.role == role) {
            return None;
        }
    }
    Some(plan.render(&source[..offset + header.len()], context, returned))
}

impl Plan<'_> {
    fn declare(&mut self, name: &str, role: Role) -> Option<usize> {
        if !identifier(name) || self.names.contains_key(name) || self.locals.len() >= u16::MAX as usize {
            return None;
        }
        if role == Role::Theme && self.locals.iter().any(|local| local.role == role) {
            return None;
        }
        let index = self.locals.len();
        self.locals.push(Local { role, live: true });
        self.names.insert(name.to_owned(), index);
        Some(index)
    }

    fn local(&self, name: &str, role: Role) -> Option<usize> {
        let index = *self.names.get(name)?;
        let local = &self.locals[index];
        (local.live && local.role == role).then_some(index)
    }

    fn borrowed(&self, value: &str, role: Role) -> Option<usize> {
        self.local(value.strip_prefix('&')?, role)
    }

    fn style_base(&self, value: &str) -> Option<usize> {
        let name = value.strip_suffix(&format!(".upcast_ref::<{BASE}>()"))?;
        self.local(name, Role::Theme).or_else(|| self.local(name, Role::Style))
    }

    fn moved_selector(&mut self, value: &str) -> Option<usize> {
        let index = self.local(value, Role::Selector)?;
        self.locals[index].live = false;
        Some(index)
    }

    fn step(&mut self, operation: &'static str, operands: &[Operand]) -> Option<()> {
        let operation = self.operations.add(operation)?;
        let mut args = [Operand::Unused; 4];
        args.get_mut(..operands.len())?.copy_from_slice(operands);
        self.steps.push(Step { operation, operands: args, comments: Vec::new() });
        Some(())
    }

    fn property(&mut self, value: &str) -> Option<u16> {
        let value = self.getter_expressions.get(value).copied().unwrap_or(value);
        let (name, args) = call(value)?;
        if name != "rt::property" || args.len() != 1 {
            return None;
        }
        let (getter, getter_args) = call(args[0])?;
        if !absolute_path(getter) || !getter_args.is_empty() {
            return None;
        }
        self.properties.add(value.to_owned())
    }

    fn text(&mut self, value: &str) -> Option<u16> {
        string_literal(value).then_some(())?;
        self.texts.add(value.to_owned())
    }

    fn position(&mut self, line: &str, column: &str) -> Option<u16> {
        self.positions.add((line.parse().ok()?, column.parse().ok()?))
    }

    fn selector(&mut self, value: &str) -> Option<u16> {
        let (name, args) = call(value)?;
        if let Some(count) = self.selector_builders.get(name) {
            if args.len() != *count || !args.iter().all(|arg| selector_operand(arg)) {
                return None;
            }
        } else {
            let method = name.strip_prefix(&format!("{SELECTORS}::"))?;
            let count = match method {
                "nesting" | "template" | "child" | "descendant" => 1,
                "class" | "name" | "of_type_info" | "is_type_info" => 2,
                "nth_child" | "nth_last_child" => 3,
                _ => return None,
            };
            if args.len() != count || args[0] != NONE || !args[1..].iter().all(|arg| selector_operand(arg)) {
                return None;
            }
        }
        self.selectors.add(value.to_owned())
    }

    fn dynamic_run(&mut self, value: &str) -> Option<u16> {
        let rows = value.strip_prefix('{')?.strip_suffix('}')?.trim().strip_prefix("&[")?.strip_suffix(']')?.trim();
        let mut result = Vec::new();
        for row in split_top(rows, ',')? {
            let row = row.strip_prefix('(')?.strip_suffix(')')?;
            let pair = split_top(row, ',')?;
            if pair.len() != 2 || !self.getter_expressions.contains_key(pair[0]) {
                return None;
            }
            result.push([self.property(pair[0])?, self.text(pair[1])?]);
        }
        self.dynamic_runs.add(result)
    }

    fn statement(&mut self, statement: &str) -> Option<()> {
        if let Some(declaration) = statement.strip_prefix("let ") {
            let (name, value) = declaration.split_once(" = ")?;
            return self.declaration(name, value);
        }
        if statement == "context.pop_parent()" {
            self.parents.pop()?;
            return self.step("pop_parent", &[]);
        }
        let (value, fallible) = strip_question(statement);
        let (name, args) = call(value)?;
        match name {
            "rt::push_parent" if !fallible && args.len() == 2 && args[0] == "&context" => {
                let local_name = args[1].strip_prefix('&')?;
                let index = *self.names.get(local_name)?;
                let local = &self.locals[index];
                if !local.live { return None; }
                let operation = match local.role {
                    Role::Theme => "push_theme",
                    Role::Style => "push_style",
                    Role::Template => "push_template",
                    _ => return None,
                };
                self.parents.push(index);
                self.step(operation, &[Operand::Local(index)])
            }
            "rt::add_setter" if !fallible && args.len() == 3 => {
                let parent = self.style_base(args[0])?;
                let property = self.property(args[1])?;
                closed_value(args[2]).then_some(())?;
                let value = self.values.add(args[2].to_owned())?;
                self.step("literal_setter", &[Operand::Local(parent), Operand::Import(property), Operand::Import(value)])
            }
            "rt::add_dynamic_resource_setter_with_parent" if !fallible && args.len() == 4 && args[0] == "&context" => {
                let parent = self.style_base(args[1])?;
                let property = self.property(args[2])?;
                let key = self.text(args[3])?;
                self.step("dynamic_setter", &[Operand::Local(parent), Operand::Import(property), Operand::Import(key)])
            }
            "rt::style_program::attach" if fallible && args.len() == 4 => {
                let parent = self.style_base(args[0])?;
                let child = self.borrowed(args[1], Role::Style)?;
                if self.parents.contains(&child) { return None; }
                let position = self.position(args[2], args[3])?;
                self.step("attach", &[Operand::Local(parent), Operand::Local(child), Operand::Import(position)])
            }
            "rt::style_program::finish_dynamic" if fallible && args.len() == 6 && args[0] == "&context" => {
                let parent = self.style_base(args[1])?;
                let child = self.borrowed(args[2], Role::Style)?;
                if self.parents.pop()? != child { return None; }
                let run = self.dynamic_run(args[3])?;
                let position = self.position(args[4], args[5])?;
                self.step("finish_dynamic", &[Operand::Local(parent), Operand::Local(child), Operand::Import(run), Operand::Import(position)])
            }
            "rt::add_setter_value" | "rt::add_setter_value_with_parent" if !fallible => {
                let with_parent = name.ends_with("_with_parent");
                let args = if with_parent {
                    if args.len() != 4 || args[0] != "&context" { return None; }
                    &args[1..]
                } else {
                    if args.len() != 3 { return None; }
                    &args[..]
                };
                let parent = self.style_base(args[0])?;
                let setter = self.borrowed(args[1], Role::Setter)?;
                let template = self.local(cloned_object(args[2])?, Role::Template)?;
                if with_parent && self.parents.pop()? != setter { return None; }
                self.step(if with_parent { "setter_template_with_parent" } else { "setter_template" }, &[Operand::Local(parent), Operand::Local(setter), Operand::Local(template)])
            }
            _ if name == format!("{THEME}::__markup_set_TargetType") && !fallible && args.len() == 2 => {
                let theme = self.borrowed(args[0], Role::Theme)?;
                let ty = single_call(args[1], SOME)?;
                static_type(ty).then_some(())?;
                let ty = self.types.add(ty.to_owned())?;
                self.step("target_type", &[Operand::Local(theme), Operand::Import(ty)])
            }
            _ if name == format!("{STYLE}::__markup_set_Selector") && !fallible && args.len() == 2 => {
                let style = self.borrowed(args[0], Role::Style)?;
                let selector = single_call(args[1], SOME)?.strip_suffix(".clone()")?;
                let selector = self.local(selector, Role::Selector)?;
                self.step("set_selector", &[Operand::Local(style), Operand::Local(selector)])
            }
            _ if name == format!("{TEMPLATE}::__markup_set_Content") && !fallible && args.len() == 2 => {
                let template = self.borrowed(args[0], Role::Template)?;
                let deferred = self.local(cloned_object(args[1])?, Role::Deferred)?;
                self.step("template_content", &[Operand::Local(template), Operand::Local(deferred)])
            }
            _ if fallible && name.ends_with("_add_style") => {
                let builder = name.strip_suffix("_add_style")?;
                let count = *self.selector_builders.get(builder)?;
                if args.len() != count + 5 || args[0] != "&context" { return None; }
                let parent = self.style_base(args[1])?;
                let selector = self.selector(&format!("{builder}({})", args[2..2 + count].join(", ")))?;
                let run = self.dynamic_run(args[2 + count])?;
                let position = self.position(args[3 + count], args[4 + count])?;
                self.step("add_selected_style", &[Operand::Local(parent), Operand::Import(selector), Operand::Import(run), Operand::Import(position)])
            }
            _ => None,
        }
    }

    fn declaration(&mut self, name: &str, value: &str) -> Option<()> {
        let (value, fallible) = strip_question(value);
        let (callee, args) = call(value)?;
        if callee == format!("{THEME}::new") && !fallible && args.is_empty() {
            let local = self.declare(name, Role::Theme)?;
            return self.step("new_theme", &[Operand::Local(local)]);
        }
        if callee == format!("{STYLE}::new") && !fallible && args.is_empty() {
            let local = self.declare(name, Role::Style)?;
            return self.step("new_style", &[Operand::Local(local)]);
        }
        if callee == "rt::style_program::begin" && !fallible && args == ["&context"] {
            let local = self.declare(name, Role::Style)?;
            self.parents.push(local);
            return self.step("begin_style", &[Operand::Local(local)]);
        }
        if let Some(builder) = callee.strip_suffix("_begin_style").filter(|_| !fallible) {
            let count = *self.selector_builders.get(builder)?;
            if args.len() != count + 1 || args[0] != "&context" { return None; }
            let selector = self.selector(&format!("{builder}({})", args[1..].join(", ")))?;
            let local = self.declare(name, Role::Style)?;
            self.parents.push(local);
            return self.step("selected_style", &[Operand::Local(local), Operand::Import(selector)]);
        }
        if callee == format!("{SELECTORS}::or") && !fallible && args.len() == 1 {
            let moved = split_top(args[0].strip_prefix('[')?.strip_suffix(']')?, ',')?;
            let operation = match moved.len() { 2 => "or_two", 3 => "or_three", _ => return None };
            let mut operands = vec![Operand::Unused];
            for value in moved { operands.push(Operand::Local(self.moved_selector(value)?)); }
            let local = self.declare(name, Role::Selector)?;
            operands[0] = Operand::Local(local);
            return self.step(operation, &operands);
        }
        if !fallible && (self.selector_builders.contains_key(callee) || callee.starts_with(&format!("{SELECTORS}::"))) {
            let selector = self.selector(value)?;
            let local = self.declare(name, Role::Selector)?;
            return self.step("new_selector", &[Operand::Local(local), Operand::Import(selector)]);
        }
        if !fallible && matches!(callee, "rt::new_setter" | "rt::new_setter_with_parent") {
            let with_parent = callee.ends_with("_with_parent");
            let property = if with_parent {
                if args.len() != 2 || args[0] != "&context" { return None; }
                self.property(args[1])?
            } else {
                if args.len() != 1 { return None; }
                self.property(args[0])?
            };
            let local = self.declare(name, Role::Setter)?;
            if with_parent { self.parents.push(local); }
            return self.step(if with_parent { "new_setter_with_parent" } else { "new_setter" }, &[Operand::Local(local), Operand::Import(property)]);
        }
        if callee == format!("{TEMPLATE}::__markup_new_0") && !fallible && args.is_empty() {
            let local = self.declare(name, Role::Template)?;
            return self.step("new_template", &[Operand::Local(local)]);
        }
        if callee == "rt::defer" && fallible && args.len() == 5 && args[1] == "&context" {
            let ty = single_call(args[0], "rt::class_handle")?;
            if !static_type(ty) || !identifier(args[2]) { return None; }
            let line: i32 = args[3].parse().ok()?;
            let column: i32 = args[4].parse().ok()?;
            let site = self.deferred.add(format!("{RUNTIME}::DeferredSite {{ result_type: {ty}, build: {}, line: {line}, column: {column} }}", args[2]))?;
            let local = self.declare(name, Role::Deferred)?;
            return self.step("defer", &[Operand::Local(local), Operand::Import(site)]);
        }
        None
    }

    fn render(&self, header: &str, context: &str, returned: usize) -> String {
        let mut output = header.to_owned();
        writeln!(output, "    {context}").unwrap();
        writeln!(output, "    const PROGRAM: {RUNTIME}::Program = {RUNTIME}::Program {{").unwrap();
        writeln!(output, "        operations: &[{}],", self.operations.0.iter().map(|op| format!("{RUNTIME}::{op}")).collect::<Vec<_>>().join(", ")).unwrap();
        output.push_str("        instructions: &[\n");
        for step in &self.steps {
            for comment in &step.comments {
                writeln!(output, "            {comment}").unwrap();
            }
            let args = step.operands.map(|operand| match operand {
                Operand::Local(index) => (self.locals.len() - 1 - index) as u16,
                Operand::Import(index) => index,
                Operand::Unused => 0,
            });
            writeln!(output, "            {RUNTIME}::Instruction {{ operation: {}, args: {args:?} }},", step.operation).unwrap();
        }
        output.push_str("        ],\n");
        writeln!(output, "        imports: {RUNTIME}::Imports {{").unwrap();
        let properties: Vec<_> = self.properties.0.iter().map(|expression| {
            self.getter_names.get(expression.as_str()).map_or_else(|| format!("|| {expression}"), |name| (*name).to_owned())
        }).collect();
        writeln!(output, "            properties: &[{}],", properties.join(", ")).unwrap();
        for (name, values) in [("values", &self.values), ("selectors", &self.selectors)] {
            writeln!(output, "            {name}: &[{}],", values.0.iter().map(|value| format!("|| {value}")).collect::<Vec<_>>().join(", ")).unwrap();
        }
        for (name, values) in [("types", &self.types), ("texts", &self.texts), ("deferred", &self.deferred)] {
            writeln!(output, "            {name}: &[{}],", values.0.join(", ")).unwrap();
        }
        writeln!(output, "            positions: &{:?},", self.positions.0).unwrap();
        writeln!(output, "            dynamic_runs: &[{}],", self.dynamic_runs.0.iter().map(|rows| format!("&{rows:?}")).collect::<Vec<_>>().join(", ")).unwrap();
        output.push_str("        },\n    };\n");
        writeln!(output, "    let mut locals = [const {{ {RUNTIME}::Local::Empty }}; {}];", self.locals.len()).unwrap();
        writeln!(output, "    {RUNTIME}::run(&context, &PROGRAM, &mut locals, {})\n}}", self.locals.len() - 1 - returned).unwrap();
        output
    }
}

fn identifier(value: &str) -> bool {
    let mut chars = value.bytes();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

fn absolute_path(value: &str) -> bool {
    value.strip_prefix("::").is_some_and(|value| value.split("::").all(identifier))
}

fn static_type(value: &str) -> bool {
    value.strip_prefix('<').and_then(|v| v.strip_suffix(" as ::ferroui_base::StaticType>::TYPE")).is_some_and(absolute_path)
}

fn string_literal(value: &str) -> bool {
    if !value.starts_with('"') { return false; }
    let mut escaped = false;
    for (index, byte) in value.bytes().enumerate().skip(1) {
        if escaped { escaped = false; }
        else if byte == b'\\' { escaped = true; }
        else if byte == b'"' { return index == value.len() - 1; }
        else if byte == b'\n' || byte == b'\r' { return false; }
    }
    false
}

fn number(value: &str) -> bool {
    let value = value.strip_prefix('-').unwrap_or(value);
    let value = value.strip_suffix("_f64").or_else(|| value.strip_suffix("_f32"))
        .or_else(|| value.strip_suffix("_i32")).or_else(|| value.strip_suffix("_u32"))
        .or_else(|| value.strip_suffix("_i64")).or_else(|| value.strip_suffix("_u64"))
        .unwrap_or(value);
    !value.is_empty() && value.as_bytes()[0].is_ascii_digit()
        && value.bytes().all(|byte| byte.is_ascii_digit() || matches!(byte, b'.' | b'e' | b'E' | b'-' | b'+'))
        && value.parse::<f64>().is_ok()
}

fn selector_operand(value: &str) -> bool {
    string_literal(value) || number(value) || static_type(value)
}

/// A closed expression may move owned constructor arguments, but must not borrow
/// a destructible temporary: its lifetime would otherwise end inside the import
/// callback instead of after the outer add_setter statement. Primitive literal
/// borrows (the emitter's Clone::clone(&true), for example) have no such cleanup.
fn closed_value(value: &str) -> bool {
    if matches!(value, "true" | "false") || string_literal(value) || number(value) || static_type(value) || absolute_path(value) {
        return true;
    }
    if let Some(value) = value.strip_prefix('&') {
        return matches!(value, "true" | "false") || string_literal(value) || number(value);
    }
    let Some((name, args)) = call(value) else { return false; };
    (absolute_path(name) || name.strip_prefix("rt::").is_some_and(identifier))
        && args.iter().all(|value| closed_value(value))
}

fn strip_question(value: &str) -> (&str, bool) {
    value.strip_suffix('?').map_or((value, false), |value| (value, true))
}

fn call(value: &str) -> Option<(&str, Vec<&str>)> {
    let (name, arguments) = value.split_once('(')?;
    if name.is_empty() || name.trim() != name { return None; }
    let arguments = arguments.strip_suffix(')')?;
    Some((name, split_top(arguments, ',')?))
}

fn single_call<'a>(value: &'a str, expected: &str) -> Option<&'a str> {
    let (name, args) = call(value)?;
    (name == expected && args.len() == 1).then(|| args[0])
}

fn cloned_object(value: &str) -> Option<&str> {
    single_call(single_call(value, "rt::to_object")?, CLONE)?.strip_prefix('&')
}

/// The only non-parenthesis braces accepted by the consumer are the existing
/// dynamic-run `&[(getter, key)]` blocks. Tracking them here does not authorize
/// arbitrary scopes; the exact statement parser must still recognize the body.
fn split_top(value: &str, separator: char) -> Option<Vec<&str>> {
    let mut pieces = Vec::new();
    let mut stack = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut start = 0;
    for (index, character) in value.char_indices() {
        if quoted {
            if escaped { escaped = false; }
            else if character == '\\' { escaped = true; }
            else if character == '"' { quoted = false; }
            continue;
        }
        match character {
            '"' => quoted = true,
            '\'' => return None,
            '(' => stack.push(')'), '[' => stack.push(']'), '{' => stack.push('}'), '<' => stack.push('>'),
            ')' | ']' | '}' | '>' => if stack.pop()? != character { return None; },
            character if character == separator && stack.is_empty() => {
                let part = value[start..index].trim();
                if part.is_empty() { return None; }
                pieces.push(part);
                start = index + character.len_utf8();
            }
            _ => {},
        }
    }
    if quoted || !stack.is_empty() { return None; }
    let last = value[start..].trim();
    if !last.is_empty() { pieces.push(last); }
    Some(pieces)
}

fn statements_in(lines: &[&str]) -> Option<Vec<(String, Vec<String>)>> {
    let mut result = Vec::new();
    let mut statement = String::new();
    let mut comments = Vec::new();
    for &line in lines {
        if line.starts_with("// ") {
            comments.push(line.to_owned());
            continue;
        }
        statement.push_str(line);
        statement.push('\n');
        if line.ends_with(';') {
            if let Some(parts) = split_top(&statement, ';') {
                if parts.len() != 1 { return None; }
                result.push((parts[0].to_owned(), std::mem::take(&mut comments)));
                statement.clear();
            }
        }
    }
    (statement.is_empty() && comments.is_empty()).then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPACITY: &str = "rt::property(::ferroui_base::Visual::opacity_property())";
    const BUILDERS: &[(&str, usize)] = &[("__build_selector_shape_0", 1)];

    // These are the existing emitter's complete statements, including the
    // parent scope of Setter.Value and the retained template/deferred locals.
    fn factory(extra: &str) -> String {
        format!(r#"fn build_theme(
    service_provider: &::std::rc::Rc<dyn ::ferroui_base::metadata::IServiceProvider>,
) -> ::core::result::Result<::ferroui_base::metadata::MarkupValue, ::ferroui_markup_xaml::XamlLoadError> {{
    let context = rt::deferred_context(service_provider, &DOCUMENT);
    let control_theme_0 = {THEME}::new();
    rt::push_parent(&context, &control_theme_0);
    {THEME}::__markup_set_TargetType(&control_theme_0, {SOME}(<::ferroui_controls::Button as ::ferroui_base::StaticType>::TYPE));
    let setter_0 = rt::new_setter_with_parent(&context, rt::property(::ferroui_controls::TemplatedControl::template_property()));
    let control_template_0 = {TEMPLATE}::__markup_new_0();
    rt::push_parent(&context, &control_template_0);
    let deferred_0 = rt::defer(rt::class_handle(<::ferroui_controls::Control as ::ferroui_base::StaticType>::TYPE), &context, build_child, 7, 9)?;
    {TEMPLATE}::__markup_set_Content(&control_template_0, rt::to_object({CLONE}(&deferred_0)));
    context.pop_parent();
    rt::add_setter_value_with_parent(&context, control_theme_0.upcast_ref::<{BASE}>(), &setter_0, rt::to_object({CLONE}(&control_template_0)));
    let style_0 = {STYLE}::new();
    let selector_0 = __build_selector_shape_0(":hover");
    {STYLE}::__markup_set_Selector(&style_0, {SOME}(selector_0.clone()));
    rt::add_setter(style_0.upcast_ref::<{BASE}>(), {OPACITY}, rt::to_object(0.5_f64));
    rt::style_program::attach(control_theme_0.upcast_ref::<{BASE}>(), &style_0, 12, 3)?;
    {extra}
    context.pop_parent();
    ::core::result::Result::Ok(rt::to_value(control_theme_0.clone()))
}}
"#)
    }

    fn lower_fixture(source: &str) -> Option<String> {
        lower("build_theme", source, BUILDERS.iter().copied(), [(OPACITY, "__build_style_property_0")])
    }

    #[test]
    fn complete_nested_factory_keeps_local_order_and_lazy_imports() {
        let extra = format!(r#"
    let style_1 = __build_selector_shape_0_begin_style(&context, ":disabled");
    rt::style_program::finish_dynamic(&context, control_theme_0.upcast_ref::<{BASE}>(), &style_1, {{
        &[
            // theme.xaml(20,4) Value
            (__build_style_property_0, "Dimmed"),
            (__build_style_property_0, "Dimmed"),
        ]
    }}, 19, 3)?;
    __build_selector_shape_0_add_style(&context, control_theme_0.upcast_ref::<{BASE}>(), ":checked", {{
        &[(__build_style_property_0, "Dimmed")]
    }}, 24, 5)?;
    rt::add_setter(control_theme_0.upcast_ref::<{BASE}>(), {OPACITY}, rt::to_object(0.5_f64));"#);
        let lowered = lower_fixture(&factory(&extra)).expect("the complete closed grammar");
        assert!(lowered.contains("Local::Empty }; 7]"));
        // Declaration zero occupies the last array element; the returned root
        // remains a clone, with all other locals alive until run has returned.
        assert!(lowered.contains("operation: 0, args: [6, 0, 0, 0]"));
        assert!(lowered.contains("run(&context, &PROGRAM, &mut locals, 6)"));
        assert_eq!(lowered.matches("|| rt::to_object(0.5_f64)").count(), 1);
        assert_eq!(lowered.matches("__build_style_property_0").count(), 1);
        assert!(lowered.contains("dynamic_runs: &[&[[1, 0], [1, 0]], &[[1, 0]]]"));
        assert!(lowered.contains("control_theme_program::selected_style"));
        assert!(lowered.contains("control_theme_program::finish_dynamic"));
        assert!(lowered.contains("control_theme_program::add_selected_style"));
        assert!(lowered.contains("positions: &[(12, 3), (19, 3), (24, 5)]"));
        assert!(lowered.contains("build: build_child, line: 7, column: 9"));
        assert!(!lowered.contains("Vec::") && !lowered.contains("Box::") && !lowered.contains("unsafe"));
    }

    #[test]
    fn every_stored_role_must_already_be_present_in_the_resource() {
        let original = factory("");
        let declarations = [
            (Role::Theme, format!("    let control_theme_0 = {THEME}::new();\n")),
            (Role::Style, format!("    let style_0 = {STYLE}::new();\n")),
            (Role::Selector, "    let selector_0 = __build_selector_shape_0(\":hover\");\n".to_owned()),
            (Role::Setter, "    let setter_0 = rt::new_setter_with_parent(&context, rt::property(::ferroui_controls::TemplatedControl::template_property()));\n".to_owned()),
            (Role::Template, format!("    let control_template_0 = {TEMPLATE}::__markup_new_0();\n")),
            (Role::Deferred, "    let deferred_0 = rt::defer(rt::class_handle(<::ferroui_controls::Control as ::ferroui_base::StaticType>::TYPE), &context, build_child, 7, 9)?;\n".to_owned()),
        ];
        for (_, declaration) in declarations {
            assert!(original.contains(&declaration));
            assert!(lower_fixture(&original.replace(&declaration, "")).is_none());
        }
        // A valid factory with only a fused selector temporary must also decline:
        // constructing a selector inside a helper does not create a stored role.
        let fused_only = original
            .replace(&format!("let style_0 = {STYLE}::new();"), "let style_0 = __build_selector_shape_0_begin_style(&context, \":hover\");")
            .replace("    let selector_0 = __build_selector_shape_0(\":hover\");\n", "")
            .replace(&format!("    {STYLE}::__markup_set_Selector(&style_0, {SOME}(selector_0.clone()));\n"), "")
            .replace("    rt::style_program::attach(", "    context.pop_parent();\n    rt::style_program::attach(");
        assert!(lower_fixture(&fused_only).is_none());

        // An otherwise valid empty template does not authorize Deferred's drop
        // glue. No undefined local or unbalanced parent is involved here.
        let empty_template = original
            .replace("    let deferred_0 = rt::defer(rt::class_handle(<::ferroui_controls::Control as ::ferroui_base::StaticType>::TYPE), &context, build_child, 7, 9)?;\n", "")
            .replace(&format!("    {TEMPLATE}::__markup_set_Content(&control_template_0, rt::to_object({CLONE}(&deferred_0)));\n"), "");
        assert!(lower_fixture(&empty_template).is_none());

        // These variants remain well-formed straight-line Rust: there is no
        // use of a removed local, and every remaining parent push is balanced.
        // They isolate the role-presence rule from ordinary grammar errors.
        let no_style = original.lines().filter(|line| {
            !line.contains("let style_0 =") && !line.contains("__markup_set_Selector")
                && !line.contains("rt::add_setter(style_0") && !line.contains("rt::style_program::attach(")
        }).collect::<Vec<_>>().join("\n") + "\n";
        assert!(lower_fixture(&no_style).is_none());
        let no_setter = original.lines().filter(|line| {
            !line.contains("let setter_0 =") && !line.contains("rt::add_setter_value_with_parent(")
        }).collect::<Vec<_>>().join("\n") + "\n";
        assert!(lower_fixture(&no_setter).is_none());
        let no_template = original
            .replace(&format!("    let control_template_0 = {TEMPLATE}::__markup_new_0();\n"), "")
            .replace("    rt::push_parent(&context, &control_template_0);\n", "")
            .replace(&format!("    {TEMPLATE}::__markup_set_Content(&control_template_0, rt::to_object({CLONE}(&deferred_0)));\n"), "")
            .lines().filter(|line| !line.contains("rt::add_setter_value_with_parent(")).collect::<Vec<_>>().join("\n") + "\n";
        assert!(lower_fixture(&no_template).is_none());
    }

    #[test]
    fn selector_arrays_move_each_local_once_in_original_order() {
        let original = factory("");
        let combined = original.replace(
            "let selector_0 = __build_selector_shape_0(\":hover\");",
            &format!("let selector_a = __build_selector_shape_0(\":hover\");\n    let selector_b = __build_selector_shape_0(\":focus\");\n    let selector_0 = {SELECTORS}::or([selector_a, selector_b]);"),
        );
        let lowered = lower_fixture(&combined).expect("moved array operands");
        assert!(lowered.contains("control_theme_program::or_two"));
        assert!(lowered.contains("args: [0, 2, 1, 0]"));
        assert!(lower_fixture(&combined.replace("[selector_a, selector_b]", "[selector_a, selector_a]")).is_none());
        assert!(lower_fixture(&combined.replace("Some(selector_0.clone())", "Some(selector_a.clone())")).is_none());
        assert!(lower_fixture(&combined.replace("[selector_a, selector_b]", "[selector_a.clone(), selector_b]")).is_none());
    }

    #[test]
    fn unknown_lifecycle_statements_decline_the_whole_function() {
        for extra in [
            "let transition_0 = ::ferroui_base::animation::DoubleTransition::new();",
            "let animation_0 = ::ferroui_base::animation::Animation::new();",
            "let resources_0 = ::ferroui_controls::ResourceDictionary::new();",
            "let provided_0 = rt::provide_static_resource(&context, \"Base\", callback, 1, 2)?;",
            "control_theme_0.begin_init();",
            "drop(style_0);",
            "{ let temporary = ::ferroui_base::styling::Style::new(); };",
            "let style_0 = ::ferroui_base::styling::Style::new();",
            "let _scope = context.push_namespace_scope(&NAMESPACES);",
        ] {
            let source = factory(extra);
            let rewritten = lower_fixture(&source).unwrap_or_else(|| source.clone());
            assert_eq!(rewritten, source, "unsupported statement: {extra}");
        }
        assert!(lower_fixture(&factory("").replace("rt::to_value(control_theme_0.clone())", "rt::to_value(control_theme_0)")).is_none());
        assert!(lower_fixture(&factory("context.pop_parent();")).is_none());
        let source = factory("");
        assert!(lower_fixture(&source.replace("    let context =", "    // theme.xaml(1,1) Context\n    let context =")).is_none());
        assert!(lower_fixture(&source.replace("    ::core::result::Result::Ok(", "    // theme.xaml(30,1) Return\n    ::core::result::Result::Ok(")).is_none());
        assert!(lower_fixture(&source.replace("control_theme_0.clone()))\n}", "control_theme_0.clone()))\n    // theme.xaml(30,1) Return\n}")).is_none());
    }

    #[test]
    fn borrowed_nonprimitive_temporaries_do_not_move_into_leaf_callbacks() {
        for expression in [
            "rt::to_object(::core::clone::Clone::clone(&true))",
            "rt::to_object(::ferroui_base::Thickness::new(1.0_f64))",
            "rt::to_object(::example::Owned::new(::example::Argument::new()))",
            "::core::option::Option::None",
        ] { assert!(closed_value(expression), "{expression}"); }
        for expression in [
            "rt::to_object(::core::clone::Clone::clone(&::example::Owned::new()))",
            "rt::to_object(::core::clone::Clone::clone(&::example::Value::Variant))",
            "rt::to_object(::example::Value::new(&context))",
            "rt::to_object(value.clone())",
            "rt::to_object(::example::Value::parse(\"x\")?)",
        ] { assert!(!closed_value(expression), "{expression}"); }
        let extra = format!("rt::add_setter(control_theme_0.upcast_ref::<{BASE}>(), {OPACITY}, rt::to_object(::core::clone::Clone::clone(&::example::Owned::new())));");
        assert!(lower_fixture(&factory(&extra)).is_none());
    }

    #[test]
    fn imports_of_other_resources_are_not_rooted_by_the_shared_program() {
        let source = factory("");
        let lowered = lower(
            "build_theme", &source,
            [("__build_selector_shape_0", 1), ("unused_selector", 2)],
            [(OPACITY, "__build_style_property_0"), ("rt::property(::UnusedControl::unused_property())", "unused_getter")],
        ).unwrap();
        assert!(!lowered.contains("unused") && !lowered.contains("UnusedControl"));
        let other = lower_fixture(&source.replace("build_child", "build_other_child").replace("::ferroui_controls::Button", "::app::OtherButton")).unwrap();
        assert!(!lowered.contains("OtherButton") && !lowered.contains("build_other_child"));
        assert!(!other.contains("::ferroui_controls::Button") && !other.contains("build: build_child,"));
        // A handler is rooted only if this resource actually imports it.
        assert!(!lowered.contains("control_theme_program::add_selected_style"));
        assert!(!lowered.contains("control_theme_program::dynamic_setter"));
    }
}
