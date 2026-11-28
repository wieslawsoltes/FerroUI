use super::TextArea;
use crate::{TextViewPosition, VisualLine, VisualLineError, VisualYPosition};
use ferroui_base::Point;
use ferroui_base::media::CharacterHit;
use ferroui_edit_core::document::TextDocument;
use std::rc::Rc;

impl TextArea {
    /// Returns the estimated document-space top of a one-based logical line.
    /// Rows measured by the layout contribute their actual heights.
    pub fn get_visual_top_by_document_line(&self, line: usize) -> Result<f64, VisualLineError> {
        let document = self.position_document()?;
        document.get_line_by_number(line)?;
        Ok(self.heights.borrow().top(line - 1))
    }

    /// Finds the visual line at a document-space Y coordinate in the arranged view.
    pub fn get_visual_line_from_visual_top(
        &self,
        visual_top: f64,
    ) -> Result<Option<Rc<VisualLine>>, VisualLineError> {
        Ok(self
            .visible_line_at_visual_top(visual_top)?
            .map(|(line, _)| line.display.clone()))
    }

    /// Projects a position into the document, including the selected side of a
    /// soft line break. Coordinates are device-independent pixels.
    pub fn get_visual_position(
        &self,
        position: TextViewPosition,
        y_mode: VisualYPosition,
    ) -> Result<Point, VisualLineError> {
        let document = self.position_document()?;
        document.get_line_by_number(position.line)?;
        let index = self.heights.borrow().visible_owner(position.line - 1);
        if self.heights.borrow().is_hidden(index) {
            return Err(VisualLineError::NoVisibleLine {
                line_number: position.line,
            });
        }
        let version = document.version();
        let line = self.try_line_layout(index)?;
        if self.position_document()? != document
            || document.version() != version
            || !self
                .cache
                .borrow()
                .get(&index)
                .is_some_and(|cached| Rc::ptr_eq(cached, &line))
        {
            return Err(VisualLineError::ConstructionInvalidated);
        }
        let visual = match position.visual_column {
            Some(column) => column,
            None => {
                let offset = document.get_offset(position.location())?;
                line.display
                    .get_visual_column(offset.saturating_sub(line.offset))
            }
        };
        let limit = line.display.visual_length_with_end_of_line_marker();
        let column = visual.min(limit) as i32;
        let rows = line.layout.text_lines();
        let row_index = if visual >= limit {
            rows.len().saturating_sub(1)
        } else {
            line.layout
                .get_line_index_from_character_index(column, position.is_at_end_of_line)
                .max(0) as usize
        };
        let row = rows.get(row_index).ok_or(VisualLineError::NoVisibleLine {
            line_number: position.line,
        })?;
        let origin = line.layout.line_origin(row_index);
        let row_top = self.heights.borrow().top(index) + origin.y;
        let row_height = row.height();
        let baseline = row_top + row.baseline();
        let metrics = self.default_text_metrics();
        let text_top = baseline - metrics.baseline;
        let y = match y_mode {
            VisualYPosition::LineTop => row_top,
            VisualYPosition::LineMiddle => row_top + row_height / 2.0,
            VisualYPosition::LineBottom => row_top + row_height,
            VisualYPosition::TextTop => text_top,
            VisualYPosition::TextMiddle => text_top + metrics.height / 2.0,
            VisualYPosition::TextBottom => text_top + metrics.height,
            VisualYPosition::Baseline => baseline,
        };
        let x = if visual > limit {
            row.start()
                + row.width_including_trailing_whitespace()
                + (visual - limit) as f64 * self.wide_space_width()
        } else {
            row.get_distance_from_character_hit(CharacterHit::new(column))
        };
        Ok(Point::new(origin.x + x, y))
    }

    /// Resolves a document-space point to the nearest text boundary in a visible line.
    pub fn get_position(&self, point: Point) -> Result<Option<TextViewPosition>, VisualLineError> {
        self.hit_test_visual_position(point, false)
    }

    /// Resolves a document-space point to the start of its shaped cluster.
    pub fn get_position_floor(
        &self,
        point: Point,
    ) -> Result<Option<TextViewPosition>, VisualLineError> {
        self.hit_test_visual_position(point, true)
    }

    fn position_document(&self) -> Result<TextDocument, VisualLineError> {
        self.editor()
            .map(|editor| editor.document())
            .ok_or_else(|| VisualLineError::Extension("The text area has no document".into()))
    }

    fn hit_test_visual_position(
        &self,
        point: Point,
        floor: bool,
    ) -> Result<Option<TextViewPosition>, VisualLineError> {
        let editor = self
            .editor()
            .ok_or_else(|| VisualLineError::Extension("The text area has no document".into()))?;
        if !point.x.is_finite() || !point.y.is_finite() {
            return Ok(None);
        }
        let Some((line, top)) = self.visible_line_at_visual_top(point.y)? else {
            return Ok(None);
        };
        let Some(row_index) = line.layout.line_index_at_y(point.y - top) else {
            return Ok(None);
        };
        let rows = line.layout.text_lines();
        let row = &rows[row_index];
        let x = point.x - line.layout.line_origin(row_index).x;
        let right = row.start() + row.width_including_trailing_whitespace();
        let row_end = (row.first_text_source_index() + row.length()).max(0) as usize;
        let limit = line.display.visual_length_with_end_of_line_marker();
        let (visual, at_end) =
            if x > right && editor.options().enable_virtual_space && row_index + 1 == rows.len() {
                let cells = (x - right) / self.wide_space_width();
                let extra = if floor { cells.floor() } else { cells.round() };
                let column = limit.saturating_add(extra as usize);
                (column, floor || column >= row_end)
            } else if floor && x > right {
                (row_end, true)
            } else {
                let hit = row.get_character_hit_from_distance(x);
                let trailing = if floor { 0 } else { hit.trailing_length() };
                let column = (hit.first_character_index() + trailing).max(0) as usize;
                (column.min(limit), !floor && column >= row_end)
            };
        let offset = line.offset + line.display.get_relative_offset(visual);
        let location = line.display.context().document().get_location(offset)?;
        let mut position = TextViewPosition::with_visual_column(location, visual);
        position.is_at_end_of_line = at_end;
        Ok(Some(position))
    }
}
