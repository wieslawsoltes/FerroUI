use std::fmt;

use ferroui_pretext::{LayoutCursor, PreparedTextWithSegments, PretextLayout};

use crate::{
    CircleObstacle, Interval, ObstacleLayoutHelper, PositionedLine, PreparedTextMetrics,
    RectObstacle,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutError {
    InvalidLineHeight,
    InvalidMaximumWidth,
    InvalidGeometry,
    InvalidMinimumSlotWidth,
    /// The next row cannot be represented as a strictly larger finite `f64`.
    CoordinateOverflow,
}

impl fmt::Display for LayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidLineHeight => "line height must be finite and positive",
            Self::InvalidMaximumWidth => "maximum width must be positive and at most 2^53",
            Self::InvalidGeometry => {
                "geometry must have finite coordinates and nonnegative dimensions"
            }
            Self::InvalidMinimumSlotWidth => "minimum slot width must be finite and nonnegative",
            Self::CoordinateOverflow => {
                "advancing the line band does not produce a larger finite coordinate"
            }
        })
    }
}

impl std::error::Error for LayoutError {}

/// Flow text through an ordered set of rectangular columns, avoiding obstacles.
pub struct ColumnFlowLayout;

impl ColumnFlowLayout {
    /// Text that does not fit the supplied columns is left unmaterialized. Fully
    /// blocked rows advance vertically without consuming source text. Exactly
    /// one (the widest) available horizontal slot is used on each row.
    pub fn layout_into_columns(
        prepared: &PreparedTextWithSegments,
        columns: &[RectObstacle],
        rectangles: &[RectObstacle],
        circles: &[CircleObstacle],
        line_height: f64,
        min_slot_width: f64,
    ) -> Result<Vec<PositionedLine>, LayoutError> {
        if !line_height.is_finite() || line_height <= 0.0 {
            return Err(LayoutError::InvalidLineHeight);
        }
        if !min_slot_width.is_finite() || min_slot_width < 0.0 {
            return Err(LayoutError::InvalidMinimumSlotWidth);
        }
        if columns.iter().chain(rectangles).any(|rect| {
            rect.width < 0.0
                || rect.height < 0.0
                || ![
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    rect.right(),
                    rect.bottom(),
                ]
                .iter()
                .all(|value| value.is_finite())
        }) || circles.iter().any(|circle| {
            circle.radius < 0.0
                || ![circle.x, circle.y, circle.radius]
                    .iter()
                    .all(|value| value.is_finite())
        }) {
            return Err(LayoutError::InvalidGeometry);
        }

        let mut lines = Vec::new();
        let mut cursor = LayoutCursor {
            segment_index: 0,
            grapheme_index: 0,
        };
        for &column in columns {
            // No row can acquire a usable slot in these cases. In particular,
            // do not iterate height / line_height times for a zero-width or
            // completely excluded column with a very small positive line height.
            if column.width <= 0.0
                || column.width < min_slot_width
                || column.height < line_height
                || rectangles.iter().any(|rect| {
                    rect.x <= column.x
                        && rect.right() >= column.right()
                        && rect.y <= column.y
                        && rect.bottom() >= column.bottom()
                })
            {
                continue;
            }
            let mut y = column.y;
            while !PreparedTextMetrics::is_end(prepared, cursor) {
                let next_y = y + line_height;
                if !next_y.is_finite() || next_y <= y {
                    return Err(LayoutError::CoordinateOverflow);
                }
                if next_y > column.bottom() {
                    break;
                }
                if let Some(slot) =
                    best_slot(column, y, next_y, rectangles, circles, min_slot_width)
                {
                    let Some(next_line) =
                        PretextLayout::layout_next_line(prepared, cursor, slot.width())
                    else {
                        return Ok(lines);
                    };
                    lines.push(PositionedLine {
                        text: next_line.text,
                        x: slot.left,
                        y,
                        width: next_line.width,
                    });
                    cursor = next_line.end;
                }
                y = next_y;
            }
        }
        Ok(lines)
    }
}

fn best_slot(
    column: RectObstacle,
    band_top: f64,
    band_bottom: f64,
    rectangles: &[RectObstacle],
    circles: &[CircleObstacle],
    min_slot_width: f64,
) -> Option<Interval> {
    let rectangles = rectangles.iter().filter_map(|rect| {
        (rect.height > 0.0 && band_bottom > rect.y && band_top < rect.bottom()).then_some(
            Interval {
                left: rect.x,
                right: rect.right(),
            },
        )
    });
    let circles = circles.iter().filter_map(|circle| {
        ObstacleLayoutHelper::circle_interval_for_band(
            circle.x,
            circle.y,
            circle.radius,
            band_top,
            band_bottom,
            0.0,
            0.0,
        )
    });
    let slots = ObstacleLayoutHelper::carve_text_line_slots(
        Interval {
            left: column.x,
            right: column.right(),
        },
        rectangles.chain(circles),
        min_slot_width,
    );
    ObstacleLayoutHelper::pick_slot(&slots, false)
}
