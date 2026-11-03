//! Geographic maps adapted from the pinned `LiveChartsCore/Geo`: the default (equirectangular),
//! Mercator and orthographic projections with their inverses, maps read from GeoJSON into layers
//! of lands, and the bundled world map of the source.

mod control_coordinates_projector;
mod drawn_map;
mod map_projector;
mod maps;
mod mercator_projector;
mod orthographic_projector;

pub use control_coordinates_projector::ControlCoordinatesProjector;
pub use drawn_map::{
    DrawnMap, LandData, LandDefinition, MapLayer, DEFAULT_LAYER_FILL, DEFAULT_LAYER_STROKE,
};
pub use map_projector::{GeographicBounds, MapProjection, MapProjector};
pub use maps::Maps;
pub use mercator_projector::MercatorProjector;
pub use orthographic_projector::OrthographicProjector;

#[cfg(test)]
mod geo_tests;
