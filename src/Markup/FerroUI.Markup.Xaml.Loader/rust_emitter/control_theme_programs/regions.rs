//! Contiguous, caller-owned Style/Selector regions inside a deferred theme.
//!
//! Unsupported statements remain byte-for-byte in their original scope. Each
//! accepted region replaces only a consecutive set of declarations/statements;
//! its reversed local array is declared at that same point and remains in the
//! containing function until normal cleanup. No region local may be used again,
//! including through a later move, borrow, shadowing declaration or assignment.
//! The only external value is one borrowed, already-declared StyleBase parent.
//!
//! The minimum is a deterministic construction-size heuristic, not evidence of a
//! native saving. Source/codegen and whole-application measurements remain gates.

use super::{absolute_path, call, closed_value, identifier, selector_operand, single_call, split_top, string_literal, Pool, BASE, NONE, SELECTORS, SOME, STYLE, THEME};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

const MIN_STATEMENTS: usize = 16;
const RUNTIME: &str = "rt::style_region";
const PROGRAM: &str = "__STYLE_REGION_PROGRAM";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role { Theme, Style, Selector }

struct Statement {
    start: usize,
    end: usize,
    text: String,
    comments: Vec<String>,
}

struct Known<'a> {
    builders: HashMap<&'a str, usize>,
    getter_expressions: HashMap<&'a str, &'a str>,
    getter_names: HashMap<&'a str, &'a str>,
}

impl Known<'_> {
    fn selector(&self, value: &str) -> bool {
        let Some((name, args)) = call(value) else { return false; };
        if let Some(count) = self.builders.get(name) {
            return args.len() == *count && args.iter().all(|arg| selector_operand(arg));
        }
        let Some(method) = name.strip_prefix(&format!("{SELECTORS}::")) else { return false; };
        let count = match method {
            "nesting" | "template" | "child" | "descendant" => 1,
            "class" | "name" | "of_type_info" | "is_type_info" => 2,
            "nth_child" | "nth_last_child" => 3,
            _ => return false,
        };
        args.len() == count && args[0] == NONE && args[1..].iter().all(|arg| selector_operand(arg))
    }

    fn property(&self, value: &str) -> Option<String> {
        let value = self.getter_expressions.get(value).copied().unwrap_or(value);
        let getter = single_call(value, "rt::property")?;
        let (path, args) = call(getter)?;
        (absolute_path(path) && args.is_empty()).then(|| value.to_owned())
    }

    fn dynamic_run(&self, value: &str) -> Option<Vec<(String, String)>> {
        let rows = value.strip_prefix('{')?.strip_suffix('}')?.trim().strip_prefix("&[")?.strip_suffix(']')?.trim();
        split_top(rows, ',')?.into_iter().map(|row| {
            let pair = split_top(row.strip_prefix('(')?.strip_suffix(')')?, ',')?;
            if pair.len() != 2 || !self.getter_expressions.contains_key(pair[0]) || !string_literal(pair[1]) {
                return None;
            }
            Some((self.property(pair[0])?, pair[1].to_owned()))
        }).collect()
    }

    fn declaration(&self, statement: &str) -> Option<(String, Role)> {
        let (name, value) = statement.strip_prefix("let ")?.split_once(" = ")?;
        if !identifier(name) { return None; }
        if value == format!("{THEME}::new()") { return Some((name.to_owned(), Role::Theme)); }
        if value == format!("{STYLE}::new()") || value == "rt::style_program::begin(&context)" {
            return Some((name.to_owned(), Role::Style));
        }
        if let Some((callee, args)) = call(value) {
            if let Some(builder) = callee.strip_suffix("_begin_style") {
                let count = *self.builders.get(builder)?;
                if args.len() == count + 1 && args[0] == "&context" && args[1..].iter().all(|arg| selector_operand(arg)) {
                    return Some((name.to_owned(), Role::Style));
                }
            }
        }
        self.selector(value).then(|| (name.to_owned(), Role::Selector))
    }
}

