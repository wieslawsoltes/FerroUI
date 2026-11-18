//! WinUI's `PivotCurveGenerator` (`dxaml/xcp/components/pivot/PivotCurveGenerator.cpp` and
//! `inc/PivotCurveGenerator.h` of microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`;
//! MIT, Copyright (c) Microsoft Corporation, see NOTICE.md), ported one to one.
//!
//! The generator computes the parametric curves of the parallax of the headers: piecewise linear
//! maps from the horizontal offset of the scroll viewer (the primary offsets) to the offset of the
//! header panel (the secondary offsets). While the headers wrap (dynamic headers) each section
//! the viewport moves shifts the headers by the width of one header; static headers shift so
//! that the selected one stays in view.

use super::pivot_common::{are_close, header_shift_offset, positive_mod, PivotAnimationDirection};

/// WinUI's `c_overrun`: the groups of sections the dynamic curve covers on each side.
const OVERRUN: u32 = 3;

/// The curves of the header parallax (WinUI's `PivotCurveGenerator`).
#[derive(Debug, Clone)]
pub(crate) struct PivotCurveGenerator {
    curves_dirty: bool,
    current_index: i32,
    current_offset: f64,
    current_header_panel_offset: f64,
    current_parametric_curve_offset: f64,
    section_width: f64,
    last_curve_centerpoint: f64,
    pending_header_item_width_change: bool,
    current_header_item_widths: Vec<f64>,
    pending_header_item_widths: Vec<f64>,
    current_header_item_width_total_size: f64,
    pending_header_item_width_total_size: f64,
}

impl Default for PivotCurveGenerator {
    fn default() -> Self {
        Self {
            curves_dirty: true,
            current_index: 0,
            current_offset: 0.0,
            current_header_panel_offset: 0.0,
            current_parametric_curve_offset: 0.0,
            section_width: 0.0,
            last_curve_centerpoint: 0.0,
            pending_header_item_width_change: false,
            current_header_item_widths: Vec::new(),
            pending_header_item_widths: Vec::new(),
            current_header_item_width_total_size: 0.0,
            pending_header_item_width_total_size: 0.0,
        }
    }
}

impl PivotCurveGenerator {
    pub(crate) fn selected_index_changed_event(&mut self, index: i32, animation_hint: PivotAnimationDirection) {
        let item_count = self.current_header_item_widths.len() as i32;
        let items_to_shift = match animation_hint {
            PivotAnimationDirection::Right => positive_mod(index - self.current_index, item_count),
            PivotAnimationDirection::Left => -(item_count - positive_mod(index - self.current_index, item_count)),
            PivotAnimationDirection::Reset => {
                self.curves_dirty = true;
                0
            }
            PivotAnimationDirection::Center => 0,
        };
        if item_count == 0 || self.section_width <= 0.0 {
            return;
        }
        let sign = if items_to_shift < 0 { -1 } else { 1 };
        let visual_offset_index = (self.current_offset / self.section_width + 0.5) as i64;
        let base_index = (visual_offset_index - self.current_index as i64).rem_euclid(item_count as i64);
        let mut header_offset = self.current_header_panel_offset;
        let mut curve_offset = self.current_parametric_curve_offset;
        let mut panel_offset = self.current_offset;
        let mut item_index = visual_offset_index;
        let end = visual_offset_index + items_to_shift as i64;
        while item_index != end {
            if sign < 0 {
                let width = self.current_header_item_widths
                    [(item_index - base_index - 1).rem_euclid(item_count as i64) as usize];
                header_offset -= width;
                curve_offset -= self.section_width - width;
                panel_offset -= self.section_width;
            } else {
                let width =
                    self.current_header_item_widths[(item_index - base_index).rem_euclid(item_count as i64) as usize];
                header_offset += width;
                curve_offset += self.section_width - width;
                panel_offset += self.section_width;
            }
            item_index += sign as i64;
        }
        self.current_parametric_curve_offset = curve_offset;
        self.current_header_panel_offset = header_offset;
        self.current_offset = panel_offset;
        self.current_index = index;
    }

