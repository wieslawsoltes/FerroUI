//! Explicit grid tracks and atomic native-authoring commands. Empty track
//! lists retain the legacy grid geometry and serialized document shape.
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum GridTrack {
    /// A fixed logical-pixel track, including an intentional zero-width track.
    Fixed(f64),
    /// Size from the intrinsic demand of children occupying this track.
    Content,
    /// A positive share of the remaining space on a fixed-size axis.
    Fraction(f64),
}

impl GridTrack {
    pub fn validate(self) -> Result<(), DocumentError> {
        let valid = match self {
            Self::Fixed(value) => value.is_finite() && value >= 0.0,
            Self::Content => true,
            Self::Fraction(value) => value.is_finite() && value > 0.0,
        };
        if valid {
            Ok(())
        } else {
            Err(DocumentError::Invalid(
                "Grid tracks need a finite nonnegative size or a positive fraction weight".into(),
            ))
        }
    }
}

impl AutoLayout {
    pub fn has_explicit_grid_tracks(&self) -> bool {
        !self.column_tracks.is_empty() || !self.row_tracks.is_empty()
    }

    pub fn validate_grid_tracks(&self) -> Result<(), DocumentError> {
        if self.column_tracks.len() > MAX_GRID_COLUMNS
            || self.row_tracks.len() > MAX_GRID_ROWS
            || (self.axis == LayoutAxis::Grid && self.column_tracks.len() > self.columns)
            || (self.axis == LayoutAxis::Grid
                && self
                    .row_tracks
                    .len()
                    .checked_mul(self.columns)
                    .is_none_or(|cells| cells > MAX_GRID_CELLS))
        {
            return Err(DocumentError::Invalid(
                "Explicit tracks exceed the grid's column, row or cell bounds".into(),
            ));
        }
        for track in self.column_tracks.iter().chain(&self.row_tracks) {
            track.validate()?;
        }
        Ok(())
    }
}

/// Comma-separated authoring syntax: `120, auto, 2fr`. A blank list removes
/// explicit tracks. Empty items are rejected so an accidental double comma
/// cannot silently shift pinned cells to a different column.
pub fn parse_grid_tracks(value: &str, maximum: usize) -> Result<Vec<GridTrack>, DocumentError> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    for token in value.split(',') {
        if result.len() >= maximum.min(MAX_GRID_ROWS) {
            return Err(DocumentError::Invalid(format!(
                "Use at most {maximum} grid tracks"
            )));
        }
        let token = token.trim().to_ascii_lowercase();
        let track = if token == "auto" || token == "content" || token == "hug" {
            GridTrack::Content
        } else if let Some(weight) = token.strip_suffix("fr") {
            GridTrack::Fraction(weight.trim().parse::<f64>().map_err(|_| {
                DocumentError::Invalid("A fractional track needs a number before fr".into())
            })?)
        } else {
            GridTrack::Fixed(
                token
                    .strip_suffix("px")
                    .unwrap_or(&token)
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| {
                        DocumentError::Invalid(
                            "Use a size, auto, or a fraction such as 2fr for each grid track"
                                .into(),
                        )
                    })?,
            )
        };
        track.validate()?;
        result.push(track);
    }
    Ok(result)
}

pub fn format_grid_tracks(tracks: &[GridTrack]) -> String {
    tracks
        .iter()
        .map(|track| match track {
            GridTrack::Fixed(value) => value.to_string(),
            GridTrack::Content => "auto".into(),
            GridTrack::Fraction(value) => format!("{value}fr"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

impl Editor {
    /// Replace both explicit track lists together. Authored columns establish
    /// the column count; an empty column list keeps the current count. Existing
    /// pins and spans must fit the new columns, otherwise nothing is changed.
    pub fn set_grid_tracks(
        &mut self,
        parent: NodeId,
        columns: Vec<GridTrack>,
        rows: Vec<GridTrack>,
    ) -> Result<LayoutResult, DocumentError> {
        if self.has_active_transaction() {
            return Err(DocumentError::Invalid(
                "Finish the active edit before changing grid tracks".into(),
            ));
        }
        crate::layout_drag::validate_layout_reorder_ancestry(&self.document, parent)?;
        let node = self
            .document
            .nodes
            .get(&parent)
            .ok_or(DocumentError::MissingNode(parent))?;
        let original = node
            .layout
            .as_ref()
            .filter(|layout| layout.axis == LayoutAxis::Grid)
            .ok_or_else(|| DocumentError::Invalid("Select a grid auto-layout frame".into()))?;
        let mut layout = original.clone();
        if !columns.is_empty() {
            layout.columns = columns.len();
        }
        layout.column_tracks = columns;
        layout.row_tracks = rows;
        layout.validate_grid_tracks()?;
        if layout == *original {
            return Ok(LayoutResult::default());
        }
        let mut candidate = self.document.clone();
        candidate.nodes.get_mut(&parent).unwrap().layout = Some(layout);
        candidate.validate_layout_node_settings(parent)?;
        self.commit_layout_candidate("Edit grid tracks", candidate)
    }
}

#[cfg(test)]
#[path = "grid_tracks_tests.rs"]
mod tests;