#[derive(Clone)]
enum Argument {
    Local(usize),
    Parent,
    Property(String),
    Value(String),
    Selector(String),
    Text(String),
    Position(i32, i32),
    DynamicRun(Vec<(String, String)>),
}

#[derive(Clone)]
struct Step {
    operation: &'static str,
    args: Vec<Argument>,
    comments: Vec<String>,
}

#[derive(Clone)]
struct Region {
    first: usize,
    last: usize,
    names: HashMap<String, usize>,
    roles: Vec<Role>,
    parents: Vec<usize>,
    external: Option<String>,
    steps: Vec<Step>,
}

impl Region {
    fn new(first: usize) -> Self {
        Self { first, last: first, names: HashMap::new(), roles: Vec::new(), parents: Vec::new(), external: None, steps: Vec::new() }
    }

    fn local(&self, name: &str, role: Role) -> Option<usize> {
        let index = *self.names.get(name)?;
        (self.roles[index] == role).then_some(index)
    }

    fn borrowed(&self, value: &str, role: Role) -> Option<usize> {
        self.local(value.strip_prefix('&')?, role)
    }

    fn parent(&mut self, value: &str, declarations: &HashMap<String, (usize, Role)>) -> Option<Argument> {
        let name = value.strip_suffix(&format!(".upcast_ref::<{BASE}>()"))?;
        if let Some(index) = self.local(name, Role::Style) { return Some(Argument::Local(index)); }
        let &(position, role) = declarations.get(name)?;
        if position >= self.first || !matches!(role, Role::Theme | Role::Style) { return None; }
        if self.external.as_deref().is_some_and(|existing| existing != name) { return None; }
        self.external = Some(name.to_owned());
        Some(Argument::Parent)
    }

    fn accept(&mut self, statement: &Statement, known: &Known<'_>, declarations: &HashMap<String, (usize, Role)>) -> Option<()> {
        let text = statement.text.as_str();
        let (operation, args) = if let Some((name, role)) = known.declaration(text).filter(|(_, role)| *role != Role::Theme) {
            if self.names.contains_key(&name) || self.roles.len() >= u16::MAX as usize { return None; }
            let slot = self.roles.len();
            self.names.insert(name, slot);
            self.roles.push(role);
            let value = text.split_once(" = ")?.1;
            match role {
                Role::Selector => ("new_selector", vec![Argument::Local(slot), Argument::Selector(value.to_owned())]),
                Role::Style if value == format!("{STYLE}::new()") => ("new_style", vec![Argument::Local(slot)]),
                Role::Style if value == "rt::style_program::begin(&context)" => {
                    self.parents.push(slot);
                    ("begin_style", vec![Argument::Local(slot)])
                }
                Role::Style => {
                    let (callee, args) = call(value)?;
                    let builder = callee.strip_suffix("_begin_style")?;
                    let selector = format!("{builder}({})", args[1..].join(", "));
                    self.parents.push(slot);
                    ("selected_style", vec![Argument::Local(slot), Argument::Selector(selector)])
                }
                Role::Theme => return None,
            }
        } else if text == "context.pop_parent()" {
            self.parents.pop()?;
            ("pop_parent", Vec::new())
        } else {
            let (text, fallible) = super::strip_question(text);
            let (callee, args) = call(text)?;
            match callee {
                "rt::push_parent" if !fallible && args.len() == 2 && args[0] == "&context" => {
                    let slot = self.borrowed(args[1], Role::Style)?;
                    self.parents.push(slot);
                    ("push_style", vec![Argument::Local(slot)])
                }
                "rt::add_setter" if !fallible && args.len() == 3 => {
                    let parent = self.parent(args[0], declarations)?;
                    let property = known.property(args[1])?;
                    if !closed_value(args[2]) { return None; }
                    ("literal_setter", vec![parent, Argument::Property(property), Argument::Value(args[2].to_owned())])
                }
                "rt::add_dynamic_resource_setter_with_parent" if !fallible && args.len() == 4 && args[0] == "&context" => {
                    let parent = self.parent(args[1], declarations)?;
                    let property = known.property(args[2])?;
                    if !string_literal(args[3]) { return None; }
                    ("dynamic_setter", vec![parent, Argument::Property(property), Argument::Text(args[3].to_owned())])
                }
                "rt::style_program::attach" if fallible && args.len() == 4 => {
                    let parent = self.parent(args[0], declarations)?;
                    let child = self.borrowed(args[1], Role::Style)?;
                    if self.parents.contains(&child) { return None; }
                    ("attach", vec![parent, Argument::Local(child), position(args[2], args[3])?])
                }
                "rt::style_program::finish_dynamic" if fallible && args.len() == 6 && args[0] == "&context" => {
                    let parent = self.parent(args[1], declarations)?;
                    let child = self.borrowed(args[2], Role::Style)?;
                    if self.parents.pop()? != child { return None; }
                    ("finish_dynamic", vec![parent, Argument::Local(child), Argument::DynamicRun(known.dynamic_run(args[3])?), position(args[4], args[5])?])
                }
                _ if callee == format!("{STYLE}::__markup_set_Selector") && !fallible && args.len() == 2 => {
                    let style = self.borrowed(args[0], Role::Style)?;
                    let selector = single_call(args[1], SOME)?.strip_suffix(".clone()")?;
                    let selector = self.local(selector, Role::Selector)?;
                    ("set_selector", vec![Argument::Local(style), Argument::Local(selector)])
                }
                _ if callee.ends_with("_add_style") && fallible => {
                    let builder = callee.strip_suffix("_add_style")?;
                    let count = *known.builders.get(builder)?;
                    if args.len() != count + 5 || args[0] != "&context" || !args[2..2 + count].iter().all(|arg| selector_operand(arg)) { return None; }
                    let parent = self.parent(args[1], declarations)?;
                    let selector = format!("{builder}({})", args[2..2 + count].join(", "));
                    ("add_selected_style", vec![parent, Argument::Selector(selector), Argument::DynamicRun(known.dynamic_run(args[2 + count])?), position(args[3 + count], args[4 + count])?])
                }
                _ => return None,
            }
        };
        self.steps.push(Step { operation, args, comments: statement.comments.clone() });
        Some(())
    }

