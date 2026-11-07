//! Control theme tables (#568, step 5 of docs/porting/theme-trimming.md): the factory of a
//! control theme written as a static table that `rt::build_control_theme` interprets, its
//! selector chains as static slices that `rt::build_selector` interprets, the run-time choice of
//! the setter of a resource value as one call of `rt::assign_resource_value`, and consecutive
//! template bindings on one object as one call of `rt::bind_template_bindings` over a table.
//!
//! As for the rows of step 1 ([`super::theme_rows`]), the shapes are recognised where the
//! emitter has just written them, in the complete body of the deferred content (the factory, a
//! template) or in the lines of one assignment, with the statements matched exactly. A factory
//! becomes a table when every statement at its top level is part of a member the table holds:
//! the target type, `BasedOn` as `{StaticResource}`, setters and nested styles (recursively).
//! Unlike a closed grammar of values, a setter value of any other shape is still accepted: its
//! statements become a function of the factory (`<factory>_value_<n>`), provided they name no
//! local of the factory but their own and the context. Anything else (an animation, a dictionary
//! of resources, the name scope) keeps the native code of the factory.
//!
//! Each factory keeps its own table and value functions, so that a unit of the theme
//! (step 2, [`super::theme_units`]) still refers to its factories only. What the tables of one
//! generated function share are leaves: the function of each property definition
//! (`<function>_property_<n>`), the selector chains (`<FUNCTION>_SELECTOR_<n>`, identical chains
//! share one) and the values of property selectors. The items are written in the order they are
//! first needed, so the output does not depend on hashing.

use std::collections::{HashMap, HashSet};

use super::{declared_local, identifiers, is_identifier, is_string_literal};

const STYLE_BASE: &str = "::ferroui_base::styling::StyleBase";
const CONTROL_THEME: &str = "::ferroui_base::styling::ControlTheme";
const STYLE: &str = "::ferroui_base::styling::Style";
const SELECTORS: &str = "::ferroui_base::styling::Selectors";
const SETTER: &str = "::ferroui_base::styling::Setter";
const SETTER_BASE: &str = "::ferroui_base::styling::SetterBase";
const STATIC_RESOURCE: &str = "::ferroui_markup_xaml::markup_extensions::StaticResourceExtension";
const CONTROL_TEMPLATE: &str = "::ferroui_markup_xaml::templates::ControlTemplate";
const THICKNESS: &str = "::ferroui_base::Thickness";
const CORNER_RADIUS: &str = "::ferroui_base::CornerRadius";
const FERRO_OBJECT: &str = "::ferroui_base::FerroObject";
const NONE: &str = "::core::option::Option::None";
const SOME: &str = "::core::option::Option::Some(";
const CONTEXT_TYPE: &str = "::std::rc::Rc<::ferroui_markup_xaml::xaml_il::runtime::XamlIlContext>";
const MARKUP_VALUE: &str = "::ferroui_base::metadata::MarkupValue";
const LOAD_ERROR: &str = "::ferroui_markup_xaml::XamlLoadError";
/// The shortest run of template bindings on one object written as a table.
const MIN_BINDING_RUN: usize = 2;

// --- the shared items of a generated function -------------------------------------------

/// The items the tables of one generated function (the function of a document and its
/// deferred content) share, and every item written for them, in the order they were first
/// needed.
pub(super) struct StylePool {
    /// The name of the generated function: the items are named after it.
    prefix: String,
    /// The document, for the comments of the items.
    document_text: String,
    /// The function of each property definition, by its expression.
    properties: HashMap<String, String>,
    /// The static of each selector chain, by the text of its slice.
    selectors: HashMap<String, String>,
    /// The function of each value of a property selector, by its expression.
    selector_values: HashMap<String, String>,
    /// The number of tables of template bindings written.
    binding_tables: usize,
    /// The items, in the order they were written.
    pub(super) items: Vec<String>,
}

impl StylePool {
    pub(super) fn new(function_name: &str, document_text: &str) -> Self {
        Self {
            prefix: function_name.to_string(),
            document_text: document_text.to_string(),
            properties: HashMap::new(),
            selectors: HashMap::new(),
            selector_values: HashMap::new(),
            binding_tables: 0,
            items: Vec::new(),
        }
    }

    /// The function that returns the property definition `expression` (`rt::property(..)` or
    /// `rt::class_property(..)`), written the first time it is needed.
    fn property(&mut self, expression: &str) -> String {
        if let Some(name) = self.properties.get(expression) {
            return name.clone();
        }
        let name = format!("{}_property_{}", self.prefix, self.properties.len());
        self.items.push(format!(
            "/// A property definition of the tables of `{}` (#568).\nfn {name}() -> &'static ::ferroui_base::FerroProperty {{\n    {expression}\n}}\n",
            self.document_text
        ));
        self.properties.insert(expression.to_string(), name.clone());
        name
    }

    /// The function that returns the value `expression` of a property selector, as the
    /// selector holds it (`Rc::new(expression)`).
    fn selector_value(&mut self, expression: &str) -> String {
        if let Some(name) = self.selector_values.get(expression) {
            return name.clone();
        }
        let name = format!("{}_selector_value_{}", self.prefix, self.selector_values.len());
        self.items.push(format!(
            "/// The value of a property selector of `{}` (#568).\nfn {name}() -> ::ferroui_base::BoxedValue {{\n    ::std::rc::Rc::new({expression})\n}}\n",
            self.document_text
        ));
        self.selector_values.insert(expression.to_string(), name.clone());
        name
    }

