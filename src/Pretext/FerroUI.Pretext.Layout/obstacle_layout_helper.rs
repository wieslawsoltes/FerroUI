use crate::{Interval, RectObstacle};

/// Geometry utilities for carving horizontal text slots from exclusion shapes.
pub struct ObstacleLayoutHelper;

impl ObstacleLayoutHelper {
    /// Carve all blocked intervals, then discard slots narrower than the minimum.
    /// The surviving intervals remain ordered from left to right.
    pub fn carve_text_line_slots(
        base_slot: Interval,
        blocked: impl IntoIterator<Item = Interval>,
        min_slot_width: f64,
    ) -> Vec<Interval> {
        if !base_slot.left.is_finite()
            || !base_slot.right.is_finite()
            || base_slot.width() <= 0.0
            || !min_slot_width.is_finite()
            || min_slot_width < 0.0
        {
            return Vec::new();
        }
        let mut slots = vec![base_slot];
        let mut next = Vec::new();
        for interval in blocked {
            if !interval.left.is_finite() || !interval.right.is_finite() || interval.width() <= 0.0
            {
                continue;
            }
            next.clear();
            for slot in &slots {
                if interval.right <= slot.left || interval.left >= slot.right {
                    next.push(*slot);
                    continue;
                }
                if interval.left > slot.left {
                    next.push(Interval {
                        left: slot.left,
                        right: interval.left,
                    });
                }
                if interval.right < slot.right {
                    next.push(Interval {
                        left: interval.right,
                        right: slot.right,
                    });
                }
            }
            std::mem::swap(&mut slots, &mut next);
            if slots.is_empty() {
                break;
            }
        }
        slots.retain(|slot| slot.width() >= min_slot_width);
        slots
    }

    /// The widest horizontal intersection of a circle with the entire line band.
    /// A tangent band has no occupied area and yields `None`.
    #[allow(clippy::too_many_arguments)]
    pub fn circle_interval_for_band(
        cx: f64,
        cy: f64,
        radius: f64,
        band_top: f64,
        band_bottom: f64,
        horizontal_padding: f64,
        vertical_padding: f64,
    ) -> Option<Interval> {
        Self::ellipse_interval_for_band(
            cx,
            cy,
            radius,
            radius,
            band_top,
            band_bottom,
            horizontal_padding,
            vertical_padding,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn ellipse_interval_for_band(
        cx: f64,
        cy: f64,
        radius_x: f64,
        radius_y: f64,
        band_top: f64,
        band_bottom: f64,
        horizontal_padding: f64,
        vertical_padding: f64,
    ) -> Option<Interval> {
        if ![
            cx,
            cy,
            radius_x,
            radius_y,
            band_top,
            band_bottom,
            horizontal_padding,
            vertical_padding,
        ]
        .iter()
        .all(|value| value.is_finite())
            || radius_x <= 0.0
            || radius_y <= 0.0
            || band_top > band_bottom
            || horizontal_padding < 0.0
            || vertical_padding < 0.0
        {
            return None;
        }
        let top = band_top - vertical_padding;
        let bottom = band_bottom + vertical_padding;
        if top >= cy + radius_y || bottom <= cy - radius_y {
            return None;
        }
        let min_dy = if cy >= top && cy <= bottom {
            0.0
        } else if cy < top {
            top - cy
        } else {
            cy - bottom
        };
        if min_dy >= radius_y {
            return None;
        }
        let normalized_dy = min_dy / radius_y;
        let max_dx = radius_x * (1.0 - normalized_dy * normalized_dy).max(0.0).sqrt();
        let interval = Interval {
            left: cx - max_dx - horizontal_padding,
            right: cx + max_dx + horizontal_padding,
        };
        (interval.left.is_finite() && interval.right.is_finite()).then_some(interval)
    }

    pub fn get_rect_intervals_for_band(
        rects: &[RectObstacle],
        band_top: f64,
        band_bottom: f64,
        horizontal_padding: f64,
        vertical_padding: f64,
    ) -> Vec<Interval> {
        if ![band_top, band_bottom, horizontal_padding, vertical_padding]
            .iter()
            .all(|value| value.is_finite())
            || band_top > band_bottom
            || horizontal_padding < 0.0
            || vertical_padding < 0.0
        {
            return Vec::new();
        }
        let mut intervals = Vec::with_capacity(rects.len());
        for rect in rects {
            if rect.width <= 0.0
                || rect.height <= 0.0
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
                || band_bottom <= rect.y - vertical_padding
                || band_top >= rect.bottom() + vertical_padding
            {
                continue;
            }
            let interval = Interval {
                left: rect.x - horizontal_padding,
                right: rect.right() + horizontal_padding,
            };
            if interval.left.is_finite() && interval.right.is_finite() {
                intervals.push(interval);
            }
        }
        intervals
    }

    /// Pick the widest usable slot and resolve equal widths by horizontal order.
    /// Empty inputs return `None` instead of indexing an absent first interval.
    pub fn pick_slot(slots: &[Interval], prefer_right_on_tie: bool) -> Option<Interval> {
        let mut best = None;
        for &candidate in slots {
            if !candidate.left.is_finite()
                || !candidate.right.is_finite()
                || candidate.width() <= 0.0
            {
                continue;
            }
            let Some(current): Option<Interval> = best else {
                best = Some(candidate);
                continue;
            };
            if candidate.width() > current.width()
                || (candidate.width() == current.width()
                    && if prefer_right_on_tie {
                        candidate.left > current.left
                    } else {
                        candidate.left < current.left
                    })
            {
                best = Some(candidate);
            }
        }
        best
    }
}
