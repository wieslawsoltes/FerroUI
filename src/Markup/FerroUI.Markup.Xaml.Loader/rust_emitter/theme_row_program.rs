//! Encoding of individually linkable resource programs. Each run has its own string and
//! namespace imports, so removing a theme unit removes its operands as well.
use super::RowKind;
use ferroui_markup::{compiled_operands as wire, resource_opcodes as op};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct OperandRow {
    pub(super) key: String,
    pub(super) value: String,
    pub(super) declaration: (i32, i32),
    pub(super) value_position: (i32, i32),
    pub(super) namespaces: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Program {
    bytes: Vec<u8>,
    strings: Vec<String>,
    namespaces: Vec<String>,
}

pub(super) fn opcode(kind: RowKind) -> u8 {
    match kind {
        RowKind::ResourceAlias => op::ALIAS,
        RowKind::LiteralBrush => op::LITERAL_BRUSH,
        RowKind::DynamicBrush => op::DYNAMIC_BRUSH,
        RowKind::StaticBrush => op::STATIC_BRUSH,
        RowKind::Double => op::DOUBLE,
        RowKind::Thickness => op::THICKNESS,
        RowKind::CornerRadius => op::CORNER_RADIUS,
        RowKind::Color => op::COLOR,
        RowKind::Text => op::TEXT,
    }
}

pub(super) fn method(kind: RowKind) -> &'static str {
    kind.builder().strip_prefix("rt::").expect("a resource builder in rt")
}

fn unsigned(bytes: &mut Vec<u8>, value: usize) {
    wire::unsigned(bytes, value.try_into().expect("resource run fits in 32 bits"));
}

fn delta(bytes: &mut Vec<u8>, current: i32, previous: i32) {
    wire::signed(bytes, current.wrapping_sub(previous));
}

fn number(bytes: &mut Vec<u8>, literal: &str) {
    wire::number(bytes, super::super::theme_styles::number_literal(literal));
}

#[derive(Default)]
struct Imports<'a> {
    ids: HashMap<&'a str, usize>,
    values: Vec<String>,
}

impl<'a> Imports<'a> {
    fn id(&mut self, item: &'a str) -> usize {
        *self.ids.entry(item).or_insert_with(|| {
            let id = self.values.len();
            self.values.push(item.to_owned());
            id
        })
    }
}

impl Program {
    pub(super) fn encode<'a>(kind: RowKind, rows: impl ExactSizeIterator<Item = &'a OperandRow>) -> Self {
        let mut bytes = vec![op::VERSION, opcode(kind)];
        unsigned(&mut bytes, rows.len());
        let mut strings = Imports::default();
        let mut namespaces = Imports::default();
        let mut previous = (0, 0);
        for row in rows {
            unsigned(&mut bytes, strings.id(&row.key));
            let (line, column) = row.declaration;
            delta(&mut bytes, line, previous.0);
            delta(&mut bytes, column, previous.1);
            previous = row.declaration;
            if kind.is_deferred() {
                unsigned(&mut bytes, row.namespaces.as_deref().map_or(0, |name| namespaces.id(name) + 1));
                if kind != RowKind::LiteralBrush {
                    delta(&mut bytes, row.value_position.0, line);
                    delta(&mut bytes, row.value_position.1, column);
                }
            }
            match kind {
                RowKind::ResourceAlias | RowKind::DynamicBrush | RowKind::StaticBrush | RowKind::Text =>
                    unsigned(&mut bytes, strings.id(&row.value)),
                RowKind::LiteralBrush | RowKind::Color => {
                    let argb: u32 = row.value.trim_end_matches("_u32").parse().expect("a recognized ARGB literal");
                    bytes.extend(argb.to_le_bytes());
                }
                RowKind::Double => number(&mut bytes, &row.value),
                RowKind::Thickness | RowKind::CornerRadius => {
                    for component in row.value[1..row.value.len() - 1].split(", ") {
                        number(&mut bytes, component);
                    }
                }
            }
        }
        Self { bytes, strings: strings.values, namespaces: namespaces.values }
    }

    pub(super) fn source(&self, name: &str, kind: RowKind, image: &super::super::BytecodeImage) -> String {
        let bytes = image.program(&self.bytes);
        format!("static {name}: rt::TypedResourceProgram<{}> = rt::TypedResourceProgram::new({bytes}, &[{}], &[{}]);\n",
                kind.row_type(), self.strings.join(", "), self.namespaces.join(", "))
    }
}