    fn closed(&self, last_use: &HashMap<String, usize>) -> bool {
        self.steps.len() >= MIN_STATEMENTS && self.parents.is_empty() && self.external.is_some()
            && self.roles.contains(&Role::Style) && self.roles.contains(&Role::Selector)
            && self.names.keys().all(|name| last_use.get(name).is_some_and(|last| *last <= self.last))
    }
}

fn position(line: &str, column: &str) -> Option<Argument> {
    Some(Argument::Position(line.parse().ok()?, column.parse().ok()?))
}

#[derive(Default)]
struct Imports {
    operations: Pool<&'static str>,
    properties: Pool<String>,
    values: Pool<String>,
    selectors: Pool<String>,
    texts: Pool<String>,
    positions: Pool<(i32, i32)>,
    dynamic_runs: Pool<Vec<[u16; 2]>>,
}

impl Imports {
    fn argument(&mut self, argument: &Argument, locals: usize) -> Option<u16> {
        match argument {
            Argument::Local(index) => u16::try_from(locals.checked_sub(index + 1)?).ok(),
            Argument::Parent => Some(u16::MAX),
            Argument::Property(value) => self.properties.add(value.clone()),
            Argument::Value(value) => self.values.add(value.clone()),
            Argument::Selector(value) => self.selectors.add(value.clone()),
            Argument::Text(value) => self.texts.add(value.clone()),
            Argument::Position(line, column) => self.positions.add((*line, *column)),
            Argument::DynamicRun(rows) => {
                let rows = rows.iter().map(|(property, key)| {
                    Some([self.properties.add(property.clone())?, self.texts.add(key.clone())?])
                }).collect::<Option<Vec<_>>>()?;
                self.dynamic_runs.add(rows)
            }
        }
    }

