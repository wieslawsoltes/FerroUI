// Copyright 2026 FerroUI contributors.
// SPDX-License-Identifier: Apache-2.0
//! Deterministic Material sheet anchors and gesture settling.
//!
//! Based on AndroidX Material3 1.5.0-beta02, `BottomSheet.kt` and `SheetDefaults.kt`,
//! revision e4d933c98df7baa7ea2b83eea6e086915961e0ff. The new sheet API retains a
//! partial semantic state even when a short sheet shares its expanded coordinate.

/// The semantic state of a Material bottom sheet.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MaterialSheetState {
    #[default]
    Hidden,
    PartiallyExpanded,
    Expanded,
}

ferroui_base::ferro_markup_enum!(
    MaterialSheetState { Hidden, PartiallyExpanded, Expanded },
    { namespace: "FerroUI.Themes.Material" }
);

/// Sheet top coordinates in a viewport, expressed in device-independent pixels.
///
/// The partial and expanded coordinates may be equal. Do not use a map keyed by
/// floating-point position to represent the semantic states: it loses information.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MaterialSheetAnchors {
    pub hidden: f64,
    pub partially_expanded: f64,
    pub expanded: f64,
}

impl MaterialSheetAnchors {
    pub const POSITIONAL_THRESHOLD: f64 = 56.0;
    pub const VELOCITY_THRESHOLD: f64 = 125.0;
    pub const VELOCITY_DAMPING_ZONE: f64 = 125.0;

    /// Recomputes anchors for the current usable viewport and measured sheet.
    /// Nonfinite dimensions are treated as zero; oversized sheets fill the viewport.
    pub fn new(viewport_height: f64, sheet_height: f64) -> Self {
        let height = finite_nonnegative(viewport_height);
        let sheet = finite_nonnegative(sheet_height).min(height);
        Self {
            hidden: height,
            partially_expanded: height - (height * 0.5).min(sheet),
            expanded: height - sheet,
        }
    }

    pub fn offset(self, state: MaterialSheetState) -> f64 {
        match state {
            MaterialSheetState::Hidden => self.hidden,
            MaterialSheetState::PartiallyExpanded => self.partially_expanded,
            MaterialSheetState::Expanded => self.expanded,
        }
    }

    pub fn partial_matches_expanded(self) -> bool {
        (self.partially_expanded - self.expanded).abs() <= f64::EPSILON
    }

    /// Preserves the fully visible short-sheet state when its content grows.
    pub fn reconcile_target(
        self,
        previous: Self,
        target: MaterialSheetState,
        partial_enabled: bool,
    ) -> MaterialSheetState {
        if target == MaterialSheetState::PartiallyExpanded
            && (!partial_enabled
                || (previous.partial_matches_expanded() && !self.partial_matches_expanded()))
        {
            MaterialSheetState::Expanded
        } else {
            target
        }
    }

    /// Chooses an adjacent legal anchor using the source's 56dp / 125dp/s thresholds.
    /// A high-velocity fling follows its direction; a slow drag must cross the
    /// positional threshold from the last settled state. Large drags can traverse
    /// multiple anchors. `allow_hidden=false` is useful for persistent sheets.
    pub fn settle_target(
        self,
        current: MaterialSheetState,
        offset: f64,
        velocity: f64,
        partial_enabled: bool,
        allow_hidden: bool,
    ) -> MaterialSheetState {
        use MaterialSheetState::*;
        let mut anchors = vec![(self.expanded, Expanded)];
        if partial_enabled && !self.partial_matches_expanded() {
            anchors.push((self.partially_expanded, PartiallyExpanded));
        }
        if allow_hidden && self.hidden > self.expanded {
            anchors.push((self.hidden, Hidden));
        }
        let from = self.offset(current);
        let offset = if offset.is_finite() {
            offset.clamp(self.expanded, self.hidden)
        } else {
            from
        };
        let velocity = if velocity.is_finite() { velocity } else { 0.0 };
        let downward = velocity >= Self::VELOCITY_THRESHOLD
            || (velocity > -Self::VELOCITY_THRESHOLD && offset > from);
        let upward = velocity <= -Self::VELOCITY_THRESHOLD
            || (velocity < Self::VELOCITY_THRESHOLD && offset < from);
        let result = if downward {
            let next = anchors
                .iter()
                .find(|(position, _)| *position > offset + 0.001)
                .or_else(|| anchors.last())
                .copied()
                .unwrap();
            if velocity >= Self::VELOCITY_THRESHOLD {
                next.1
            } else if offset - from >= Self::POSITIONAL_THRESHOLD.min((next.0 - from).abs()) {
                let mut chosen = current;
                let mut previous = from;
                for &(position, state) in anchors.iter().filter(|(position, _)| *position > from) {
                    if offset >= previous + Self::POSITIONAL_THRESHOLD.min(position - previous) {
                        chosen = state;
                    }
                    previous = position;
                }
                chosen
            } else {
                current
            }
        } else if upward {
            let next = anchors
                .iter()
                .rev()
                .find(|(position, _)| *position < offset - 0.001)
                .or_else(|| anchors.first())
                .copied()
                .unwrap();
            if velocity <= -Self::VELOCITY_THRESHOLD {
                next.1
            } else if from - offset >= Self::POSITIONAL_THRESHOLD.min((from - next.0).abs()) {
                let mut chosen = current;
                let mut previous = from;
                for &(position, state) in anchors
                    .iter()
                    .rev()
                    .filter(|(position, _)| *position < from)
                {
                    if offset <= previous - Self::POSITIONAL_THRESHOLD.min(previous - position) {
                        chosen = state;
                    }
                    previous = position;
                }
                chosen
            } else {
                current
            }
        } else {
            current
        };
        if result == Hidden && !allow_hidden {
            Expanded
        } else if result == PartiallyExpanded && !partial_enabled {
            Expanded
        } else {
            result
        }
    }

