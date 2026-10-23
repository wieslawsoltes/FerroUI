//! Responsive editor guides, independent of document auto layout.
//!
//! The fixed/stretch alignment contract follows Figma's public LayoutGrid API:
//! https://developers.figma.com/docs/plugins/api/LayoutGrid/
//! The portable representation uses `None` for an automatic section count rather
//! than a non-JSON infinity. The renderer resolves only visible bands, so even a
//! million authored sections never allocates a million rectangles.
use crate::{DocumentError, Editor, LayoutGrid, Node, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

mod constraints;
pub use constraints::*;
mod bindings;
pub use bindings::*;
mod snapping;
pub use snapping::*;

pub const MAX_LAYOUT_GUIDES: usize = 64;
pub const MAX_LAYOUT_GUIDE_SECTIONS: u32 = 1_000_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutGridAlignment {
    Start,
    Center,
    End,
    #[default]
    Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutGridAxis {
    pub alignment: LayoutGridAlignment,
    pub count: Option<u32>,
    pub gutter: f64,
    /// Margin on both edges for stretch; distance from the selected edge for
    /// start/end; ignored for centered guides.
    pub offset: f64,
}

impl Default for LayoutGridAxis {
    fn default() -> Self {
        Self { alignment: LayoutGridAlignment::Stretch, count: Some(12), gutter: 20.0, offset: 0.0 }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LayoutGridPattern {
    #[default]
    Square,
    Columns { #[serde(flatten)] axis: LayoutGridAxis },
    Rows { #[serde(flatten)] axis: LayoutGridAxis },
}

impl LayoutGridPattern {
    pub fn is_square(&self) -> bool { matches!(self, Self::Square) }
    pub fn axis(self) -> Option<LayoutGridAxis> {
        match self { Self::Square => None, Self::Columns { axis } | Self::Rows { axis } => Some(axis) }
    }
    pub fn with_axis(self, axis: LayoutGridAxis) -> Self {
        match self { Self::Square => Self::Square, Self::Columns { .. } => Self::Columns { axis }, Self::Rows { .. } => Self::Rows { axis } }
    }
    pub fn validate(self) -> Result<(), DocumentError> {
        if let Some(axis) = self.axis() {
            if axis.count.is_some_and(|count| count == 0 || count > MAX_LAYOUT_GUIDE_SECTIONS)
                || !axis.gutter.is_finite() || axis.gutter < 0.0
                || !axis.offset.is_finite() || axis.offset < 0.0 {
                return Err(DocumentError::Invalid(format!("Layout guide count must be Auto or 1–{MAX_LAYOUT_GUIDE_SECTIONS}; gutter and margin/offset must be nonnegative and finite.")));
            }
        }
        Ok(())
    }
}

/// Constant-size description of equally spaced guide bands in local coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutGridBands {
    pub start: f64,
    pub size: f64,
    pub step: f64,
    pub count: u32,
}

impl LayoutGrid {
    /// Resolve frame-relative row/column bands without allocating geometry.
    /// An undersized stretch frame produces no bands until it has positive
    /// section space. Fixed guides may extend beyond the frame and are clipped.
    pub fn bands(self, extent: f64) -> Option<LayoutGridBands> {
        self.validate().ok()?;
        let axis = self.pattern.axis()?;
        if !extent.is_finite() || extent <= 0.0 { return None; }
        let available = match axis.alignment {
            LayoutGridAlignment::Center => extent,
            LayoutGridAlignment::Stretch => extent - 2.0 * axis.offset,
            _ => extent - axis.offset,
        };
        let count = match axis.count {
            Some(count) => count,
            None => {
                let count = ((available + axis.gutter) / (self.size + axis.gutter)).floor();
                if !count.is_finite() || count < 1.0 || count > u32::MAX as f64 { return None; }
                count as u32
            }
        };
        let size = if axis.alignment == LayoutGridAlignment::Stretch {
            (available - (count - 1) as f64 * axis.gutter) / count as f64
        } else { self.size };
        let step = size + axis.gutter;
        let total = count as f64 * size + (count - 1) as f64 * axis.gutter;
        let start = match axis.alignment {
            LayoutGridAlignment::Start | LayoutGridAlignment::Stretch => axis.offset,
            LayoutGridAlignment::Center => (extent - total) / 2.0,
            LayoutGridAlignment::End => extent - axis.offset - total,
        };
        if ![size, step, total, start].iter().all(|value| value.is_finite())
            || size <= 0.0 || step <= 0.0 || start + size <= start { return None; }
        Some(LayoutGridBands { start, size, step, count })
    }
}

impl LayoutGridBands {
    /// The visible section interval, computed arithmetically in O(1). Callers
    /// apply their own density/work budget before drawing the returned range.
    pub fn visible_range(self, start: f64, end: f64) -> std::ops::Range<u32> {
        if !start.is_finite() || !end.is_finite() || start >= end { return 0..0; }
        let first = ((start - self.start - self.size) / self.step).floor() + 1.0;
        let last = ((end - self.start) / self.step).ceil();
        let first = first.clamp(0.0, self.count as f64) as u32;
        let last = last.clamp(first as f64, self.count as f64) as u32;
        first..last
    }
}

pub fn validate_layout_grids(grids: &[LayoutGrid]) -> Result<(), DocumentError> {
    if grids.len() > MAX_LAYOUT_GUIDES {
        return Err(DocumentError::Invalid(format!("A frame supports at most {MAX_LAYOUT_GUIDES} layout guides.")));
    }
    for grid in grids { grid.validate()?; }
    Ok(())
}

impl Node {
    pub fn layout_guides(&self) -> impl DoubleEndedIterator<Item = &LayoutGrid> {
        self.layout_grid.iter().chain(self.layout_grid_overlays.iter())
    }
    pub fn layout_guide(&self, index: usize) -> Option<&LayoutGrid> {
        if index == 0 { self.layout_grid.as_ref() } else { self.layout_grid_overlays.get(index - 1) }
    }
    pub fn layout_guide_mut(&mut self, index: usize) -> Option<&mut LayoutGrid> {
        if index == 0 { self.layout_grid.as_mut() } else { self.layout_grid_overlays.get_mut(index - 1) }
    }
    pub fn layout_guide_count(&self) -> usize {
        usize::from(self.layout_grid.is_some()) + self.layout_grid_overlays.len()
    }
    /// Set a previously validated stack while retaining the legacy first field.
    pub fn set_layout_guides(&mut self, grids: &[LayoutGrid]) {
        self.layout_grid = grids.first().copied();
        self.layout_grid_overlays = grids.get(1..).unwrap_or_default().to_vec();
    }
}

/// Change guide orientation while retaining the target's current axis settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutGridKind {
    Square,
    Columns,
    Rows,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LayoutGuideProperty {
    Kind(LayoutGridKind),
    Pattern(LayoutGridPattern),
    Size(f64),
    Color(crate::Color),
    Visible(bool),
    Alignment(LayoutGridAlignment),
    Count(Option<u32>),
    Gutter(f64),
    Offset(f64),
}

impl LayoutGuideProperty {
    fn validate(self) -> Result<(), DocumentError> {
        // Check even when a mixed selection has no matching row, so malformed
        // commands cannot become silently accepted depending on selection.
        let mut probe = LayoutGrid { pattern: LayoutGridPattern::Columns { axis: LayoutGridAxis::default() }, ..LayoutGrid::default() };
        self.apply(&mut probe);
        probe.validate()
    }

    fn apply(self, grid: &mut LayoutGrid) {
        match self {
            Self::Kind(kind) => {
                let axis = grid.pattern.axis().unwrap_or_default();
                grid.pattern = match kind {
                    LayoutGridKind::Square => LayoutGridPattern::Square,
                    LayoutGridKind::Columns => LayoutGridPattern::Columns { axis },
                    LayoutGridKind::Rows => LayoutGridPattern::Rows { axis },
                };
            }
            Self::Pattern(pattern) => grid.pattern = pattern,
            Self::Size(size) => grid.size = size,
            Self::Color(color) => grid.color = color,
            Self::Visible(visible) => grid.visible = visible,
            _ => {
                let Some(mut axis) = grid.pattern.axis() else { return; };
                match self {
                    Self::Alignment(value) => axis.alignment = value,
                    Self::Count(value) => axis.count = value,
                    Self::Gutter(value) => axis.gutter = value,
                    Self::Offset(value) => axis.offset = value,
                    _ => unreachable!(),
                }
                grid.pattern = grid.pattern.with_axis(axis);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LayoutGuideEdit {
    Add(LayoutGrid),
    Set { index: usize, property: LayoutGuideProperty },
    Remove(usize),
    Duplicate(usize),
    Move { index: usize, to: usize },
    Clear,
}

impl LayoutGuideEdit {
    pub fn apply(self, grids: &mut Vec<LayoutGrid>) -> Result<(), DocumentError> {
        if let Self::Set { property, .. } = self { property.validate()?; }
        if let Self::Add(grid) = self { grid.validate()?; }
        match self {
            Self::Add(grid) => grids.push(grid),
            Self::Set { index, property } => {
                if let Some(grid) = grids.get_mut(index) { property.apply(grid); }
            }
            Self::Remove(index) if index < grids.len() => { grids.remove(index); }
            Self::Duplicate(index) if index < grids.len() => { grids.insert(index, grids[index]); }
            Self::Move { index, to } if index < grids.len() && to < grids.len() => {
                let grid = grids.remove(index); grids.insert(to, grid);
            }
            Self::Clear => grids.clear(),
            _ => {}
        }
        validate_layout_grids(grids)
    }
}

impl Editor {
    /// Validate every changed target before mutation. Per-index visibility is
    /// local; geometry, colors and structural changes detach the shared style.
    /// Missing rows on a mixed selection are untouched rather than fabricated.
    pub fn edit_layout_guides(&mut self, ids: &[NodeId], edit: LayoutGuideEdit) -> Result<usize, DocumentError> {
        if let LayoutGuideEdit::Set { property, .. } = edit { property.validate()?; }
        if let LayoutGuideEdit::Add(grid) = edit { grid.validate()?; }
        let mut changes = Vec::new();
        for id in ids.iter().copied().collect::<BTreeSet<_>>() {
            let node = self.document.nodes.get(&id).ok_or(DocumentError::MissingNode(id))?;
            if self.document.effective_locked(id) || !LayoutGrid::supports(node.kind) { continue; }
            let mut grids: Vec<_> = node.layout_guides().copied().collect();
            let before = grids.clone();
            edit.apply(&mut grids)?;
            let bindings = bindings::edited_bindings(node, edit);
            if grids != before || bindings != node.variable_bindings || (matches!(edit, LayoutGuideEdit::Clear) && node.grid_style.is_some()) {
                changes.push((id, (grids, bindings)));
            }
        }
        if changes.is_empty() { return Ok(0); }
        let count = changes.len();
        let map: std::collections::BTreeMap<_, _> = changes.into_iter().collect();
        let ids: Vec<_> = map.keys().copied().collect();
        self.update_nodes(&ids, "Edit layout guides", |node| {
            let (grids, bindings) = &map[&node.id];
            node.set_layout_guides(grids);
            node.variable_bindings = bindings.clone();
            if node.layout_grid.is_none() { node.grid_style = None; }
            node.overridden_properties.insert(if matches!(edit, LayoutGuideEdit::Set { property: LayoutGuideProperty::Visible(_), .. }) {
                "layout_grid_visible"
            } else { "layout_grid" }.into());
        });
        Ok(count)
    }
}

#[cfg(test)]
#[path = "layout_guides_tests.rs"]
mod tests;
