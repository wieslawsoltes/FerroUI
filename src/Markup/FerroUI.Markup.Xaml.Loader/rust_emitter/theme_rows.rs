//! Theme resource rows (#568, step 1 of docs/porting/theme-trimming.md): runs of resources of
//! one shape added to one dictionary, written as a static row table and one call of the
//! builder of the shape (`rt::add_resource_aliases`, `rt::add_double_resources`, ...) instead of
//! a build function per deferred resource and a statement per plain value.
//!
//! The shapes are recognised where the emitter writes them. A deferred resource is recognised
//! when its build function is complete ([`deferred_row`]): the statements of the body must be
//! exactly those the emitter writes for one of the four shapes of section 5.1, with only the
//! key, the colour and the positions free. Its addition is recognised where the emitter fuses it
//! into `rt::add_deferred_resource` ([`deferred_addition`]), and a plain value where the emitter
//! writes the call of `IDictionary.Add` ([`value_row_statement`]). Either becomes a
//! [`RowStatement`]: the statement as the emitter wrote it, kept in the body, and the row that
//! replaces it.
//!
//! When the statements of a body are complete, [`row_runs`] replaces each run of consecutive
//! row statements of one shape and one dictionary by one call of the builder of the shape over a
//! table of the rows. A run ends at any other statement, including a statement of another
//! shape, so the order of the additions is kept, and where the unit of the theme the statements
//! are entries of changes (#568, step 2: the markers of [`super::theme_units`]), so that a unit
//! keeps its rows. A run shorter than the minimum of its shape
//! ([`RowKind::min_run`]) stays as it was written; everything that is not recognised stays as it
//! was written. The build functions of the deferred rows of a run are then no longer emitted.

use std::collections::{HashMap, HashSet};

use super::super::emit_types::{EmitTypes, FrameworkType};
use super::theme_units::unit_marker;
use super::{absolute, identifiers, is_identifier, is_string_literal, static_resource_alias};

/// The shortest run of plain values that is written as a row table. A deferred row is written
/// as a row even alone, since its row replaces a build function of its own; a single plain value
/// is one statement either way.
const MIN_VALUE_RUN: usize = 2;

/// A shape of resource whose additions are written as rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum RowKind {
    /// `{StaticResource Key}` alone.
    ResourceAlias,
    /// A `SolidColorBrush` with a literal colour.
    LiteralBrush,
    /// A `SolidColorBrush` whose colour is `{DynamicResource Key}`.
    DynamicBrush,
    /// A `SolidColorBrush` whose colour is `{StaticResource Key}`.
    StaticBrush,
    /// A number (`x:Double`).
    Double,
    /// A `Thickness` of four components.
    Thickness,
    /// A `CornerRadius` of four components.
    CornerRadius,
    /// A `Color` (`Color.FromUInt32`).
    Color,
    /// A text (`x:String`).
    Text,
}

impl RowKind {
    /// The Rust type of a row of the shape.
    fn row_type(self) -> &'static str {
        match self {
            Self::ResourceAlias => "rt::ResourceAliasRow",
            Self::LiteralBrush => "rt::LiteralBrushRow",
            Self::DynamicBrush => "rt::DynamicBrushRow",
            Self::StaticBrush => "rt::StaticBrushRow",
            Self::Double => "rt::ValueRow<f64>",
            Self::Thickness | Self::CornerRadius => "rt::ValueRow<[f64; 4]>",
            Self::Color => "rt::ValueRow<u32>",
            Self::Text => "rt::ValueRow<&'static str>",
        }
    }

    /// The builder of the shape in `rt`.
    fn builder(self) -> &'static str {
        match self {
            Self::ResourceAlias => "rt::add_resource_aliases",
            Self::LiteralBrush => "rt::add_literal_brushes",
            Self::DynamicBrush => "rt::add_dynamic_brushes",
            Self::StaticBrush => "rt::add_static_brushes",
            Self::Double => "rt::add_double_resources",
            Self::Thickness => "rt::add_thickness_resources",
            Self::CornerRadius => "rt::add_corner_radius_resources",
            Self::Color => "rt::add_color_resources",
            Self::Text => "rt::add_string_resources",
        }
    }

    /// What the rows of the shape are, for the comment of a table.
    fn description(self) -> &'static str {
        match self {
            Self::ResourceAlias => "The resource aliases",
            Self::LiteralBrush => "The brushes with a literal colour",
            Self::DynamicBrush => "The brushes bound to a dynamic resource",
            Self::StaticBrush => "The brushes whose colour is a static resource",
            Self::Double => "The numbers",
            Self::Thickness => "The thicknesses",
            Self::CornerRadius => "The corner radii",
            Self::Color => "The colours",
            Self::Text => "The texts",
        }
    }

    /// Whether a row of the shape is deferred content (its builder takes the context and the
    /// document of the declaration).
    fn is_deferred(self) -> bool {
        matches!(self, Self::ResourceAlias | Self::LiteralBrush | Self::DynamicBrush | Self::StaticBrush)
    }

    /// The shortest run of the shape that is written as a row table.
    pub(super) fn min_run(self) -> usize {
        match self.is_deferred() {
            true => 1,
            false => MIN_VALUE_RUN,
        }
    }
}

