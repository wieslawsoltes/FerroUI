//! The projections of a geographic map (`MapProjection`, `MapProjector`) and the bounds a flat
//! projection is framed on.

use super::{ControlCoordinatesProjector, MercatorProjector, OrthographicProjector};
use ferroui_base::Point;

/// The projection of a map (`MapProjection`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MapProjection {
    /// Longitude and latitude mapped linearly to the control (equirectangular), from the
    /// top-left corner.
    #[default]
    Default,
    /// The Mercator projection of map engines such as Google or Bing maps.
    Mercator,
    /// A globe; the far side of it is clipped at the horizon.
    Orthographic,
}
ferroui_base::ferro_markup_enum!(MapProjection {
    Default,
    Mercator,
    Orthographic
});

/// The latitudes and longitudes, in degrees, a flat projection is framed on (the
/// `MinLatitude`, `MaxLatitude`, `MinLongitude` and `MaxLongitude` of `IGeoMapView`). `None`, or
/// `NaN` as in the source, takes the default of the projection; an inverted pair is swapped, and
/// a pair of equal values (or with an infinite span) takes the defaults of its axis. The
/// orthographic projection ignores them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GeographicBounds {
    pub min_latitude: Option<f64>,
    pub max_latitude: Option<f64>,
    pub min_longitude: Option<f64>,
    pub max_longitude: Option<f64>,
}
impl GeographicBounds {
    pub const fn new(
        min_latitude: f64,
        max_latitude: f64,
        min_longitude: f64,
        max_longitude: f64,
    ) -> Self {
        Self {
            min_latitude: Some(min_latitude),
            max_latitude: Some(max_latitude),
            min_longitude: Some(min_longitude),
            max_longitude: Some(max_longitude),
        }
    }

    /// `NormalizeBounds` of the flat projectors: `[min_lat, max_lat, min_lon, max_lon]`.
    pub(crate) fn normalized(self, defaults: [f64; 4]) -> [f64; 4] {
        let pick =
            |value: Option<f64>, default: f64| value.filter(|v| !v.is_nan()).unwrap_or(default);
        let mut min_lat = pick(self.min_latitude, defaults[0]);
        let mut max_lat = pick(self.max_latitude, defaults[1]);
        let mut min_lon = pick(self.min_longitude, defaults[2]);
        let mut max_lon = pick(self.max_longitude, defaults[3]);
        if min_lat > max_lat {
            std::mem::swap(&mut min_lat, &mut max_lat);
        }
        if min_lon > max_lon {
            std::mem::swap(&mut min_lon, &mut max_lon);
        }
        // An infinite bound has no finite image either; it takes the defaults with the
        // degenerate pair of the source.
        if (min_lat - max_lat).abs() <= f64::from_bits(1) || !(max_lat - min_lat).is_finite() {
            min_lat = defaults[0];
            max_lat = defaults[1];
        }
        if (min_lon - max_lon).abs() <= f64::from_bits(1) || !(max_lon - min_lon).is_finite() {
            min_lon = defaults[2];
            max_lon = defaults[3];
        }
        [min_lat, max_lat, min_lon, max_lon]
    }
}

/// A projector built for one map area (`MapProjector`): the source's abstract class is the enum
/// of its three projections. Screen positions are `f64` where the source uses `float`.
#[derive(Clone, Debug, PartialEq)]
pub enum MapProjector {
    Default(ControlCoordinatesProjector),
    Mercator(MercatorProjector),
    Orthographic(OrthographicProjector),
}

impl MapProjector {
    /// The width of the map area.
    pub fn map_width(&self) -> f64 {
        match self {
            Self::Default(p) => p.map_width(),
            Self::Mercator(p) => p.map_width(),
            Self::Orthographic(p) => p.map_width(),
        }
    }
    /// The height of the map area.
    pub fn map_height(&self) -> f64 {
        match self {
            Self::Default(p) => p.map_height(),
            Self::Mercator(p) => p.map_height(),
            Self::Orthographic(p) => p.map_height(),
        }
    }
    /// The left of the map area.
    pub fn x_offset(&self) -> f64 {
        match self {
            Self::Default(p) => p.x_offset(),
            Self::Mercator(p) => p.x_offset(),
            Self::Orthographic(p) => p.x_offset(),
        }
    }
    /// The top of the map area.
    pub fn y_offset(&self) -> f64 {
        match self {
            Self::Default(p) => p.y_offset(),
            Self::Mercator(p) => p.y_offset(),
            Self::Orthographic(p) => p.y_offset(),
        }
    }
    /// The map area, which the chart clips the lands to.
    pub fn map_rect(&self) -> ferroui_base::Rect {
        ferroui_base::Rect::new(
            self.x_offset(),
            self.y_offset(),
            self.map_width(),
            self.map_height(),
        )
    }
    /// Projects a longitude and latitude, in degrees, to the screen (`ToMap`).
    pub fn to_map(&self, longitude: f64, latitude: f64) -> Point {
        match self {
            Self::Default(p) => p.to_map(longitude, latitude),
            Self::Mercator(p) => p.to_map(longitude, latitude),
            Self::Orthographic(p) => p.to_map(longitude, latitude),
        }
    }
    /// Whether the coordinate is on the visible side of the projection (`IsVisible`); always
    /// true for the flat projections.
    pub fn is_visible(&self, longitude: f64, latitude: f64) -> bool {
        match self {
            Self::Orthographic(p) => p.is_visible(longitude, latitude),
            _ => true,
        }
    }
    /// The inverse of [`Self::to_map`] (`ToCoordinates`): the longitude and latitude of a screen
    /// position, `None` where the projection has none (outside the disc of a globe).
    pub fn to_coordinates(&self, screen: Point) -> Option<(f64, f64)> {
        match self {
            Self::Default(p) => p.to_coordinates(screen),
            Self::Mercator(p) => p.to_coordinates(screen),
            Self::Orthographic(p) => p.to_coordinates(screen),
        }
    }
    /// The globe of an orthographic projector.
    pub fn as_orthographic(&self) -> Option<&OrthographicProjector> {
        match self {
            Self::Orthographic(p) => Some(p),
            _ => None,
        }
    }
}
