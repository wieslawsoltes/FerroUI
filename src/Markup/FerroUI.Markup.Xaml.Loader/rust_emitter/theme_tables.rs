//! Lower exactly recognized, typed deferred dictionary additions to data. The normal emitter
//! still handles every other operation. A run ends at any intervening operation or change of
//! dictionary, lexical scope or factory. No global opcode dispatcher can root other handlers.

use ferroui_xaml_binary::{Kind, Resource, Value};
use std::collections::HashMap;
use std::fmt::Write as _;

mod constants;
use constants::Constant;

use super::emitter::DocumentFunctions;
use super::source::rust_string_literal;

#[derive(Debug)]
pub(super) enum Operand {
    Alias(String),
    DynamicBrush(String),
    StaticBrush(String),
    LiteralBrush(u32),
}

impl Operand {
    fn kind(&self) -> Kind {
        match self {
            Self::Alias(_) => Kind::Alias,
            Self::DynamicBrush(_) => Kind::DynamicBrush,
            Self::StaticBrush(_) => Kind::StaticBrush,
            Self::LiteralBrush(_) => Kind::LiteralBrush,
        }
    }
    fn value(&self) -> Value<'_> {
        match self {
            Self::Alias(text) | Self::DynamicBrush(text) | Self::StaticBrush(text) => Value::Text(text),
            Self::LiteralBrush(argb) => Value::Argb(*argb),
        }
    }
}

pub(super) struct Factory {
    pub operand: Operand,
    pub value_line: u32,
    pub value_column: u32,
}

/// Read exactly the literal syntax written by rust_string_literal. Unrecognized syntax
/// simply leaves the independently valid direct-call lowering intact.
fn text_literal(literal: &str) -> Option<String> {
    let inner = literal.strip_prefix('"')?.strip_suffix('"')?;
    let mut text = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        text.push(match c {
            '"' => return None,
            '\\' => match chars.next()? {
                '\\' => '\\',
                '"' => '"',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '0' => '\0',
                'u' => {
                    if chars.next()? != '{' {
                        return None;
                    }
                    let mut scalar = 0u32;
                    let mut digits = 0;
                    loop {
                        let digit = chars.next()?;
                        if digit == '}' {
                            break;
                        }
                        scalar = scalar.checked_mul(16)?.checked_add(digit.to_digit(16)?)?;
                        digits += 1;
                    }
                    if digits == 0 {
                        return None;
                    }
                    char::from_u32(scalar)?
                }
                _ => return None,
            },
            _ => c,
        });
    }
    Some(text)
}

impl Factory {
    pub fn alias(key: &str, line: &str, column: &str) -> Option<Self> {
        Some(Self {
            operand: Operand::Alias(text_literal(key)?),
            value_line: line.parse().ok()?,
            value_column: column.parse().ok()?,
        })
    }

    /// Only the entire known brush body may be folded. Additional properties, source info,
    /// subscriptions, parent-stack operations or constructor changes take the ordinary path.
    pub fn brush(lines: &[String], returned: &str) -> Option<Self> {
        const CREATE: &str = "let solid_color_brush_0 = ::ferroui_base::media::SolidColorBrush::new();";
        const PUSH: &str = "rt::push_parent(&context, &solid_color_brush_0);";
        const SET: &str = "solid_color_brush_0.set_value(::ferroui_base::media::SolidColorBrush::color_property(), ::core::clone::Clone::clone(&value_0));";
        const BIND: &str = "rt::bind_dynamic_resource(&context, solid_color_brush_0.upcast_ref::<::ferroui_base::FerroObject>(), ::ferroui_base::media::SolidColorBrush::color_property(), ";
        if returned != "rt::to_value(solid_color_brush_0.clone())" {
            return None;
        }
        let mut steps = lines.iter().map(|l| l.trim_start()).filter(|l| !l.starts_with("// "));
        if steps.next()? != CREATE {
            return None;
        }
        let value = steps.next()?;
        let factory = if value == PUSH {
            let set = steps.next()?;
            if !set.starts_with(BIND) {
                return Self::static_brush(set, steps);
            }
            let args = set.strip_prefix(BIND)?.strip_suffix(")?;")?;
            let (args, column) = args.rsplit_once(", ")?;
            let (key, line) = args.rsplit_once(", ")?;
            if steps.next()? != "context.pop_parent();" {
                return None;
            }
            Self {
                operand: Operand::DynamicBrush(text_literal(key)?),
                value_line: line.parse().ok()?,
                value_column: column.parse().ok()?,
            }
        } else {
            let argb = value
                .strip_prefix("let value_0 = ::ferroui_base::media::Color::__markup_FromUInt32_2(")?
                .strip_suffix("_u32);")?;
            if steps.next()? != SET {
                return None;
            }
            Self {
                operand: Operand::LiteralBrush(argb.parse().ok()?),
                value_line: 0,
                value_column: 0,
            }
        };
        steps.next().is_none().then_some(factory)
    }
    /// Match the complete setter plan for a static resource. An extension may provide an
    /// unset sentinel, a binding or a Color. All fallible steps must use one location.
    fn static_brush<'a>(create: &str, steps: impl Iterator<Item = &'a str>) -> Option<Self> {
        const PROP: &str = "::ferroui_base::media::SolidColorBrush::color_property()";
        const OBJ: &str = "solid_color_brush_0.upcast_ref::<::ferroui_base::FerroObject>()";
        let arguments = create.strip_prefix("let provided_0 = rt::provide_static_resource(&context, ")?.strip_suffix(")?;")?;
        let (key, location) = arguments.rsplit_once(&format!(", || rt::property_value({PROP}), "))?;
        let (line, column) = location.split_once(", ")?;
        let expected = [
            "let value_0: ::ferroui_base::metadata::MarkupValue = rt::to_value(provided_0);".to_owned(),
            format!("rt::set_local_value({OBJ}, || {PROP}, value_0, || rt::markup_handle(<::ferroui_base::media::Color as ::ferroui_base::metadata::MarkupTyped>::MARKUP), false, \"FerroUI.Media.SolidColorBrush.set_Color\", \"Color\", {location})?;"),
            "context.pop_parent();".to_owned(),
        ];
        if !steps.eq(expected.iter().map(String::as_str)) {
            return None;
        }
        Some(Self {
            operand: Operand::StaticBrush(text_literal(key)?),
            value_line: line.parse().ok()?,
            value_column: column.parse().ok()?,
        })
    }
}