/// The Rust paths of the framework types the recognised statements name, as the type system
/// states them (the emitter writes the same paths).
pub(super) struct RowPaths {
    /// `FerroUI.Controls.IResourceDictionary`.
    pub(super) resource_dictionary_contract: String,
    /// `FerroUI.Media.SolidColorBrush`.
    pub(super) solid_color_brush: String,
    /// `FerroUI.Media.Color`.
    pub(super) color: String,
    /// `FerroUI.Thickness`.
    pub(super) thickness: String,
    /// `FerroUI.CornerRadius`.
    pub(super) corner_radius: String,
    /// `FerroUI.Markup.Xaml.MarkupExtensions.StaticResourceExtension`.
    pub(super) static_resource_extension: String,
}

impl RowPaths {
    /// The paths `types` states; `None` (nothing is written as rows) if it lacks one.
    pub(super) fn of(types: &dyn EmitTypes) -> Option<Self> {
        let path = |type_: FrameworkType| types.framework_path(type_).map(|path| absolute(&path));
        Some(Self {
            resource_dictionary_contract: path(FrameworkType::IResourceDictionary)?,
            solid_color_brush: path(FrameworkType::SolidColorBrush)?,
            color: path(FrameworkType::Color)?,
            thickness: path(FrameworkType::Thickness)?,
            corner_radius: path(FrameworkType::CornerRadius)?,
            static_resource_extension: path(FrameworkType::StaticResourceExtension)?,
        })
    }
}

/// The deferred content of a resource in one of the four shapes, without its key and the
/// position of its addition: the shape, the operand (the target key, the ARGB value or the key
/// of the colour, as Rust literal text), the position of the value, and the namespace table of
/// the declaration when it is not the table of the document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DeferredRow {
    pub(super) kind: RowKind,
    pub(super) operand: String,
    pub(super) value_position: (i32, i32),
    pub(super) namespaces: Option<String>,
}

/// A statement of a body the emitter wrote that a row can replace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RowStatement {
    pub(super) kind: RowKind,
    /// The local of the dictionary the statement adds to.
    pub(super) dictionary: String,
    /// The row, as a Rust struct literal of the row type of the shape.
    pub(super) row: String,
    /// The statement that must precede the statement, and the local it declares, which the row
    /// replaces as well (the local of a colour).
    pub(super) prefix: Option<(String, String)>,
    /// The slot of the build function of a deferred row, which is not emitted when the row is.
    pub(super) slot: Option<usize>,
}

/// The statements of a body without the indentation and the position markers.
fn code_statements(lines: &[String]) -> Vec<&str> {
    lines.iter().map(|line| line.trim_start()).filter(|line| !line.starts_with("//")).collect()
}