    /// The slice text of the chain `steps`, its leaves written into the pool.
    fn chain(&mut self, steps: &[Step]) -> String {
        let mut rendered = Vec::with_capacity(steps.len());
        for step in steps {
            rendered.push(match step {
                Step::Nesting => "rt::SelectorStep::Nesting".to_string(),
                Step::Class(name) => format!("rt::SelectorStep::Class({name})"),
                Step::Name(name) => format!("rt::SelectorStep::Name({name})"),
                Step::Template => "rt::SelectorStep::Template".to_string(),
                Step::Child => "rt::SelectorStep::Child".to_string(),
                Step::Descendant => "rt::SelectorStep::Descendant".to_string(),
                Step::OfType(type_) => format!("rt::SelectorStep::OfType({type_})"),
                Step::Is(type_) => format!("rt::SelectorStep::Is({type_})"),
                Step::NthChild(step, offset) => format!("rt::SelectorStep::NthChild({step}, {offset})"),
                Step::NthLastChild(step, offset) => format!("rt::SelectorStep::NthLastChild({step}, {offset})"),
                Step::Not(argument) => format!("rt::SelectorStep::Not {{ argument: {} }}", self.chain(argument)),
                Step::Or(alternatives) => {
                    let alternatives: Vec<String> = alternatives.iter().map(|alternative| self.chain(alternative)).collect();
                    format!("rt::SelectorStep::Or {{ alternatives: &[{}] }}", alternatives.join(", "))
                }
                Step::PropertyEquals { property, value } => {
                    let property = self.property(property);
                    let value = self.selector_value(value);
                    format!("rt::SelectorStep::PropertyEquals {{ property: {property}, value: {value} }}")
                }
            });
        }
        format!("&[{}]", rendered.join(", "))
    }

    /// The static of the selector chain `steps`: one per distinct chain.
    fn selector(&mut self, steps: &[Step]) -> String {
        let chain = self.chain(steps);
        if let Some(name) = self.selectors.get(&chain) {
            return name.clone();
        }
        let name = format!("{}_SELECTOR_{}", self.prefix.to_uppercase(), self.selectors.len());
        self.items.push(format!(
            "/// A selector chain of `{}`, built by `rt::build_selector` (#568).\nstatic {name}: &[rt::SelectorStep] = {chain};\n",
            self.document_text
        ));
        self.selectors.insert(chain, name.clone());
        name
    }
}

// --- the parts of a statement ------------------------------------------------------------

/// `text` without its string literals (each replaced by `""`).
fn without_strings(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_string = false;
    let mut escaped = false;
    for character in text.chars() {
        if in_string {
            match character {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => {
                    in_string = false;
                    out.push('"');
                }
                _ => {}
            }
            continue;
        }
        if character == '"' {
            in_string = true;
        }
        out.push(character);
    }
    out
}

/// The Rust string literal at the start of `text` and the text after it.
fn split_string_literal(text: &str) -> Option<(&str, &str)> {
    if !text.starts_with('"') {
        return None;
    }
    let mut escaped = false;
    for (index, character) in text.char_indices().skip(1) {
        match (escaped, character) {
            (true, _) => escaped = false,
            (false, '\\') => escaped = true,
            (false, '"') => return Some(text.split_at(index + 1)),
            _ => {}
        }
    }
    None
}

/// Whether `text` is the call of an accessor without arguments by its absolute path
/// (`::ferroui_controls::Button::background_property()`).
fn is_plain_call(text: &str) -> bool {
    text.starts_with("::") && text.ends_with("()") && !text[..text.len() - 2].contains(['(', ')', ' ', '<', '>', ','])
}

/// Whether `text` is `<Path as ::ferroui_base::StaticType>::TYPE` for an absolute path.
fn is_static_type(text: &str) -> bool {
    text.strip_prefix('<')
        .and_then(|rest| rest.strip_suffix(" as ::ferroui_base::StaticType>::TYPE"))
        .is_some_and(|path| path.starts_with("::") && !path.contains(['(', ')', ' ', '<', '>', ',']))
}

/// The property definition at the start of `text`, as generated code names it
/// (`rt::property(<accessor>())` or `rt::class_property("<class>")`), and the text after it.
fn split_property(text: &str) -> Option<(&str, &str)> {
    if let Some(rest) = text.strip_prefix("rt::property(") {
        let end = rest.find("())")? + 2;
        let accessor = &rest[..end];
        if !is_plain_call(accessor) {
            return None;
        }
        let length = "rt::property(".len() + end + 1;
        return Some(text.split_at(length));
    }
    let rest = text.strip_prefix("rt::class_property(")?;
    let (_, after) = split_string_literal(rest)?;
    let after = after.strip_prefix(')')?;
    Some(text.split_at(text.len() - after.len()))
}

/// `(line, position)` of the text `"line, position"`.
fn position_of(text: &str) -> Option<(i32, i32)> {
    let (line, position) = text.split_once(", ")?;
    Some((line.parse().ok()?, position.parse().ok()?))
}

/// Whether `text` is a literal the emitter writes for an `f64` (`f64_literal`).
fn is_f64_literal(text: &str) -> bool {
    matches!(text, "::core::primitive::f64::NAN" | "::core::primitive::f64::INFINITY" | "::core::primitive::f64::NEG_INFINITY")
        || text.strip_suffix("_f64").is_some_and(|number| number.parse::<f64>().is_ok())
}

/// The text of `ThemeValue` of a constant the emitter writes as the value of `rt::add_setter`:
/// null, a boolean, an `i32`, an `f64`, a text, a thickness or a corner radius of literals.
fn constant_value(value: &str) -> Option<String> {
    if value == NONE {
        return Some("rt::ThemeValue::Null".to_string());
    }
    if let Some(text) = value.strip_prefix("rt::string_object(").and_then(|rest| rest.strip_suffix(')')) {
        return is_string_literal(text).then(|| format!("rt::ThemeValue::Text({text})"));
    }
    let inner = value.strip_prefix("rt::to_object(")?.strip_suffix(')')?;
    match inner {
        "::core::clone::Clone::clone(&true)" => return Some("rt::ThemeValue::Bool(true)".to_string()),
        "::core::clone::Clone::clone(&false)" => return Some("rt::ThemeValue::Bool(false)".to_string()),
        _ => {}
    }
    if is_f64_literal(inner) {
        return Some(format!("rt::ThemeValue::Double({inner})"));
    }
    if inner.strip_suffix("_i32").is_some_and(|number| number.parse::<i32>().is_ok()) {
        return Some(format!("rt::ThemeValue::Int({inner})"));
    }
    for (path, variant) in [(THICKNESS, "Thickness"), (CORNER_RADIUS, "CornerRadius")] {
        if let Some(arguments) = inner.strip_prefix(&format!("{path}::__markup_new_3(")).and_then(|rest| rest.strip_suffix(')')) {
            let arguments: Vec<&str> = arguments.split(", ").collect();
            if arguments.len() == 4 && arguments.iter().all(|argument| is_f64_literal(argument)) {
                return Some(format!("rt::ThemeValue::{variant}([{}])", arguments.join(", ")));
            }
        }
    }
    None
}

