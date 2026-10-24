//! Mercator projection with the upstream asymmetric default crop.
// Source: LiveChartsCore/Geo/MercatorProjector.cs.

use super::{GeoCoordinate, GeographicBounds, MapProjector};
use ferroui_base::{Point, Rect};

#[derive(Clone, Debug, PartialEq)]
pub struct MercatorProjector {
    bounds: Rect,
    limits: [f64; 4],
    min_mercator: f64,
    mercator_range: f64,
}

impl MercatorProjector {
    pub const DEFAULT_MIN_LATITUDE: f64 = -65.0;
    pub const DEFAULT_MAX_LATITUDE: f64 = 85.0;
    pub const DEFAULT_MIN_LONGITUDE: f64 = -180.0;
    pub const DEFAULT_MAX_LONGITUDE: f64 = 180.0;

    pub fn new(bounds: Rect, limits: GeographicBounds) -> Self {
        let limits = Self::limits(limits);
        let min_mercator = mercator_n(limits[0]);
        Self {
            bounds,
            limits,
            min_mercator,
            mercator_range: mercator_n(limits[1]) - min_mercator,
        }
    }

    fn limits(limits: GeographicBounds) -> [f64; 4] {
        let mut limits = limits.normalized(Self::DEFAULT_MIN_LATITUDE, Self::DEFAULT_MAX_LATITUDE);
        // Geographic poles have no finite Mercator image. Keep user-provided
        // bounds finite rather than letting infinities enter the native scene.
        limits[0] = limits[0].clamp(-89.999_999, 89.999_999);
        limits[1] = limits[1].clamp(-89.999_999, 89.999_999);
        if limits[0] == limits[1] {
            limits[0] = Self::DEFAULT_MIN_LATITUDE;
            limits[1] = Self::DEFAULT_MAX_LATITUDE;
        }
        limits
    }

    pub fn preferred_ratio(limits: GeographicBounds) -> f64 {
        let [min_lat, max_lat, min_lon, max_lon] = Self::limits(limits);
        (max_lon - min_lon).to_radians() / (mercator_n(max_lat) - mercator_n(min_lat))
    }
}

fn mercator_n(latitude: f64) -> f64 {
    (std::f64::consts::FRAC_PI_4 + latitude.to_radians() * 0.5)
        .tan()
        .ln()
}

impl MapProjector for MercatorProjector {
    fn map_bounds(&self) -> Rect {
        self.bounds
    }

    fn to_map(&self, longitude: f64, latitude: f64) -> Point {
        let latitude = latitude.clamp(-89.999_999, 89.999_999);
        Point::new(
            self.bounds.x
                + (longitude - self.limits[2]) / (self.limits[3] - self.limits[2])
                    * self.bounds.width,
            self.bounds.y
                + self.bounds.height
                    * (1.0 - (mercator_n(latitude) - self.min_mercator) / self.mercator_range),
        )
    }

    fn to_coordinates(&self, point: Point) -> Option<GeoCoordinate> {
        if !super::map_projector::valid_rect(self.bounds)
            || !point.x.is_finite()
            || !point.y.is_finite()
        {
            return None;
        }
        let mercator = (1.0 - (point.y - self.bounds.y) / self.bounds.height) * self.mercator_range
            + self.min_mercator;
        let coordinate = GeoCoordinate::new(
            self.limits[2]
                + (point.x - self.bounds.x) / self.bounds.width * (self.limits[3] - self.limits[2]),
            (2.0 * mercator.exp().atan() - std::f64::consts::FRAC_PI_2).to_degrees(),
        );
        coordinate.is_finite().then_some(coordinate)
    }
}