    fn region(&mut self, region: &Region, index: usize) -> Option<String> {
        let mut text = format!("\n    const __STYLE_REGION_STEPS_{index}: &[{RUNTIME}::Instruction] = &[\n");
        for step in &region.steps {
            let operation = self.operations.add(step.operation)?;
            let mut args = [0; 4];
            for (index, argument) in step.args.iter().enumerate() {
                *args.get_mut(index)? = self.argument(argument, region.roles.len())?;
            }
            for comment in &step.comments { writeln!(text, "        {comment}").unwrap(); }
            writeln!(text, "        {RUNTIME}::Instruction {{ operation: {operation}, args: {args:?} }},").unwrap();
        }
        text.push_str("    ];\n");
        writeln!(text, "    let mut __style_region_locals_{index} = [const {{ {RUNTIME}::Local::Empty }}; {}];", region.roles.len()).unwrap();
        write!(text, "    {RUNTIME}::run(&context, {}.upcast_ref::<{BASE}>(), &{PROGRAM}, __STYLE_REGION_STEPS_{index}, &mut __style_region_locals_{index})?;", region.external.as_ref()?).unwrap();
        Some(text)
    }

    fn render(&self, known: &Known<'_>) -> String {
        let mut text = format!("\n    const {PROGRAM}: {RUNTIME}::Program = {RUNTIME}::Program {{\n");
        writeln!(text, "        operations: &[{}],", self.operations.0.iter().map(|op| format!("{RUNTIME}::{op}")).collect::<Vec<_>>().join(", ")).unwrap();
        writeln!(text, "        imports: {RUNTIME}::Imports {{").unwrap();
        let properties: Vec<_> = self.properties.0.iter().map(|expression| {
            known.getter_names.get(expression.as_str()).map_or_else(|| format!("|| {expression}"), |name| (*name).to_owned())
        }).collect();
        writeln!(text, "            properties: &[{}],", properties.join(", ")).unwrap();
        for (name, values) in [("values", &self.values), ("selectors", &self.selectors)] {
            writeln!(text, "            {name}: &[{}],", values.0.iter().map(|value| format!("|| {value}")).collect::<Vec<_>>().join(", ")).unwrap();
        }
        writeln!(text, "            texts: &[{}],", self.texts.0.join(", ")).unwrap();
        writeln!(text, "            positions: &{:?},", self.positions.0).unwrap();
        writeln!(text, "            dynamic_runs: &[{}],", self.dynamic_runs.0.iter().map(|rows| format!("&{rows:?}")).collect::<Vec<_>>().join(", ")).unwrap();
        text.push_str("        },\n    };\n");
        text
    }
}