// --- the tree of a factory ---------------------------------------------------------------

/// A step of a selector chain, with its operands as generated code wrote them.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Step {
    Nesting,
    Class(String),
    Name(String),
    Template,
    Child,
    Descendant,
    OfType(String),
    Is(String),
    NthChild(i32, i32),
    NthLastChild(i32, i32),
    Not(Vec<Step>),
    Or(Vec<Vec<Step>>),
    PropertyEquals { property: String, value: String },
}

/// The key of `BasedOn`: a string literal or a type (`<X as StaticType>::TYPE`).
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Key {
    Text(String),
    Type(String),
}

/// How a setter is made (`rt::SetterForm`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Form {
    Plain,
    Created,
    Parented,
}

/// A control template as a setter value (`rt::ThemeTemplate`).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Template {
    pub(super) target_type: Option<String>,
    pub(super) content_type: String,
    pub(super) build: String,
    pub(super) parent: bool,
    pub(super) position: (i32, i32),
}

/// The value of a setter.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Value {
    Dynamic(String),
    Static { key: String, position: (i32, i32) },
    Template(Template),
    /// The text of a constant `rt::ThemeValue`.
    Constant(String),
    /// An expression that needs no context and cannot fail.
    Expression(String),
    /// The statements of the value (indented for the body of a function) and the value.
    Computed { statements: Vec<String>, value: String, uses_context: bool },
    Absent,
}

/// A member of a control theme or of a nested style.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Member {
    TargetType(String),
    BasedOn { key: Key, position: (i32, i32) },
    Setter { property: String, form: Form, value: Value },
    Style(Box<NestedStyle>),
}

/// A nested style.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct NestedStyle {
    pub(super) parent: bool,
    pub(super) selector: Option<Vec<Step>>,
    pub(super) members: Vec<Member>,
    pub(super) position: (i32, i32),
}

/// A control theme factory as a tree.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Theme {
    pub(super) parent: bool,
    pub(super) members: Vec<Member>,
}

/// A line of code of a body: its indentation, its text without it, and the line as written.
struct Code<'s> {
    indent: usize,
    text: &'s str,
    line: &'s str,
}

/// The parser of a factory body.
struct Parser<'s> {
    code: Vec<Code<'s>>,
    index: usize,
    /// The indentation of the top level of the body.
    top: usize,
    /// The locals the body declares.
    locals: HashSet<String>,
}

