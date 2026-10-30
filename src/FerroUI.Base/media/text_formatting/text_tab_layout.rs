//! Position-dependent tab advances are resolved after shaping, independently for
//! each displayed line. Cached shaping and previously published lines stay immutable.

use std::rc::Rc;

use super::{
    BidiReorderer, GlyphInfo, ShapedBuffer, ShapedTextRun, TabAlignment, TabLeader, TextParagraphProperties, TextRun,
};
use crate::media::FlowDirection;

pub(super) fn resolve_tabs(
    runs: &mut Vec<Rc<dyn TextRun>>,
    properties: &dyn TextParagraphProperties,
    paragraph_width: f64,
) -> bool {
    let Some(stops) = properties.tab_stops() else {
        return false;
    };
    if !runs.iter().any(|run| contains_tab(&**run)) {
        return false;
    }
    split_tabs(runs);
    let rtl = properties.flow_direction() == FlowDirection::RightToLeft;
    let origin = properties.tab_origin();
    let origin = if origin.is_finite() { origin } else { 0.0 };
    let limit = origin + paragraph_width;
    let interval = properties.default_incremental_tab();
    let interval = if interval.is_finite() && interval > 0.0 {
        interval
    } else {
        properties.default_text_run_properties().font_rendering_em_size() * 4.0
    };
    let mut cursor = origin;
    for index in 0..runs.len() {
        if !is_tab(&*runs[index]) {
            cursor += run_width(&*runs[index]);
            continue;
        }
        let explicit = stops
            .iter()
            .filter(|stop| {
                stop.alignment != TabAlignment::Bar
                    && stop.position.is_finite()
                    && stop.position > cursor
                    && stop.position <= limit
            })
            .min_by(|a, b| a.position.total_cmp(&b.position));
        let (position, alignment, leader) = if let Some(stop) = explicit {
            (stop.position, stop.alignment, stop.leader)
        } else {
            // Arithmetic, not a scan through implicit stops. A first-line hanging
            // indent may put the cursor before the page origin.
            let position = ((cursor / interval).floor() + 1.0).max(0.0) * interval;
            (
                position,
                if rtl { TabAlignment::Right } else { TabAlignment::Left },
                TabLeader::Spaces,
            )
        };
        let requires_field = alignment == TabAlignment::Center
            || alignment == TabAlignment::Decimal
            || (alignment == TabAlignment::Right && !rtl)
            || (alignment == TabAlignment::Left && rtl);
        let (field_width, decimal) = if requires_field {
            let field_end = runs[index + 1..]
                .iter()
                .position(|run| is_tab(&**run) || run.as_text_end_of_line().is_some())
                .map_or(runs.len(), |i| index + 1 + i);
            let field = &runs[index + 1..field_end];
            if alignment == TabAlignment::Decimal {
                field_metrics(field, properties.flow_direction())
            } else {
                (field.iter().map(|run| run_width(&**run)).sum(), None)
            }
        } else {
            (0.0, None)
        };
        let alignment_offset = match alignment {
            TabAlignment::Left => {
                if rtl {
                    field_width
                } else {
                    0.0
                }
            }
            TabAlignment::Right => {
                if rtl {
                    0.0
                } else {
                    field_width
                }
            }
            TabAlignment::Center => field_width * 0.5,
            TabAlignment::Decimal => {
                let physical = decimal.unwrap_or(field_width);
                if rtl {
                    field_width - physical
                } else {
                    physical
                }
            }
            TabAlignment::Bar => 0.0,
        };
        let advance = (position - cursor - alignment_offset).max(0.0);
        let Some(shaped) = runs[index].downcast_ref::<ShapedTextRun>() else {
            continue;
        };
        let source = shaped.shaped_buffer();
        let buffer = ShapedBuffer::new(
            source.text().clone(),
            1,
            source.glyph_typeface().clone(),
            source.font_rendering_em_size(),
            properties.flow_direction() as i8,
        );
        let space = source.glyph_typeface().character_to_glyph_map().get_glyph(' ' as i32);
        buffer.set(0, GlyphInfo::new(space, 0, advance));
        let replacement = ShapedTextRun::with_tab_layout(buffer, shaped.run_properties().clone(), leader, alignment);
        shaped.dispose();
        runs[index] = Rc::new(replacement);
        cursor += advance;
    }
    true
}