pub(super) fn lower<'a>(
    function: &str,
    source: &str,
    selector_builders: impl IntoIterator<Item = (&'a str, usize)>,
    property_getters: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Option<String> {
    if !identifier(function) { return None; }
    let header = format!("fn {function}(\n    service_provider: &::std::rc::Rc<dyn ::ferroui_base::metadata::IServiceProvider>,\n) -> ::core::result::Result<::ferroui_base::metadata::MarkupValue, ::ferroui_markup_xaml::XamlLoadError> {{\n");
    let offset = source.find(&header)?;
    if !source[..offset].lines().all(|line| line.starts_with("/// ")) { return None; }
    let body_start = offset + header.len();
    let body = source[body_start..].strip_suffix("}\n")?;
    let statements = statements(body)?;
    let context = statements.first()?.text.as_str();
    if !context.starts_with("let context = rt::deferred_context(service_provider, &") || !context.ends_with(')') { return None; }
    let getters: Vec<_> = property_getters.into_iter().collect();
    let known = Known {
        builders: selector_builders.into_iter().collect(),
        getter_expressions: getters.iter().map(|&(expression, name)| (name, expression)).collect(),
        getter_names: getters.into_iter().collect(),
    };
    let mut declarations = HashMap::new();
    let mut declared_names = HashSet::new();
    let mut last_use = HashMap::new();
    for (index, statement) in statements.iter().enumerate() {
        // Keep unknown declarations too: a shadow of a recognized name is a
        // conservative whole-factory barrier, even if it would be legal Rust.
        if let Some(declaration) = statement.text.strip_prefix("let ").and_then(|s| s.split_once(" = ")).map(|(name, _)| name.trim_start_matches("mut ")) {
            if identifier(declaration) && !declared_names.insert(declaration.to_owned()) { return None; }
        }
        if let Some((name, role)) = known.declaration(&statement.text) { declarations.insert(name, (index, role)); }
        for name in identifiers(&statement.text)? {
            if name == PROGRAM || name.starts_with("__STYLE_REGION_STEPS_") || name.starts_with("__style_region_locals_") { return None; }
            last_use.insert(name, index);
        }
    }
    // Only themes are in this prototype's scope. Visual object initialization,
    // dictionary installation and unrelated factory kinds retain their route.
    if !declarations.values().any(|(_, role)| *role == Role::Theme) { return None; }
    let mut regions = Vec::new();
    let mut start = 0;
    while start < statements.len() {
        if !known.declaration(&statements[start].text).is_some_and(|(_, role)| role == Role::Style) {
            start += 1;
            continue;
        }
        let mut candidate = Region::new(start);
        let mut best = None;
        for (last, statement) in statements.iter().enumerate().skip(start) {
            if candidate.accept(statement, &known, &declarations).is_none() { break; }
            candidate.last = last;
            if candidate.closed(&last_use) { best = Some(candidate.clone()); }
        }
        if let Some(region) = best {
            start = region.last + 1;
            regions.push(region);
        } else { start += 1; }
    }
    if regions.is_empty() { return None; }
    // Pool only the committed region snapshots. Probing a rejected suffix must
    // never root its callbacks/types in a shared import table.
    let mut imports = Imports::default();
    let replacements = regions.iter().enumerate().map(|(index, region)| imports.region(region, index)).collect::<Option<Vec<_>>>()?;
    let mut output = source[..body_start].to_owned();
    let mut cursor = 0;
    for (index, (region, replacement)) in regions.iter().zip(replacements).enumerate() {
        let first = statements[region.first].start;
        let last = statements[region.last].end;
        output.push_str(&body[cursor..first]);
        if index == 0 { output.push_str(&imports.render(&known)); }
        output.push_str(&replacement);
        cursor = last;
    }
    output.push_str(&body[cursor..]);
    output.push_str("}\n");
    Some(output)
}

fn string_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'"' => return Some(index + 1),
            _ => index += 1,
        }
    }
    None
}

fn character_end(source: &str, start: usize) -> Option<Option<usize>> {
    let tail = &source[start + 1..];
    let first = tail.chars().next()?;
    if first == '\\' {
        let mut escaped = true;
        for (offset, character) in tail.char_indices().skip(1) {
            if escaped { escaped = false; }
            else if character == '\\' { escaped = true; }
            else if character == '\'' { return Some(Some(start + 1 + offset + 1)); }
        }
        return None;
    }
    if tail[first.len_utf8()..].starts_with('\'') { return Some(Some(start + 1 + first.len_utf8() + 1)); }
    (first.is_alphabetic() || first == '_').then_some(None) // a lifetime, not a character
}

fn statement(source: &str, start: usize, end: usize, terminated: bool) -> Statement {
    let mut comments = Vec::new();
    let mut text = Vec::new();
    for line in source[start..end].lines() {
        let line = line.trim();
        if line.starts_with("//") { comments.push(line.to_owned()); }
        else if !line.is_empty() { text.push(line); }
    }
    let mut text = text.join("\n");
    if terminated { text.pop(); }
    Statement { start, end, text, comments }
}

