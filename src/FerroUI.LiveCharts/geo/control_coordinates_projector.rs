//! Equirectangular projection and its inverse.
// Source: LiveChartsCore/Geo/ControlCoordinatesProjector.cs.

use super::{GeoCoordinate, GeographicBounds, MapProjector};
use ferroui_base::{Point, Rect};

#[derive(Clone, Debug, PartialEq)]
pub struct ControlCoordinatesProjector {
    bounds: Rect,
    limits: [f64; 4],
}

impl ControlCoordinatesProjector {
    pub const DEFAULT_MIN_LATITUDE: f64 = -90.0;
    pub const DEFAULT_MAX_LATITUDE: f64 = 90.0;
    pub const DEFAULT_MIN_LONGITUDE: f64 = -180.0;
    pub const DEFAULT_MAX_LONGITUDE: f64 = 180.0;
    pub fn new(bounds: Rect, limits: GeographicBounds) -> Self {
        Self {
            bounds,
            limits: limits.normalized(-90.0, 90.0),
        }
    }

    pub fn preferred_ratio(limits: GeographicBounds) -> f64 {
        let [min_lat, max_lat, min_lon, max_lon] = limits.normalized(-90.0, 90.0);
        (max_lon - min_lon) / (max_lat - min_lat)
    }
}

impl MapProjector for ControlCoordinatesProjector {
    fn map_bounds(&self) -> Rect {
        self.bounds
    }

    fn to_map(&self, longitude: f64, latitude: f64) -> Point {
        let [min_lat, max_lat, min_lon, max_lon] = self.limits;
        Point::new(
            self.bounds.x + (longitude - min_lon) / (max_lon - min_lon) * self.bounds.width,
            self.bounds.y + (max_lat - latitude) / (max_lat - min_lat) * self.bounds.height,
        )
    }

    fn to_coordinates(&self, point: Point) -> Option<GeoCoordinate> {
        if !super::map_projector::valid_rect(self.bounds)
            || !point.x.is_finite()
            || !point.y.is_finite()
        {
            return None;
        }
        let [min_lat, max_lat, min_lon, max_lon] = self.limits;
        let coordinate = GeoCoordinate::new(
            min_lon + (point.x - self.bounds.x) / self.bounds.width * (max_lon - min_lon),
            max_lat - (point.y - self.bounds.y) / self.bounds.height * (max_lat - min_lat),
        );
        coordinate.is_finite().then_some(coordinate)
    }
}