impl<'s> Parser<'s> {
    /// The text of the next line, at the top level of the body.
    fn peek(&self) -> Option<&'s str> {
        self.code.get(self.index).filter(|code| code.indent == self.top).map(|code| code.text)
    }

    /// The text of the next line at the top level, consumed.
    fn next(&mut self) -> Option<&'s str> {
        let text = self.peek()?;
        self.index += 1;
        Some(text)
    }

    /// Consumes the next line if it is `expected`.
    fn take_if(&mut self, expected: &str) -> bool {
        let found = self.peek() == Some(expected);
        if found {
            self.index += 1;
        }
        found
    }

    /// Whether `texts` name no local of the body but those of `own`, nor the name scope or the
    /// service provider of the build; with `context` allowed when `context` is set.
    fn closed(&self, texts: &[&str], own: &HashSet<String>, context: bool) -> bool {
        texts.iter().flat_map(|text| identifiers(text)).all(|name| match name.as_str() {
            "context" => context,
            "name_scope" | "service_provider" | "root" => false,
            name => !self.locals.contains(name) || own.contains(name),
        })
    }

    /// The members of `receiver` up to the end of its members: the end of the body, the pop of
    /// its parent or the addition of a style.
    fn members(&mut self, receiver: &str) -> Option<Vec<Member>> {
        let mut members = Vec::new();
        loop {
            let text = self.peek()?;
            if text == "context.pop_parent();" || text.starts_with(&format!("{STYLE_BASE}::__markup_Add_1(")) {
                return Some(members);
            }
            members.push(self.member(receiver, false)?);
        }
    }

    /// The member of `receiver` that starts at the next line; `theme` when `receiver` is the
    /// control theme itself.
    fn member(&mut self, receiver: &str, theme: bool) -> Option<Member> {
        let text = self.next()?;
        let style = format!("{receiver}.upcast_ref::<{STYLE_BASE}>()");
        if theme {
            if let Some(rest) = text.strip_prefix(&format!("{CONTROL_THEME}::__markup_set_TargetType(&{receiver}, {SOME}")) {
                let type_ = rest.strip_suffix("));")?;
                return is_static_type(type_).then(|| Member::TargetType(type_.to_string()));
            }
            if text.starts_with("let static_resource_extension_") {
                return self.based_on(text, receiver);
            }
        }
        if let Some(rest) = text.strip_prefix(&format!("rt::add_dynamic_resource_setter_with_parent(&context, {style}, ")) {
            let (property, rest) = split_property(rest)?;
            let key = rest.strip_prefix(", ")?.strip_suffix(");")?;
            return is_string_literal(key).then(|| Member::Setter {
                property: property.to_string(),
                form: Form::Plain,
                value: Value::Dynamic(key.to_string()),
            });
        }
        if let Some(rest) = text.strip_prefix(&format!("rt::add_setter({style}, ")) {
            let (property, rest) = split_property(rest)?;
            let value = rest.strip_prefix(", ")?.strip_suffix(");")?;
            return Some(Member::Setter { property: property.to_string(), form: Form::Plain, value: self.plain_value(value)? });
        }
        if let Some((local, created)) = text.strip_prefix("let ").and_then(|rest| rest.split_once(" = ")) {
            if !is_identifier(local) {
                return None;
            }
            if let Some(property) = created.strip_prefix("rt::new_setter(").and_then(|rest| rest.strip_suffix(");")) {
                return self.setter_with_value(local, property, &style, Form::Created);
            }
            if let Some(property) = created.strip_prefix("rt::new_setter_with_parent(&context, ").and_then(|rest| rest.strip_suffix(");")) {
                return self.setter_with_value(local, property, &style, Form::Parented);
            }
            if created == format!("{SETTER}::__markup_new_0();") {
                return self.setter_without_value(local, &style);
            }
            if created == format!("{STYLE}::new();") {
                return self.nested_style(local, receiver);
            }
        }
        None
    }

    /// `BasedOn="{StaticResource key}"`: the extension, `ProvideValue` and the assignment, at
    /// one position. `text` is the first statement.
    fn based_on(&mut self, text: &str, receiver: &str) -> Option<Member> {
        let (extension, created) = text.strip_prefix("let ")?.split_once(" = ")?;
        let key = created.strip_prefix(&format!("{STATIC_RESOURCE}::__markup_new_1("))?.strip_suffix(");")?;
        let key = if let Some(text) = key.strip_prefix("rt::string_object(").and_then(|rest| rest.strip_suffix(')')) {
            Key::Text(is_string_literal(text).then(|| text.to_string())?)
        } else {
            let type_ = key.strip_prefix("rt::boxed(")?.strip_suffix(')')?;
            Key::Type(is_static_type(type_).then(|| type_.to_string())?)
        };
        let theme_type = format!("<{CONTROL_THEME} as ::ferroui_base::StaticType>::TYPE");
        let provide = self.next()?;
        let (provided, call) = provide.strip_prefix("let ")?.split_once(" = ")?;
        let position = call
            .strip_prefix(&format!(
                "rt::provide_value_invoked(&context, rt::boxed(rt::clr_property_info(rt::class_markup({theme_type}), \"BasedOn\", false, rt::class_handle({theme_type}), false)), &{extension}, {STATIC_RESOURCE}::__markup_ProvideValue_0, "
            ))?
            .strip_suffix(")?;")?;
        let (line, column) = position_of(position)?;
        let assign = self.next()?;
        let expected = format!(
            "{CONTROL_THEME}::__markup_set_BasedOn(&{receiver}, rt::exact(rt::cast_checked({provided}, rt::class_handle({theme_type}), \"FerroUI.Styling.ControlTheme\", {line}, {column})?, \"FerroUI.Styling.ControlTheme.set_BasedOn\", 0, {line}, {column})?);"
        );
        (is_identifier(extension) && is_identifier(provided) && assign == expected)
            .then_some(Member::BasedOn { key, position: (line, column) })
    }

    /// The value of `rt::add_setter`, which has no statements of its own.
    fn plain_value(&self, value: &str) -> Option<Value> {
        if let Some(constant) = constant_value(value) {
            return Some(Value::Constant(constant));
        }
        let own = HashSet::new();
        if !self.closed(&[value], &own, true) {
            return None;
        }
        let uses_context = identifiers(value).iter().any(|name| name == "context");
        let fallible = without_strings(value).contains('?') || identifiers(value).iter().any(|name| name == "return");
        Some(match uses_context || fallible {
            true => Value::Computed { statements: Vec::new(), value: value.to_string(), uses_context },
            false => Value::Expression(value.to_string()),
        })
    }

    /// A setter created before its value (`rt::new_setter`, `rt::new_setter_with_parent`): the
    /// statements up to the addition of the value, then the addition.
    fn setter_with_value(&mut self, local: &str, property: &str, style: &str, form: Form) -> Option<Member> {
        let (property, rest) = split_property(property)?;
        if !rest.is_empty() {
            return None;
        }
        let end = match form {
            Form::Parented => format!("rt::add_setter_value_with_parent(&context, {style}, &{local}, "),
            _ => format!("rt::add_setter_value({style}, &{local}, "),
        };
        let start = self.index;
        let last = (start..self.code.len()).find(|&index| self.code[index].indent == self.top && self.code[index].text.starts_with(&end))?;
        let statements = &self.code[start..last];
        if statements.iter().any(|code| code.indent < self.top) {
            return None;
        }
        let value = self.code[last].text.strip_prefix(&end)?.strip_suffix(");")?;
        let value = self.value_with_statements(statements, value)?;
        self.index = last + 1;
        Some(Member::Setter { property: property.to_string(), form, value })
    }

    /// The value of a setter created before it: a `{StaticResource}`, a control template, or the
    /// statements as a function of the factory.
    fn value_with_statements(&self, statements: &[Code<'s>], value: &str) -> Option<Value> {
        let texts: Vec<&str> = statements.iter().map(|code| code.text).collect();
        if let Some(value) = static_resource_value(&texts, value) {
            return Some(value);
        }
        let top: Vec<&str> = statements.iter().filter(|code| code.indent == self.top).map(|code| code.text).collect();
        if top.len() == statements.len() {
            if let Some(template) = template_value(&texts, value) {
                return Some(Value::Template(template));
            }
        }
        let own: HashSet<String> = texts.iter().filter_map(|text| declared_local(text)).map(|(name, _)| name).collect();
        let mut all = texts.clone();
        all.push(value);
        if !self.closed(&all, &own, true) {
            return None;
        }
        let uses_context = all.iter().any(|text| identifiers(text).iter().any(|name| name == "context"));
        let statements = statements.iter().map(|code| format!("    {}", &code.line[self.top..])).collect();
        Some(Value::Computed { statements, value: value.to_string(), uses_context })
    }

    /// `new Setter { Property = .. }` without a value, added to the style as it is.
    fn setter_without_value(&mut self, local: &str, style: &str) -> Option<Member> {
        let property = self
            .next()?
            .strip_prefix(&format!("{SETTER}::__markup_set_Property(&{local}, {SOME}"))?
            .strip_suffix("));")?;
        let (property, rest) = split_property(property)?;
        let add = self.next()?;
        let expected =
            format!("{STYLE_BASE}::__markup_Add_0({style}, ::core::clone::Clone::clone(&{local}) as ::std::rc::Rc<dyn {SETTER_BASE}>);");
        (rest.is_empty() && add == expected).then(|| Member::Setter { property: property.to_string(), form: Form::Plain, value: Value::Absent })
    }

    /// A nested style `local` of `receiver`: the style, its place on the parent stack, its
    /// selector, its members, then its addition to `receiver`.
    fn nested_style(&mut self, local: &str, receiver: &str) -> Option<Member> {
        let parent = self.take_if(&format!("context.push_parent(rt::to_value({local}.clone()));"));
        let selector = match self.peek()?.starts_with("let selector_") {
            true => Some(self.selector(local)?),
            false => None,
        };
        let members = self.members(local)?;
        if parent && !self.take_if("context.pop_parent();") {
            return None;
        }
        let position = self
            .next()?
            .strip_prefix(&format!("{STYLE_BASE}::__markup_Add_1({receiver}.upcast_ref::<{STYLE_BASE}>(), rt::cast({local}.clone(), "))?
            .strip_suffix(")?);")?;
        let position = position_of(position)?;
        Some(Member::Style(Box::new(NestedStyle { parent, selector, members, position })))
    }

    /// The selector chain of the style `style`: its statements up to `Style.Selector`.
    fn selector(&mut self, style: &str) -> Option<Vec<Step>> {
        let mut chains: Vec<(String, Vec<Step>)> = Vec::new();
        let take = |chains: &mut Vec<(String, Vec<Step>)>, local: &str| -> Option<Vec<Step>> {
            let index = chains.iter().position(|(name, _)| name == local)?;
            Some(chains.remove(index).1)
        };
        loop {
            let text = self.next()?;
            if let Some(rest) = text.strip_prefix(&format!("{STYLE}::__markup_set_Selector(&{style}, {SOME}")) {
                let local = rest.strip_suffix(".clone()));")?;
                let chain = take(&mut chains, local)?;
                return chains.is_empty().then_some(chain);
            }
            let (local, call) = text.strip_prefix("let ")?.split_once(" = ")?;
            let (operation, arguments) = call.strip_prefix(&format!("{SELECTORS}::"))?.strip_suffix(");")?.split_once('(')?;
            if !is_identifier(local) {
                return None;
            }
            if operation == "or" {
                let alternatives = arguments.strip_prefix('[')?.strip_suffix(']')?;
                let mut list = Vec::new();
                for alternative in alternatives.split(", ") {
                    list.push(take(&mut chains, alternative)?);
                }
                if list.len() < 2 {
                    return None;
                }
                chains.push((local.to_string(), vec![Step::Or(list)]));
                continue;
            }
            let (previous, rest) = match arguments.strip_prefix(NONE) {
                Some(rest) => (None, rest),
                None => {
                    let rest = arguments.strip_prefix(SOME)?;
                    let (previous, rest) = rest.split_once(')')?;
                    (Some(previous), rest)
                }
            };
            let operand = rest.strip_prefix(", ");
            let step = match operation {
                "nesting" | "template" | "child" | "descendant" if rest.is_empty() => match operation {
                    "nesting" => Step::Nesting,
                    "template" => Step::Template,
                    "child" => Step::Child,
                    _ => Step::Descendant,
                },
                "class" | "name" => {
                    let name = operand.filter(|name| is_string_literal(name))?.to_string();
                    match operation {
                        "class" => Step::Class(name),
                        _ => Step::Name(name),
                    }
                }
                "of_type_info" | "is_type_info" => {
                    let type_ = operand.filter(|type_| is_static_type(type_))?.to_string();
                    match operation {
                        "of_type_info" => Step::OfType(type_),
                        _ => Step::Is(type_),
                    }
                }
                "nth_child" | "nth_last_child" => {
                    let (step, offset) = position_of(operand?)?;
                    match operation {
                        "nth_child" => Step::NthChild(step, offset),
                        _ => Step::NthLastChild(step, offset),
                    }
                }
                "not" => Step::Not(take(&mut chains, operand?)?),
                "property_equals_untyped" => {
                    let (property, rest) = split_property(operand?)?;
                    let value = rest.strip_prefix(", ::std::rc::Rc::new(")?.strip_suffix(')')?;
                    let fallible = without_strings(value).contains('?');
                    if fallible || !self.closed(&[value], &HashSet::new(), false) {
                        return None;
                    }
                    Step::PropertyEquals { property: property.to_string(), value: value.to_string() }
                }
                _ => return None,
            };
            let mut chain = match previous {
                Some(previous) => take(&mut chains, previous)?,
                None => Vec::new(),
            };
            chain.push(step);
            chains.push((local.to_string(), chain));
        }
    }
}