/// `(line, position)` of the text `"line, position"`.
fn position_of(text: &str) -> Option<(i32, i32)> {
    let (line, position) = text.split_once(", ")?;
    Some((line.parse().ok()?, position.parse().ok()?))
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

/// The statement text of the namespace table of a row.
fn namespaces_text(namespaces: &Option<String>) -> String {
    match namespaces {
        Some(table) => format!("::core::option::Option::Some({table})"),
        None => "::core::option::Option::None".to_string(),
    }
}

/// Whether `text` is a literal the emitter writes for an `f64` (`f64_literal`).
fn is_f64_literal(text: &str) -> bool {
    matches!(text, "::core::primitive::f64::NAN" | "::core::primitive::f64::INFINITY" | "::core::primitive::f64::NEG_INFINITY")
        || text.strip_suffix("_f64").is_some_and(|number| number.parse::<f64>().is_ok())
}

/// Whether `text` is a literal the emitter writes for a `u32`.
fn is_u32_literal(text: &str) -> bool {
    text.strip_suffix("_u32").is_some_and(|number| number.parse::<u32>().is_ok())
}

/// The deferred content whose body the emitter wrote as `lines`, returning `returned`, if it is
/// one of the four shapes of resource the rows cover. `namespaces` is the namespace table of the
/// declaration when it is not the table of the document. The statements of the body must be
/// exactly those of the shape; any other statement, member or order is not a row.
pub(super) fn deferred_row(paths: &RowPaths, lines: &[String], returned: &str, namespaces: Option<&str>) -> Option<DeferredRow> {
    let namespaces = namespaces.map(str::to_string);
    if let Some(alias) = static_resource_alias(&paths.static_resource_extension, lines, returned) {
        return Some(DeferredRow {
            kind: RowKind::ResourceAlias,
            operand: alias.key.to_string(),
            value_position: (alias.line.parse().ok()?, alias.column.parse().ok()?),
            namespaces,
        });
    }
    let brush = &paths.solid_color_brush;
    if returned != "rt::to_value(solid_color_brush_0.clone())" {
        return None;
    }
    let statements = code_statements(lines);
    let create = format!("let solid_color_brush_0 = {brush}::new();");
    let color_property = format!("{brush}::color_property()");
    let target = "solid_color_brush_0.upcast_ref::<::ferroui_base::FerroObject>()";
    let (first, rest) = statements.split_first()?;
    if *first != create {
        return None;
    }
    // A literal colour.
    if let [color, set] = rest {
        let argb = color.strip_prefix(&format!("let value_0 = {}::__markup_FromUInt32_2(", paths.color))?.strip_suffix(");")?;
        let expected = format!("solid_color_brush_0.set_value({color_property}, ::core::clone::Clone::clone(&value_0));");
        return (is_u32_literal(argb) && *set == expected).then(|| DeferredRow {
            kind: RowKind::LiteralBrush,
            operand: argb.to_string(),
            value_position: (0, 0),
            namespaces,
        });
    }
    let push = "context.push_parent(rt::to_value(solid_color_brush_0.clone()));";
    let [pushed, value @ .., popped] = rest else { return None };
    if *pushed != push || *popped != "context.pop_parent();" {
        return None;
    }
    // A colour bound to a dynamic resource.
    if let [bind] = value {
        let rest = bind.strip_prefix(&format!("rt::bind_dynamic_resource(&context, {target}, {color_property}, "))?;
        let (key, rest) = split_string_literal(rest)?;
        let position = position_of(rest.strip_prefix(", ")?.strip_suffix(")?;")?)?;
        return Some(DeferredRow { kind: RowKind::DynamicBrush, operand: key.to_string(), value_position: position, namespaces });
    }
    // A colour that is a static resource, with the setter chosen at run time.
    let extension = &paths.static_resource_extension;
    let key = value.first()?.strip_prefix(&format!(
        "let static_resource_extension_0 = {extension}::__markup_new_1(rt::string_object("
    ))?;
    let (key, rest) = split_string_literal(key)?;
    if rest != "));" {
        return None;
    }
    let provide = value.get(1)?.strip_prefix(&format!(
        "let provided_0 = rt::provide_value_invoked(&context, rt::property_value({color_property}), &static_resource_extension_0, {extension}::__markup_ProvideValue_0, "
    ))?;
    let (line, position) = position_of(provide.strip_suffix(")?;")?)?;
    let markup = |path: &str| format!("rt::markup_handle(<{path} as ::ferroui_base::metadata::MarkupTyped>::MARKUP)");
    let expected = [
        format!("let static_resource_extension_0 = {extension}::__markup_new_1(rt::string_object({key}));"),
        format!(
            "let provided_0 = rt::provide_value_invoked(&context, rt::property_value({color_property}), &static_resource_extension_0, {extension}::__markup_ProvideValue_0, {line}, {position})?;"
        ),
        "let value_0: ::ferroui_base::metadata::MarkupValue = rt::to_value(provided_0);".to_string(),
        format!("if rt::is_instance(&value_0, {}) {{", markup("::ferroui_base::UnsetValueType")),
        format!("rt::unset_value({target}, {color_property});"),
        format!("}} else if rt::is_instance(&value_0, {}) {{", markup("dyn ::ferroui_base::data::BindingBase")),
        format!("rt::bind({target}, {color_property}, value_0.clone(), {line}, {position})?;"),
        format!("}} else if rt::is_instance(&value_0, {}) {{", markup(&paths.color)),
        format!(
            "solid_color_brush_0.set_value({color_property}, rt::exact(value_0.clone(), \"FerroUI.Media.SolidColorBrush.set_Color\", 0, {line}, {position})?);"
        ),
        "} else {".to_string(),
        format!("return ::core::result::Result::Err(rt::no_setter(\"Color\", &value_0, {line}, {position}));"),
        "}".to_string(),
    ];
    (value.len() == expected.len() && value.iter().zip(&expected).all(|(statement, expected)| *statement == expected.as_str())).then(|| DeferredRow {
        kind: RowKind::StaticBrush,
        operand: key.to_string(),
        value_position: (line, position),
        namespaces,
    })
}

/// The parts of `rt::add_deferred_resource(&dictionary, key, ValueType::object(), &context, build,
/// line, position)?;`, the statement the emitter fuses the addition of a deferred resource into.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DeferredAddition<'s> {
    pub(super) dictionary: &'s str,
    pub(super) key: &'s str,
    pub(super) build: &'s str,
    pub(super) position: (i32, i32),
}