/// Top-level semicolons delimit statements. Unknown blocks/closures stay opaque;
/// no candidate is discovered inside one. Literal/comment contents never change
/// the scope depth or become evidence of a variable use.
fn statements(source: &str) -> Option<Vec<Statement>> {
    let mut result = Vec::new();
    let mut stack = Vec::new();
    let mut index = 0;
    let mut start = 0;
    let bytes = source.as_bytes();
    while index < bytes.len() {
        if source[index..].starts_with("//") {
            index += source[index..].find('\n').unwrap_or(source.len() - index);
            continue;
        }
        if ["/*", "r#", "br#", "r\"", "br\""].iter().any(|prefix| source[index..].starts_with(prefix)) { return None; }
        match bytes[index] {
            b'"' => { index = string_end(source, index)?; continue; }
            b'\'' => if let Some(end) = character_end(source, index)? { index = end; continue; },
            b'(' => stack.push(b')'), b'[' => stack.push(b']'), b'{' => stack.push(b'}'),
            b')' | b']' | b'}' => if stack.pop()? != bytes[index] { return None; },
            b';' if stack.is_empty() => {
                result.push(statement(source, start, index + 1, true));
                start = index + 1;
            }
            _ => {},
        }
        index += source[index..].chars().next()?.len_utf8();
    }
    if !stack.is_empty() { return None; }
    if !source[start..].trim().is_empty() { result.push(statement(source, start, source.len(), false)); }
    Some(result)
}

