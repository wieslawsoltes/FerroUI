//! Intern deferred factories after unused slots have been eliminated. A shape has the
//! exact same typed operations, local lifetimes and namespace table.
//! A calling thunk owns its literal, type and child-factory operands, so an unused
//! resource and its imports can be discarded together.
use std::collections::HashMap;
use std::fmt::Write;
use std::ops::Range;

#[derive(Copy, Clone, PartialEq, Eq)]
enum Kind {
    Text,
    Float,
    Position,
    StaticType,
    DeferredBuild,
}

impl Kind {
    fn rust(self) -> &'static str {
        match self {
            Self::Text => "&'static str",
            Self::Float => "f64",
            Self::Position => "i32",
            Self::StaticType => "&'static ::ferroui_base::TypeInfo",
            Self::DeferredBuild => "rt::DeferredBuild",
        }
    }
    fn marker(self) -> &'static str {
        match self {
            Self::Text => "\x01",
            Self::Float => "\x02",
            Self::Position => "\x03",
            Self::StaticType => "\x04",
            Self::DeferredBuild => "\x05",
        }
    }
}

struct Operand {
    span: Range<usize>,
    kind: Kind,
}

fn has_location(call: &str) -> bool {
    matches!(
        call,
        "rt::bind"
            | "rt::cast"
            | "rt::cast_checked"
            | "rt::exact"
            | "rt::instance"
            | "rt::invoked"
            | "rt::provide_value_invoked"
            | "rt::provide_static_resource"
            | "rt::set_local_value"
            | "rt::no_setter"
            | "rt::bind_dynamic_resource"
            | "rt::bind_template_binding"
            | "rt::apply_template_binding"
            | "rt::bind_or_set_priority"
            | "rt::defer"
            | "rt::add_deferred_resource"
            | "rt::register_name"
            | "rt::register_name_value"
            | "rt::complete_root_name_scope"
            | "rt::static_resource_content"
    )
}

struct Call<'a> {
    name: &'a str,
    start: usize,
    previous: Option<Range<usize>>,
    in_closure: bool,
}

// A direct reference to an associated constant has no runtime registration or getter.
// Keep the reference in the calling thunk so a discarded resource cannot root its type.
fn is_static_type(value: &str) -> bool {
    value
        .strip_prefix("<::")
        .and_then(|s| s.strip_suffix(" as ::ferroui_base::StaticType>::TYPE"))
        .is_some_and(|path| path.split("::").all(super::emitter::is_identifier))
}