    pub(crate) fn sync_selected_item_offset(&mut self, new_offset: f64) {
        if !are_close(new_offset, self.current_offset) {
            self.current_offset = new_offset;
            self.calculate_initial_offsets();
        }
    }

    pub(crate) fn sync_section_width(&mut self, new_width: f64) {
        if !are_close(new_width, self.section_width) {
            self.section_width = new_width;
            self.calculate_initial_offsets();
        }
    }

    fn calculate_initial_offsets(&mut self) {
        self.flush_pending_header_item_width_change();
        self.curves_dirty = true;
        let item_count = self.current_header_item_widths.len() as i64;
        if item_count == 0 || self.section_width <= 0.0 {
            return;
        }
        let visual_offset_index = (self.current_offset / self.section_width + 0.5) as i64;
        let base_index = (visual_offset_index - self.current_index as i64).rem_euclid(item_count);
        let repeating_group_count = (visual_offset_index - base_index - self.current_index as i64) / item_count;
        let total_section_item_size = self.section_width * item_count as f64;
        let mut header_panel_offset = repeating_group_count as f64 * self.current_header_item_width_total_size;
        let mut curve_offset =
            repeating_group_count as f64 * (total_section_item_size - self.current_header_item_width_total_size);
        for item_index in 0..base_index {
            let width = self.current_header_item_widths[(item_count - item_index - 1) as usize];
            header_panel_offset += width;
            curve_offset += self.section_width - width;
        }
        for item_index in 0..(visual_offset_index - base_index).rem_euclid(item_count) {
            let width = self.current_header_item_widths[item_index as usize];
            header_panel_offset += width;
            curve_offset += self.section_width - width;
        }
        self.current_header_panel_offset = header_panel_offset;
        self.current_parametric_curve_offset = curve_offset;
    }

    pub(crate) fn sync_items_size_and_order(&mut self, sizes: Vec<f64>, total_size: f64) {
        if self.current_header_item_widths.is_empty() {
            self.current_header_item_widths = sizes;
            self.current_header_item_width_total_size = total_size;
            self.pending_header_item_width_change = false;
            self.calculate_initial_offsets();
            self.curves_dirty = true;
        } else if self.current_header_item_widths != sizes {
            self.pending_header_item_widths = sizes;
            self.pending_header_item_width_total_size = total_size;
            self.pending_header_item_width_change = true;
            self.curves_dirty = true;
        }
    }

    pub(crate) fn are_curves_dirty(&self, using_dynamic_headers: bool) -> bool {
        self.curves_dirty
            || (using_dynamic_headers
                && (self.current_offset - self.last_curve_centerpoint).abs() > self.section_width * OVERRUN as f64)
    }

    /// The curve of the dynamic headers: `(primary, secondary)` points.
    pub(crate) fn dynamic_curve_segments(&mut self, panel_multiplier: u32) -> (Vec<f64>, Vec<f64>) {
        self.flush_pending_header_item_width_change();
        let item_count = self.current_header_item_widths.len() as i64;
        let mut primary = Vec::new();
        let mut secondary = Vec::new();
        if item_count == 0 || self.section_width <= 0.0 {
            primary.push(0.0);
            primary.push(self.current_offset * 2.0);
            secondary.push(self.current_offset);
            secondary.push(self.current_offset);
        } else {
            let visual_offset_index = (self.current_offset / self.section_width + 0.5) as i64;
            let base_index = (visual_offset_index - self.current_index as i64).rem_euclid(item_count);
            let repeating_group_count = (visual_offset_index - base_index - self.current_index as i64) / item_count;
            let item_group_count = 1 + OVERRUN as i64 * 2;
            let total_section_item_size = self.section_width * item_count as f64;
            let offset_count = 3 + item_count * item_group_count;

            primary.push(0.0);
            let mut position = total_section_item_size * (repeating_group_count - OVERRUN as i64) as f64
                + base_index as f64 * self.section_width;
            for _ in 1..offset_count - 1 {
                primary.push(position);
                position += self.section_width;
            }
            primary.push(self.section_width * panel_multiplier as f64);

            let mut position = (total_section_item_size - self.current_header_item_width_total_size)
                * (repeating_group_count - OVERRUN as i64) as f64;
            for item_index in 0..base_index {
                position +=
                    self.section_width - self.current_header_item_widths[(item_count - item_index - 1) as usize];
            }
            let mut expected_curve_offset = position;
            expected_curve_offset +=
                OVERRUN as f64 * (total_section_item_size - self.current_header_item_width_total_size);
            for item_index in 0..self.current_index.max(0) as usize {
                if let Some(width) = self.current_header_item_widths.get(item_index) {
                    expected_curve_offset += self.section_width - width;
                }
            }
            let correction = self.current_parametric_curve_offset - expected_curve_offset;
            position += correction;
            secondary.push(-position);
            for _ in 0..item_group_count {
                for width in &self.current_header_item_widths {
                    secondary.push(-position);
                    position += self.section_width - width;
                }
            }
            secondary.push(-position);
            secondary.push(-position);
        }
        self.curves_dirty = false;
        self.last_curve_centerpoint = self.current_offset;
        (primary, secondary)
    }