struct Body {
    factory: Factory,
    document: String,
    slot: usize,
}

enum SiteBody {
    Intrinsic(Body),
    Import(String),
    Constant(Constant),
}

struct Site {
    direct: String,
    dictionary: String,
    key: String,
    line: u32,
    column: u32,
    body: SiteBody,
}

impl Site {
    fn same_run(&self, other: &Self) -> bool {
        self.dictionary == other.dictionary && match (&self.body, &other.body) {
            (SiteBody::Intrinsic(a), SiteBody::Intrinsic(b)) => {
                a.document == b.document && a.factory.operand.kind() == b.factory.operand.kind()
            }
            // Imported builders carry their own document and lexical namespace tables.
            (SiteBody::Import(_), SiteBody::Import(_)) => true,
            (SiteBody::Constant(a), SiteBody::Constant(b)) => a.layout().0 == b.layout().0,
            _ => false,
        }
    }

    fn row(&self, slot: usize) -> Resource<'_> {
        let (value, value_line, value_column) = match &self.body {
            SiteBody::Intrinsic(b) => (b.factory.operand.value(), b.factory.value_line, b.factory.value_column),
            SiteBody::Import(_) => (Value::Import(slot as u32), self.line, self.column),
            SiteBody::Constant(c) => (c.value(), self.line, self.column),
        };
        Resource { key: &self.key, value, line: self.line, column: self.column, value_line, value_column }
    }
}

#[derive(Default)]
pub(super) struct ThemeTables {
    factories: HashMap<String, Body>,
    sites: Vec<Site>,
    documents: HashMap<String, String>,
    tables: usize,
    pub constants: String,
}

const SITE: &str = "@theme-site ";
fn site(line: &str) -> Option<usize> {
    line.trim_start().strip_prefix(SITE)?.parse().ok()
}

impl ThemeTables {
    pub fn site_count(&self) -> usize { self.sites.len() }


    pub fn factory(&mut self, build: &str, slot: usize, document: &str, factory: Factory) {
        self.factories.insert(
            build.to_owned(),
            Body {
                factory,
                document: document.to_owned(),
                slot,
            },
        );
    }

    /// Substitutes a private IR marker only at the one immediate, typed AddDeferred
    /// consumption. A factory passed as a template or custom result type retains its function.
    pub fn insertion(
        &mut self,
        build: &str,
        dictionary: &str,
        key: &str,
        position: &str,
        direct: &str,
    ) -> Option<String> {
        let key = text_literal(key)?;
        let (line, column) = position.split_once(", ")?;
        let (line, column) = (line.parse().ok()?, column.parse().ok()?);
        let body = match self.factories.remove(build) {
            Some(body) => SiteBody::Intrinsic(body),
            None => SiteBody::Import(build.to_owned()),
        };
        let index = self.sites.len();
        self.sites.push(Site {
            direct: direct.to_owned(),
            dictionary: dictionary.to_owned(),
            key,
            line,
            column,
            body,
        });
        Some(format!("@theme-site {index}"))
    }

