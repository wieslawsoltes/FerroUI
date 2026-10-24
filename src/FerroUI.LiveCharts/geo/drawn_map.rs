//! Owned GeoJSON polygon maps, named layers, and built-in world data.
// Source: LiveChartsCore/Geo/{DrawnMap,MapLayer,LandDefinition,LandData,Maps}.cs.
// Copyright (c) 2021 Alberto Rodriguez Orozco & LiveCharts Contributors. MIT.

use super::GeoCoordinate;
use crate::scene::ChartError;
use ferroui_base::media::Color;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LandBounds {
    pub min_longitude: f64,
    pub min_latitude: f64,
    pub max_longitude: f64,
    pub max_latitude: f64,
}
impl LandBounds {
    pub fn center(self) -> GeoCoordinate {
        GeoCoordinate::new(
            self.min_longitude * 0.5 + self.max_longitude * 0.5,
            self.min_latitude * 0.5 + self.max_latitude * 0.5,
        )
    }
}

/// The first ring is the exterior. Subsequent rings are holes, retained separately.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GeoPolygon {
    pub rings: Vec<Vec<GeoCoordinate>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LandDefinition {
    pub short_name: String,
    pub name: String,
    pub set_of: String,
    pub polygons: Vec<GeoPolygon>,
    pub bounds: Option<LandBounds>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapLayer {
    pub name: String,
    pub process_index: i32,
    pub is_visible: bool,
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub stroke_width: f64,
    pub lands: BTreeMap<String, LandDefinition>,
}
impl Default for MapLayer {
    fn default() -> Self {
        Self::new("default")
    }
}
impl MapLayer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            process_index: 0,
            is_visible: true,
            fill: None,
            stroke: None,
            stroke_width: 1.0,
            lands: BTreeMap::new(),
        }
    }

    /// Validates the complete input before changing the layer.
    pub fn add_geojson(&mut self, json: &str) -> Result<usize, ChartError> {
        let value: Value = serde_json::from_str(json)
            .map_err(|error| ChartError::InvalidData(format!("Invalid GeoJSON: {error}")))?;
        let features = value
            .get("features")
            .and_then(Value::as_array)
            .ok_or_else(|| ChartError::InvalidData("GeoJSON features must be an array".into()))?;
        let mut additions = Vec::new();
        for (feature_index, feature) in features.iter().enumerate() {
            let Some(geometry) = feature.get("geometry") else {
                continue;
            };
            let Some(coordinates) = geometry.get("coordinates") else {
                continue;
            };
            let Some(kind) = geometry.get("type").and_then(Value::as_str) else {
                continue;
            };
            if coordinates.is_null() {
                continue;
            }
            let polygons = match kind {
                "Polygon" => vec![read_polygon(coordinates, feature_index)?],
                "MultiPolygon" => coordinates
                    .as_array()
                    .ok_or_else(|| coordinate_error(feature_index))?
                    .iter()
                    .map(|value| read_polygon(value, feature_index))
                    .collect::<Result<Vec<_>, _>>()?,
                _ => continue,
            };
            let properties = feature.get("properties");
            let name =
                property(properties, "name").unwrap_or_else(|| format!("feature_{feature_index}"));
            let short_name = property(properties, "shortName").unwrap_or_else(|| name.clone());
            let set_of = property(properties, "setOf").unwrap_or_else(|| "?".into());
            let mut bounds: Option<LandBounds> = None;
            for point in polygons.iter().flat_map(|polygon| &polygon.rings).flatten() {
                if let Some(bounds) = &mut bounds {
                    bounds.min_longitude = bounds.min_longitude.min(point.longitude);
                    bounds.max_longitude = bounds.max_longitude.max(point.longitude);
                    bounds.min_latitude = bounds.min_latitude.min(point.latitude);
                    bounds.max_latitude = bounds.max_latitude.max(point.latitude);
                } else {
                    bounds = Some(LandBounds {
                        min_longitude: point.longitude,
                        max_longitude: point.longitude,
                        min_latitude: point.latitude,
                        max_latitude: point.latitude,
                    });
                }
            }
            additions.push((
                feature_index,
                LandDefinition {
                    short_name,
                    name,
                    set_of,
                    polygons,
                    bounds,
                },
            ));
        }
        let count = additions.len();
        for (feature_index, land) in additions {
            let mut key = land.short_name.clone();
            if self.lands.contains_key(&key) {
                key = format!("{}_{feature_index}", land.short_name);
            }
            let mut collision = 2;
            while self.lands.contains_key(&key) {
                key = format!("{}_{feature_index}_{collision}", land.short_name);
                collision += 1;
            }
            self.lands.insert(key, land);
        }
        Ok(count)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawnMap {
    /// Layer insertion order is retained; drawing uses `process_index` first.
    pub layers: Vec<MapLayer>,
}
impl DrawnMap {
    pub fn from_geojson(json: &str) -> Result<Self, ChartError> {
        let mut layer = MapLayer::default();
        layer.add_geojson(json)?;
        Ok(Self {
            layers: vec![layer],
        })
    }

    pub fn from_geojson_bytes(bytes: &[u8]) -> Result<Self, ChartError> {
        let json = std::str::from_utf8(bytes)
            .map_err(|e| ChartError::InvalidData(format!("GeoJSON must be UTF-8: {e}")))?;
        Self::from_geojson(json)
    }

    /// Parses the pinned upstream world map without filesystem or network access.
    pub fn world() -> Result<Self, ChartError> {
        Self::from_geojson(include_str!("world.geojson"))
    }

    pub fn add_layer_from_geojson(
        &mut self,
        name: impl Into<String>,
        json: &str,
    ) -> Result<usize, ChartError> {
        let name = name.into();
        if let Some(layer) = self.layers.iter_mut().find(|layer| layer.name == name) {
            return layer.add_geojson(json);
        }
        let mut layer = MapLayer::new(name);
        let count = layer.add_geojson(json)?;
        self.layers.push(layer);
        Ok(count)
    }

    pub fn find_land(&self, name: &str) -> Option<&LandDefinition> {
        let name = name.to_lowercase();
        self.layers.iter().find_map(|layer| layer.lands.get(&name))
    }

    pub fn find_land_mut(&mut self, name: &str) -> Option<&mut LandDefinition> {
        let name = name.to_lowercase();
        self.layers
            .iter_mut()
            .find_map(|layer| layer.lands.get_mut(&name))
    }
}

fn property(properties: Option<&Value>, key: &str) -> Option<String> {
    let properties = properties?.as_object()?;
    let value = properties.get(key).or_else(|| {
        properties
            .iter()
            .find_map(|(candidate, value)| candidate.eq_ignore_ascii_case(key).then_some(value))
    })?;
    match value {
        Value::String(text) => Some(text.to_lowercase()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn coordinate_error(feature: usize) -> ChartError {
    ChartError::InvalidData(format!("GeoJSON feature {feature} has invalid polygon coordinates; each position needs finite longitude and latitude"))
}

fn read_polygon(value: &Value, feature: usize) -> Result<GeoPolygon, ChartError> {
    let values = value.as_array().ok_or_else(|| coordinate_error(feature))?;
    let mut rings = Vec::with_capacity(values.len());
    for value in values {
        let values = value.as_array().ok_or_else(|| coordinate_error(feature))?;
        let mut ring = Vec::with_capacity(values.len() + 1);
        for value in values {
            let coordinates = value.as_array().ok_or_else(|| coordinate_error(feature))?;
            let longitude = coordinates
                .first()
                .and_then(Value::as_f64)
                .filter(|n| n.is_finite())
                .ok_or_else(|| coordinate_error(feature))?;
            let latitude = coordinates
                .get(1)
                .and_then(Value::as_f64)
                .filter(|n| n.is_finite())
                .ok_or_else(|| coordinate_error(feature))?;
            ring.push(GeoCoordinate::new(longitude, latitude));
        }
        if ring.len() >= 3 && ring.first() != ring.last() {
            ring.push(ring[0]);
        }
        rings.push(ring);
    }
    Ok(GeoPolygon { rings })
}

#[cfg(test)]
mod tests {
    use super::*;
    const CUSTOM: &str = r#"{"type":"FeatureCollection","features":[
        {"type":"Feature","properties":{"name":"Piemonte","code_num":1},"geometry":{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,1],[0,0]]]}},
        {"type":"Feature","properties":{"name":"Sicilia","code_num":19},"geometry":{"type":"MultiPolygon","coordinates":[[[[10,10],[11,10],[11,11],[10,11],[10,10]]]]}},
        {"type":"Feature","properties":{"label":"no-name-key"},"geometry":{"type":"Polygon","coordinates":[[[5,5],[6,5],[6,6],[5,5]]]}},
        {"type":"Feature","properties":{"name":"Capital"},"geometry":{"type":"Point","coordinates":[0,0]}},
        {"type":"Feature","properties":{"name":"Highway"},"geometry":{"type":"MultiLineString","coordinates":[[[20,20],[21,21]],[[22,22],[23,23]]]}}
    ]}"#;
    const MIXED: &str = r#"{"type":"FeatureCollection","features":[
        {"type":"Feature","properties":{"Name":"Alpha","SHORTNAME":"AL"},"geometry":{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,1],[0,0]]]}},
        {"type":"Feature","properties":{"name":"Duplicate"},"geometry":{"type":"Polygon","coordinates":[[[2,2],[3,2],[3,3],[2,3],[2,2]]]}},
        {"type":"Feature","properties":{"name":"Duplicate"},"geometry":{"type":"Polygon","coordinates":[[[4,4],[5,4],[5,5],[4,5],[4,4]]]}}
    ]}"#;
    // Direct ports of CoreTests/OtherTests/GeoJsonImportTests.cs.
    #[test]
    fn import_custom_geojson_parses_mixed_polygon_shapes() {
        let map = DrawnMap::from_geojson(CUSTOM).unwrap();
        assert_eq!(map.layers[0].name, "default");
        assert_eq!(map.layers[0].lands.len(), 3);
        assert!(map.find_land("piemonte").is_some());
        assert!(map.find_land("sicilia").is_some());
        assert!(map.layers[0]
            .lands
            .keys()
            .any(|key| key.starts_with("feature_")));
    }
    #[test]
    fn import_built_in_world_map_still_loads() {
        let map = DrawnMap::world().unwrap();
        assert!(map.layers[0].lands.len() > 100);
        assert!(map.find_land("bra").is_some());
    }
    #[test]
    fn import_mixed_case_property_keys_resolve_case_insensitively() {
        let map = DrawnMap::from_geojson(MIXED).unwrap();
        assert!(map.find_land("al").is_some());
        assert_eq!(map.find_land("AL").unwrap().name, "alpha");
    }
    #[test]
    fn import_duplicate_short_names_do_not_abort_import() {
        let map = DrawnMap::from_geojson(MIXED).unwrap();
        assert_eq!(map.layers[0].lands.len(), 3);
        assert!(map.find_land("duplicate").is_some());
        assert!(map.find_land("duplicate_2").is_some());
    }
    #[test]
    fn malformed_coordinates_leave_existing_layer_unchanged() {
        let mut map = DrawnMap::from_geojson(CUSTOM).unwrap();
        let original = map.clone();
        assert!(map.layers[0].add_geojson(r#"{"features":[{"geometry":{"type":"Polygon","coordinates":[[[0,"bad"],[1,1],[0,0]]]}}]}"#).is_err());
        assert_eq!(map, original);
        assert!(DrawnMap::from_geojson(r#"{"features":null}"#).is_err());
    }
    #[test]
    fn holes_and_altitude_coordinates_are_preserved() {
        let map = DrawnMap::from_geojson(r#"{"features":[{"geometry":{"type":"Polygon","coordinates":[[[0,0,30],[10,0,30],[10,10,30],[0,10,30],[0,0,30]],[[2,2],[4,2],[4,4],[2,4],[2,2]]]}}]}"#).unwrap();
        let land = map.find_land("feature_0").unwrap();
        assert_eq!(land.polygons[0].rings.len(), 2);
        assert_eq!(land.bounds.unwrap().max_latitude, 10.0);
    }
}