// This examines only the emitter's single-line call expressions. Copy unknown syntax
// verbatim. Never hoist constructors, getters, or closure operands.
fn operands(line: &str, offset: usize, out: &mut Vec<Operand>) {
    let mut calls: Vec<Call<'_>> = Vec::new();
    let mut i = 0;
    let b = line.as_bytes();
    while i < b.len() {
        match b[i] {
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
            }
            b'(' => {
                let prefix = &line[..i];
                let name = prefix
                    .rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == ':'))
                    .next()
                    .unwrap();
                let in_closure = calls
                    .last()
                    .is_some_and(|c| c.in_closure || line[c.start..i].contains("||"));
                calls.push(Call {
                    name,
                    start: i + 1,
                    previous: None,
                    in_closure,
                });
            }
            end @ (b',' | b')') => {
                if let Some(call) = calls.last_mut() {
                    let raw = &line[call.start..i];
                    let arg = raw.trim();
                    let start = call.start + raw.len() - raw.trim_start().len();
                    let span = start..start + arg.len();
                    if !call.in_closure && !call.name.is_empty() {
                        let kind = if arg
                            .strip_suffix("_f64")
                            .is_some_and(|n| n.parse::<f64>().is_ok())
                        {
                            Some(Kind::Float)
                        } else if matches!(call.name, "rt::defer" | "rt::add_deferred_resource")
                            && call.previous.as_ref().is_some_and(|r| &line[r.clone()] == "&context")
                            && super::emitter::is_identifier(arg)
                        {
                            // Both signatures take a DeferredBuild immediately after the
                            // context. Passing that fn item through the thunk retains only
                            // this resource's child and does not evaluate any XAML code.
                            Some(Kind::DeferredBuild)
                        } else if is_static_type(arg) {
                            Some(Kind::StaticType)
                        } else if super::emitter::is_string_literal(arg) {
                            Some(Kind::Text)
                        } else {
                            None
                        };
                        if let Some(kind) = kind {
                            out.push(Operand {
                                span: offset + span.start..offset + span.end,
                                kind,
                            });
                        }
                        if end == b')' && has_location(call.name) && arg.parse::<i32>().is_ok() {
                            if let Some(prev) = &call.previous {
                                if line[prev.clone()].parse::<i32>().is_ok() {
                                    for r in [prev, &span] {
                                        out.push(Operand {
                                            span: offset + r.start..offset + r.end,
                                            kind: Kind::Position,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    call.previous = Some(span);
                    call.start = i + 1;
                }
                if end == b')' {
                    calls.pop();
                }
            }
            _ => {}
        }
        i += 1;
    }
}

struct Factory {
    slot: usize,
    header_end: usize,
    markers: String,
    values: Vec<String>,
}

struct Group {
    kinds: Vec<Kind>,
    factories: Vec<Factory>,
}

// Only consider ordinary straight-line generated functions. Macro arguments, promoted
// arrays, character literals and nested items can have lexical or const requirements.
fn classify(slot: usize, function: &str) -> Option<(String, Vec<Kind>, Factory)> {
    let header_end = function.find("XamlLoadError> {\n")? + "XamlLoadError> {\n".len();
    let body = function[header_end..].strip_suffix("}\n")?;
    if body.len() > 48 * 1024 {
        return None;
    }
    let mut markers = String::new();
    let mut program = String::new();
    let mut sites = Vec::new();
    for line in body.lines() {
        if line.trim_start().starts_with("//") {
            writeln!(markers, "{line}").unwrap();
            continue;
        }
        if line.contains(['!', '\'', '[']) || line.replace("||", "").contains('|') {
            return None;
        }
        // The emitter only produces local item declarations outside deferred bodies.
        if line.trim_start().starts_with("const ") || line.trim_start().starts_with("static ") {
            return None;
        }
        operands(line, program.len(), &mut sites);
        writeln!(program, "{line}").unwrap();
    }
    sites.sort_unstable_by_key(|s| s.span.start);
    let mut shape = String::with_capacity(program.len());
    let mut values = Vec::with_capacity(sites.len());
    let mut kinds = Vec::with_capacity(sites.len());
    let mut start = 0;
    for site in sites {
        shape.push_str(&program[start..site.span.start]);
        shape.push_str(site.kind.marker());
        values.push(program[site.span.clone()].to_owned());
        kinds.push(site.kind);
        start = site.span.end;
    }
    shape.push_str(&program[start..]);
    Some((
        shape,
        kinds,
        Factory {
            slot,
            header_end,
            markers,
            values,
        },
    ))
}

impl Group {
    fn emit(&self, name: &str, shape: &str, functions: &mut [String]) {
        let first = &self.factories[0];
        let mut parameters = Vec::new();
        let mut fields: Vec<usize> = Vec::new();
        let mut body = String::new();
        let mut slot = 0;
        for part in shape.split_inclusive(['\x01', '\x02', '\x03', '\x04', '\x05']) {
            let Some(code) = part.strip_suffix(['\x01', '\x02', '\x03', '\x04', '\x05']) else {
                body.push_str(part);
                continue;
            };
            body.push_str(code);
            if self
                .factories
                .iter()
                .all(|f| f.values[slot] == first.values[slot])
            {
                body.push_str(&first.values[slot]);
            } else {
                let index = fields
                    .iter()
                    .position(|&other| {
                        self.kinds[other] == self.kinds[slot]
                            && self
                                .factories
                                .iter()
                                .all(|f| f.values[other] == f.values[slot])
                    })
                    .unwrap_or_else(|| {
                        let index = fields.len();
                        fields.push(slot);
                        parameters.push(format!("arg{index}: {}", self.kinds[slot].rust()));
                        index
                    });
                // Bound both the calling convention and the cost of comparing columns.
                if fields.len() > 12 {
                    return;
                }
                write!(body, "arg{index}").unwrap();
            }
            slot += 1;
        }
        // Retain the representative XAML markers on the actual typed operations for
        // rustc's position map. Every other thunk keeps its own declaration markers.
        let original = functions[first.slot][first.header_end..]
            .strip_suffix("}\n")
            .unwrap();
        let mut instructions = body.lines();
        let mut mapped_body = String::new();
        for line in original.lines() {
            let mapped = if line.trim_start().starts_with("//") {
                line
            } else {
                instructions.next().unwrap()
            };
            writeln!(mapped_body, "{mapped}").unwrap();
        }
        debug_assert!(instructions.next().is_none());
        let mut shared = format!("#[inline(never)]\nfn {name}(\n    service_provider: &::std::rc::Rc<dyn ::ferroui_base::metadata::IServiceProvider>,\n");
        for p in parameters {
            writeln!(shared, "    {p},").unwrap();
        }
        writeln!(shared, ") -> ::core::result::Result<::ferroui_base::metadata::MarkupValue, ::ferroui_markup_xaml::XamlLoadError> {{\n{mapped_body}}}").unwrap();
        let mut wrappers = Vec::new();
        for f in &self.factories {
            let mut tail = if f.slot == first.slot {
                String::new()
            } else {
                f.markers.clone()
            };
            write!(tail, "    {name}(service_provider").unwrap();
            for &i in &fields {
                write!(tail, ", {}", f.values[i]).unwrap();
            }
            tail.push_str(")\n}\n");
            wrappers.push(tail);
        }
        let old: usize = self
            .factories
            .iter()
            .map(|f| functions[f.slot].len() - f.header_end)
            .sum();
        if shared.len() + wrappers.iter().map(String::len).sum::<usize>() >= old {
            return;
        }
        for (f, tail) in self.factories.iter().zip(wrappers) {
            functions[f.slot].truncate(f.header_end);
            functions[f.slot].push_str(&tail);
        }
        functions[first.slot].push('\n');
        functions[first.slot].push_str(&shared);
    }
}

pub(super) fn share(document: &str, functions: &mut [String]) {
    let mut groups: HashMap<String, Group> = HashMap::new();
    for (slot, function) in functions.iter().enumerate() {
        let Some((shape, kinds, factory)) = classify(slot, function) else {
            continue;
        };
        groups
            .entry(shape)
            .or_insert_with(|| Group {
                kinds,
                factories: Vec::new(),
            })
            .factories
            .push(factory);
    }
    for (shape, group) in groups {
        if group.factories.len() >= 2 {
            let name = format!("__{document}_factory_shape_{}", group.factories[0].slot);
            group.emit(&name, &shape, functions);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locations_belong_to_the_matching_call_and_getter_closures_keep_their_literals() {
        let line = r#"rt::invoked(target(rt::cast(x, 10, 4)?, || fixed("frozen"), "a,\"b", -1.5_f64), 20, 3)?;"#;
        let mut found = Vec::new();
        operands(line, 0, &mut found);
        let found: Vec<_> = found
            .iter()
            .map(|op| (&line[op.span.clone()], op.kind.rust()))
            .collect();
        assert_eq!(
            found,
            [
                ("10", "i32"),
                ("4", "i32"),
                (r#""a,\"b""#, "&'static str"),
                ("-1.5_f64", "f64"),
                ("20", "i32"),
                ("3", "i32")
            ]
        );
    }

    #[test]
    fn resolved_types_stay_in_thunks_and_runtime_getters_stay_in_the_body() {
        let factory = |id, class| format!(r#"fn site_{id}(service_provider: &Provider) -> Result<Value, XamlLoadError> {{
    let parent = ControlTheme::new();
    ControlTheme::set_target(&parent, Some(<::controls::{class} as ::ferroui_base::StaticType>::TYPE));
    let base = rt::provide_static_resource(&context, "base", || descriptor(<::controls::Theme as ::ferroui_base::StaticType>::TYPE), {id}, 2)?;
    let base = rt::cast_checked(base, getter(), "Theme", {id}, 2)?;
    let child = rt::defer(result_type, &context, child_{id}, {id}, 2)?;
    Ok(set_base(parent, base, child))
}}
"#);
        let mut factories = vec![factory(0, "Button"), factory(1, "TextBox"), factory(2, "Slider")];
        share("doc", &mut factories);
        assert!(factories[0].contains("arg0: &'static ::ferroui_base::TypeInfo"));
        assert!(factories[0].contains("ControlTheme::set_target(&parent, Some(arg0))"));
        assert!(factories[0].contains("rt::cast_checked(base, getter(), \"Theme\", arg1, 2)"));
        assert!(factories[0].contains("|| descriptor(<::controls::Theme as ::ferroui_base::StaticType>::TYPE)"));
        assert!(factories[0].contains("arg2: rt::DeferredBuild"));
        assert!(!factories[0].contains("child_1"));
        assert!(!factories[0].contains("::controls::TextBox"));
        assert!(factories[1].contains("__doc_factory_shape_0(service_provider, <::controls::TextBox as ::ferroui_base::StaticType>::TYPE, 1, child_1)"));
        assert!(!is_static_type("<::controls::Generic<T> as ::ferroui_base::StaticType>::TYPE"));
    }

    #[test]
    fn distinct_imports_cannot_share_a_factory_and_unused_slots_stay_empty() {
        fn factory(n: usize, constructor: &str, key: &str) -> String {
            let mut body = format!("fn doc_deferred_{n}(service_provider: &Provider) -> Result<Value, XamlLoadError> {{\n");
            writeln!(body, "    // doc.xaml({n},4) Brush").unwrap();
            // Enough repeated work to justify an out-of-line typed implementation.
            for _ in 0..8 {
                writeln!(
                    body,
                    "    rt::invoked({constructor}(rt::string_object({key:?})), {n}, 4)?;"
                )
                .unwrap();
            }
            body.push_str("    Ok(value)\n}\n");
            body
        }
        let third = factory(2, "different_constructor", "c");
        let mut functions = vec![
            factory(0, "new_brush", "a"),
            factory(1, "new_brush", "b"),
            third.clone(),
            String::new(),
        ];
        share("doc", &mut functions);
        assert!(functions[0].contains("fn __doc_factory_shape_0("));
        assert!(functions[0].contains("// doc.xaml(0,4) Brush\n    rt::invoked("));
        assert!(functions[1].contains("__doc_factory_shape_0(service_provider, \"b\", 1)"));
        assert_eq!(functions[2], third);
        assert!(functions[3].is_empty());
    }
}
