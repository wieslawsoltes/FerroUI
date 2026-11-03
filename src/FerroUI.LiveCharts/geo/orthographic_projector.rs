//! The orthographic (globe) projection (`OrthographicProjector`) and its horizon.

use ferroui_base::Point;

/// Projects longitude and latitude on a globe facing the centre coordinate. The far side of the
/// globe is not visible.
#[derive(Clone, Debug, PartialEq)]
pub struct OrthographicProjector {
    width: f64,
    height: f64,
    offset_x: f64,
    offset_y: f64,
    center_lon: f64,
    center_lat: f64,
    sin_center_lat: f64,
    cos_center_lat: f64,
    radius: f64,
    screen_center_x: f64,
    screen_center_y: f64,
}

impl OrthographicProjector {
    /// The aspect ratio of a circular globe.
    pub const PREFERRED_RATIO: [f64; 2] = [1.0, 1.0];

    /// A globe in the map area of the given size at the given offset, facing `center_lon` and
    /// `center_lat` (degrees). Its radius is half of the smaller side of the area.
    pub fn new(
        map_width: f64,
        map_height: f64,
        offset_x: f64,
        offset_y: f64,
        center_lon: f64,
        center_lat: f64,
    ) -> Self {
        let (sin_center_lat, cos_center_lat) = center_lat.to_radians().sin_cos();
        Self {
            width: map_width,
            height: map_height,
            offset_x,
            offset_y,
            center_lon,
            center_lat,
            sin_center_lat,
            cos_center_lat,
            radius: map_width.min(map_height) / 2.0,
            screen_center_x: map_width / 2.0 + offset_x,
            screen_center_y: map_height / 2.0 + offset_y,
        }
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
    /// The longitude the globe faces.
    pub fn center_longitude(&self) -> f64 {
        self.center_lon
    }
    /// The latitude the globe faces.
    pub fn center_latitude(&self) -> f64 {
        self.center_lat
    }
    /// The radius of the globe on the screen.
    pub fn radius(&self) -> f64 {
        self.radius
    }
    /// The screen position of the centre of the globe.
    pub fn screen_center_x(&self) -> f64 {
        self.screen_center_x
    }
    pub fn screen_center_y(&self) -> f64 {
        self.screen_center_y
    }

    /// Whether the coordinate is on the near hemisphere: the cosine of its angular distance from
    /// the centre is positive.
    pub fn is_visible(&self, longitude: f64, latitude: f64) -> bool {
        let lat = latitude.to_radians();
        let lon_diff = (longitude - self.center_lon).to_radians();
        let cos_c =
            self.sin_center_lat * lat.sin() + self.cos_center_lat * lat.cos() * lon_diff.cos();
        cos_c > 0.0
    }

    pub fn to_map(&self, longitude: f64, latitude: f64) -> Point {
        let (sin_lat, cos_lat) = latitude.to_radians().sin_cos();
        let (sin_lon, cos_lon) = (longitude - self.center_lon).to_radians().sin_cos();
        let px = self.radius * cos_lat * sin_lon;
        let py = self.radius
            * (self.cos_center_lat * sin_lat - self.sin_center_lat * cos_lat * cos_lon);
        Point::new(self.screen_center_x + px, self.screen_center_y - py)
    }

    /// The inverse orthographic projection (Snyder, Map Projections, USGS): only the near
    /// hemisphere is recoverable, and a position outside the disc has no coordinate.
    pub fn to_coordinates(&self, screen: Point) -> Option<(f64, f64)> {
        let px = screen.x - self.screen_center_x;
        let py = self.screen_center_y - screen.y;
        let rho = (px * px + py * py).sqrt();
        if !(rho <= self.radius) {
            return None;
        }
        if rho < f64::from_bits(1) {
            return Some((self.center_lon, self.center_lat));
        }
        let sin_c = rho / self.radius;
        let cos_c = (1.0 - sin_c * sin_c).sqrt();
        let lat = (cos_c * self.sin_center_lat + py * sin_c * self.cos_center_lat / rho).asin();
        let lon = (px * sin_c)
            .atan2(rho * self.cos_center_lat * cos_c - py * self.sin_center_lat * sin_c);
        Some((self.center_lon + lon.to_degrees(), lat.to_degrees()))
    }
}