/// The parts of the fused addition `statement` of a deferred resource whose result type is
/// `object`, added to a local dictionary in the current context; `None` for any other form.
pub(super) fn deferred_addition(statement: &str) -> Option<DeferredAddition<'_>> {
    let rest = statement.strip_prefix("rt::add_deferred_resource(&")?;
    let (dictionary, rest) = rest.split_once(", ")?;
    let (key, rest) = split_string_literal(rest)?;
    let rest = rest.strip_prefix(", ::ferroui_base::data::core::ValueType::object(), &context, ")?;
    let (build, rest) = rest.split_once(", ")?;
    let position = position_of(rest.strip_suffix(")?;")?)?;
    (is_identifier(dictionary) && is_identifier(build) && is_string_literal(key)).then_some(DeferredAddition {
        dictionary,
        key,
        build,
        position,
    })
}

/// The row statement of the addition `addition` of the deferred content `row` whose build
/// function has the slot `slot`.
pub(super) fn deferred_row_statement(addition: &DeferredAddition<'_>, row: &DeferredRow, slot: usize) -> RowStatement {
    let DeferredAddition { dictionary, key, position: (line, position), .. } = addition;
    let namespaces = namespaces_text(&row.namespaces);
    let (value_line, value_position) = row.value_position;
    let operand = &row.operand;
    let text = match row.kind {
        RowKind::ResourceAlias => format!(
            "rt::ResourceAliasRow {{ key: {key}, target: {operand}, namespaces: {namespaces}, line: {line}, position: {position}, value_line: {value_line}, value_position: {value_position} }}"
        ),
        RowKind::LiteralBrush => format!(
            "rt::LiteralBrushRow {{ key: {key}, color: {operand}, namespaces: {namespaces}, line: {line}, position: {position} }}"
        ),
        kind => format!(
            "{} {{ key: {key}, color_key: {operand}, namespaces: {namespaces}, line: {line}, position: {position}, value_line: {value_line}, value_position: {value_position} }}",
            kind.row_type()
        ),
    };
    RowStatement { kind: row.kind, dictionary: dictionary.to_string(), row: text, prefix: None, slot: Some(slot) }
}

