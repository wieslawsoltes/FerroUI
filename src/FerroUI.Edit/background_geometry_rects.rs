//! Document/visual ranges projected through native shaped text rows.
//! Copyright (c) 2014 AlphaSierraPapa for the SharpDevelop Team; MIT, see NOTICE.md.

use crate::VisualLine;
use crate::background_geometry_builder::{BackgroundGeometryError, validate_rectangle};
use ferroui_base::media::CharacterHit;
use ferroui_base::media::text_formatting::{TextLayout, TextLine};
use ferroui_base::{Point, Rect};
use ferroui_edit_core::document::ISegment;
use ferroui_edit_core::editing::SelectionSegment;

/// A retained visual line and its matching native layout. `origin` is its
/// displayed top-left, including gutter translation and horizontal/vertical
/// scrolling. No view or document borrow is retained by projection results.
#[derive(Clone, Copy)]
pub struct BackgroundGeometryLine<'a> {
    pub visual_line: &'a VisualLine,
    pub layout: &'a TextLayout,
    pub origin: Point,
}

/// Metrics used for native selection/background rectangle projection.
#[derive(Clone, Copy, Debug)]
pub struct BackgroundGeometryViewport {
    /// Content width before scrolling, normally max(content extent, viewport).
    /// Exclude the gutter when origins include the gutter's translation.
    pub full_width: f64,
    pub default_line_height: f64,
    pub wide_space_width: f64,
    pub empty_line_selection_width: f64,
}
impl Default for BackgroundGeometryViewport {
    fn default() -> Self {
        Self {
            full_width: 0.0,
            default_line_height: 0.0,
            wide_space_width: 1.0,
            empty_line_selection_width: 1.0,
        }
    }
}
impl BackgroundGeometryViewport {
    fn validate(self) -> Result<(), BackgroundGeometryError> {
        if ![
            self.full_width,
            self.default_line_height,
            self.wide_space_width,
            self.empty_line_selection_width,
        ]
        .into_iter()
        .all(f64::is_finite)
            || self.full_width < 0.0
            || self.default_line_height < 0.0
            || self.wide_space_width <= 0.0
            || self.empty_line_selection_width < 0.0
        {
            return Err(BackgroundGeometryError::InvalidViewport);
        }
        Ok(())
    }
}

/// Projects a UTF-16 document segment over supplied retained lines in document
/// order. Offscreen lines are never constructed. Offsets are clamped to each
/// retained line snapshot's document, matching upstream range coercion.
pub fn get_rects_for_segment<'a>(
    lines: impl IntoIterator<Item = BackgroundGeometryLine<'a>>,
    segment: &dyn ISegment,
    viewport: BackgroundGeometryViewport,
    extend_to_full_width_at_line_end: bool,
) -> Result<Vec<Rect>, BackgroundGeometryError> {
    rects_for_endpoints(
        lines,
        segment.offset(),
        segment.end_offset(),
        0,
        0,
        viewport,
        extend_to_full_width_at_line_end,
    )
}

/// Preserves directed selections' virtual-space endpoints after ordering them.
/// Virtual spaces are physical cells after the final formatted row's right edge,
/// consistent with native rectangular selection and RTL virtual caret placement.
pub fn get_rects_for_selection<'a>(
    lines: impl IntoIterator<Item = BackgroundGeometryLine<'a>>,
    selection: SelectionSegment,
    viewport: BackgroundGeometryViewport,
    extend_to_full_width_at_line_end: bool,
) -> Result<Vec<Rect>, BackgroundGeometryError> {
    rects_for_endpoints(
        lines,
        selection.start_offset(),
        selection.end_offset(),
        selection.start_virtual_space(),
        selection.end_virtual_space(),
        viewport,
        extend_to_full_width_at_line_end,
    )
}

fn rects_for_endpoints<'a>(
    lines: impl IntoIterator<Item = BackgroundGeometryLine<'a>>,
    start: usize,
    end: usize,
    start_virtual: usize,
    end_virtual: usize,
    viewport: BackgroundGeometryViewport,
    extend: bool,
) -> Result<Vec<Rect>, BackgroundGeometryError> {
    viewport.validate()?;
    if start > end {
        return Err(BackgroundGeometryError::InvalidVisualRange);
    }
    let mut result = Vec::new();
    for line in lines {
        let document_length = line.visual_line.context().document().text_length();
        let start = start.min(document_length);
        let end = end.min(document_length);
        let line_start = line.visual_line.first_document_line().offset();
        let line_end = line.visual_line.last_document_line().end_offset();
        if line_start > end {
            break;
        }
        if line_end < start {
            continue;
        }
        let visual_length = line.visual_line.visual_length_with_end_of_line_marker();
        let column_at = |offset: usize, extra: usize| {
            if extra != 0 && offset == line_end {
                visual_length
                    .checked_add(extra)
                    .filter(|column| *column != usize::MAX)
                    .ok_or(BackgroundGeometryError::InvalidVisualRange)
            } else {
                Ok(line.visual_line.get_visual_column(offset - line_start))
            }
        };
        let start_column = if start < line_start {
            0
        } else {
            column_at(start, start_virtual)?
        };
        let end_column = if end > line_end {
            if extend { usize::MAX } else { visual_length }
        } else {
            column_at(end, end_virtual)?
        };
        result.extend(get_rects_from_visual_segment(
            line,
            start_column,
            end_column,
            viewport,
        )?);
    }
    Ok(result)
}