    /// The curve of the static headers: `(primary, secondary)` points.
    pub(crate) fn static_curve_segments(&mut self, viewport_size: f64) -> (Vec<f64>, Vec<f64>) {
        self.flush_pending_header_item_width_change();
        let mut primary = vec![0.0];
        let mut secondary = vec![0.0];
        let mut accumulated = 0.0;
        for (i, width) in self.current_header_item_widths.iter().enumerate() {
            // The space left of the item, its width, the space right of it.
            let a = accumulated;
            let b = *width;
            let c = (self.current_header_item_width_total_size - (a + b)).max(0.0);
            primary.push(self.section_width * i as f64);
            secondary.push(header_shift_offset(a, b, c, viewport_size));
            accumulated += width;
        }
        let last_primary = *primary.last().unwrap_or(&0.0);
        let last_secondary = *secondary.last().unwrap_or(&0.0);
        primary.push(last_primary);
        secondary.push(last_secondary);
        self.curves_dirty = false;
        self.last_curve_centerpoint = self.current_offset;
        (primary, secondary)
    }

    pub(crate) fn header_panel_offset(&self) -> f64 {
        self.current_header_panel_offset
    }

    pub(crate) fn current_section_offset(&self) -> f64 {
        self.current_offset
    }

    fn flush_pending_header_item_width_change(&mut self) {
        if self.pending_header_item_width_change {
            self.current_header_item_widths = std::mem::take(&mut self.pending_header_item_widths);
            self.current_header_item_width_total_size = self.pending_header_item_width_total_size;
            self.pending_header_item_width_change = false;
        }
    }
}

/// A composition expression of the piecewise linear map through `points` (the parametric curve
/// of WinUI's secondary content relationship), of the scalar expression `input`: constant before
/// the first and after the last point. The points are sorted by their primary offset.
pub(crate) fn piecewise_linear_expression(primary: &[f64], secondary: &[f64], input: &str) -> String {
    let points: Vec<(f64, f64)> = primary.iter().copied().zip(secondary.iter().copied()).collect();
    match points.len() {
        0 => "0".to_owned(),
        1 => format!("{}", points[0].1 as f32),
        _ => {
            let mut expression = format!("{}", points[points.len() - 1].1 as f32);
            for window in points.windows(2).rev() {
                let ((p0, v0), (p1, v1)) = (window[0], window[1]);
                let segment = if (p1 - p0).abs() < f64::EPSILON {
                    format!("{}", v1 as f32)
                } else {
                    let slope = (v1 - v0) / (p1 - p0);
                    format!("({} + ({} - {}) * {})", v0 as f32, input, p0 as f32, slope as f32)
                };
                expression = format!("({} < {} ? {} : {})", input, p1 as f32, segment, expression);
            }
            let (p0, v0) = points[0];
            format!("({} < {} ? {} : {})", input, p0 as f32, v0 as f32, expression)
        }
    }
}

