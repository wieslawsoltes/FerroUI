//! The default (equirectangular) projection (`ControlCoordinatesProjector`).

use super::GeographicBounds;
use ferroui_base::Point;

/// Longitude and latitude mapped linearly to the map area, from the top-left corner.
#[derive(Clone, Debug, PartialEq)]
pub struct ControlCoordinatesProjector {
    width: f64,
    height: f64,
    offset_x: f64,
    offset_y: f64,
    /// `[min_lat, max_lat, min_lon, max_lon]`, normalized.
    limits: [f64; 4],
}

impl ControlCoordinatesProjector {
    /// The whole earth: from -90 to 90 degrees of latitude.
    pub const DEFAULT_MIN_LATITUDE: f64 = -90.0;
    pub const DEFAULT_MAX_LATITUDE: f64 = 90.0;
    pub const DEFAULT_MIN_LONGITUDE: f64 = -180.0;
    pub const DEFAULT_MAX_LONGITUDE: f64 = 180.0;
    const DEFAULTS: [f64; 4] = [
        Self::DEFAULT_MIN_LATITUDE,
        Self::DEFAULT_MAX_LATITUDE,
        Self::DEFAULT_MIN_LONGITUDE,
        Self::DEFAULT_MAX_LONGITUDE,
    ];

    /// A projector of the map area of the given size at the given offset, framed on `bounds`.
    pub fn new(
        map_width: f64,
        map_height: f64,
        offset_x: f64,
        offset_y: f64,
        bounds: GeographicBounds,
    ) -> Self {
        Self {
            width: map_width,
            height: map_height,
            offset_x,
            offset_y,
            limits: bounds.normalized(Self::DEFAULTS),
        }
    }

    /// The aspect ratio, width to height, of the framed bounds (`GetPreferredRatio`).
    pub fn preferred_ratio(bounds: GeographicBounds) -> [f64; 2] {
        let [min_lat, max_lat, min_lon, max_lon] = bounds.normalized(Self::DEFAULTS);
        [max_lon - min_lon, max_lat - min_lat]
    }

    pub fn map_width(&self) -> f64 {
        self.width
    }
    pub fn map_height(&self) -> f64 {
        self.height
    }
    pub fn x_offset(&self) -> f64 {
        self.offset_x
    }
    pub fn y_offset(&self) -> f64 {
        self.offset_y
    }

    pub fn to_map(&self, longitude: f64, latitude: f64) -> Point {
        let [min_lat, max_lat, min_lon, max_lon] = self.limits;
        Point::new(
            self.offset_x + (longitude - min_lon) / (max_lon - min_lon) * self.width,
            self.offset_y + (max_lat - latitude) / (max_lat - min_lat) * self.height,
        )
    }

    /// The linear inverse: every position has a coordinate, possibly outside the bounds.
    pub fn to_coordinates(&self, screen: Point) -> Option<(f64, f64)> {
        let [min_lat, max_lat, min_lon, max_lon] = self.limits;
        let longitude = min_lon + (screen.x - self.offset_x) / self.width * (max_lon - min_lon);
        let latitude = max_lat - (screen.y - self.offset_y) / self.height * (max_lat - min_lat);
        (longitude.is_finite() && latitude.is_finite()).then_some((longitude, latitude))
    }
}
