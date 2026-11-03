//! The bundled world map and the fitting of a projection into a map area (`Maps`).

use super::{
    ControlCoordinatesProjector, DrawnMap, GeographicBounds, MapProjection, MapProjector,
    MercatorProjector, OrthographicProjector,
};
use crate::scene::ChartError;
use std::sync::OnceLock;

/// The world map of the source, `LiveChartsCore/Geo/world.geojson` (177 countries, keyed by
/// their ISO 3166-1 alpha-3 codes in lower case), embedded as the source embeds it.
const WORLD_GEOJSON: &str = include_str!("world.geojson");

/// The maps of the source (`Maps`).
pub struct Maps;

impl Maps {
    /// The bundled world map (`GetWorldMap`), parsed once and shared.
    pub fn world_map() -> &'static DrawnMap {
        static WORLD: OnceLock<DrawnMap> = OnceLock::new();
        WORLD.get_or_init(|| {
            DrawnMap::from_geojson(WORLD_GEOJSON).expect("the bundled world map is valid GeoJSON")
        })
    }

    /// A map of one layer, `default`, read from GeoJSON text (`GetMapFromStreamReader`).
    pub fn map_from_geojson(json: &str) -> Result<DrawnMap, ChartError> {
        DrawnMap::from_geojson(json)
    }

    /// A projector of the projection fitted into the map area (`BuildProjector`): the area keeps
    /// the aspect ratio of the projection, centred on the shorter side, and the orthographic
    /// globe faces `center` (longitude, latitude) in degrees.
    pub fn build_projector(
        projection: MapProjection,
        map_size: [f64; 2],
        base_offset: [f64; 2],
        center: (f64, f64),
        bounds: GeographicBounds,
    ) -> MapProjector {
        let ratio = match projection {
            MapProjection::Default => ControlCoordinatesProjector::preferred_ratio(bounds),
            MapProjection::Orthographic => OrthographicProjector::PREFERRED_RATIO,
            MapProjection::Mercator => MercatorProjector::preferred_ratio(bounds),
        };
        let [mut width, mut height] = map_size;
        let [mut ox, mut oy] = base_offset;
        let normalized_w = width / ratio[0];
        let normalized_h = height / ratio[1];
        if normalized_w < normalized_h {
            let h = width * ratio[1] / ratio[0];
            oy += (height - h) * 0.5;
            height = h;
        } else {
            let w = height * ratio[0] / ratio[1];
            ox += (width - w) * 0.5;
            width = w;
        }
        match projection {
            MapProjection::Default => MapProjector::Default(ControlCoordinatesProjector::new(
                width, height, ox, oy, bounds,
            )),
            MapProjection::Orthographic => MapProjector::Orthographic(OrthographicProjector::new(
                width, height, ox, oy, center.0, center.1,
            )),
            MapProjection::Mercator => {
                MapProjector::Mercator(MercatorProjector::new(width, height, ox, oy, bounds))
            }
        }
    }
}
