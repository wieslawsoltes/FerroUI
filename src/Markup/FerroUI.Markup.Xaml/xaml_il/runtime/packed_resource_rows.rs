//! Packed row tables of theme resources (#855): the four deferred shapes of
//! `theme_resources.rs` as records of 16-bit fields over one pool of the strings of the table,
//! instead of a slice of rows with sized strings, optional namespace slices and 32-bit
//! positions. Not a port of an upstream file.
//!
//! A table is a [`PackedRows`]: `strings` holds every key and target of the table, each once,
//! `namespaces` the namespace tables its rows declare, and `records` one record of 16-bit fields
//! for each row, in the order of the rows. A string is its byte offset and its length in the
//! pool; a namespace table is `0` (the namespaces of the document) or its index in `namespaces`
//! plus one; a colour is its low and its high half. The emitter writes a table only when every
//! value fits (an offset, a length, a line or a position beyond 16 bits keeps the slice of rows),
//! and the reader checks every range: a record that does not decode is the error of its table,
//! not a panic.
//!
//! | Shape | Record |
//! |---|---|
//! | [`ResourceAliasRow`] | key offset, key length, target offset, target length, namespaces, line, position, value line, value position |
//! | [`LiteralBrushRow`] | key offset, key length, colour low, colour high, namespaces, line, position |
//! | [`DynamicBrushRow`], [`StaticBrushRow`] | key offset, key length, colour key offset, colour key length, namespaces, line, position, value line, value position |

use std::marker::PhantomData;

use super::compact::RowSource;
use super::{DynamicBrushRow, LiteralBrushRow, ResourceAliasRow, StaticBrushRow};
use crate::xaml_il::runtime::compiled::XmlNamespaceTable;
use crate::XamlLoadError;

/// A packed table of deferred theme resources of one shape (#855; module documentation of
/// `packed_resource_rows.rs` for the layout). Written by the emitter, read by
/// `rt::add_packed_resource_aliases`, `rt::add_packed_literal_brushes`,
/// `rt::add_packed_dynamic_brushes` and `rt::add_packed_static_brushes`.
#[derive(Clone, Copy, Debug)]
pub struct PackedRows {
    /// The keys and targets of the rows, each once, concatenated.
    pub strings: &'static str,
    /// The namespace tables the rows declare; a record names one by its index plus one.
    pub namespaces: &'static [XmlNamespaceTable],
    /// The records of the rows, in order, each as wide as its shape states.
    pub records: &'static [u16],
}

/// The error of a row of a packed table that cannot be read.
pub(super) fn out_of_range(index: usize) -> XamlLoadError {
    XamlLoadError::with_message(format!("row {index} of a compiled resource table is out of range"))
}

/// A row shape that has a packed record.
pub(super) trait Packed: Sized {
    /// The number of 16-bit fields of a record.
    const WIDTH: usize;
    /// The row of `fields`, a record of `table`, if its strings and namespaces are in range.
    fn decode(table: &PackedRows, fields: &[u16]) -> Option<Self>;
}

fn string(table: &PackedRows, offset: u16, length: u16) -> Option<&'static str> {
    let start = usize::from(offset);
    table.strings.get(start..start.checked_add(usize::from(length))?)
}

fn namespaces(table: &PackedRows, index: u16) -> Option<Option<XmlNamespaceTable>> {
    match index {
        0 => Some(None),
        index => table.namespaces.get(usize::from(index) - 1).copied().map(Some),
    }
}

impl Packed for ResourceAliasRow {
    const WIDTH: usize = 9;
    fn decode(table: &PackedRows, fields: &[u16]) -> Option<Self> {
        let &[key, key_length, target, target_length, scope, line, position, value_line, value_position] = fields else {
            return None;
        };
        Some(Self {
            key: string(table, key, key_length)?,
            target: string(table, target, target_length)?,
            namespaces: namespaces(table, scope)?,
            line: line.into(),
            position: position.into(),
            value_line: value_line.into(),
            value_position: value_position.into(),
        })
    }
}

impl Packed for LiteralBrushRow {
    const WIDTH: usize = 7;
    fn decode(table: &PackedRows, fields: &[u16]) -> Option<Self> {
        let &[key, key_length, low, high, scope, line, position] = fields else {
            return None;
        };
        Some(Self {
            key: string(table, key, key_length)?,
            color: u32::from(low) | (u32::from(high) << 16),
            namespaces: namespaces(table, scope)?,
            line: line.into(),
            position: position.into(),
        })
    }
}

macro_rules! packed_brush {
    ($row:ident) => {
        impl Packed for $row {
            const WIDTH: usize = 9;
            fn decode(table: &PackedRows, fields: &[u16]) -> Option<Self> {
                let &[key, key_length, color, color_length, scope, line, position, value_line, value_position] = fields
                else {
                    return None;
                };
                Some(Self {
                    key: string(table, key, key_length)?,
                    color_key: string(table, color, color_length)?,
                    namespaces: namespaces(table, scope)?,
                    line: line.into(),
                    position: position.into(),
                    value_line: value_line.into(),
                    value_position: value_position.into(),
                })
            }
        }
    };
}
packed_brush!(DynamicBrushRow);
packed_brush!(StaticBrushRow);

/// The rows of a packed table, read as rows of `R`.
pub(super) struct PackedSource<R> {
    table: &'static PackedRows,
    shape: PhantomData<fn() -> R>,
}

impl<R> Clone for PackedSource<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R> Copy for PackedSource<R> {}

impl<R> PackedSource<R> {
    pub(super) fn new(table: &'static PackedRows) -> Self {
        Self { table, shape: PhantomData }
    }
}

impl<R: Packed + super::compact::Row + Copy + 'static> RowSource for PackedSource<R> {
    type Row = R;
    fn len(&self) -> usize {
        self.table.records.len() / R::WIDTH
    }
    fn row(&self, index: usize) -> Result<R, XamlLoadError> {
        let start = index.checked_mul(R::WIDTH).ok_or_else(|| out_of_range(index))?;
        let fields = self.table.records.get(start..start + R::WIDTH).ok_or_else(|| out_of_range(index))?;
        R::decode(self.table, fields).ok_or_else(|| out_of_range(index))
    }
}