    pub fn lower(
        &mut self,
        lines: &mut Vec<String>,
        functions: &mut [String],
        function: &str,
        documents: &DocumentFunctions<'_>,
    ) {
        // Keep small functions readable, and preserve their original statement-level position maps.
        if self.sites.is_empty() {
            return;
        }
        let count = lines.iter().filter_map(|line| site(line)).take(8).count();
        if count == 0 { return; }
        if count < 8 && documents.binary.is_none() {
            for line in std::mem::take(lines) {
                if let Some(index) = site(&line) {
                    let indent = &line[..line.len() - line.trim_start().len()];
                    lines.extend(self.sites[index].direct.lines().map(|s| format!("{indent}{s}")));
                } else {
                    lines.push(line);
                }
            }
            return;
        }

        let mut input = std::mem::take(lines);
        let mut at = 0;
        while at < input.len() {
            let Some(first) = site(&input[at]) else {
                lines.push(std::mem::take(&mut input[at]));
                at += 1;
                continue;
            };
            let indent = &input[at][..input[at].len() - input[at].trim_start().len()];
            let mut run = vec![first];
            at += 1;
            // A location comment records a source range but does not execute. Leave the
            // next marker in the stream if its statement belongs to another run.
            while at < input.len() {
                let next = at + usize::from(input[at].trim_start().starts_with("// "));
                let Some(index) = input.get(next).and_then(|line| site(line)) else {
                    break;
                };
                if !self.sites[first].same_run(&self.sites[index]) {
                    break;
                }
                run.push(index);
                at = next + 1;
            }
            let first = &self.sites[first];
            let rows: Vec<_> = run.iter().enumerate().map(|(slot, &index)| self.sites[index].row(slot)).collect();
            if let SiteBody::Constant(value) = &first.body {
                let (kind, helper, ty) = value.layout();
                let (module, data) = if let Some(block) = documents.binary_resource_batch(function, kind, &rows) {
                    ("binary_resources", block)
                } else {
                    let data = format!("{}_THEME_CONSTANTS_{}", function.to_uppercase(), self.tables);
                    self.tables += 1;
                    writeln!(self.constants, "static {data}: &[rt::theme_resources::ConstantRow<{ty}>] = &[").unwrap();
                    for row in rows {
                        let operand = match row.value {
                            Value::Argb(x) => format!("0x{x:08x}"),
                            Value::Double(x) => format!("{x:#x}"),
                            Value::Quad(xs) => format!("[{:#x}, {:#x}, {:#x}, {:#x}]", xs[0], xs[1], xs[2], xs[3]),
                            _ => unreachable!("constant operand"),
                        };
                        writeln!(self.constants, "    ({}, {operand}, {}, {}),", rust_string_literal(row.key), row.line, row.column).unwrap();
                    }
                    self.constants.push_str("];\n");
                    ("theme_resources", data)
                };
                lines.push(format!("{indent}rt::{module}::{helper}({}, {data})?;", first.dictionary));
                continue;
            }
            let SiteBody::Intrinsic(body) = &first.body else {
                // A singleton stays a direct call; importing one function would only add indirection.
                if run.len() == 1 {
                    lines.push(format!("{indent}{}", first.direct));
                    continue;
                }
                let data = format!("{}_THEME_FACTORIES_{}", function.to_uppercase(), self.tables);
                self.tables += 1;
                let binary = documents.binary_resource_batch(function, Kind::DeferredFactory, &rows);
                let ty = if binary.is_some() { "rt::DeferredBuild" } else { "rt::theme_resources::FactoryRow" };
                writeln!(self.constants, "static {data}: &[{ty}] = &[").unwrap();
                for &index in &run {
                    let site = &self.sites[index];
                    let SiteBody::Import(build) = &site.body else { unreachable!("homogeneous run") };
                    if binary.is_some() {
                        writeln!(self.constants, "    {build},").unwrap();
                    } else {
                        writeln!(self.constants, "    ({}, {build}, {}, {}),", rust_string_literal(&site.key), site.line, site.column).unwrap();
                    }
                }
                self.constants.push_str("];\n");
                let (module, args) = match binary {
                    Some(block) => ("binary_resources", format!("{data}, {block}")),
                    None => ("theme_resources", data),
                };
                lines.push(format!("{indent}rt::{module}::deferred_factories({}, &context, {args})?;", first.dictionary));
                continue;
            };
            let kind = body.factory.operand.kind();
            let helper = match kind {
                Kind::Alias => "aliases",
                Kind::LiteralBrush => "literal_brushes",
                Kind::DynamicBrush => "dynamic_brushes",
                Kind::StaticBrush => "static_brushes",
                _ => unreachable!("non-intrinsic run handled separately"),
            };
            let document = &body.document;
            let document = if document.starts_with("rt::DocumentInfo {") {
                let next = self.documents.len();
                self.documents
                    .entry(document.clone())
                    .or_insert_with(|| {
                        let name = format!("{}_THEME_SCOPE_{next}", function.to_uppercase());
                        writeln!(self.constants, "static {name}: rt::DocumentInfo = {document};").unwrap();
                        name
                    })
                    .as_str()
            } else {
                document.as_str()
            };
            let (module, data) = if let Some(bytes) = documents.binary_resource_batch(function, kind, &rows) {
                ("binary_resources", bytes)
            } else {
                let data = format!("{}_THEME_ROWS_{}", function.to_uppercase(), self.tables);
                self.tables += 1;
                let operand = if kind == Kind::LiteralBrush { "u32" } else { "&str" };
                writeln!(
                    self.constants,
                    "static {data}: &[rt::theme_resources::Row<{operand}>] = &["
                )
                .unwrap();
                for row in rows {
                    let value = match row.value {
                        Value::Text(text) => rust_string_literal(text),
                        Value::Argb(argb) => format!("0x{argb:08x}"),
                        _ => unreachable!("non-intrinsic run handled separately"),
                    };
                    writeln!(
                        self.constants,
                        "    ({}, {value}, {}, {}, {}, {}),",
                        rust_string_literal(row.key),
                        row.line,
                        row.column,
                        row.value_line,
                        row.value_column
                    )
                    .unwrap();
                }
                self.constants.push_str("];\n");
                ("theme_resources", data)
            };
            lines.push(format!(
                "{indent}rt::{module}::{helper}({}, &context, &{document}, {data})?;",
                first.dictionary
            ));
            for index in run {
                if let SiteBody::Intrinsic(body) = &self.sites[index].body { functions[body.slot].clear(); }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiler_literals_round_trip_without_changing_keys() {
        for key in ["", "A, B", "\0\t\r\n", "the \"quoted\" \\key", "Ł💡"] {
            assert_eq!(text_literal(&rust_string_literal(key)).as_deref(), Some(key));
        }
        for invalid in ["key", "\"key\".into()", "\"a\" + \"b\"", r#""\u{d800}""#] {
            assert_eq!(text_literal(invalid), None);
        }
    }

    fn alias(tables: &mut ThemeTables, functions: &mut Vec<String>, dict: &str, scope: &str) -> String {
        let slot = functions.len();
        let name = format!("build_deferred_{slot}");
        functions.push(format!("fn {name}() {{}}"));
        tables.factory(&name, slot, scope, Factory::alias("\"target\"", "7", "11").unwrap());
        let line = tables
            .insertion(&name, dict, &format!("\"key_{slot}\""), "2, 5", "direct();")
            .unwrap();
        format!("    {line}")
    }

    #[test]
    fn short_conditional_bodies_keep_direct_calls_and_builders() {
        let mut tables = ThemeTables::default();
        let mut functions = Vec::new();
        let mut lines = vec![format!("    {}", alias(&mut tables, &mut functions, "&dict", "DOC"))];
        tables.lower(&mut lines, &mut functions, "build", &DocumentFunctions::default());
        assert_eq!(lines, ["        direct();"]);
        assert_eq!(functions.len(), 1);
        assert!(!functions[0].is_empty());
        assert!(tables.constants.is_empty());
    }

    #[test]
    fn table_calls_do_not_cross_effects_scope_or_dictionary_boundaries() {
        let mut tables = ThemeTables::default();
        let mut functions = Vec::new();
        let mut lines = Vec::new();
        for i in 0..9 {
            if i == 2 {
                lines.push("    side_effect()?;".into());
            }
            let dict = if i < 4 { "&one" } else { "&two" };
            let scope = if i < 6 { "A" } else { "B" };
            lines.push("    // dictionary.xaml(2,5) Content (resource)".into());
            lines.push(alias(&mut tables, &mut functions, dict, scope));
        }
        tables.lower(&mut lines, &mut functions, "build", &DocumentFunctions::default());
        let calls: Vec<_> = lines
            .iter()
            .filter(|l| !l.trim_start().starts_with("//"))
            .map(|l| l.trim())
            .collect();
        assert_eq!(
            calls,
            [
                "rt::theme_resources::aliases(&one, &context, &A, BUILD_THEME_ROWS_0)?;",
                "side_effect()?;",
                "rt::theme_resources::aliases(&one, &context, &A, BUILD_THEME_ROWS_1)?;",
                "rt::theme_resources::aliases(&two, &context, &A, BUILD_THEME_ROWS_2)?;",
                "rt::theme_resources::aliases(&two, &context, &B, BUILD_THEME_ROWS_3)?;",
            ]
        );
        assert!(functions.iter().all(String::is_empty));
        assert!(!tables.constants.contains("brush"));
        assert_eq!(tables.constants.matches("\"target\", 2, 5, 7, 11").count(), 9);
    }
}