fn contains_tab(run: &dyn TextRun) -> bool {
    run.downcast_ref::<ShapedTextRun>().map_or_else(
        || run.text_span().contains(&(b'\t' as u16)),
        ShapedTextRun::contains_tab,
    )
}

fn is_tab(run: &dyn TextRun) -> bool {
    run.text_span() == [b'\t' as u16]
}

fn run_width(run: &dyn TextRun) -> f64 {
    if let Some(shaped) = run.downcast_ref::<ShapedTextRun>() {
        shaped.shaped_buffer().total_glyph_advance()
    } else {
        run.as_drawable().map_or(0.0, |drawable| drawable.size().width)
    }
}

/// The decimal's physical left edge in the visual field, independent of its
/// logical index, formatting boundaries, fallback font or embedding direction.
fn field_metrics(runs: &[Rc<dyn TextRun>], flow: FlowDirection) -> (f64, Option<f64>) {
    let mut target = None;
    for run in runs {
        let Some(shaped) = run.downcast_ref::<ShapedTextRun>() else {
            continue;
        };
        let separator: Vec<u16> = shaped
            .run_properties()
            .culture_info()
            .map(|culture| culture.number_format().number_decimal_separator().to_owned())
            .unwrap_or_else(|| ".".to_owned())
            .encode_utf16()
            .collect();
        if separator.is_empty() {
            continue;
        }
        if let Some(index) = shaped
            .text_span()
            .windows(separator.len())
            .position(|text| text == separator)
        {
            let buffer = shaped.shaped_buffer();
            let left = if buffer.is_left_to_right() {
                buffer.get_char_range_width(0, index as i32)
            } else {
                buffer.total_glyph_advance() - buffer.get_char_range_width(0, (index + separator.len()) as i32)
            };
            target = Some((run.clone(), left));
            break;
        }
    }
    let mut visual_runs = runs.to_vec();
    BidiReorderer::bidi_reorder(&mut visual_runs, flow, 0);
    let mut width = 0.0;
    let mut decimal = None;
    for run in visual_runs {
        if let Some((target, left)) = &target {
            if Rc::ptr_eq(target, &run) {
                decimal = Some(width + left);
            }
        }
        width += run_width(&*run);
    }
    (width, decimal)
}

/// Tab characters are standalone shaped runs. Adjacent text retains the already
/// shaped glyphs, clusters and properties. Splits share immutable backing storage.
fn split_tabs(runs: &mut Vec<Rc<dyn TextRun>>) {
    let source = std::mem::take(runs);
    runs.reserve(source.len());
    for run in source {
        if !contains_tab(&*run) || run.length() == 1 {
            runs.push(run);
            continue;
        }
        let Some(mut remaining) = run.clone().downcast_rc::<ShapedTextRun>() else {
            runs.push(run);
            continue;
        };
        loop {
            let text = remaining.text_span();
            let length = text
                .iter()
                .position(|unit| *unit == b'\t' as u16)
                .map_or(text.len(), |index| if index == 0 { 1 } else { index });
            let starts_with_tab = text.first() == Some(&(b'\t' as u16));
            if length == text.len() {
                remaining.set_tab_presence(starts_with_tab);
                runs.push(remaining);
                break;
            }
            let split = remaining.split(length as i32);
            remaining.dispose();
            if let Some(first) = split.first {
                first.set_tab_presence(starts_with_tab);
                runs.push(first);
            }
            let Some(second) = split.second else {
                break;
            };
            remaining = second;
        }
    }
}