/// Evaluates the piecewise linear map through `points` at `x`, as the expression of
/// [`piecewise_linear_expression`] does (for the tests).
#[cfg(test)]
pub(crate) fn evaluate_piecewise_linear(primary: &[f64], secondary: &[f64], x: f64) -> f64 {
    let n = primary.len().min(secondary.len());
    if n == 0 {
        return 0.0;
    }
    if x <= primary[0] {
        return secondary[0];
    }
    for i in 1..n {
        if x < primary[i] {
            let (p0, p1, v0, v1) = (primary[i - 1], primary[i], secondary[i - 1], secondary[i]);
            if (p1 - p0).abs() < f64::EPSILON {
                return v1;
            }
            return v0 + (x - p0) * (v1 - v0) / (p1 - p0);
        }
    }
    secondary[n - 1]
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDTH: f64 = 400.0;

    fn generator(widths: &[f64], offset: f64) -> PivotCurveGenerator {
        let mut generator = PivotCurveGenerator::default();
        generator.sync_section_width(WIDTH);
        generator.sync_items_size_and_order(widths.to_vec(), widths.iter().sum());
        generator.sync_selected_item_offset(offset);
        generator
    }

    #[test]
    fn the_header_panel_offset_is_the_widths_before_the_visual_section() {
        // Section 5 from the start, item 0 selected: one full group of three (300) and the two
        // items before the selected one, counted from the end (WinUI's base index of 2).
        let g = generator(&[100.0, 120.0, 80.0], 5.0 * WIDTH);
        assert_eq!(g.header_panel_offset(), 300.0 + 80.0 + 120.0);
    }

    #[test]
    fn moving_right_shifts_the_headers_by_the_width_of_the_current_header() {
        let mut g = generator(&[100.0, 120.0, 80.0], 3.0 * WIDTH);
        let before = g.header_panel_offset();
        g.selected_index_changed_event(1, PivotAnimationDirection::Right);
        assert_eq!(g.header_panel_offset() - before, 100.0);
        assert_eq!(g.current_section_offset(), 4.0 * WIDTH);
        g.selected_index_changed_event(0, PivotAnimationDirection::Left);
        assert_eq!(g.header_panel_offset(), before);
    }

    #[test]
    fn the_dynamic_curve_moves_one_header_per_section() {
        let mut g = generator(&[100.0, 120.0, 80.0], 30.0 * WIDTH);
        let (primary, secondary) = g.dynamic_curve_segments(1000);
        assert_eq!(primary.len(), 3 + 3 * 7);
        assert_eq!(primary.len(), secondary.len());
        // Between two inner points the secondary offset changes by the width of the section less
        // the width of the header: the header moves by its width relative to the content.
        let step = secondary[1] - secondary[2];
        assert_eq!(step, WIDTH - 100.0);
        assert!(!g.are_curves_dirty(true));
    }

    #[test]
    fn the_static_curve_keeps_the_selected_header_in_view() {
        let mut g = generator(&[200.0, 200.0, 200.0], 0.0);
        let (primary, secondary) = g.static_curve_segments(300.0);
        assert_eq!(primary, vec![0.0, 0.0, WIDTH, 2.0 * WIDTH, 2.0 * WIDTH]);
        assert_eq!(secondary[1], 0.0);
        assert_eq!(secondary[3], 300.0);
    }

    #[test]
    fn the_expression_and_the_evaluation_agree() {
        let primary = [0.0, 100.0, 300.0];
        let secondary = [0.0, 50.0, 50.0];
        assert_eq!(evaluate_piecewise_linear(&primary, &secondary, 50.0), 25.0);
        assert_eq!(evaluate_piecewise_linear(&primary, &secondary, 1000.0), 50.0);
        let expression = piecewise_linear_expression(&primary, &secondary, "x");
        assert!(expression.starts_with("(x < 0 ? 0 : (x < 100 ?"));
    }
}
