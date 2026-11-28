//! The parts of a text-pattern range that do not depend on the text model: unit
//! boundaries, moves by unit, the single selection and the geometry of a text view
//! in the coordinates of its visual root. The rich text range (`rich_text_automation`)
//! and the plain text range of `TextBox` (`plain_text_automation`) share them.

use super::{ProviderError, TextAttributeValue, TextProviderError};
use ferroui_base::media::{Color, IBrush, TextAlignment};
use ferroui_base::{Matrix, Rect, Ref, Visual};
use std::ops::Range;
use std::rc::Rc;

pub(super) fn paragraph_units(text: &str) -> Vec<Range<usize>> {
    crate::rich_text::rich_text_unit_ranges(text, crate::rich_text::UnicodeTextUnit::Paragraph)
}

pub(super) fn enclosing_unit_index(units: &[Range<usize>], position: usize) -> Option<usize> {
    units
        .iter()
        .position(|unit| unit.start <= position && (position < unit.end || unit.is_empty()))
        .or_else(|| units.len().checked_sub(1))
}

pub(super) fn enclosing_unit(units: &[Range<usize>], position: usize) -> Option<&Range<usize>> {
    enclosing_unit_index(units, position).map(|index| &units[index])
}

pub(super) fn move_position(units: &[Range<usize>], position: usize, count: i32) -> (usize, i32) {
    if count == 0 {
        return (position, 0);
    }
    let mut boundaries: Vec<_> = units
        .iter()
        .flat_map(|range| [range.start, range.end])
        .collect();
    boundaries.sort_unstable();
    boundaries.dedup();
    let remaining = count.unsigned_abs() as usize;
    if count > 0 {
        let first = boundaries.partition_point(|value| *value <= position);
        let moved = remaining.min(boundaries.len() - first);
        if moved == 0 {
            (position, 0)
        } else {
            (boundaries[first + moved - 1], moved as i32)
        }
    } else {
        let end = boundaries.partition_point(|value| *value < position);
        let moved = remaining.min(end);
        if moved == 0 {
            (position, 0)
        } else {
            (boundaries[end - moved], (-(moved as i64)) as i32)
        }
    }
}

/// `ITextRangeProvider::Move`: a degenerate range moves its position by `count`
/// boundaries; any other range becomes the unit `count` units away from the unit
/// at its start. Returns the new range (`None`: the range stays as it is) and the
/// number of units moved.
pub(super) fn move_range_by(
    units: Vec<Range<usize>>,
    range: Range<usize>,
    count: i32,
) -> (Option<Range<usize>>, i32) {
    if count == 0 || units.is_empty() {
        return (None, 0);
    }
    if range.is_empty() {
        let (position, moved) = move_position(&units, range.start, count);
        return (Some(position..position), moved);
    }
    let nonempty: Vec<_> = units.into_iter().filter(|unit| !unit.is_empty()).collect();
    let Some(current) = enclosing_unit_index(&nonempty, range.start) else {
        return (None, 0);
    };
    let target = (current as i128 + count as i128)
        .clamp(0, nonempty.len().saturating_sub(1) as i128) as usize;
    let moved = target as i128 - current as i128;
    if moved == 0 {
        (None, 0)
    } else {
        (Some(nonempty[target].clone()), moved as i32)
    }
}

/// The distance between two endpoints, as `CompareEndpoints` reports it.
pub(super) fn endpoint_distance(left: usize, right: usize) -> i32 {
    (left as i128 - right as i128).clamp(i32::MIN as i128, i32::MAX as i128) as i32
}

/// `AddToSelection` of a control with one contiguous selection: the selection
/// after `added` is added to `current`.
pub(super) fn selection_with(
    current: Range<usize>,
    added: Range<usize>,
) -> Result<Range<usize>, ProviderError> {
    if current.is_empty() || added.is_empty() {
        return Ok(added);
    }
    if added.end < current.start || current.end < added.start {
        return Err(TextProviderError::MultipleSelectionNotSupported.into());
    }
    Ok(current.start.min(added.start)..current.end.max(added.end))
}

/// `RemoveFromSelection` of a control with one contiguous selection: the
/// selection after `removed` is taken out of `current`.
pub(super) fn selection_without(
    current: Range<usize>,
    removed: Range<usize>,
) -> Result<Range<usize>, ProviderError> {
    if removed.is_empty() {
        return Ok(removed);
    }
    Ok(
        if current.is_empty() || removed.end <= current.start || current.end <= removed.start {
            current
        } else if removed.start <= current.start && current.end <= removed.end {
            current.start..current.start
        } else if removed.start <= current.start {
            removed.end..current.end
        } else if current.end <= removed.end {
            current.start..removed.start
        } else {
            return Err(TextProviderError::MultipleSelectionNotSupported.into());
        },
    )
}

/// `GetText` with a limit in UTF-16 code units; a limit never splits a surrogate pair.
pub(super) fn truncate_utf16(text: String, max_length: Option<usize>) -> String {
    let Some(max_length) = max_length else {
        return text;
    };
    let mut length = 0;
    text.chars()
        .take_while(|ch| {
            length += ch.len_utf16();
            length <= max_length
        })
        .collect()
}

/// The transform of a text view to its visual root and the clip of its ancestors.
pub struct ViewGeometry {
    transform: Matrix,
    clip: Rect,
}

impl ViewGeometry {
    /// Captures the visible geometry of a view in its own visual root.
    pub fn in_root(view: &Visual, root: &Ref<Visual>) -> Option<Self> {
        if !view.is_effectively_visible() {
            return None;
        }
        // The portable provider reports one root coordinate space. Overflow
        // pages in a different presentation root cannot be expressed in it.
        let transform = view.transform_to_visual(root)?;
        let mut clip = Rect::from_size(root.bounds().size());
        let mut current = Some(view.to_ref());
        while let Some(visual) = current {
            let to_root = visual.transform_to_visual(root)?;
            if visual.clip_to_bounds() {
                clip = clip
                    .intersect(Rect::from_size(visual.bounds().size()).transform_to_aabb(to_root));
            }
            if let Some(geometry) = visual.clip() {
                clip = clip.intersect(geometry.bounds().transform_to_aabb(to_root));
            }
            current = visual.visual_parent();
        }
        Some(Self { transform, clip })
    }

    /// Transforms a local rectangle to root coordinates and clips it.
    pub fn visible(&self, local: Rect) -> Rect {
        local.transform_to_aabb(self.transform).intersect(self.clip)
    }

    /// Returns the bounds of the root clip in view coordinates.
    pub fn local_clip(&self) -> Option<Rect> {
        Some(self.clip.transform_to_aabb(self.transform.try_invert()?))
    }
}

/// The color attribute of a brush: the color of a solid color brush, without its
/// alpha; any other brush has no single color.
pub(super) fn brush_color(brush: Option<&Rc<dyn IBrush>>) -> TextAttributeValue {
    brush.and_then(|brush| brush.as_solid_color_brush()).map_or(
        TextAttributeValue::NotSupported,
        |brush| {
            let color = brush.color();
            TextAttributeValue::Color(Color::from_rgb(color.r, color.g, color.b))
        },
    )
}

/// The value of the horizontal text alignment attribute (left, centered, right,
/// justified) of an alignment in a flow direction.
pub(super) fn horizontal_text_alignment(alignment: TextAlignment, rtl: bool) -> i32 {
    match alignment {
        TextAlignment::Center => 1,
        TextAlignment::Right => 2,
        TextAlignment::Justify => 3,
        TextAlignment::Start if rtl => 2,
        TextAlignment::End if !rtl => 2,
        _ => 0,
    }
}