/// The local of the dictionary that `call`, a call of `IDictionary.Add` the emitter wrote,
/// adds a plain value to, with its row statement, if the value is a number, a thickness, a corner
/// radius, a colour or a text of literal form. `previous` is the statement before the call (the
/// local that holds a colour). The caller checks that the local holds a `ResourceDictionary`.
pub(super) fn value_row_statement(paths: &RowPaths, call: &str, previous: Option<&str>) -> Option<RowStatement> {
    let rest = call.strip_prefix(&format!("rt::invoked(<dyn {}>::__markup_Add_0(&rt::cast(", paths.resource_dictionary_contract))?;
    let (dictionary, rest) = rest.split_once(".clone(), ")?;
    let (cast_position, rest) = rest.split_once(")?, rt::string_object(")?;
    let (key, rest) = split_string_literal(rest)?;
    let rest = rest.strip_prefix("), ")?;
    // The value, then `), line, position)?`.
    let (value, position) = rest.strip_suffix(")?")?.rsplit_once("), ")?;
    let position = position_of(position)?;
    if !is_identifier(dictionary) || position_of(cast_position)? != position {
        return None;
    }
    let mut prefix = None;
    let (kind, value) = if let Some(text) = value.strip_prefix("rt::string_object(").and_then(|value| value.strip_suffix(')')) {
        (RowKind::Text, is_string_literal(text).then(|| text.to_string())?)
    } else {
        let value = value.strip_prefix("rt::to_object(")?.strip_suffix(')')?;
        let components = |path: &str| -> Option<String> {
            let arguments = value.strip_prefix(&format!("{path}::__markup_new_3("))?.strip_suffix(')')?;
            let arguments: Vec<&str> = arguments.split(", ").collect();
            (arguments.len() == 4 && arguments.iter().all(|argument| is_f64_literal(argument))).then(|| format!("[{}]", arguments.join(", ")))
        };
        if is_f64_literal(value) {
            (RowKind::Double, value.to_string())
        } else if let Some(components) = components(&paths.thickness) {
            (RowKind::Thickness, components)
        } else if let Some(components) = components(&paths.corner_radius) {
            (RowKind::CornerRadius, components)
        } else {
            let local = value.strip_prefix("::core::clone::Clone::clone(&")?.strip_suffix(')')?;
            let previous = previous?.trim_start();
            let argb = previous
                .strip_prefix(&format!("let {local} = {}::__markup_FromUInt32_2(", paths.color))?
                .strip_suffix(");")?;
            if !is_identifier(local) || !is_u32_literal(argb) {
                return None;
            }
            prefix = Some((previous.to_string(), local.to_string()));
            (RowKind::Color, argb.to_string())
        }
    };
    let (line, position) = position;
    Some(RowStatement {
        kind,
        dictionary: dictionary.to_string(),
        row: format!("rt::ValueRow {{ key: {key}, value: {value}, line: {line}, position: {position} }}"),
        prefix,
        slot: None,
    })
}

/// A run of row statements written as one table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RowRun {
    pub(super) kind: RowKind,
    pub(super) rows: Vec<String>,
    /// The slots of the build functions the rows replace.
    pub(super) slots: Vec<usize>,
}

/// A row statement found in a body: the index of its first line (its position markers), of
/// its first line of code (its prefix if it has one) and of the statement itself.
struct Found<'s> {
    start: usize,
    code: usize,
    statement: usize,
    entry: &'s RowStatement,
    /// The unit of the theme the statement is an entry of (#568, the markers of
    /// [`super::theme_units`]); a run holds the entries of one unit only.
    unit: Option<u32>,
}

/// The number of spaces a line is indented by.
fn indentation(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The row statement whose first line of code is `lines[code]`, its position markers from
/// `start`: the statement itself, or the prefix it requires followed by the statement, when the
/// local of the prefix is named by those two lines only (`uses`).
fn found_at<'s>(
    lines: &[String],
    statements: &'s HashMap<String, RowStatement>,
    uses: &HashMap<&str, usize>,
    start: usize,
    code: usize,
) -> Option<Found<'s>> {
    let text = lines[code].trim_start();
    if let Some(entry) = statements.get(text).filter(|entry| entry.prefix.is_none()) {
        return Some(Found { start, code, statement: code, entry, unit: None });
    }
    let next = lines.get(code + 1)?;
    let entry = statements.get(next.trim_start())?;
    let (prefix, local) = entry.prefix.as_ref()?;
    (prefix == text && uses.get(local.as_str()) == Some(&2) && indentation(&lines[code]) == indentation(next))
        .then_some(Found { start, code, statement: code + 1, entry, unit: None })
}

