//! The Mercator projection (`MercatorProjector`) with the source's asymmetric default crop.

use super::GeographicBounds;
use ferroui_base::Point;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

/// The last latitude with a finite Mercator image; the poles map to infinity.
const MAX_MERCATOR_LATITUDE: f64 = 89.999_999;

/// Projects longitude and latitude with the Mercator projection. Lands outside the bounds are
/// extrapolated past the map area, which the chart clips.
#[derive(Clone, Debug, PartialEq)]
pub struct MercatorProjector {
    width: f64,
    height: f64,
    offset_x: f64,
    offset_y: f64,
    /// `[min_lat, max_lat, min_lon, max_lon]`, normalized.
    limits: [f64; 4],
    min_lat_merc_n: f64,
    merc_n_range: f64,
}

impl MercatorProjector {
    /// -65 degrees crops the empty band below the populated continents.
    pub const DEFAULT_MIN_LATITUDE: f64 = -65.0;
    /// 85 degrees, the usual Mercator limit, keeps Greenland whole.
    pub const DEFAULT_MAX_LATITUDE: f64 = 85.0;
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
        let limits = Self::limits(bounds);
        let min_lat_merc_n = merc_n(limits[0]);
        Self {
            width: map_width,
            height: map_height,
            offset_x,
            offset_y,
            limits,
            min_lat_merc_n,
            merc_n_range: merc_n(limits[1]) - min_lat_merc_n,
        }
    }

    /// The normalized bounds; a bound at a pole moves to the last finite latitude.
    fn limits(bounds: GeographicBounds) -> [f64; 4] {
        let mut limits = bounds.normalized(Self::DEFAULTS);
        limits[0] = limits[0].clamp(-MAX_MERCATOR_LATITUDE, MAX_MERCATOR_LATITUDE);
        limits[1] = limits[1].clamp(-MAX_MERCATOR_LATITUDE, MAX_MERCATOR_LATITUDE);
        if limits[0] == limits[1] {
            limits[0] = Self::DEFAULT_MIN_LATITUDE;
            limits[1] = Self::DEFAULT_MAX_LATITUDE;
        }
        limits
    }

    /// The conformal aspect ratio, width to height, of the framed bounds (`GetPreferredRatio`):
    /// about 1.355 for the default crop, 1 for the symmetric 85 degrees.
    pub fn preferred_ratio(bounds: GeographicBounds) -> [f64; 2] {
        let [min_lat, max_lat, min_lon, max_lon] = Self::limits(bounds);
        let merc_n_range = merc_n(max_lat) - merc_n(min_lat);
        [(max_lon - min_lon).to_radians() / merc_n_range, 1.0]
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

    /// The minimum longitude spans to the left edge and the maximum to the right; the Mercator
    /// images of the latitude bounds span the height. A latitude at a pole takes the last finite
    /// latitude, so that the poles of a map (Antarctica) stay finite.
    pub fn to_map(&self, longitude: f64, latitude: f64) -> Point {
        let merc_n = merc_n(latitude.clamp(-MAX_MERCATOR_LATITUDE, MAX_MERCATOR_LATITUDE));
        let py = self.height - self.height * (merc_n - self.min_lat_merc_n) / self.merc_n_range;
        let [_, _, min_lon, max_lon] = self.limits;
        Point::new(
            (longitude - min_lon) * (self.width / (max_lon - min_lon)) + self.offset_x,
            py + self.offset_y,
        )
    }

    /// The closed-form inverse: the linear longitude, then the inverse Gudermannian of the
    /// Mercator value, `2 atan(e^n) - pi/2`.
    pub fn to_coordinates(&self, screen: Point) -> Option<(f64, f64)> {
        let [_, _, min_lon, max_lon] = self.limits;
        let longitude = min_lon + (screen.x - self.offset_x) / self.width * (max_lon - min_lon);
        let py = screen.y - self.offset_y;
        let merc_n = (self.height - py) / self.height * self.merc_n_range + self.min_lat_merc_n;
        let latitude = (2.0 * merc_n.exp().atan() - FRAC_PI_2).to_degrees();
        (longitude.is_finite() && latitude.is_finite()).then_some((longitude, latitude))
    }
}

/// The Mercator image of a latitude in degrees: `ln(tan(pi/4 + lat/2))`.
fn merc_n(latitude: f64) -> f64 {
    (FRAC_PI_4 + latitude.to_radians() / 2.0).tan().ln()
}