fn identifiers(source: &str) -> Option<HashSet<String>> {
    let mut names = HashSet::new();
    let mut index = 0;
    let bytes = source.as_bytes();
    while index < bytes.len() {
        if bytes[index] == b'"' { index = string_end(source, index)?; continue; }
        if bytes[index] == b'\'' {
            if let Some(end) = character_end(source, index)? { index = end; continue; }
        }
        if bytes[index].is_ascii_alphabetic() || bytes[index] == b'_' {
            let start = index;
            index += 1;
            while index < bytes.len() && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_') { index += 1; }
            names.insert(source[start..index].to_owned());
        } else { index += source[index..].chars().next()?.len_utf8(); }
    }
    Some(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROPERTY: &str = "rt::property(::ferroui_base::Visual::opacity_property())";

    fn group(start: usize, count: usize, parent: &str) -> String {
        let mut source = String::new();
        for index in start..start + count {
            writeln!(source, "    // theme.xaml({},3) Style", index + 10).unwrap();
            writeln!(source, "    let style_{index} = {STYLE}::new();").unwrap();
            writeln!(source, "    let selector_{index} = __build_selector_shape_0(\":state{index}\");").unwrap();
            writeln!(source, "    {STYLE}::__markup_set_Selector(&style_{index}, {SOME}(selector_{index}.clone()));").unwrap();
            writeln!(source, "    rt::add_setter(style_{index}.upcast_ref::<{BASE}>(), {PROPERTY}, rt::to_object(0.5_f64));").unwrap();
            writeln!(source, "    rt::style_program::attach({parent}.upcast_ref::<{BASE}>(), &style_{index}, {}, 3)?;", index + 10).unwrap();
        }
        source
    }

    fn factory(body: &str) -> String {
        format!("/// Deferred theme.\nfn build_theme(\n    service_provider: &::std::rc::Rc<dyn ::ferroui_base::metadata::IServiceProvider>,\n) -> ::core::result::Result<::ferroui_base::metadata::MarkupValue, ::ferroui_markup_xaml::XamlLoadError> {{\n    let context = rt::deferred_context(service_provider, &DOCUMENT);\n    let control_theme_0 = {THEME}::new();\n    rt::push_parent(&context, &control_theme_0);\n{body}    context.pop_parent();\n    ::core::result::Result::Ok(rt::to_value(control_theme_0.clone()))\n}}\n")
    }

    fn lower_fixture(source: &str) -> Option<String> {
        lower("build_theme", source, [("__build_selector_shape_0", 1)], [(PROPERTY, "__build_style_property_0")])
    }

    #[test]
    fn regions_stay_between_original_declarations_and_share_only_lazy_imports() {
        let before = "    // theme.xaml(1,1) Before\n    let before = ::example::DropProbe::new(1);\n";
        let between = "    // theme.xaml(30,1) Resource\n    let resource = rt::provide_static_resource(&context, \"Unchanged\", || rt::setter_value_property(), 30, 1)?;\n";
        let after = "    // theme.xaml(70,1) After\n    let after = ::example::DropProbe::new(2);\n";
        let original = factory(&format!("{before}{}{between}{}{after}", group(0, 4, "control_theme_0"), group(10, 4, "control_theme_0")));
        let lowered = lower_fixture(&original).unwrap();
        assert!(lowered.contains(before) && lowered.contains(between) && lowered.contains(after));
        let first = lowered.find("let mut __style_region_locals_0").unwrap();
        let second = lowered.find("let mut __style_region_locals_1").unwrap();
        assert!(lowered.find("let before =").unwrap() < first);
        assert!(first < lowered.find("let resource =").unwrap());
        assert!(lowered.find("let resource =").unwrap() < second);
        assert!(second < lowered.find("let after =").unwrap());
        assert_eq!(lowered.matches("const __STYLE_REGION_PROGRAM:").count(), 1);
        assert_eq!(lowered.matches("__build_style_property_0").count(), 1);
        assert_eq!(lowered.matches("|| rt::to_object(0.5_f64)").count(), 1);
        assert_eq!(lowered.matches("Local::Empty }; 8]").count(), 2);
        // The literal setter also uses [7, 0, 0, 0]. Check the first
        // new_style instruction, not an operand tuple shared by other ops.
        assert_eq!(lowered.matches("Instruction { operation: 0, args: [7, 0, 0, 0] }").count(), 2);
        assert!(lowered.contains("args: [65535, 7, 0, 0]"));
        assert_eq!(lowered.matches("rt::style_region::run(").count(), 2);
        for line in original.lines().filter(|line| line.trim_start().starts_with("//")) {
            assert_eq!(lowered.matches(line.trim()).count(), 1, "source marker: {line}");
        }
        assert!(lowered.ends_with("    ::core::result::Result::Ok(rt::to_value(control_theme_0.clone()))\n}\n"));
    }

    #[test]
    fn sixteen_statements_is_a_deterministic_minimum() {
        let fifteen = group(0, 3, "control_theme_0");
        assert!(lower_fixture(&factory(&fifteen)).is_none());
        let extra = format!("    rt::add_setter(control_theme_0.upcast_ref::<{BASE}>(), {PROPERTY}, rt::to_object(0.5_f64));\n");
        let lowered = lower_fixture(&factory(&format!("{fifteen}{extra}"))).unwrap();
        assert_eq!(lowered.matches("rt::style_region::Instruction {").count(), 16);
        assert!(lowered.contains("Local::Empty }; 6]"));
    }

    #[test]
    fn later_use_moves_borrows_reassignment_and_scope_boundaries_decline() {
        for later in [
            "::example::observe(&style_0);",
            "let moved = style_0;",
            "::example::observe(selector_0.clone());",
            "style_0 = ::ferroui_base::styling::Style::new();",
            "let style_0 = ::ferroui_base::styling::Style::new();",
        ] {
            let source = factory(&format!("{}    {later}\n", group(0, 4, "control_theme_0")));
            assert!(lower_fixture(&source).is_none(), "{later}");
        }
        let scoped = factory(&format!("    {{\n{}    }}\n", group(0, 4, "control_theme_0")));
        assert!(lower_fixture(&scoped).is_none());
        let moved = factory(&format!("{}    let combined = {SELECTORS}::or([selector_0, selector_1]);\n{}", group(0, 2, "control_theme_0"), group(2, 2, "control_theme_0")));
        assert!(lower_fixture(&moved).is_none());
    }

    #[test]
    fn both_stored_roles_are_required_even_when_a_helper_constructs_one() {
        let mut styles_only = String::new();
        for index in 0..4 {
            writeln!(styles_only, "    let style_{index} = __build_selector_shape_0_begin_style(&context, \":disabled\");").unwrap();
            writeln!(styles_only, "    rt::add_setter(style_{index}.upcast_ref::<{BASE}>(), {PROPERTY}, rt::to_object(0.5_f64));").unwrap();
            styles_only.push_str("    context.pop_parent();\n");
            writeln!(styles_only, "    rt::style_program::attach(control_theme_0.upcast_ref::<{BASE}>(), &style_{index}, 2, 3)?;").unwrap();
        }
        assert!(lower_fixture(&factory(&styles_only)).is_none());
        let selectors_only = (0..20).map(|index| format!("    let selector_{index} = __build_selector_shape_0(\":disabled\");\n")).collect::<String>();
        assert!(lower_fixture(&factory(&selectors_only)).is_none());
    }

    #[test]
    fn rejected_prefixes_cannot_contribute_imports_to_a_later_region() {
        let rejected = group(0, 4, "control_theme_0").replace("rt::to_object(0.5_f64)", "rt::to_object(::example::Unwanted::new())");
        let barrier = "    ::example::observe(&style_0);\n";
        let source = factory(&format!("{rejected}{barrier}{}", group(10, 4, "control_theme_0")));
        let lowered = lower_fixture(&source).unwrap();
        assert!(lowered.contains(&rejected));
        assert!(lowered.contains(barrier));
        let imports = lowered.split_once("const __STYLE_REGION_PROGRAM:").unwrap().1.split_once("const __STYLE_REGION_STEPS_0:").unwrap().0;
        assert!(!imports.contains("Unwanted"));
        assert!(!imports.contains(":state0") && !imports.contains(":state3"));
        assert!(imports.contains(":state10") && imports.contains(":state13"));
        assert_eq!(lowered.matches("rt::style_region::run(").count(), 1);
    }

    #[test]
    fn external_parent_changes_split_regions_without_capturing_or_cloning_it() {
        let declaration = format!("    let outer_style = {STYLE}::new();\n    ::example::observe(&outer_style);\n");
        let source = factory(&format!("{declaration}{}{}", group(0, 4, "control_theme_0"), group(10, 4, "outer_style")));
        let lowered = lower_fixture(&source).unwrap();
        assert!(lowered.contains(&declaration));
        assert_eq!(lowered.matches("rt::style_region::run(").count(), 2);
        assert!(lowered.contains(&format!("run(&context, control_theme_0.upcast_ref::<{BASE}>(),")));
        assert!(lowered.contains(&format!("run(&context, outer_style.upcast_ref::<{BASE}>(),")));
        assert!(!lowered.contains("outer_style.clone()"));
        let imports = lowered.split_once("const __STYLE_REGION_PROGRAM:").unwrap().1.split_once("const __STYLE_REGION_STEPS_0:").unwrap().0;
        assert!(!imports.contains("outer_style") && !imports.contains("control_theme_0"));
    }

    #[test]
    fn parent_stack_must_balance_inside_each_region_and_unsupported_text_is_exact() {
        let balanced = group(0, 4, "control_theme_0").replace(
            &format!("let style_0 = {STYLE}::new();"),
            "let style_0 = rt::style_program::begin(&context);",
        ).replace(
            &format!("    rt::style_program::attach(control_theme_0.upcast_ref::<{BASE}>(), &style_0,"),
            &format!("    context.pop_parent();\n    rt::style_program::attach(control_theme_0.upcast_ref::<{BASE}>(), &style_0,"),
        );
        let lowered = lower_fixture(&factory(&balanced)).unwrap();
        assert!(lowered.contains("rt::style_region::begin_style") && lowered.contains("rt::style_region::pop_parent"));
        let unbalanced = balanced.replace("    context.pop_parent();\n    rt::style_program::attach", "    rt::style_program::attach");
        assert!(lower_fixture(&factory(&unbalanced)).is_none());
        let unsupported = "    // theme.xaml(40,2) Animation\n    let animation = ::ferroui_base::animation::Animation::new();\n    // theme.xaml(41,2) Exact\n    ::example::unknown(\"literal style_0; { }\");\n";
        let source = factory(&format!("{}{unsupported}", group(0, 4, "control_theme_0")));
        let lowered = lower_fixture(&source).unwrap();
        assert!(lowered.contains(unsupported));
        // The name in a string is not a later use of a caller-owned local.
        assert_eq!(lowered.matches("rt::style_region::run(").count(), 1);
    }
}
