//! Lower the typed construction statements of the Rust emitter. Only exact forms emitted
//! for resolved constructors and registered property setters are admitted. Lexical boundaries
//! and statements with another calling convention end a program.
use super::{is_identifier, is_string_literal, string_literal_of, theme_styles};
use ferroui_markup::construction_opcodes as op;
use std::collections::HashMap;

#[path = "construction_literals.rs"]
mod literals;
pub(super) use literals::closed_value;

type Types = HashMap<String, String>;
const NAMESPACE_SCOPE: &str = "::ferroui_markup_xaml::xaml_il::runtime::XamlStaticProviderScope";
const SETTER: &str = "::std::rc::Rc<::ferroui_base::styling::Setter>";
const MARKUP_VALUE: &str = "::ferroui_base::metadata::MarkupValue";

fn class<'a>(types: &'a Types, name: &str) -> Option<&'a str> {
    types
        .get(name)?
        .strip_prefix("::ferroui_base::Ref<")?
        .strip_suffix('>')
}

fn absolute_path(text: &str) -> bool {
    text.strip_prefix("::")
        .or_else(|| text.strip_prefix("crate::"))
        .is_some_and(|path| path.split("::").all(is_identifier))
}

enum Value<'a> {
    Float(f64),
    Int(i32),
    Four(&'a str, [f64; 4]),
    String(&'a str, &'static str),
    Constant(&'a str),
    Object(&'a str, &'a str),
    Child(&'a str, &'a str),
}

enum Instruction<'a> {
    New(&'a str, &'a str, &'a str),
    NewBinding(&'a str, &'a str),
    StyleLiteral(&'a str, &'a str, String, Vec<CallInput>),
    DynamicStyle(&'a str, &'a str, &'a str),
    BeginSetter { name: &'a str, property: &'a str, parented: bool },
    FinishSetter { style: &'a str, setter: &'a str, source: &'a str, object: bool, adapter: String, parented: bool },
    PropertyLiteral(&'a str, String, Vec<CallInput>),
    StaticResource(StaticResource<'a>),
    ProvideBinding(&'a str, &'a str, &'a str, i32, i32),
    Defer(&'a str, &'a str, &'a str, i32, i32),
    Call(&'a ResolvedCall, Option<&'a str>),
    Complete(Option<&'a str>, i32, i32),
    Begin(&'a str),
    End(&'a str, i32, i32),
    Bind(theme_styles::TemplateBinding<'a>, bool),
    Dynamic(&'a str, &'a str, &'a str, i32, i32),
    Parent(&'a str, &'a str, u8),
    Pop,
    ClearTargetProperty,
    ApplyThemeUnit(&'a str, u32, u32),
    Namespace(&'a str, &'a str),
    Name(&'a str, &'a str, &'a str, i32, i32),
    Set(&'a str, &'a str, &'a str, Option<&'a str>, Value<'a>),
}

#[derive(PartialEq, Eq)]
pub(super) enum CallInput {
    Register(String, bool),
    /// Read from the current XAML context. The adapter needs no encoded bytes for this input.
    DocumentBaseUri,
    Text(String),
    Double(u64),
    Int(i32),
    Uint(u32),
    Bool(bool),
}

#[derive(PartialEq, Eq)]
pub(super) enum CallKind {
    ObjectLiteral(Vec<u8>),
    Member,
    ContextMember,
    RegisterSetter,
    TargetProperty,
    Path { named: bool },
    Selector,
}

/// A native import for one previously resolved, typed member, with object register operands.
#[derive(PartialEq, Eq)]
pub(super) struct ResolvedCall {
    pub(super) kind: CallKind,
    pub(super) inputs: Vec<CallInput>,
    pub(super) import: String,
    pub(super) result: Option<(String, bool)>,
    pub(super) line: i32,
    pub(super) column: i32,
}

pub(super) type Calls = HashMap<String, Option<ResolvedCall>>;

/// The final setter argument is either a MarkupValue or a typed clone boxed as System.Object.
/// Use the native conversion verbatim for both VM locals and borrowed inputs.
fn setter_operand<'a>(value: &'a str, types: &Types, locals: &[(&str, &str, bool)]) -> Option<(&'a str, bool, String)> {
    let (value, boxed) = match value.strip_prefix("rt::to_object(") {
        Some(value) => (value.strip_suffix(')')?, true),
        None => (value, false),
    };
    let source = value.strip_prefix("::core::clone::Clone::clone(&")?.strip_suffix(')')?;
    if !is_identifier(source) {
        return None;
    }
    if !boxed {
        return Some((source, false, format!("SetterValue(|input| Ok(input.value::<{MARKUP_VALUE}>()?.clone()))")));
    }
    let (ty, object) = if let Some(&(_, ty, object)) = locals.iter().find(|&&(name, _, _)| name == source) {
        (ty, object)
    } else if let Some(ty) = class(types, source) {
        (ty, true)
    } else {
        (types.get(source)?.as_str(), false)
    };
    let read = if object { "object_ref" } else { "value" };
    Some((source, object, format!("SetterValue(|input| Ok(rt::to_object(input.{read}::<{ty}>()?.clone())))")))
}

fn parse<'a>(line: &'a str, types: &'a Types, calls: &'a Calls, locals: &[(&str, &str, bool)]) -> Option<Instruction<'a>> {
    let line = line.strip_prefix("    ")?;
    if let Some(args) = line.strip_prefix("rt::complete_root_name_scope(") {
        let (root, pos) = args.split_once(", name_scope.as_ref(), ")?;
        let (line, column) = pos.strip_suffix(")?;")?.split_once(", ")?;
        let root = match root {
            "::core::option::Option::None" => None,
            some => Some(
                some.strip_prefix("::core::option::Option::Some(&")?
                    .strip_suffix(')')?,
            ),
        };
        return Some(Instruction::Complete(
            root,
            line.parse().ok()?,
            column.parse().ok()?,
        ));
    }
    if let Some(call) = line
        .strip_suffix(';')
        .and_then(|s| calls.get(s))
        .and_then(Option::as_ref)
    {
        return Some(Instruction::Call(call, None));
    }
    if line == "context.pop_parent();" {
        return Some(Instruction::Pop);
    }
    if line == "context.set_target_property(::core::option::Option::None);" {
        return Some(Instruction::ClearTargetProperty);
    }
    if let Some(args) = line.strip_prefix("rt::apply_theme_unit(active_theme_units(), ") {
        let (indices, dictionary) = args.strip_suffix(")?;")?.split_once(", &context, &")?;
        let (unit, run) = indices.split_once(", ")?;
        if is_identifier(dictionary) {
            return Some(Instruction::ApplyThemeUnit(dictionary, unit.parse().ok()?, run.parse().ok()?));
        }
    }
    if let Some((name, table)) = line.strip_prefix("let ")
        .and_then(|s| s.strip_suffix(");"))
        .and_then(|s| s.split_once(" = rt::push_namespace_scope(&context, "))
    {
        if is_identifier(name) && is_identifier(table) {
            return Some(Instruction::Namespace(name, table));
        }
    }
    for (prefix, opcode) in [
        ("context.push_parent(rt::to_value(", op::PUSH_PARENT),
        (
            "context.set_intermediate_root_object(rt::to_value(",
            op::INTERMEDIATE_ROOT,
        ),
    ] {
        if let Some(name) = line
            .strip_prefix(prefix)
            .and_then(|s| s.strip_suffix(".clone()));"))
        {
            return Some(Instruction::Parent(name, class(types, name)?, opcode));
        }
    }
    let finish_setter = line.strip_prefix("rt::add_setter_value(").map(|args| (args, false))
        .or_else(|| line.strip_prefix("rt::add_setter_value_with_parent(&context, ").map(|args| (args, true)));
    if let Some((args, parented)) = finish_setter {
        let (style, args) = args.split_once(".upcast_ref::<::ferroui_base::styling::StyleBase>(), &")?;
        let (setter, value) = args.strip_suffix(");")?.split_once(", ")?;
        if !is_identifier(style) || class(types, style).is_none() || !is_identifier(setter) {
            return None;
        }
        let (source, object, adapter) = setter_operand(value, types, locals)?;
        return Some(Instruction::FinishSetter { style, setter, source, object, adapter, parented });
    }
    let style_setter = line
        .strip_prefix("rt::add_setter(")
        .map(|args| (args, false))
        .or_else(|| {
            line.strip_prefix("rt::add_dynamic_resource_setter_with_parent(&context, ")
                .map(|args| (args, true))
        });
    if let Some((args, dynamic)) = style_setter {
        let (style, args) =
            args.split_once(".upcast_ref::<::ferroui_base::styling::StyleBase>(), ")?;
        if !is_identifier(style) || class(types, style).is_none() {
            return None;
        }
        let (property, rest) = theme_styles::split_property(args)?;
        let value = rest.strip_prefix(", ")?.strip_suffix(");")?;
        if dynamic {
            return is_string_literal(value)
                .then_some(Instruction::DynamicStyle(style, property, value));
        }
        let mut operands = Vec::new();
        let expression = closed_value(value, &mut operands)?;
        return Some(Instruction::StyleLiteral(
            style, property, expression, operands,
        ));
    }
    if let Some(args) = line.strip_prefix("rt::bind_dynamic_resource(&context, ") {
        let (name, rest) = args.split_once(".upcast_ref::<::ferroui_base::FerroObject>(), ")?;
        let (property, rest) = rest.split_once(", ")?;
        let (rest, column) = rest.strip_suffix(")?;")?.rsplit_once(", ")?;
        let (literal, line) = rest.rsplit_once(", ")?;
        if !is_identifier(name)
            || !absolute_path(property.strip_suffix("()")?)
            || !is_string_literal(literal)
        {
            return None;
        }
        return Some(Instruction::Dynamic(
            name,
            property,
            literal,
            line.parse().ok()?,
            column.parse().ok()?,
        ));
    }
    if let Some(binding) = theme_styles::template_binding(line) {
        return Some(Instruction::Bind(binding, false));
    }
    if let Some(binding) = theme_styles::template_binding_from(line) {
        return Some(Instruction::Bind(binding, true));
    }
    if let Some(args) = line.strip_prefix("rt::register_name_value(name_scope.as_ref(), ") {
        let (literal, args) =
            args.split_once(", ::std::rc::Rc::new(::core::clone::Clone::clone(&")?;
        let (name, pos) = args.split_once(")), ")?;
        let (line, column) = pos.strip_suffix(")?;")?.split_once(", ")?;
        if !is_string_literal(literal) {
            return None;
        }
        return Some(Instruction::Name(
            name,
            class(types, name)?,
            literal,
            line.parse().ok()?,
            column.parse().ok()?,
        ));
    }
    if let Some(binding) = line.strip_prefix("let ") {
        let (name, value) = binding.split_once(" = ")?;
        let expr = value.strip_suffix(';')?;
        let setter = expr.strip_prefix("rt::new_setter(").map(|args| (args, false))
            .or_else(|| expr.strip_prefix("rt::new_setter_with_parent(&context, ").map(|args| (args, true)));
        if let Some((args, parented)) = setter {
            let (property, rest) = theme_styles::split_property(args)?;
            return (is_identifier(name) && rest == ")")
                .then_some(Instruction::BeginSetter { name, property, parented });
        }
        if let Some(path) = expr.strip_prefix("::ferroui_markup_xaml::markup_extensions::CompiledBindingExtension::__markup_new_1(")
            .and_then(|s| s.strip_suffix(".clone())")).filter(|s| is_identifier(s)) {
            return Some(Instruction::NewBinding(name, path));
        }
        if let Some((ty, operands)) = expr
            .strip_prefix("rt::defer(")
            .and_then(|s| s.strip_suffix(")?"))
            .and_then(|s| s.split_once(", &context, "))
        {
            let mut fields = operands.split(", ");
            let factory = fields.next().filter(|s| is_identifier(s))?;
            let line = fields.next()?.parse().ok()?;
            let column = fields.next()?.parse().ok()?;
            if fields.next().is_none() {
                return Some(Instruction::Defer(name, ty, factory, line, column));
            }
        }
        if let Some(call) = calls.get(expr).and_then(Option::as_ref) {
            if is_identifier(name) && call.result.is_some() {
                return Some(Instruction::Call(call, Some(name)));
            }
        }
        let class = class(types, name)?;
        let constructor = expr.strip_prefix(class)?;
        let trimmed = constructor
            .strip_prefix("::with_parts(::core::option::Option::None, ")
            .and_then(|s| s.strip_suffix(')'))
            .is_some_and(absolute_path);
        return (absolute_path(class) && (constructor == "::new()" || trimmed))
            .then_some(Instruction::New(name, class, expr));
    }
    if let Some(name) = line.strip_suffix(".begin_init();") {
        class(types, name)?;
        return Some(Instruction::Begin(name));
    }
    if let Some(value) = line.strip_prefix("rt::invoked(") {
        let (name, pos) = value.split_once(".try_end_init(), ")?;
        class(types, name)?;
        let (line, column) = pos.strip_suffix(")?;")?.split_once(", ")?;
        return Some(Instruction::End(
            name,
            line.parse().ok()?,
            column.parse().ok()?,
        ));
    }
    let (target, call) = line.strip_suffix(");")?.split_once('.')?;
    class(types, target)?;
    let (method, args) = call.split_once('(')?;
    if !matches!(
        method,
        "set_value" | "set_direct_value" | "set_value_with_priority"
    ) {
        return None;
    }
    let (property, expr) = args.split_once(", ")?;
    if !absolute_path(property.strip_suffix("()")?) {
        return None;
    }
    let (expr, priority) = if method == "set_value_with_priority" {
        let (expr, priority) = expr.rsplit_once(", ")?;
        if !priority.starts_with("::ferroui_base::data::BindingPriority::")
            || !absolute_path(priority)
        {
            return None;
        }
        (expr, Some(priority))
    } else {
        (expr, None)
    };
    let expr = match expr {
        "::core::clone::Clone::clone(&true)" => "true",
        "::core::clone::Clone::clone(&false)" => "false",
        expr => expr,
    };
    let value = if let Some(n) = expr.strip_suffix("_i32") {
        Value::Int(n.parse().ok()?)
    } else if let Some((factory, arguments)) = expr.split_once('(').filter(|(factory, _)| {
        matches!(
            *factory,
            "::ferroui_base::Thickness::__markup_new_3"
                | "::ferroui_base::CornerRadius::__markup_new_3"
        )
    }) {
        let mut parts = arguments.strip_suffix(')')?.split(", ");
        let mut values = [0.; 4];
        for value in &mut values {
            *value = parts.next()?.strip_suffix("_f64")?.parse().ok()?;
        }
        if parts.next().is_some() {
            return None;
        }
        Value::Four(factory, values)
    } else if let Some(n) = expr.strip_suffix("_f64") {
        Value::Float(n.parse().ok()?)
    } else if let Some(literal) = expr
        .strip_prefix("rt::string_object(")
        .and_then(|s| s.strip_suffix(')'))
        .filter(|s| is_string_literal(s))
    {
        Value::String(literal, "rt::string_object(value)")
    } else if let Some(literal) = string_literal_of(expr) {
        Value::String(literal, "value.to_owned()")
    } else if let Some(literal) = expr
        .strip_prefix("::core::option::Option::Some(")
        .and_then(|s| s.strip_suffix(')'))
        .and_then(string_literal_of)
    {
        Value::String(literal, "Some(value.to_owned())")
    } else if let Some(name) = expr
        .strip_prefix("rt::to_object(::core::clone::Clone::clone(&")
        .and_then(|s| s.strip_suffix("))"))
    {
        Value::Object(name, class(types, name)?)
    } else if let Some(upcast) = expr
        .strip_prefix("::core::option::Option::Some(::core::clone::Clone::clone(&")
        .and_then(|s| s.strip_suffix(">())"))
    {
        let (name, target) = upcast.split_once(").upcast::<")?;
        if !is_identifier(name) || !absolute_path(target) {
            return None;
        }
        Value::Child(name, target)
    } else if matches!(expr, "true" | "false") || absolute_path(expr) {
        Value::Constant(expr)
    } else {
        let mut operands = Vec::new();
        let expression = closed_value(expr, &mut operands)?;
        let priority = priority.map_or_else(String::new, |p| format!(", {p}"));
        let param = if operands.is_empty() { "_" } else { "args" };
        let adapter = format!("LiteralSetter(|target, {param}| {{ target.{method}({property}, {expression}{priority}); Ok(()) }})");
        return Some(Instruction::PropertyLiteral(target, adapter, operands));
    };
    Some(Instruction::Set(target, method, property, priority, value))
}

use ferroui_markup::compiled_operands::{number, unsigned as uint};

#[derive(Default)]
struct Imports {
    operations: Vec<String>,
    strings: Vec<String>,
}

impl Imports {
    fn encode(code: &mut Vec<u8>, pool: &mut Vec<String>, value: String) {
        let index = pool
            .iter()
            .position(|entry| *entry == value)
            .unwrap_or_else(|| {
                pool.push(value);
                pool.len() - 1
            });
        uint(
            code,
            u32::try_from(index).expect("XAML construction import"),
        );
    }
}

#[derive(Default)]
struct Program<'a> {
    code: Vec<u8>,
    uses_imports: bool,
    inputs: Vec<(&'a str, bool)>,
    locals: Vec<(&'a str, &'a str, bool)>,
    uses_scope: bool,
    uses_context: bool,
}

impl<'a> Program<'a> {
    fn drop_namespace(&mut self, line: &str) -> bool {
        let Some(name) = line.strip_prefix("    ::core::mem::drop(").and_then(|s| s.strip_suffix(");")) else { return false };
        let Some(slot) = self.locals.iter().position(|&(local, ty, _)| local == name && ty == NAMESPACE_SCOPE) else { return false };
        self.code.push(op::DROP_NAMESPACE);
        uint(&mut self.code, u32::try_from(slot).expect("XAML namespace register"));
        true
    }

    /// A dynamic setter consumes its typed result when it converts it to MarkupValue.
    /// Transfer only registers produced in this region at their final use. A borrowed
    /// external input keeps its native conversion and original ownership.
    fn move_to_markup(&mut self, line: &'a str, last_use: &HashMap<String, usize>, index: usize, imports: &mut Imports) -> bool {
        let Some((name, source)) = line.strip_prefix("    let ")
            .and_then(|s| s.strip_suffix(");"))
            .and_then(|s| s.split_once(": ::ferroui_base::metadata::MarkupValue = rt::to_value("))
        else { return false };
        if !is_identifier(name) || last_use.get(source) != Some(&index) {
            return false;
        }
        let Some((slot, &(_, ty, false))) = self.locals.iter().enumerate().find(|(_, local)| local.0 == source)
        else { return false };
        self.code.push(op::MOVE_TO_MARKUP);
        uint(&mut self.code, u32::try_from(slot).expect("XAML value register"));
        self.import(imports, format!("MoveToMarkup(|register| Ok(rt::to_value(register.into_value::<{ty}>()?)))"));
        self.locals.push((name, "::ferroui_base::metadata::MarkupValue", false));
        true
    }

    fn register(&mut self, name: &'a str) {
        self.operand(name, true);
    }

    fn operand(&mut self, name: &'a str, object: bool) {
        let index = match self.locals.iter().position(|(local, _, _)| *local == name) {
            Some(i) => 2 * i + 1,
            None => {
                let i = self
                    .inputs
                    .iter()
                    .position(|(s, _)| *s == name)
                    .unwrap_or_else(|| {
                        self.inputs.push((name, object));
                        self.inputs.len() - 1
                    });
                2 * i
            }
        };
        uint(
            &mut self.code,
            u32::try_from(index).expect("XAML construction register"),
        );
    }

    fn import(&mut self, imports: &mut Imports, expression: String) {
        self.uses_imports = true;
        Imports::encode(&mut self.code, &mut imports.operations, expression);
    }

    fn text(&mut self, imports: &mut Imports, literal: &str) {
        self.uses_imports = true;
        Imports::encode(&mut self.code, &mut imports.strings, literal.to_owned());
    }

    fn scalar(&mut self, imports: &mut Imports, scalar: &CallInput) {
        match scalar {
            CallInput::DocumentBaseUri => self.uses_context = true,
            CallInput::Text(literal) => self.text(imports, literal),
            CallInput::Double(bits) => number(&mut self.code, f64::from_bits(*bits)),
            CallInput::Uint(value) => self.code.extend_from_slice(&value.to_le_bytes()),
            CallInput::Int(value) => {
                ferroui_markup::compiled_operands::signed(&mut self.code, *value)
            }
            CallInput::Bool(value) => self.code.push((*value).into()),
            CallInput::Register(..) => unreachable!("closed XAML value contains a register"),
        }
    }

    fn finish_operands(&mut self, start: usize) {
        let encoded = self.code.split_off(start);
        uint(
            &mut self.code,
            u32::try_from(encoded.len()).expect("XAML call operands"),
        );
        self.code.extend(encoded);
    }

    fn literal_operands(&mut self, imports: &mut Imports, operands: &[CallInput]) {
        let start = self.code.len();
        for operand in operands {
            self.scalar(imports, operand);
        }
        self.finish_operands(start);
    }

    fn add(&mut self, instruction: Instruction<'a>, imports: &mut Imports) {
        match instruction {
            Instruction::Namespace(name, table) => {
                self.uses_context = true;
                self.code.push(op::PUSH_NAMESPACE);
                self.import(imports, format!("Namespaces(|| {table})"));
                self.locals.push((name, NAMESPACE_SCOPE, false));
            }
            Instruction::ApplyThemeUnit(dictionary, unit, run) => {
                self.uses_context = true;
                self.code.push(op::APPLY_THEME_UNIT);
                self.register(dictionary);
                self.import(imports, "ThemeParts(active_theme_units)".to_string());
                uint(&mut self.code, unit);
                uint(&mut self.code, run);
            }
            Instruction::ClearTargetProperty => {
                self.uses_context = true;
                self.code.push(op::CLEAR_TARGET_PROPERTY);
            }
            Instruction::Complete(root, line, column) => {
                self.uses_scope = true;
                if let Some(name) = root {
                    self.code.push(op::COMPLETE_SCOPE);
                    self.register(name);
                } else {
                    self.code.push(op::COMPLETE_PLAIN_SCOPE);
                }
                uint(&mut self.code, line as u32);
                uint(&mut self.code, column as u32);
            }
            Instruction::Call(call, result) => {
                let opcode = match call.kind {
                    CallKind::ObjectLiteral(ref data) => {
                        self.code.push(op::OBJECT_LITERAL);
                        self.import(imports, call.import.clone());
                        uint(&mut self.code, u32::try_from(data.len()).expect("XAML literal length"));
                        self.code.extend_from_slice(data);
                        let (ty, object) = call.result.as_ref().expect("resolved object literal type");
                        self.locals.push((result.expect("object literal register"), ty, *object));
                        return;
                    }
                    CallKind::RegisterSetter => {
                        let [CallInput::Register(target, true), CallInput::Register(source, object)] = &call.inputs[..] else {
                            unreachable!("a resolved register setter has a target and a source")
                        };
                        self.code.push(op::SET_REGISTER);
                        self.register(target);
                        self.import(imports, call.import.clone());
                        self.operand(source, *object);
                        uint(&mut self.code, call.line as u32);
                        uint(&mut self.code, call.column as u32);
                        return;
                    }
                    CallKind::TargetProperty => {
                        self.uses_context = true;
                        self.code.push(op::SET_TARGET_PROPERTY);
                        self.import(imports, call.import.clone());
                        return;
                    }
                    CallKind::Selector => op::BUILD_SELECTOR,
                    CallKind::Path { named } => {
                        self.uses_scope |= named;
                        op::BUILD_PATH
                    }
                    CallKind::Member | CallKind::ContextMember => {
                        self.uses_context |= matches!(call.kind, CallKind::ContextMember);
                        if result.is_some() {
                            op::CALL_RESULT
                        } else {
                            op::CALL
                        }
                    }
                };
                self.code.push(opcode);
                self.import(imports, call.import.clone());
                if opcode == op::BUILD_SELECTOR {
                    // There are no runtime operands; the selector program owns its typed imports.
                    let (ty, object) = call.result.as_ref().expect("a selector result");
                    self.locals
                        .push((result.expect("a selector register"), ty, *object));
                    return;
                }
                uint(&mut self.code, call.line as u32);
                uint(&mut self.code, call.column as u32);
                let start = self.code.len();
                for input in &call.inputs {
                    match input {
                        CallInput::Register(name, object) => self.operand(name, *object),
                        scalar => self.scalar(imports, scalar),
                    }
                }
                self.finish_operands(start);
                if let Some(name) = result {
                    let (ty, object) = call.result.as_ref().unwrap();
                    self.locals.push((name, ty, *object));
                }
            }
            Instruction::StyleLiteral(style, property, expression, operands) => {
                self.code.push(op::ADD_STYLE_LITERAL);
                self.register(style);
                self.import(imports, format!("Property(|| {property})"));
                let param = if operands.is_empty() { "_" } else { "args" };
                self.import(imports, format!("LiteralValue(|{param}| Ok({expression}))"));
                self.literal_operands(imports, &operands);
            }
            Instruction::BeginSetter { name, property, parented } => {
                self.uses_context |= parented;
                self.code.push(if parented { op::BEGIN_PARENTED_SETTER } else { op::BEGIN_SETTER });
                self.import(imports, format!("Property(|| {property})"));
                self.locals.push((name, SETTER, false));
            }
            Instruction::FinishSetter { style, setter, source, object, adapter, parented } => {
                self.uses_context |= parented;
                self.code.push(if parented { op::FINISH_PARENTED_SETTER } else { op::FINISH_SETTER });
                self.register(style);
                self.operand(setter, false);
                self.import(imports, adapter);
                self.operand(source, object);
            }
            Instruction::DynamicStyle(style, property, key) => {
                self.uses_context = true;
                self.code.push(op::ADD_STYLE_DYNAMIC);
                self.register(style);
                self.import(imports, format!("Property(|| {property})"));
                self.text(imports, key);
            }
            Instruction::PropertyLiteral(target, adapter, operands) => {
                self.code.push(op::SET_PROPERTY_LITERAL);
                self.register(target);
                self.import(imports, adapter);
                self.literal_operands(imports, &operands);
            }
            Instruction::StaticResource(site) => {
                self.uses_context = true;
                self.code.push(op::STATIC_RESOURCE);
                self.register(site.target);
                self.text(imports, site.key);
                self.import(
                    imports,
                    format!("Property(|| rt::property({}))", site.property),
                );
                let priority = if site.template {
                    "Some(::ferroui_base::data::BindingPriority::Template)"
                } else {
                    "None"
                };
                self.import(imports, format!("ResourceSetter(|target, value, line, position| rt::assign_resource_value(target, {}, value, &rt::ResourceChoice {{ {}, line, position }}, {priority}))", site.property, site.choice));
                uint(&mut self.code, site.position.0 as u32);
                uint(&mut self.code, site.position.1 as u32);
                self.locals.push((site.extension, "::std::rc::Rc<::ferroui_markup_xaml::markup_extensions::StaticResourceExtension>", false));
            }
            Instruction::NewBinding(name, path) => {
                self.code.push(op::NEW_BINDING);
                self.operand(path, false);
                self.locals.push((name, "::std::rc::Rc<::ferroui_markup_xaml::markup_extensions::CompiledBindingExtension>", false));
            }
            Instruction::ProvideBinding(target, extension, property, line, column) => {
                self.uses_context = true;
                self.code.push(op::PROVIDE_BINDING);
                self.register(target);
                self.operand(extension, false);
                self.import(imports, format!("Property(|| {property})"));
                uint(&mut self.code, line as u32);
                uint(&mut self.code, column as u32);
            }
            Instruction::Defer(name, ty, factory, line, column) => {
                self.uses_context = true;
                self.code.push(op::DEFER);
                self.import(imports, format!("DeferredType(|| {ty})"));
                self.import(imports, format!("Deferred({factory})"));
                uint(&mut self.code, line as u32);
                uint(&mut self.code, column as u32);
                self.locals.push((
                    name,
                    "::std::rc::Rc<::ferroui_markup_xaml::xaml_il::runtime::DeferredContent>",
                    false,
                ));
            }
            Instruction::New(name, class, expr) => {
                self.code.push(op::NEW);
                self.import(imports, format!("New(|| {expr}.upcast())"));
                self.locals.push((name, class, true));
            }
            Instruction::Begin(target) => {
                self.code.push(op::BEGIN_INIT);
                self.register(target);
            }
            Instruction::End(target, line, column) => {
                self.code.push(op::END_INIT);
                self.register(target);
                uint(&mut self.code, line as u32);
                uint(&mut self.code, column as u32);
            }
            Instruction::Parent(name, ty, opcode) => {
                self.uses_context = true;
                self.code.push(opcode);
                self.register(name);
                self.import(imports, format!("Markup(|value| Ok(rt::to_value(rt::construction_object::<{ty}>(Some(value.clone()))?)))"));
            }
            Instruction::Pop => {
                self.uses_context = true;
                self.code.push(op::POP_PARENT);
            }
            Instruction::Dynamic(name, property, key, line, column) => {
                self.uses_context = true;
                self.code.push(op::DYNAMIC_RESOURCE);
                self.register(name);
                self.import(imports, format!("Property(|| rt::property({property}))"));
                self.text(imports, key);
                uint(&mut self.code, line as u32);
                uint(&mut self.code, column as u32);
            }
            Instruction::Bind(binding, source_first) => {
                let (opcode, first, second) = if source_first {
                    (op::TEMPLATE_BIND_FROM, binding.source, binding.target)
                } else {
                    (op::TEMPLATE_BIND, binding.target, binding.source)
                };
                self.code.push(opcode);
                self.register(binding.object);
                self.import(imports, format!("Property(|| rt::property({first}))"));
                self.import(imports, format!("Property(|| rt::property({second}))"));
                uint(&mut self.code, binding.position.0 as u32);
                uint(&mut self.code, binding.position.1 as u32);
            }
            Instruction::Name(name, class, literal, line, column) => {
                self.uses_scope = true;
                self.code.push(op::REGISTER_NAME);
                self.register(name);
                self.text(imports, literal);
                self.import(imports, format!("BoxObject(|value| Ok(::std::rc::Rc::new(rt::construction_object::<{class}>(Some(value.clone()))?)))"));
                uint(&mut self.code, line as u32);
                uint(&mut self.code, column as u32);
            }
            Instruction::Set(target, method, property, priority, value) => {
                let priority =
                    priority.map_or_else(String::new, |priority| format!(", {priority}"));
                let opcode = match &value {
                    Value::Int(_) => op::SET_INT,
                    Value::Four(_, _) => op::SET_FOUR,
                    Value::Float(_) => op::SET_F64,
                    Value::String(_, _) => op::SET_STRING,
                    Value::Constant(_) => op::SET_CONSTANT,
                    Value::Object(_, _) | Value::Child(_, _) => op::SET_OBJECT,
                };
                self.code.push(opcode);
                self.register(target);
                match value {
                    Value::Int(n) => {
                        self.import(imports, format!(
                            "Int(|target, value| {{ target.{method}({property}, value{priority}); }})"
                        ));
                        ferroui_markup::compiled_operands::signed(&mut self.code, n);
                    }
                    Value::Four(factory, ns) => {
                        self.import(imports, format!("Four(|target, [a, b, c, d]| {{ target.{method}({property}, {factory}(a, b, c, d){priority}); }})"));
                        for n in ns {
                            number(&mut self.code, n);
                        }
                    }
                    Value::Float(n) => {
                        self.import(imports, format!(
                            "Float(|target, value| {{ target.{method}({property}, value{priority}); }})"
                        ));
                        number(&mut self.code, n);
                    }
                    Value::String(literal, value) => {
                        debug_assert!(is_string_literal(literal));
                        self.import(imports, format!(
                            "String(|target, value| {{ target.{method}({property}, {value}{priority}); }})"
                        ));
                        self.text(imports, literal);
                    }
                    Value::Constant(value) => self.import(
                        imports,
                        format!(
                        "Constant(|target| {{ target.{method}({property}, {value}{priority}); }})"
                    ),
                    ),
                    Value::Child(source, base) => {
                        self.import(imports, format!("Object(|target, value| {{ let value = rt::construction_object::<{base}>(Some(value.clone()))?; target.{method}({property}, Some(value){priority}); Ok(()) }})"));
                        self.register(source);
                    }
                    Value::Object(source, class) => {
                        self.import(imports, format!("Object(|target, value| {{ let value = rt::construction_object::<{class}>(Some(value.clone()))?; target.{method}({property}, rt::to_object(value){priority}); Ok(()) }})"));
                        self.register(source);
                    }
                }
            }
        }
    }

    fn write(
        self,
        output: &mut Vec<String>,
        image: &super::BytecodeImage,
        live: impl Fn(&str) -> bool,
    ) {
        if self.code.is_empty() {
            return;
        }
        let bytes = image.program(&self.code);
        let imports = if self.uses_imports {
            "&CONSTRUCTION_IMPORTS"
        } else {
            "&rt::ConstructionImports::EMPTY"
        };
        let inputs = self
            .inputs
            .iter()
            .map(|(s, object)| if *object {
                format!("rt::ConstructionInput::Object({s}.upcast_ref::<::ferroui_base::FerroObject>())")
            } else {
                format!("rt::ConstructionInput::Value(&{s})")
            })
            .collect::<Vec<_>>()
            .join(", ");
        let scope = if self.uses_scope {
            "name_scope.as_ref()"
        } else {
            "None"
        };
        let context = if self.uses_context {
            "Some(&context)"
        } else {
            "None"
        };
        let program = format!("rt::ConstructionProgram {{ code: {bytes}, imports: {imports} }}");
        let args = format!("&[{inputs}], {scope}, {context}");
        if self.locals.is_empty() {
            output.push(format!("    {program}.apply({args})?;"));
        } else if self.locals.iter().all(|(name, _, _)| !live(name)) {
            output.push(format!(
                "    let _construction_objects = {program}.hold({args})?;"
            ));
        } else {
            let mut names = Vec::new();
            let mut values = Vec::new();
            let mut locals = self.locals.into_iter().peekable();
            while let Some((name, ty, object)) = locals.next() {
                if live(name) {
                    names.push(name.to_string());
                    let extract = if object { "next_object" } else { "next_value" };
                    values.push(format!("created.{extract}::<{ty}>()?"));
                } else {
                    let mut count = 1;
                    while locals.next_if(|(name, _, _)| !live(name)).is_some() {
                        count += 1;
                    }
                    names.push(format!("_held_{name}"));
                    values.push(format!("created.hold({count})?"));
                }
            }
            let names = names.join(", ");
            let values = values.join(", ");
            output.push(format!(
                "    let mut construction_results = {program}.results({args})?;"
            ));
            output.push(format!("    let ({names},) = {{ let mut created = construction_results.reader(); ({values},) }};"));
        }
    }
}

/// The native setter first binds its literal priority so a conditional assignment can share
/// it. Inline a single adjacent use at the last argument of a setter. Complex branches keep
/// their original binding. Only the generated, side-effect-free enum literal is movable.
fn inline_priorities(lines: &[String]) -> Vec<String> {
    let mut output = Vec::with_capacity(lines.len());
    let mut iter = lines.iter().peekable();
    while let Some(line) = iter.next() {
        if let Some((name, priority)) = line
            .strip_prefix("    let ")
            .and_then(|s| s.strip_suffix(';'))
            .and_then(|s| s.split_once(" = "))
        {
            if name.starts_with("priority_")
                && priority == "::ferroui_base::data::BindingPriority::Template"
            {
                if let Some(setter) = iter
                    .peek()
                    .filter(|s| s.contains(".set_value_with_priority("))
                {
                    if let Some(prefix) = setter.strip_suffix(&format!(", {name});")) {
                        // A later use can still refer to this declaration.
                        output.push(line.clone());
                        output.push(format!("{prefix}, {priority});"));
                        iter.next();
                        continue;
                    }
                }
            }
        }
        output.push(line.clone());
    }
    output
}

struct StaticResource<'a> {
    target: &'a str,
    key: &'a str,
    extension: &'a str,
    property: &'a str,
    choice: &'a str,
    position: (i32, i32),
    template: bool,
}

/// Join the resolved provide-value call and its registered-property assignment. Only the
/// literal-key constructor and the compiler's single setter choice at the same position
/// qualify. Check that the temporary locals do not escape this sequence.
fn static_resource<'a>(
    lines: &'a [String],
    start: usize,
    uses: &HashMap<String, usize>,
) -> Option<(StaticResource<'a>, usize)> {
    const EXTENSION: &str = "::ferroui_markup_xaml::markup_extensions::StaticResourceExtension";
    let priority = lines
        .get(start)?
        .strip_prefix("    let ")
        .and_then(|s| s.strip_suffix(" = ::ferroui_base::data::BindingPriority::Template;"))
        .filter(|s| is_identifier(s));
    let first = start + usize::from(priority.is_some());
    let [create, provide, convert, assign] = lines.get(first..first + 4)? else {
        return None;
    };
    let (extension, key) =
        super::keyed_resource_extension(EXTENSION, create.strip_prefix("    ")?)?;
    let (provided, rest) = provide
        .strip_prefix("    let ")?
        .split_once(" = rt::provide_value_invoked(&context, rt::property_value(")?;
    let (property, coords) = rest.split_once(&format!(
        "), &{extension}, {EXTENSION}::__markup_ProvideValue_0, "
    ))?;
    if !absolute_path(property.strip_suffix("()")?) || !is_identifier(provided) {
        return None;
    }
    let (line, col) = coords.strip_suffix(")?;")?.split_once(", ")?;
    let position = (line.parse().ok()?, col.parse().ok()?);
    let value = convert.strip_prefix("    let ")?.strip_suffix(&format!(
        ": ::ferroui_base::metadata::MarkupValue = rt::to_value({provided});"
    ))?;
    let (target, choice) = assign.strip_prefix("    rt::assign_resource_value(")?
        .split_once(&format!(".upcast_ref::<::ferroui_base::FerroObject>(), {property}, {value}, &rt::ResourceChoice {{ "))?;
    let priority_expr = match priority {
        Some(name) => format!("::core::option::Option::Some({name})"),
        None => "::core::option::Option::None".to_owned(),
    };
    let choice = choice.strip_suffix(&format!(
        ", line: {line}, position: {col} }}, {priority_expr})?;"
    ))?;
    if !is_identifier(value)
        || !is_identifier(target)
        || uses.get(extension) != Some(&(first + 1))
        || uses.get(provided) != Some(&(first + 2))
        || uses.get(value) != Some(&(first + 3))
        || priority.is_some_and(|name| uses.get(name) != Some(&(first + 3)))
    {
        return None;
    }
    Some((
        StaticResource {
            target,
            key,
            extension,
            property,
            choice,
            position,
            template: priority.is_some(),
        },
        first + 4,
    ))
}

/// The emitted ProvideValueTarget scope and registered-property binding form one
/// operation when the provided value is consumed immediately. Preserve both property
/// accessor calls, the scope reset, and the position of the original Bind.
fn provided_binding(lines: &[String]) -> Option<(Instruction<'_>, &str)> {
    let [set, provide, clear, bind] = lines.get(..4)? else {
        return None;
    };
    let property = set
        .strip_prefix("    context.set_target_property(rt::property_value(")?
        .strip_suffix("));")?;
    if !absolute_path(property.strip_suffix("()")?)
        || clear != "    context.set_target_property(::core::option::Option::None);"
    {
        return None;
    }
    let (provided, extension) = provide.strip_prefix("    let ")?
        .split_once(" = ::ferroui_markup_xaml::markup_extensions::CompiledBindingExtension::__markup_ProvideValue_0(&")?;
    let extension = extension
        .strip_suffix(", ::core::option::Option::Some(rt::service_provider(&context)));")?;
    let (target, arguments) = bind
        .strip_prefix("    rt::bind(")?
        .split_once(".upcast_ref::<::ferroui_base::FerroObject>(), ")?;
    let mut arguments = arguments.strip_suffix(")?;")?.split(", ");
    if ![provided, extension, target].into_iter().all(is_identifier)
        || arguments.next()? != property
        || arguments.next()? != provided
    {
        return None;
    }
    let line = arguments.next()?.parse().ok()?;
    let column = arguments.next()?.parse().ok()?;
    arguments.next().is_none().then_some((
        Instruction::ProvideBinding(target, extension, property, line, column),
        provided,
    ))
}

pub(super) fn lower(
    lines: &[String],
    types: &Types,
    calls: &Calls,
    returned: &str,
    image: &super::BytecodeImage,
) -> Vec<String> {
    lower_body(lines, types, calls, returned, image, None).expect("mixed construction body")
}

/// A class document whose population is entirely representable as one program.
pub(super) fn document(
    lines: &[String],
    types: &Types,
    calls: &Calls,
    image: &super::BytecodeImage,
    document: &str,
) -> Option<Vec<String>> {
    let [root, context, scope, body @ .., result] = lines else {
        return None;
    };
    if root != "    let root = root.clone();"
        || context != &format!("    let context = rt::populate_context(service_provider, &{document}, rt::to_value(root.clone()));")
        || scope != "    let name_scope = context.name_scope_field();"
        || result != "    ::core::result::Result::Ok(())"
    {
        return None;
    }
    lower_body(
        body,
        types,
        calls,
        "",
        image,
        Some((document, class(types, "root")?)),
    )
}

fn lower_body(
    lines: &[String],
    types: &Types,
    calls: &Calls,
    returned: &str,
    image: &super::BytecodeImage,
    document: Option<(&str, &str)>,
) -> Option<Vec<String>> {
    let lines = inline_priorities(lines);
    let mut last_use = HashMap::new();
    for (i, line) in lines
        .iter()
        .map(String::as_str)
        .chain(std::iter::once(returned))
        .enumerate()
    {
        if !line.trim_start().starts_with("//") {
            for name in super::identifiers(line) {
                last_use.insert(name, i);
            }
        }
    }
    let mut output = Vec::new();
    let mut imports = Imports::default();
    let mut program = Program::default();
    let mut skip = 0;
    for (i, line) in lines.iter().enumerate() {
        if i < skip {
            continue;
        }
        if let Some((site, next)) = static_resource(&lines, i, &last_use) {
            program.add(Instruction::StaticResource(site), &mut imports);
            skip = next;
            continue;
        }
        if let Some((instruction, provided)) = provided_binding(&lines[i..]) {
            if last_use.get(provided) == Some(&(i + 3)) {
                program.add(instruction, &mut imports);
                skip = i + 4;
                continue;
            }
        }
        if program.move_to_markup(line, &last_use, i, &mut imports) || program.drop_namespace(line) {
            continue;
        }
        if line
            .strip_prefix("    let ")
            .and_then(|s| s.strip_suffix(" = ::ferroui_base::data::BindingPriority::Template;"))
            .is_some_and(|name| last_use.get(name) == Some(&i))
        {
            continue;
        }
        if line.trim_start().starts_with("//") {
            output.push(line.clone());
        } else if let Some(instruction) = parse(line, types, calls, &program.locals) {
            program.add(instruction, &mut imports);
        } else {
            if document.is_some() {
                return None;
            }
            std::mem::take(&mut program).write(&mut output, image, |name| {
                last_use.get(name).is_some_and(|last| *last >= i)
            });
            output.push(line.clone());
        }
    }
    if let Some((document, class)) = document {
        if program.code.is_empty()
            || program
                .inputs
                .iter()
                .any(|&(name, object)| name != "root" || !object)
        {
            return None;
        }
        let code = image.program(&program.code);
        let imports = if program.uses_imports {
            "&CONSTRUCTION_IMPORTS"
        } else {
            "&rt::ConstructionImports::EMPTY"
        };
        output.push(format!("    static PROGRAM: rt::ObjectDocumentProgram = rt::ObjectDocumentProgram {{ document: &{document}, construction: rt::ConstructionProgram {{ code: {code}, imports: {imports} }}, root_value: |root| Ok(rt::to_value(rt::construction_object::<{class}>(Some(root.clone()))?)) }};"));
        output.push("    PROGRAM.populate(service_provider, root.upcast_ref::<::ferroui_base::FerroObject>())".to_owned());
    } else {
        program.write(&mut output, image, |name| {
            last_use.get(name) == Some(&lines.len())
        });
    }
    if !imports.operations.is_empty() || !imports.strings.is_empty() {
        let strings = imports.strings.join(", ");
        let operations = imports
            .operations
            .iter()
            .map(|s| format!("rt::ConstructionImport::{s}"))
            .collect::<Vec<_>>()
            .join(", ");
        // Sharing ends at the function boundary so references from another factory
        // cannot retain an unused template, style, or theme unit.
        output.insert(
            0,
            format!("    const CONSTRUCTION_IMPORTS: rt::ConstructionImports = rt::ConstructionImports {{ operations: &[{operations}], strings: &[{strings}] }};"),
        );
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_setter_value_keeps_the_native_call_and_its_created_setter() {
        let types = Types::from([("style".into(), "::ferroui_base::Ref<::ferroui_base::styling::Style>".into())]);
        let lines = [
            "    let setter = rt::new_setter_with_parent(&context, rt::property(::ferroui_base::Visual::is_visible_property()));",
            "    let unknown = compute(&context)?;",
            "    rt::add_setter_value_with_parent(&context, style.upcast_ref::<::ferroui_base::styling::StyleBase>(), &setter, rt::to_object(::core::clone::Clone::clone(&unknown)));",
        ].map(String::from);
        let lowered = lower(&lines, &types, &Calls::new(), "", &super::super::BytecodeImage::default());
        assert_eq!(&lowered[lowered.len() - 2..], &lines[1..]);
        assert!(lowered.iter().any(|line| line.contains("created.next_value::<::std::rc::Rc<::ferroui_base::styling::Setter>>()?")));
    }
}
