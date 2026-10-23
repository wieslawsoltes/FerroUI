//! Orthographic globe projection, visible hemisphere and inverse picking.
// Source: LiveChartsCore/Geo/OrthographicProjector.cs.

use super::{GeoCoordinate, MapProjector};
use ferroui_base::{Point, Rect};

#[derive(Clone, Debug, PartialEq)]
pub struct OrthographicProjector {
    bounds: Rect,
    center: GeoCoordinate,
    sin_center_lat: f64,
    cos_center_lat: f64,
}

impl OrthographicProjector {
    pub const PREFERRED_RATIO: f64 = 1.0;
    pub fn new(bounds: Rect, center: GeoCoordinate) -> Self {
        let center = if center.is_finite() {
            center
        } else {
            GeoCoordinate::default()
        };
        let (sin_center_lat, cos_center_lat) = center.latitude.to_radians().sin_cos();
        Self {
            bounds,
            center,
            sin_center_lat,
            cos_center_lat,
        }
    }

    pub fn center(&self) -> GeoCoordinate {
        self.center
    }
    pub fn radius(&self) -> f64 {
        self.bounds.width.min(self.bounds.height) * 0.5
    }
    pub fn screen_center(&self) -> Point {
        Point::new(
            self.bounds.x + self.bounds.width * 0.5,
            self.bounds.y + self.bounds.height * 0.5,
        )
    }

    /// Camera-space hemisphere coordinate; positive is toward the observer.
    pub fn visibility(&self, longitude: f64, latitude: f64) -> f64 {
        self.sin_center_lat * latitude.to_radians().sin()
            + self.cos_center_lat
                * latitude.to_radians().cos()
                * (longitude - self.center.longitude).to_radians().cos()
    }
}

impl MapProjector for OrthographicProjector {
    fn map_bounds(&self) -> Rect {
        self.bounds
    }

    fn is_visible(&self, longitude: f64, latitude: f64) -> bool {
        self.visibility(longitude, latitude) > 0.0
    }

    fn to_map(&self, longitude: f64, latitude: f64) -> Point {
        let (sin_lat, cos_lat) = latitude.to_radians().sin_cos();
        let (sin_lon, cos_lon) = (longitude - self.center.longitude).to_radians().sin_cos();
        let center = self.screen_center();
        Point::new(
            center.x + self.radius() * cos_lat * sin_lon,
            center.y
                - self.radius()
                    * (self.cos_center_lat * sin_lat - self.sin_center_lat * cos_lat * cos_lon),
        )
    }

    fn to_coordinates(&self, point: Point) -> Option<GeoCoordinate> {
        if !super::map_projector::valid_rect(self.bounds)
            || !point.x.is_finite()
            || !point.y.is_finite()
        {
            return None;
        }
        let center = self.screen_center();
        let px = point.x - center.x;
        let py = center.y - point.y;
        let rho = px.hypot(py);
        if rho > self.radius() {
            return None;
        }
        if rho <= f64::EPSILON * self.radius().max(1.0) {
            return Some(self.center);
        }
        let sin_c = (rho / self.radius()).min(1.0);
        let cos_c = (1.0 - sin_c * sin_c).max(0.0).sqrt();
        let latitude = (cos_c * self.sin_center_lat + py * sin_c * self.cos_center_lat / rho)
            .clamp(-1.0, 1.0)
            .asin()
            .to_degrees();
        let longitude = self.center.longitude
            + (px * sin_c)
                .atan2(rho * self.cos_center_lat * cos_c - py * self.sin_center_lat * sin_c)
                .to_degrees();
        Some(GeoCoordinate::new(longitude, latitude))
    }
}