/// `{StaticResource key}` as the value of a setter on the parent stack or not: the extension
/// created with the key, `ProvideValue` with `Setter.Value` as the target property, the provided
/// value as it is.
fn static_resource_value(statements: &[&str], value: &str) -> Option<Value> {
    let [create, provide] = statements else { return None };
    let (extension, created) = create.strip_prefix("let ")?.split_once(" = ")?;
    let key = created
        .strip_prefix(&format!("{STATIC_RESOURCE}::__markup_new_1(rt::string_object("))?
        .strip_suffix("));")?;
    let (provided, call) = provide.strip_prefix("let ")?.split_once(" = ")?;
    let position = call
        .strip_prefix(&format!(
            "rt::provide_value_invoked(&context, rt::setter_value_property(), &{extension}, {STATIC_RESOURCE}::__markup_ProvideValue_0, "
        ))?
        .strip_suffix(")?;")?;
    let position = position_of(position)?;
    (is_identifier(extension) && is_identifier(provided) && is_string_literal(key) && value == format!("::core::clone::Clone::clone(&{provided})"))
        .then(|| Value::Static { key: key.to_string(), position })
}

/// A `ControlTemplate` as the value of a setter: created, on the parent stack or not, its target
/// type when the document states one, and its content, the deferred content of a build function.
fn template_value(statements: &[&str], value: &str) -> Option<Template> {
    let (first, mut rest) = statements.split_first()?;
    let (local, created) = first.strip_prefix("let ")?.split_once(" = ")?;
    if !is_identifier(local) || created != format!("{CONTROL_TEMPLATE}::__markup_new_0();") {
        return None;
    }
    let parent = rest.first() == Some(&format!("context.push_parent(rt::to_value({local}.clone()));").as_str());
    if parent {
        rest = &rest[1..];
        let (last, inner) = rest.split_last()?;
        if *last != "context.pop_parent();" {
            return None;
        }
        rest = inner;
    }
    let mut target_type = None;
    if let Some(type_) = rest
        .first()
        .and_then(|text| text.strip_prefix(&format!("{CONTROL_TEMPLATE}::__markup_set_TargetType(&{local}, {SOME}")))
    {
        let type_ = type_.strip_suffix("));")?;
        if !is_static_type(type_) {
            return None;
        }
        target_type = Some(type_.to_string());
        rest = &rest[1..];
    }
    let [defer, content] = rest else { return None };
    let (deferred, call) = defer.strip_prefix("let ")?.split_once(" = ")?;
    let call = call.strip_prefix("rt::defer(rt::class_handle(")?.strip_suffix(")?;")?;
    let (content_type, call) = call.split_once("), &context, ")?;
    let (build, position) = call.split_once(", ")?;
    let position = position_of(position)?;
    let expected_content = format!("{CONTROL_TEMPLATE}::__markup_set_Content(&{local}, rt::to_object(::core::clone::Clone::clone(&{deferred})));");
    (is_identifier(deferred)
        && is_static_type(content_type)
        && is_identifier(build)
        && *content == expected_content
        && value == format!("rt::to_object(::core::clone::Clone::clone(&{local}))"))
    .then(|| Template { target_type, content_type: content_type.to_string(), build: build.to_string(), parent, position })
}

