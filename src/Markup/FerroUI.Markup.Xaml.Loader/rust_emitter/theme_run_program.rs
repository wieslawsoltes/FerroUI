//! Lower a complete extracted run. Only exact resolved emitter forms are accepted. An
//! unsupported operation leaves that run intact, including its locals and evaluation order.
use super::super::{is_identifier, theme_rows::split_string_literal};
use super::Schedule;
use ferroui_markup::{compiled_operands as wire, theme_run_opcodes as op};

pub(super) fn compile(
    lines: &[String],
    dictionary: &str,
    schedule: &Schedule<'_>,
) -> Option<String> {
    let mut statements = lines
        .iter()
        .map(|line| line.trim_start())
        .filter(|s| !s.starts_with("//"));
    let mut run = Encoder {
        code: vec![op::VERSION],
        imports: vec![],
    };
    while let Some(line) = statements.next() {
        if let Some((name, args)) = line.split_once('.').filter(|(name, _)| is_identifier(name)) {
            run.rows(name, args, dictionary, schedule)?;
        } else if let Some(args) =
            line.strip_prefix(&format!("rt::add_deferred_resource(&{dictionary}, "))
        {
            run.defer_text(args)?;
        } else if let Some((local, rhs)) =
            line.strip_prefix("let ").and_then(|s| s.split_once(" = "))
        {
            if !is_identifier(local) {
                return None;
            }
            run.local(local, rhs, statements.next()?, dictionary)?;
        } else {
            run.immediate(line, dictionary)?;
        }
    }
    let bytes = schedule.image.program(&run.code);
    let document = schedule.document;
    let imports = run
        .imports
        .iter()
        .map(|item| format!("rt::ThemeRunImport::{item}"))
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!(
        "    static IMPORTS: &[rt::ThemeRunImport] = &[{imports}];\n    rt::apply_theme_run({bytes}, IMPORTS, &{document}, context, dictionary)\n"
    ))
}

fn absolute_path(text: &str) -> bool {
    text.strip_prefix("::")
        .is_some_and(|path| path.split("::").all(is_identifier))
}

fn value_add<'a>(line: &'a str, dictionary: &str) -> Option<(&'a str, &'a str, &'a str)> {
    let args = line.strip_prefix(&format!("rt::invoked(<dyn ::ferroui_base::controls::IResourceDictionary>::__markup_Add_0(&rt::cast({dictionary}.clone(), "))?;
    let (cast, args) = args.split_once(")?, rt::string_object(")?;
    let (key, args) = split_string_literal(args)?;
    let (value, position) = args
        .strip_prefix("), rt::to_object(")?
        .strip_suffix(")?;")?
        .rsplit_once(")), ")?;
    (position == cast).then_some((key, value, position))
}

struct Encoder {
    code: Vec<u8>,
    imports: Vec<String>,
}

impl Encoder {
    fn rows(
        &mut self,
        name: &str,
        args: &str,
        dictionary: &str,
        schedule: &Schedule<'_>,
    ) -> Option<()> {
        let (method, arguments) = args.strip_suffix(")?;")?.split_once('(')?;
        let (opcode, expected_method, uses_context) = schedule.rows.get(name)?.program_call();
        let document = schedule.document;
        let suffix = if uses_context {
            format!(", &context, &{document}")
        } else {
            String::new()
        };
        if method != expected_method || arguments != format!("&{dictionary}{suffix}") {
            return None;
        }
        self.code.push(opcode);
        self.import(format!("Rows(&{name})"));
        Some(())
    }

    fn defer_text(&mut self, args: &str) -> Option<()> {
        let (key, rest) = split_string_literal(args)?;
        let (factory, position) = rest
            .strip_prefix(", ::ferroui_base::data::core::ValueType::object(), &context, ")?
            .strip_suffix(")?;")?
            .split_once(", ")?;
        if !is_identifier(factory) {
            return None;
        }
        self.code.push(op::DEFER_TEXT);
        self.import(format!("Text({key})"));
        self.import(format!("Factory({factory})"));
        self.position(position)?;
        Some(())
    }

