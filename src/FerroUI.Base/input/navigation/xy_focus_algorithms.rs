use crate::input::NavigationDirection;
use crate::utilities::MathUtilities;
use crate::{Rect, Vector};
use std::f64::consts::PI;

const IN_SHADOW_THRESHOLD: f64 = 0.25;
const IN_SHADOW_THRESHOLD_FOR_SECONDARY_AXIS: f64 = 0.02;
const CONE_ANGLE: f64 = PI / 4.0;

const PRIMARY_AXIS_DISTANCE_WEIGHT: f64 = 15.0;
const SECONDARY_AXIS_DISTANCE_WEIGHT: f64 = 1.0;
const PERCENT_IN_MANIFOLD_SHADOW_WEIGHT: f64 = 10000.0;
const PERCENT_IN_SHADOW_WEIGHT: f64 = 50.0;

/// The vertical and horizontal extents directional navigation started
/// from, so that moving back and forth returns to the same elements.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct XYFocusManifolds {
    /// (left, right)
    pub v_manifold: (f64, f64),
    /// (top, bottom)
    pub h_manifold: (f64, f64),
}

impl Default for XYFocusManifolds {
    fn default() -> Self {
        Self::new()
    }
}

impl XYFocusManifolds {
    pub fn new() -> Self {
        Self { v_manifold: (-1.0, -1.0), h_manifold: (-1.0, -1.0) }
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

pub(crate) struct XYFocusAlgorithms;

impl XYFocusAlgorithms {
    pub fn get_score_proximity(
        direction: NavigationDirection,
        bounds: Rect,
        candidate_bounds: Rect,
        max_distance: f64,
        consider_secondary_axis: bool,
    ) -> f64 {
        let mut score = 0.0;

        let primary_axis_distance = Self::calculate_primary_axis_distance(direction, bounds, candidate_bounds);
        let mut secondary_axis_distance = Self::calculate_secondary_axis_distance(direction, bounds, candidate_bounds);

        if primary_axis_distance >= 0.0 {
            // We do not want to use the secondary axis if the candidate is
            // within the shadow of the element
            let (reference, potential) = if direction == NavigationDirection::Left || direction == NavigationDirection::Right
            {
                ((bounds.top(), bounds.bottom()), (candidate_bounds.top(), candidate_bounds.bottom()))
            } else {
                ((bounds.left(), bounds.right()), (candidate_bounds.left(), candidate_bounds.right()))
            };

            if !consider_secondary_axis || Self::calculate_percent_in_shadow(reference, potential) != 0.0 {
                secondary_axis_distance = 0.0;
            }

            score = max_distance - (primary_axis_distance + secondary_axis_distance);
        }

        score
    }

    pub fn get_score_projection(
        direction: NavigationDirection,
        bounds: Rect,
        candidate_bounds: Rect,
        manifolds: &XYFocusManifolds,
        max_distance: f64,
    ) -> f64 {
        let mut score = 0.0;
        let mut percent_in_manifold_shadow = 0.0;

        let (reference, current_manifold, potential) =
            if direction == NavigationDirection::Left || direction == NavigationDirection::Right {
                (
                    (bounds.top(), bounds.bottom()),
                    manifolds.h_manifold,
                    (candidate_bounds.top(), candidate_bounds.bottom()),
                )
            } else {
                (
                    (bounds.left(), bounds.right()),
                    manifolds.v_manifold,
                    (candidate_bounds.left(), candidate_bounds.right()),
                )
            };

        let mut primary_axis_distance = Self::calculate_primary_axis_distance(direction, bounds, candidate_bounds);
        let mut secondary_axis_distance = Self::calculate_secondary_axis_distance(direction, bounds, candidate_bounds);

        if primary_axis_distance >= 0.0 {
            let mut percent_in_shadow = Self::calculate_percent_in_shadow(reference, potential);

            if percent_in_shadow >= IN_SHADOW_THRESHOLD_FOR_SECONDARY_AXIS {
                percent_in_manifold_shadow = Self::calculate_percent_in_shadow(current_manifold, potential);
                secondary_axis_distance = max_distance;
            }

            // The score needs to be a positive number so we make these
            // distances positive numbers
            primary_axis_distance = max_distance - primary_axis_distance;
            secondary_axis_distance = max_distance - secondary_axis_distance;

            if percent_in_shadow >= IN_SHADOW_THRESHOLD {
                percent_in_shadow = 1.0;
                primary_axis_distance *= 2.0;
            }

            // Potential elements in the shadow get a multiplier to their
            // final score
            score = Self::calculate_score(
                percent_in_shadow,
                primary_axis_distance,
                secondary_axis_distance,
                percent_in_manifold_shadow,
            );
        }

        score
    }

    pub fn update_manifolds(
        direction: NavigationDirection,
        bounds: Rect,
        new_focus_bounds: Rect,
        manifolds: &mut XYFocusManifolds,
    ) {
        let (mut v_manifold, mut h_manifold) = (manifolds.v_manifold, manifolds.h_manifold);

        if v_manifold.1 < 0.0 {
            v_manifold = (bounds.left(), bounds.right());
        }

        if h_manifold.1 < 0.0 {
            h_manifold = (bounds.top(), bounds.bottom());
        }

        if direction == NavigationDirection::Left || direction == NavigationDirection::Right {
            h_manifold = (
                new_focus_bounds.top().max(bounds.top()).max(h_manifold.0),
                new_focus_bounds.bottom().min(bounds.bottom()).min(h_manifold.1),
            );

            // It's possible to get into a situation where the new focused
            // element to the right / left has no overlap with the current
            // edge.
            if h_manifold.1 <= h_manifold.0 {
                h_manifold = (new_focus_bounds.top(), new_focus_bounds.bottom());
            }

            v_manifold = (new_focus_bounds.left(), new_focus_bounds.right());
        } else if direction == NavigationDirection::Up || direction == NavigationDirection::Down {
            v_manifold = (
                new_focus_bounds.left().max(bounds.left()).max(v_manifold.0),
                new_focus_bounds.right().min(bounds.right()).min(v_manifold.1),
            );

            // It's possible to get into a situation where the new focused
            // element above / below has no overlap with the current edge.
            if v_manifold.1 <= v_manifold.0 {
                v_manifold = (new_focus_bounds.left(), new_focus_bounds.right());
            }

            h_manifold = (new_focus_bounds.top(), new_focus_bounds.bottom());
        }

        manifolds.v_manifold = v_manifold;
        manifolds.h_manifold = h_manifold;
    }

    fn calculate_score(
        percent_in_shadow: f64,
        primary_axis_distance: f64,
        secondary_axis_distance: f64,
        percent_in_manifold_shadow: f64,
    ) -> f64 {
        (percent_in_shadow * PERCENT_IN_SHADOW_WEIGHT)
            + (primary_axis_distance * PRIMARY_AXIS_DISTANCE_WEIGHT)
            + (secondary_axis_distance * SECONDARY_AXIS_DISTANCE_WEIGHT)
            + (percent_in_manifold_shadow * PERCENT_IN_MANIFOLD_SHADOW_WEIGHT)
    }

    pub fn should_candidate_be_considered_for_ranking(
        bounds: Rect,
        candidate_bounds: Rect,
        max_distance: f64,
        direction: NavigationDirection,
        exclusion_rect: Rect,
        ignore_cone: bool,
    ) -> bool {
        // Consider a candidate only if:
        // 1. It doesn't have an empty rect as its bounds
        // 2. It doesn't contain the currently focused element
        // 3. Its bounds don't intersect with the rect we were asked to avoid
        //    looking into (Exclusion Rect)
        // 4. Its bounds aren't contained in the rect we were asked to avoid
        //    looking into (Exclusion Rect)
        if candidate_bounds.is_empty()
            || candidate_bounds.contains_rect(bounds)
            || exclusion_rect.intersects(candidate_bounds)
            || exclusion_rect.contains_rect(candidate_bounds)
        {
            return false;
        }

        // We've decided to disable the use of the cone for vertical
        // navigation.
        if ignore_cone || direction == NavigationDirection::Down || direction == NavigationDirection::Up {
            return true;
        }

        // The origins are built from single precision values upstream.
        let mut origin_top = Vector::new(0.0, bounds.top() as f32 as f64);
        let mut origin_bottom = Vector::new(0.0, bounds.bottom() as f32 as f64);

        let candidate_as_points = [
            Vector::from(candidate_bounds.top_left()),
            Vector::from(candidate_bounds.bottom_left()),
            Vector::from(candidate_bounds.bottom_right()),
            Vector::from(candidate_bounds.top_right()),
        ];

        // We make the max distance twice the normal distance to ensure that
        // all the elements are encapsulated inside the cone. This also aids
        // in scenarios where the original max distance is still less than
        // one of the points (due to the angles)
        let max_distance = max_distance * 2.0;

        let mut cone = [Vector::default(); 4];

        // Note: our y-axis is inverted
        if direction == NavigationDirection::Left {
            // We want to start the origin one pixel to the left to cover
            // overlapping scenarios where the end of a candidate element
            // could be overlapping with the origin (before the shift)
            origin_top = Vector::new(bounds.left() - 1.0, origin_top.y);
            origin_bottom = Vector::new(bounds.left() - 1.0, origin_bottom.y);

            // We have two angles. Find a point (for each angle) on the line
            // and rotate based on the direction
            let rotation = PI; // 180 degrees
            let sides = [
                Vector::new(
                    origin_top.x + max_distance * (rotation + CONE_ANGLE).cos(),
                    origin_top.y + max_distance * (rotation + CONE_ANGLE).sin(),
                ),
                Vector::new(
                    origin_bottom.x + max_distance * (rotation - CONE_ANGLE).cos(),
                    origin_bottom.y + max_distance * (rotation - CONE_ANGLE).sin(),
                ),
            ];

            // Order points in counterclockwise direction
            cone = [origin_top, sides[0], sides[1], origin_bottom];
        } else if direction == NavigationDirection::Right {
            // We want to start the origin one pixel to the right to cover
            // overlapping scenarios where the end of a candidate element
            // could be overlapping with the origin (before the shift)
            origin_top = Vector::new(bounds.right() + 1.0, origin_top.y);
            origin_bottom = Vector::new(bounds.right() + 1.0, origin_bottom.y);

            // We have two angles. Find a point (for each angle) on the line
            // and rotate based on the direction
            let rotation: f64 = 0.0;
            let sides = [
                Vector::new(
                    origin_top.x + max_distance * (rotation + CONE_ANGLE).cos(),
                    origin_top.y + max_distance * (rotation + CONE_ANGLE).sin(),
                ),
                Vector::new(
                    origin_bottom.x + max_distance * (rotation - CONE_ANGLE).cos(),
                    origin_bottom.y + max_distance * (rotation - CONE_ANGLE).sin(),
                ),
            ];

            // Order points in counterclockwise direction
            cone = [origin_bottom, sides[0], sides[1], origin_top];
        }

        // There are three scenarios we should check that will allow us to
        // know whether we should consider the candidate element.
        // 1) The candidate element and the vision cone intersect
        // 2) The candidate element is completely inside the vision cone
        // 3) The vision cone is completely inside the bounds of the
        //    candidate element (unlikely)
        MathUtilities::do_polygons_intersect(4, &cone, 4, &candidate_as_points)
            || MathUtilities::is_entirely_contained(4, &candidate_as_points, 4, &cone)
            || MathUtilities::is_entirely_contained(4, &cone, 4, &candidate_as_points)
    }

    fn calculate_primary_axis_distance(direction: NavigationDirection, bounds: Rect, candidate_bounds: Rect) -> f64 {
        let mut primary_axis_distance = -1.0;
        let is_overlapping = bounds.intersects(candidate_bounds);

        // We shouldn't be calculating the distance from ourselves
        if bounds == candidate_bounds {
            return -1.0;
        }

        if direction == NavigationDirection::Left
            && (candidate_bounds.right() <= bounds.left() || (is_overlapping && candidate_bounds.left() <= bounds.left()))
        {
            primary_axis_distance = (bounds.left() - candidate_bounds.right()).abs();
        } else if direction == NavigationDirection::Right
            && (candidate_bounds.left() >= bounds.right()
                || (is_overlapping && candidate_bounds.right() >= bounds.right()))
        {
            primary_axis_distance = (candidate_bounds.left() - bounds.right()).abs();
        } else if direction == NavigationDirection::Up
            && (candidate_bounds.bottom() <= bounds.top() || (is_overlapping && candidate_bounds.top() <= bounds.top()))
        {
            primary_axis_distance = (bounds.top() - candidate_bounds.bottom()).abs();
        } else if direction == NavigationDirection::Down
            && (candidate_bounds.top() >= bounds.bottom()
                || (is_overlapping && candidate_bounds.bottom() >= bounds.bottom()))
        {
            primary_axis_distance = (candidate_bounds.top() - bounds.bottom()).abs();
        }

        primary_axis_distance
    }

    fn calculate_secondary_axis_distance(direction: NavigationDirection, bounds: Rect, candidate_bounds: Rect) -> f64 {
        // calculate secondary axis distance for the case where the element is
        // not in the shadow
        if direction == NavigationDirection::Left || direction == NavigationDirection::Right {
            if candidate_bounds.top() < bounds.top() {
                (bounds.top() - candidate_bounds.bottom()).abs()
            } else {
                (candidate_bounds.top() - bounds.bottom()).abs()
            }
        } else if candidate_bounds.left() < bounds.left() {
            (bounds.left() - candidate_bounds.right()).abs()
        } else {
            (candidate_bounds.left() - bounds.right()).abs()
        }
    }

    /// Calculates the percentage of the potential element that is in the
    /// shadow of the reference element.
    fn calculate_percent_in_shadow(reference_manifold: (f64, f64), potential_manifold: (f64, f64)) -> f64 {
        if reference_manifold.0 > potential_manifold.1 || reference_manifold.1 <= potential_manifold.0 {
            // Potential is not in the reference's shadow.
            return 0.0;
        }

        let shadow =
            (reference_manifold.1.min(potential_manifold.1) - reference_manifold.0.max(potential_manifold.0)).abs();

        let potential_edge_length = (potential_manifold.1 - potential_manifold.0).abs();
        let reference_edge_length = (reference_manifold.1 - reference_manifold.0).abs();

        let comparison_edge_length = reference_edge_length.min(potential_edge_length);

        if comparison_edge_length != 0.0 {
            (shadow / comparison_edge_length).min(1.0)
        } else {
            1.0
        }
    }
}