/// The tree of the factory whose body the emitter wrote as `lines`, returning `returned`, if
/// every statement of it is part of a member a table holds.
pub(super) fn parse_theme(lines: &[String], returned: &str) -> Option<Theme> {
    if returned != "rt::to_value(control_theme_0.clone())" {
        return None;
    }
    let code: Vec<Code<'_>> = lines
        .iter()
        .map(|line| (line.len() - line.trim_start().len(), line.trim_start(), line.as_str()))
        .filter(|(_, text, _)| !text.starts_with("//"))
        .map(|(indent, text, line)| Code { indent, text, line })
        .collect();
    let first = code.first()?;
    if first.text != format!("let control_theme_0 = {CONTROL_THEME}::new();") {
        return None;
    }
    let top = first.indent;
    let locals = code.iter().filter_map(|code| declared_local(code.text)).map(|(name, _)| name).collect();
    let mut parser = Parser { code, index: 1, top, locals };
    let parent = parser.take_if("context.push_parent(rt::to_value(control_theme_0.clone()));");
    let mut members = Vec::new();
    let mut popped = false;
    while parser.index < parser.code.len() {
        if parent && parser.index + 1 == parser.code.len() {
            if !parser.take_if("context.pop_parent();") {
                return None;
            }
            popped = true;
            break;
        }
        members.push(parser.member("control_theme_0", true)?);
    }
    (popped == parent).then_some(Theme { parent, members })
}

// --- the table of a factory ---------------------------------------------------------------

/// Writes the members `members` at the indentation `depth` into `out`, the value functions of
/// the factory `function` and the shared items into `pool`.
fn render_members(pool: &mut StylePool, function: &str, values: &mut Vec<String>, members: &[Member], depth: usize, out: &mut String) {
    let pad = "    ".repeat(depth);
    for member in members {
        match member {
            Member::TargetType(type_) => out.push_str(&format!("{pad}rt::StyleMember::TargetType({type_}),\n")),
            Member::BasedOn { key, position: (line, column) } => {
                let key = match key {
                    Key::Text(text) => format!("rt::ThemeKey::Text({text})"),
                    Key::Type(type_) => format!("rt::ThemeKey::Type({type_})"),
                };
                out.push_str(&format!("{pad}rt::StyleMember::BasedOn {{ key: {key}, line: {line}, position: {column} }},\n"));
            }
            Member::Setter { property, form, value } => {
                let property = pool.property(property);
                let form = match form {
                    Form::Plain => "rt::SetterForm::Plain",
                    Form::Created => "rt::SetterForm::Created",
                    Form::Parented => "rt::SetterForm::Parented",
                };
                let value = render_value(function, values, value);
                out.push_str(&format!("{pad}rt::StyleMember::Setter(rt::ThemeSetter {{ property: {property}, form: {form}, value: {value} }}),\n"));
            }
            Member::Style(style) => {
                let selector = match &style.selector {
                    Some(steps) => format!("{SOME}{})", pool.selector(steps)),
                    None => NONE.to_string(),
                };
                let (line, column) = style.position;
                out.push_str(&format!("{pad}rt::StyleMember::Style {{\n"));
                out.push_str(&format!("{pad}    parent: {},\n", style.parent));
                out.push_str(&format!("{pad}    selector: {selector},\n"));
                out.push_str(&format!("{pad}    line: {line},\n"));
                out.push_str(&format!("{pad}    position: {column},\n"));
                out.push_str(&format!("{pad}    members: &[\n"));
                render_members(pool, function, values, &style.members, depth + 2, out);
                out.push_str(&format!("{pad}    ],\n"));
                out.push_str(&format!("{pad}}},\n"));
            }
        }
    }
}

/// The text of the `rt::ThemeValue` of `value`; a value of code becomes a function of the
/// factory, added to `values`.
fn render_value(function: &str, values: &mut Vec<String>, value: &Value) -> String {
    match value {
        Value::Dynamic(key) => format!("rt::ThemeValue::Dynamic({key})"),
        Value::Static { key, position: (line, column) } => format!("rt::ThemeValue::Static {{ key: {key}, line: {line}, position: {column} }}"),
        Value::Template(template) => {
            let target_type = match &template.target_type {
                Some(type_) => format!("{SOME}{type_})"),
                None => NONE.to_string(),
            };
            let (line, column) = template.position;
            format!(
                "rt::ThemeValue::Template(rt::ThemeTemplate {{ target_type: {target_type}, content_type: {}, build: {}, parent: {}, line: {line}, position: {column} }})",
                template.content_type, template.build, template.parent
            )
        }
        Value::Constant(constant) => constant.clone(),
        Value::Expression(expression) => {
            let name = format!("{function}_value_{}", values.len());
            values.push(format!(
                "/// A setter value of the control theme built by `{function}` (#568).\nfn {name}() -> {MARKUP_VALUE} {{\n    {expression}\n}}\n"
            ));
            format!("rt::ThemeValue::Expression({name})")
        }
        Value::Computed { statements, value, uses_context } => {
            let name = format!("{function}_value_{}", values.len());
            let context = match *uses_context {
                true => "context",
                false => "_context",
            };
            let mut text = format!(
                "/// A setter value of the control theme built by `{function}` (#568).\nfn {name}(\n    {context}: &{CONTEXT_TYPE},\n) -> ::core::result::Result<{MARKUP_VALUE}, {LOAD_ERROR}> {{\n"
            );
            for statement in statements {
                text.push_str(statement);
                text.push('\n');
            }
            text.push_str(&format!("    ::core::result::Result::Ok({value})\n}}\n"));
            values.push(text);
            format!("rt::ThemeValue::Computed({name})")
        }
        Value::Absent => "rt::ThemeValue::Absent".to_string(),
    }
}