    fn local(&mut self, local: &str, rhs: &str, next: &str, dictionary: &str) -> Option<()> {
        if let Some(args) = rhs
            .strip_prefix("rt::defer(::ferroui_base::data::core::ValueType::object(), &context, ")
        {
            let (factory, declaration) = args.strip_suffix(")?;")?.split_once(", ")?;
            let args = next.strip_prefix(&format!("rt::invoked(::ferroui_base::controls::ResourceDictionary::__markup_AddDeferred_1(&{dictionary}, rt::boxed(<"))?;
            let (ty, insert) = args.split_once(&format!(" as ::ferroui_base::StaticType>::TYPE), ::core::clone::Clone::clone(&{local}) as ::std::rc::Rc<dyn ::ferroui_base::controls::IDeferredContent>), "))?;
            if !absolute_path(ty) || !is_identifier(factory) {
                return None;
            }
            self.code.push(op::DEFER_TYPE);
            self.import(format!("Type(<{ty} as ::ferroui_base::StaticType>::TYPE)"));
            self.import(format!("Factory({factory})"));
            self.position(declaration)?;
            self.position(insert.strip_suffix(")?;")?)?;
        } else {
            let (key, val, position) = value_add(next, dictionary)?;
            if val != format!("::core::clone::Clone::clone(&{local})") {
                return None;
            }
            if let Some(args) = rhs.strip_prefix("rt::invoked(") {
                let (parser, rest) = args.split_once("(::std::string::String::from(")?;
                // The emitted local is cloned into the dictionary. Only erase this
                // clone for the built-in value types verified against native output.
                if !matches!(
                    parser,
                    "::ferroui_base::media::FontWeight::__markup_parse"
                        | "::ferroui_base::media::BoxShadows::__markup_parse"
                ) {
                    return None;
                }
                let (text, rest) = split_string_literal(rest)?;
                let source = rest.strip_prefix(")), ")?.strip_suffix(")?;")?;
                self.value(op::PARSE, key, position)?;
                self.import(format!("Parse(|text, line, column| rt::invoked({parser}(::std::string::String::from(text)), line, column).map(rt::to_object))"));
                self.import(format!("Text({text})"));
                self.position(source)?;
            } else {
                let (ctor, ticks) = rhs.strip_suffix("_i64);")?.split_once('(')?;
                if ctor != "::ferroui_base::animation::SignedDuration::__markup_FromTicks_0" {
                    return None;
                }
                let ticks: i64 = ticks.parse().ok()?;
                self.value(op::FROM_I64, key, position)?;
                self.import(format!("FromI64(|ticks| rt::to_object({ctor}(ticks)))"));
                self.code.extend(ticks.to_le_bytes());
            }
        }
        Some(())
    }

    fn immediate(&mut self, line: &str, dictionary: &str) -> Option<()> {
        let (key, value, position) = value_add(line, dictionary)?;
        if let Some(flag) = value
            .strip_prefix("::core::clone::Clone::clone(&")
            .and_then(|s| s.strip_suffix(')'))
        {
            let flag = flag.parse::<bool>().ok()?;
            self.value(op::BOOL, key, position)?;
            self.code.push(flag.into());
        } else if let Some(n) = value.strip_suffix("_i32") {
            let n = n.parse().ok()?;
            self.value(op::INT, key, position)?;
            wire::signed(&mut self.code, n);
        } else if absolute_path(value) {
            self.value(op::CONSTANT, key, position)?;
            self.import(format!("Constant(|| rt::to_object({value}))"));
        } else {
            let (ctor, args) = value.strip_suffix(')')?.split_once('(')?;
            let (amount, unit) = args.split_once(", ")?;
            if !absolute_path(ctor) || !absolute_path(unit) {
                return None;
            }
            let amount = amount.strip_suffix("_f64")?.parse().ok()?;
            self.value(op::FROM_DOUBLE, key, position)?;
            self.import(format!(
                "FromDouble(|value| rt::to_object({ctor}(value, {unit})))"
            ));
            wire::number(&mut self.code, amount);
        }
        Some(())
    }

    fn import(&mut self, item: String) {
        let index = self
            .imports
            .iter()
            .position(|s| s == &item)
            .unwrap_or_else(|| {
                self.imports.push(item);
                self.imports.len() - 1
            });
        wire::unsigned(
            &mut self.code,
            index.try_into().expect("theme run import fits in 32 bits"),
        );
    }

    fn position(&mut self, source: &str) -> Option<()> {
        let (line, column) = source.split_once(", ")?;
        wire::signed(&mut self.code, line.parse().ok()?);
        wire::signed(&mut self.code, column.parse().ok()?);
        Some(())
    }

    fn value(&mut self, opcode: u8, key: &str, position: &str) -> Option<()> {
        self.code.push(opcode);
        self.import(format!("Text({key})"));
        self.position(position)
    }
}