/// Projects visual UTF-16 columns using native text bounds. A column after the
/// visual length denotes virtual space; `usize::MAX` as the end column requests
/// extension to `full_width`. Wrapped rows trim hidden trailing whitespace and
/// use full width when the range continues onto the following row.
pub fn get_rects_from_visual_segment(
    line: BackgroundGeometryLine<'_>,
    start_column: usize,
    end_column: usize,
    viewport: BackgroundGeometryViewport,
) -> Result<Vec<Rect>, BackgroundGeometryError> {
    viewport.validate()?;
    if start_column > end_column || start_column == usize::MAX {
        return Err(BackgroundGeometryError::InvalidVisualRange);
    }
    if !line.origin.x.is_finite() || !line.origin.y.is_finite() {
        return Err(BackgroundGeometryError::InvalidViewport);
    }
    let rows = line.layout.text_lines();
    let visual_length = line.visual_line.visual_length_with_end_of_line_marker();
    let mut result = Vec::new();
    let mut top = line.origin.y;
    for (index, row) in rows.iter().enumerate() {
        let last = index + 1 == rows.len();
        let row_start = usize::try_from(row.first_text_source_index())
            .map_err(|_| BackgroundGeometryError::InvalidVisualRange)?;
        let row_length = usize::try_from(row.length())
            .map_err(|_| BackgroundGeometryError::InvalidVisualRange)?;
        let trailing = row.trailing_whitespace_length().max(0) as usize;
        let row_end = row_start
            .checked_add(row_length)
            .ok_or(BackgroundGeometryError::InvalidVisualRange)?
            .saturating_sub(if last { 1 } else { trailing })
            .min(visual_length)
            .max(row_start);
        if end_column < row_start {
            break;
        }
        let y = top;
        top += row.height();
        if !last && start_column > row_end {
            continue;
        }
        let height = row.height().max(viewport.default_line_height);
        let start = start_column.max(row_start);
        let end = end_column.min(row_end);
        let mut pending = None;
        if start == end {
            // Avoid the duplicate zero-width marker at an unspaced soft-wrap
            // boundary when a selection crosses rather than ends there.
            if end == row_end && !last && end_column > end && trailing == 0 {
                continue;
            }
            if start == row_start
                && index != 0
                && start_column < start
                && rows[index - 1].trailing_whitespace_length() == 0
            {
                continue;
            }
            let x = x_for_column(
                row.as_ref(),
                start,
                visual_length,
                viewport.wide_space_width,
            )?;
            pending = Some(Rect::new(
                line.origin.x + x,
                y,
                viewport.empty_line_selection_width,
                height,
            ));
        } else if start <= row_end && end >= start {
            let start =
                i32::try_from(start).map_err(|_| BackgroundGeometryError::InvalidVisualRange)?;
            let length = i32::try_from(end - start as usize)
                .map_err(|_| BackgroundGeometryError::InvalidVisualRange)?;
            for bounds in row.get_text_bounds(start, length) {
                let bounds = bounds.rectangle();
                let left = line.origin.x + bounds.x;
                let right = line.origin.x + bounds.right();
                if let Some(previous) =
                    pending.replace(Rect::new(left.min(right), y, (right - left).abs(), height))
                {
                    result.push(previous);
                }
            }
        }
        if end_column > row_end {
            let left = if start_column > visual_length {
                // Only the final row can contain virtual space.
                let last_row = rows.last().expect("the current row exists");
                x_for_column(
                    last_row.as_ref(),
                    start_column,
                    visual_length,
                    viewport.wide_space_width,
                )?
            } else {
                row.start()
                    + if last {
                        row.width_including_trailing_whitespace()
                    } else {
                        row.width()
                    }
            };
            let right = if !last || end_column == usize::MAX {
                viewport.full_width
            } else {
                x_for_column(
                    row.as_ref(),
                    end_column,
                    visual_length,
                    viewport.wide_space_width,
                )?
            };
            let extension = Rect::new(
                line.origin.x + left.min(right),
                y,
                (right - left).abs(),
                height,
            );
            if let Some(previous) = pending {
                if extension.intersects(previous) {
                    result.push(previous.union(extension));
                } else {
                    // A trailing RTL run can leave the virtual extension
                    // separate from the logical range's final text rectangle.
                    result.push(previous);
                    result.push(extension);
                }
            } else {
                result.push(extension);
            }
        } else if let Some(previous) = pending {
            result.push(previous);
        }
    }
    for rectangle in &result {
        validate_rectangle(*rectangle)?;
    }
    Ok(result)
}

fn x_for_column(
    row: &dyn TextLine,
    column: usize,
    visual_length: usize,
    space_width: f64,
) -> Result<f64, BackgroundGeometryError> {
    let x = if column > visual_length {
        row.start()
            + row.width_including_trailing_whitespace()
            + (column - visual_length) as f64 * space_width
    } else {
        let column =
            i32::try_from(column).map_err(|_| BackgroundGeometryError::InvalidVisualRange)?;
        row.get_distance_from_character_hit(CharacterHit::new(column))
    };
    if x.is_finite() {
        Ok(x)
    } else {
        Err(BackgroundGeometryError::CoordinateOverflow)
    }
}