/// The factory `function` of the control theme `theme` as a table: the table and the value
/// functions of the factory are added to the items of `pool`, and the result is the statement
/// that builds the theme with the table (the context of the build is `context`).
pub(super) fn control_theme_table(pool: &mut StylePool, function: &str, theme: &Theme) -> String {
    let table = format!("{}_THEME", function.to_uppercase());
    let mut values = Vec::new();
    let mut members = String::new();
    render_members(pool, function, &mut values, &theme.members, 2, &mut members);
    let mut text = format!(
        "/// The control theme built by `{function}`, interpreted by `rt::build_control_theme` (#568).\nstatic {table}: rt::ControlThemeTable = rt::ControlThemeTable {{\n    parent: {},\n    members: &[\n",
        theme.parent
    );
    text.push_str(&members);
    text.push_str("    ],\n};\n");
    pool.items.push(text);
    pool.items.extend(values);
    format!("rt::build_control_theme(&context, &{table})")
}

// --- template bindings --------------------------------------------------------------------

/// The parts of `rt::bind_template_binding(<object>.upcast_ref::<FerroObject>(), <target>,
/// rt::property(<source>), <line>, <position>)?;`, the statement the emitter fuses a
/// `{TemplateBinding}` into: the object, the target and source accessors and the position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TemplateBinding<'s> {
    pub(super) object: &'s str,
    pub(super) target: &'s str,
    pub(super) source: &'s str,
    pub(super) position: (i32, i32),
}

/// The parts of `statement` if it is the fused statement of a template binding.
pub(super) fn template_binding(statement: &str) -> Option<TemplateBinding<'_>> {
    let arguments = statement.strip_prefix("rt::bind_template_binding(")?.strip_suffix(")?;")?;
    let parts: Vec<&str> = arguments.split(", ").collect();
    let &[object, target, source, line, column] = parts.as_slice() else { return None };
    let object = object.strip_suffix(&format!(".upcast_ref::<{FERRO_OBJECT}>()"))?;
    let source = source.strip_prefix("rt::property(")?.strip_suffix(')')?;
    let position = (line.parse().ok()?, column.parse().ok()?);
    (is_identifier(object) && is_plain_call(target) && is_plain_call(source)).then_some(TemplateBinding { object, target, source, position })
}

/// A template binding found in a body: the index of its first line (its position markers) and
/// of the statement.
struct FoundBinding<'s> {
    start: usize,
    statement: usize,
    indent: usize,
    binding: TemplateBinding<'s>,
}

/// Ends the run `run`: a run as long as [`MIN_BINDING_RUN`] becomes the position markers of its
/// first statement and one call of `rt::bind_template_bindings` over a new table (added to
/// `pool`); a shorter run is written as it was.
fn flush_bindings(pool: &mut StylePool, run: &mut Vec<FoundBinding<'_>>, lines: &[String], body: &mut Vec<String>) {
    let Some(first) = run.first() else { return };
    if run.len() < MIN_BINDING_RUN {
        for found in run.iter() {
            body.extend(lines[found.start..=found.statement].iter().cloned());
        }
        run.clear();
        return;
    }
    let name = format!("{}_BINDINGS_{}", pool.prefix.to_uppercase(), pool.binding_tables);
    pool.binding_tables += 1;
    let mut table = format!(
        "/// The template bindings of a run of `{}`, bound in order by `rt::bind_template_bindings` (#568).\nstatic {name}: &[rt::TemplateBindingRow] = &[\n",
        pool.document_text
    );
    for found in run.iter() {
        let TemplateBinding { target, source, position: (line, column), .. } = &found.binding;
        let target = pool.property(&format!("rt::property({target})"));
        let source = pool.property(&format!("rt::property({source})"));
        table.push_str(&format!("    rt::TemplateBindingRow {{ target: {target}, source: {source}, line: {line}, position: {column} }},\n"));
    }
    table.push_str("];\n");
    pool.items.push(table);
    body.extend(lines[first.start..first.statement].iter().cloned());
    body.push(format!(
        "{}rt::bind_template_bindings({}.upcast_ref::<{FERRO_OBJECT}>(), {name})?;",
        " ".repeat(first.indent),
        first.binding.object
    ));
    run.clear();
}

/// `lines`, a body the emitter wrote, with each run of consecutive template bindings on one
/// object (position markers between them allowed) written as one call over a table.
pub(super) fn template_binding_runs(pool: &mut StylePool, lines: Vec<String>) -> Vec<String> {
    if !lines.iter().any(|line| line.trim_start().starts_with("rt::bind_template_binding(")) {
        return lines;
    }
    let mut body = Vec::with_capacity(lines.len());
    let mut run: Vec<FoundBinding<'_>> = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let start = index;
        while index < lines.len() && lines[index].trim_start().starts_with("//") {
            index += 1;
        }
        if index == lines.len() {
            flush_bindings(pool, &mut run, &lines, &mut body);
            body.extend(lines[start..].iter().cloned());
            break;
        }
        let line = &lines[index];
        let indent = line.len() - line.trim_start().len();
        let Some(binding) = template_binding(line.trim_start()) else {
            flush_bindings(pool, &mut run, &lines, &mut body);
            body.extend(lines[start..=index].iter().cloned());
            index += 1;
            continue;
        };
        let continues = run.first().is_some_and(|first| first.binding.object == binding.object && first.indent == indent);
        if !continues {
            flush_bindings(pool, &mut run, &lines, &mut body);
        }
        run.push(FoundBinding { start, statement: index, indent, binding });
        index += 1;
    }
    flush_bindings(pool, &mut run, &lines, &mut body);
    body
}

// --- the setter of a resource value -------------------------------------------------------