    /// Damps downward fling energy in the source's 125dp hidden-boundary zone.
    /// The source preserves the fling threshold once it has already been crossed.
    pub fn damp_velocity(self, offset: f64, velocity: f64, maximum_velocity: f64) -> f64 {
        if !velocity.is_finite() {
            return 0.0;
        }
        let maximum = finite_nonnegative(maximum_velocity).max(Self::VELOCITY_THRESHOLD);
        let velocity = velocity.clamp(-maximum, maximum);
        if velocity <= Self::VELOCITY_THRESHOLD {
            return velocity;
        }
        let remaining = (self.hidden - offset).max(0.0);
        let fraction = (remaining / Self::VELOCITY_DAMPING_ZONE).clamp(0.0, 1.0);
        (velocity * fraction * fraction).max(Self::VELOCITY_THRESHOLD)
    }
}

fn finite_nonnegative(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use MaterialSheetState::*;

    #[test]
    fn deterministic_anchors_preserve_short_partial_state() {
        let short = MaterialSheetAnchors::new(800.0, 300.0);
        assert_eq!(
            short,
            MaterialSheetAnchors {
                hidden: 800.0,
                partially_expanded: 500.0,
                expanded: 500.0
            }
        );
        assert_eq!(
            short.reconcile_target(short, PartiallyExpanded, true),
            PartiallyExpanded
        );
        let tall = MaterialSheetAnchors::new(800.0, 600.0);
        assert_eq!(
            tall,
            MaterialSheetAnchors {
                hidden: 800.0,
                partially_expanded: 400.0,
                expanded: 200.0
            }
        );
        assert_eq!(
            tall.reconcile_target(short, PartiallyExpanded, true),
            Expanded
        );
        assert_eq!(
            tall.reconcile_target(tall, PartiallyExpanded, false),
            Expanded
        );
    }

    #[test]
    fn slow_drag_uses_distance_while_fling_uses_direction() {
        let a = MaterialSheetAnchors::new(800.0, 600.0);
        assert_eq!(a.settle_target(Expanded, 230.0, 0.0, true, true), Expanded);
        assert_eq!(
            a.settle_target(Expanded, 256.0, 0.0, true, true),
            PartiallyExpanded
        );
        assert_eq!(
            a.settle_target(Expanded, 205.0, 126.0, true, true),
            PartiallyExpanded
        );
        assert_eq!(
            a.settle_target(PartiallyExpanded, 395.0, -126.0, true, true),
            Expanded
        );
        assert_eq!(
            a.settle_target(PartiallyExpanded, 410.0, 126.0, true, true),
            Hidden
        );
        assert_ne!(
            a.settle_target(PartiallyExpanded, 790.0, 900.0, true, false),
            Hidden
        );
    }

    #[test]
    fn boundary_velocity_damping_retains_fling_threshold() {
        let a = MaterialSheetAnchors::new(800.0, 600.0);
        assert_eq!(a.damp_velocity(600.0, 1000.0, 8000.0), 1000.0);
        assert_eq!(a.damp_velocity(737.5, 1000.0, 8000.0), 250.0);
        assert_eq!(a.damp_velocity(799.0, 1000.0, 8000.0), 125.0);
        assert_eq!(a.damp_velocity(799.0, -1000.0, 8000.0), -1000.0);
        assert_eq!(a.damp_velocity(600.0, f64::NAN, 8000.0), 0.0);
    }

    #[test]
    fn degenerate_layout_never_produces_nonfinite_coordinates() {
        assert_eq!(
            MaterialSheetAnchors::new(f64::INFINITY, f64::NAN),
            MaterialSheetAnchors::default()
        );
        assert_eq!(MaterialSheetAnchors::new(800.0, 900.0).expanded, 0.0);
        assert_eq!(
            MaterialSheetAnchors::new(-1.0, -2.0),
            MaterialSheetAnchors::default()
        );
    }
}