/// Ends the run `run`: a run as long as the minimum of its shape becomes the position markers
/// of its first statement and the call of the builder over a new table (added to `tables`);
/// a shorter run is written as it was.
fn flush(
    run: &mut Vec<Found<'_>>,
    lines: &[String],
    body: &mut Vec<String>,
    tables: &mut Vec<(String, RowRun)>,
    document: &str,
    table_name: &mut dyn FnMut() -> String,
) {
    let Some(first) = run.first() else { return };
    let kind = first.entry.kind;
    if run.len() < kind.min_run() {
        for found in run.iter() {
            body.extend(lines[found.start..=found.statement].iter().cloned());
        }
        run.clear();
        return;
    }
    let statement = &lines[first.statement];
    let indent = &statement[..indentation(statement)];
    let name = table_name();
    body.extend(lines[first.start..first.code].iter().cloned());
    let dictionary = &first.entry.dictionary;
    body.push(match kind.is_deferred() {
        true => format!("{indent}{}(&{dictionary}, &context, &{document}, {name})?;", kind.builder()),
        false => format!("{indent}{}(&{dictionary}, {name})?;", kind.builder()),
    });
    let rows = run.iter().map(|found| found.entry.row.clone()).collect();
    let slots = run.iter().filter_map(|found| found.entry.slot).collect();
    tables.push((name, RowRun { kind, rows, slots }));
    run.clear();
}

/// `lines`, a body the emitter wrote (each line indented), with each run of consecutive row
/// statements of one shape and one dictionary that is at least as long as the minimum of its
/// shape replaced by the position markers of its first statement and one call of the builder of
/// the shape over a table. `statements` are the row statements recorded for the body, by their
/// text; `document` names the `rt::DocumentInfo` of the document; `table_name` gives the name of
/// the next table. Returns the body and the tables, in the order of the calls.
pub(super) fn row_runs(
    lines: Vec<String>,
    statements: &HashMap<String, RowStatement>,
    document: &str,
    mut table_name: impl FnMut() -> String,
) -> (Vec<String>, Vec<(String, RowRun)>) {
    if statements.is_empty() {
        return (lines, Vec::new());
    }
    // A local a row replaces with its prefix must not be named anywhere else.
    let candidates: HashSet<&str> = lines
        .iter()
        .filter_map(|line| statements.get(line.trim_start()))
        .filter_map(|entry| entry.prefix.as_ref().map(|(_, local)| local.as_str()))
        .collect();
    let mut uses: HashMap<&str, usize> = HashMap::new();
    if !candidates.is_empty() {
        for line in lines.iter().map(|line| line.trim_start()).filter(|line| !line.starts_with("//")) {
            for name in identifiers(line) {
                if let Some(candidate) = candidates.get(name.as_str()) {
                    *uses.entry(*candidate).or_insert(0) += 1;
                }
            }
        }
    }
    let mut body = Vec::with_capacity(lines.len());
    let mut tables = Vec::new();
    let mut run: Vec<Found<'_>> = Vec::new();
    // The unit of the theme the statements are entries of, from the markers between them.
    let mut unit: Option<u32> = None;
    let mut index = 0;
    while index < lines.len() {
        let start = index;
        while index < lines.len() && lines[index].trim_start().starts_with("//") {
            if let Some(marker) = unit_marker(&lines[index]) {
                unit = marker;
            }
            index += 1;
        }
        if index == lines.len() {
            flush(&mut run, &lines, &mut body, &mut tables, document, &mut table_name);
            body.extend(lines[start..].iter().cloned());
            break;
        }
        let Some(mut found) = found_at(&lines, statements, &uses, start, index) else {
            flush(&mut run, &lines, &mut body, &mut tables, document, &mut table_name);
            body.extend(lines[start..=index].iter().cloned());
            index += 1;
            continue;
        };
        found.unit = unit;
        let continues = run.first().is_some_and(|first| {
            first.entry.kind == found.entry.kind
                && first.unit == found.unit
                && first.entry.dictionary == found.entry.dictionary
                && indentation(&lines[first.code]) == indentation(&lines[found.code])
        });
        if !continues {
            flush(&mut run, &lines, &mut body, &mut tables, document, &mut table_name);
        }
        index = found.statement + 1;
        run.push(found);
    }
    flush(&mut run, &lines, &mut body, &mut tables, document, &mut table_name);
    (body, tables)
}

/// The text of the static table `name` of the rows of `run`, of the document `document_text`.
pub(super) fn row_table(name: &str, document_text: &str, run: &RowRun) -> String {
    let mut text = format!(
        "/// {} of a run of `{document_text}`, added in order by `{}` (#568).\n",
        run.kind.description(),
        run.kind.builder()
    );
    text.push_str(&format!("static {name}: &[{}] = &[\n", run.kind.row_type()));
    for row in &run.rows {
        text.push_str(&format!("    {row},\n"));
    }
    text.push_str("];\n");
    text
}

#[cfg(test)]
#[path = "theme_rows_tests.rs"]
mod tests;