/// The text of `rt::ValueHandle` of a type check generated code writes (`rt::markup_handle(..)`,
/// `rt::class_handle(..)`, `ValueType::of::<..>()`).
fn value_handle(handle: &str) -> Option<String> {
    if let Some(markup) = handle.strip_prefix("rt::markup_handle(").and_then(|rest| rest.strip_suffix(')')) {
        let valid = markup.starts_with('<') && markup.ends_with(" as ::ferroui_base::metadata::MarkupTyped>::MARKUP") && !markup.contains(['(', ')']);
        return valid.then(|| format!("rt::ValueHandle::Markup({markup})"));
    }
    if let Some(class) = handle.strip_prefix("rt::class_handle(").and_then(|rest| rest.strip_suffix(')')) {
        return is_static_type(class).then(|| format!("rt::ValueHandle::Class({class})"));
    }
    let of = handle.strip_prefix("::ferroui_base::data::core::ValueType::of::<")?.strip_suffix(">()")?;
    (!of.contains(['(', ')'])).then(|| format!("rt::ValueHandle::Of(::ferroui_base::data::core::ValueType::of::<{of}>)"))
}

/// The typed setter statement of a run-time choice: `<object>.set_value(<property>, rt::exact(
/// <value>.clone(), <member>, <index>, <line>, <position>)?);`, or `set_value_with_priority` with
/// a priority after the value. Returns the member, the index, the position and the priority.
fn typed_setter<'s>(statement: &'s str, object: &str, property: &str, local: &str) -> Option<(&'s str, &'s str, (i32, i32), Option<&'s str>)> {
    let (rest, prioritised) = match statement.strip_prefix(&format!("{object}.set_value_with_priority(")) {
        Some(rest) => (rest, true),
        None => (statement.strip_prefix(&format!("{object}.set_value("))?, false),
    };
    let rest = rest.strip_prefix(&format!("{property}, rt::exact({local}.clone(), "))?;
    let (member, rest) = split_string_literal(rest)?;
    let rest = rest.strip_prefix(", ")?;
    let (index, rest) = rest.split_once(", ")?;
    let (position, rest) = rest.split_once(")?")?;
    let priority = match prioritised {
        true => {
            let priority = rest.strip_prefix(", ")?.strip_suffix(");")?;
            Some(is_identifier(priority).then_some(priority)?)
        }
        false => {
            if rest != ");" {
                return None;
            }
            None
        }
    };
    index.parse::<usize>().ok()?;
    Some((member, index, position_of(position)?, priority))
}

/// The run-time choice of the setter of `local` that the emitter wrote as `chain` (the lines of
/// the `if` chain, relative to its indentation) as one call of `rt::assign_resource_value`, if
/// it is a chain of that helper: the unset marker (optional), a binding, an instance of one
/// type, null (optional) through the same typed setter, else the error of no setter, all on one
/// object and property and at one position. `None` (the chain is kept) for any other chain.
pub(super) fn resource_choice(local: &str, chain: &[String]) -> Option<String> {
    let lines: Vec<&str> = chain.iter().map(String::as_str).collect();
    let unset_check = format!(
        "rt::is_instance(&{local}, rt::markup_handle(<::ferroui_base::UnsetValueType as ::ferroui_base::metadata::MarkupTyped>::MARKUP))"
    );
    let binding_check = format!(
        "rt::is_instance(&{local}, rt::markup_handle(<dyn ::ferroui_base::data::BindingBase as ::ferroui_base::metadata::MarkupTyped>::MARKUP))"
    );
    let mut rest: &[&str] = &lines;
    let unset = rest.first() == Some(&format!("if {unset_check} {{").as_str());
    let unset_statement = match unset {
        true => {
            let statement = rest.get(1)?.strip_prefix("    ")?;
            rest = &rest[2..];
            if rest.first() != Some(&format!("}} else if {binding_check} {{").as_str()) {
                return None;
            }
            Some(statement)
        }
        false => {
            if rest.first() != Some(&format!("if {binding_check} {{").as_str()) {
                return None;
            }
            None
        }
    };
    let bind = rest.get(1)?.strip_prefix("    rt::bind(")?;
    let (object, bind) = bind.split_once(&format!(".upcast_ref::<{FERRO_OBJECT}>(), "))?;
    let (property, position) = bind.strip_suffix(")?;")?.split_once(&format!(", {local}.clone(), "))?;
    let position = position_of(position)?;
    if !is_identifier(object) || !is_plain_call(property) {
        return None;
    }
    if let Some(statement) = unset_statement {
        if statement != format!("rt::unset_value({object}.upcast_ref::<{FERRO_OBJECT}>(), {property});") {
            return None;
        }
    }
    let handle = rest.get(2)?.strip_prefix(&format!("}} else if rt::is_instance(&{local}, "))?.strip_suffix(") {")?;
    let handle = value_handle(handle)?;
    let setter = rest.get(3)?.strip_prefix("    ")?;
    let (member, index, setter_position, priority) = typed_setter(setter, object, property, local)?;
    rest = &rest[4..];
    let null = rest.first() == Some(&format!("}} else if {local}.is_none() {{").as_str());
    if null {
        if rest.get(1)?.strip_prefix("    ")? != setter {
            return None;
        }
        rest = &rest[2..];
    }
    let [otherwise, error, end] = rest else { return None };
    let error = error.strip_prefix("    return ::core::result::Result::Err(rt::no_setter(")?.strip_suffix("));")?;
    let (name, error) = split_string_literal(error)?;
    let error_position = position_of(error.strip_prefix(&format!(", &{local}, "))?)?;
    if *otherwise != "} else {" || *end != "}" || setter_position != position || error_position != position {
        return None;
    }
    let priority = match priority {
        Some(priority) => format!("{SOME}{priority})"),
        None => NONE.to_string(),
    };
    let (line, column) = position;
    Some(format!(
        "rt::assign_resource_value({object}.upcast_ref::<{FERRO_OBJECT}>(), {property}, {local}, &rt::ResourceChoice {{ name: {name}, member: {member}, index: {index}, unset: {unset}, instance: {handle}, null: {null}, line: {line}, position: {column} }}, {priority})?;"
    ))
}

#[cfg(test)]
#[path = "theme_styles_tests.rs"]
mod tests;
