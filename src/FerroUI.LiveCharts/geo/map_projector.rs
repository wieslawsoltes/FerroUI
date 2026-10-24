//! Projection contracts and aspect-preserving map layout.
// Source: LiveChartsCore/Geo/MapProjector.cs, MapProjection.cs and Maps.cs.

use ferroui_base::{Point, Rect};

use super::{ControlCoordinatesProjector, MercatorProjector, OrthographicProjector};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GeoCoordinate {
    pub longitude: f64,
    pub latitude: f64,
}

impl GeoCoordinate {
    pub const fn new(longitude: f64, latitude: f64) -> Self {
        Self {
            longitude,
            latitude,
        }
    }

    pub fn is_finite(self) -> bool {
        self.longitude.is_finite() && self.latitude.is_finite()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MapProjection {
    #[default]
    Default,
    Mercator,
    Orthographic,
}
ferroui_base::ferro_markup_enum!(MapProjection {
    Default,
    Mercator,
    Orthographic
});

/// Unspecified or non-finite limits use the selected projection's defaults.
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

    pub(crate) fn normalized(self, default_min_lat: f64, default_max_lat: f64) -> [f64; 4] {
        let mut min_lat = self
            .min_latitude
            .filter(|v| v.is_finite())
            .unwrap_or(default_min_lat);
        let mut max_lat = self
            .max_latitude
            .filter(|v| v.is_finite())
            .unwrap_or(default_max_lat);
        let mut min_lon = self
            .min_longitude
            .filter(|v| v.is_finite())
            .unwrap_or(-180.0);
        let mut max_lon = self
            .max_longitude
            .filter(|v| v.is_finite())
            .unwrap_or(180.0);
        if min_lat > max_lat {
            std::mem::swap(&mut min_lat, &mut max_lat);
        }
        if min_lon > max_lon {
            std::mem::swap(&mut min_lon, &mut max_lon);
        }
        if min_lat == max_lat || !(max_lat - min_lat).is_finite() {
            min_lat = default_min_lat;
            max_lat = default_max_lat;
        }
        if min_lon == max_lon || !(max_lon - min_lon).is_finite() {
            min_lon = -180.0;
            max_lon = 180.0;
        }
        [min_lat, max_lat, min_lon, max_lon]
    }
}

pub trait MapProjector {
    fn map_bounds(&self) -> Rect;
    fn to_map(&self, longitude: f64, latitude: f64) -> Point;
    fn to_coordinates(&self, point: Point) -> Option<GeoCoordinate>;
    fn is_visible(&self, _longitude: f64, _latitude: f64) -> bool {
        true
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Projector {
    Default(ControlCoordinatesProjector),
    Mercator(MercatorProjector),
    Orthographic(OrthographicProjector),
}

impl Projector {
    /// Fits the natural projection aspect into `rect`, preserving its offset.
    pub fn fit(
        projection: MapProjection,
        rect: Rect,
        limits: GeographicBounds,
        center: GeoCoordinate,
    ) -> Self {
        let ratio = match projection {
            MapProjection::Default => ControlCoordinatesProjector::preferred_ratio(limits),
            MapProjection::Mercator => MercatorProjector::preferred_ratio(limits),
            MapProjection::Orthographic => 1.0,
        };
        let mut fitted = rect;
        if rect.width / ratio < rect.height {
            fitted.height = rect.width / ratio;
            fitted.y += (rect.height - fitted.height) * 0.5;
        } else {
            fitted.width = rect.height * ratio;
            fitted.x += (rect.width - fitted.width) * 0.5;
        }
        match projection {
            MapProjection::Default => {
                Self::Default(ControlCoordinatesProjector::new(fitted, limits))
            }
            MapProjection::Mercator => Self::Mercator(MercatorProjector::new(fitted, limits)),
            MapProjection::Orthographic => {
                Self::Orthographic(OrthographicProjector::new(fitted, center))
            }
        }
    }
}

impl MapProjector for Projector {
    fn map_bounds(&self) -> Rect {
        match self {
            Self::Default(p) => p.map_bounds(),
            Self::Mercator(p) => p.map_bounds(),
            Self::Orthographic(p) => p.map_bounds(),
        }
    }
    fn to_map(&self, longitude: f64, latitude: f64) -> Point {
        match self {
            Self::Default(p) => p.to_map(longitude, latitude),
            Self::Mercator(p) => p.to_map(longitude, latitude),
            Self::Orthographic(p) => p.to_map(longitude, latitude),
        }
    }
    fn to_coordinates(&self, point: Point) -> Option<GeoCoordinate> {
        match self {
            Self::Default(p) => p.to_coordinates(point),
            Self::Mercator(p) => p.to_coordinates(point),
            Self::Orthographic(p) => p.to_coordinates(point),
        }
    }
    fn is_visible(&self, longitude: f64, latitude: f64) -> bool {
        match self {
            Self::Default(p) => p.is_visible(longitude, latitude),
            Self::Mercator(p) => p.is_visible(longitude, latitude),
            Self::Orthographic(p) => p.is_visible(longitude, latitude),
        }
    }
}

pub(crate) fn valid_rect(rect: Rect) -> bool {
    rect.x.is_finite()
        && rect.y.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && rect.width > 0.0
        && rect.height > 0.0
}
