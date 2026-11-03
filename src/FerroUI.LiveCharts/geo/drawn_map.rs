//! Maps read from GeoJSON (`DrawnMap`, `MapLayer`, `LandDefinition`, `LandData`, and the
//! `GeoJsonFile`, `GeoJsonFeature` and `MultiPoligonGeometry` contracts they are read from).

use crate::scene::ChartError;
use ferroui_base::media::Color;
use ferroui_base::Point;
use serde_json::{Map, Value};
use std::collections::HashMap;

/// The stroke of a layer read without paints of its own (`AddLayerFromStreamReader`).
pub const DEFAULT_LAYER_STROKE: Color = Color::from_rgb(33, 150, 243);
/// The fill of a layer read without paints of its own.
pub const DEFAULT_LAYER_FILL: Color = Color::from_argb(50, 33, 150, 243);

/// One ring of a land (`LandData`): its coordinates, longitude in `x` and latitude in `y`, and
/// their bounds. Every ring of every polygon of a feature is a land shape of its own, as in the
/// source, which fills the inner rings of a polygon too.
#[derive(Clone, Debug, PartialEq)]
pub struct LandData {
    pub coordinates: Vec<Point>,
    /// The maximum longitude and latitude.
    pub max_bounds: [f64; 2],
    /// The minimum longitude and latitude.
    pub min_bounds: [f64; 2],
    /// The diagonal of the bounds, which orders the rings of a land.
    pub bounds_hypotenuse: f64,
}

impl LandData {
    pub fn new(coordinates: Vec<Point>) -> Self {
        let mut max_bounds = [f64::MIN, f64::MIN];
        let mut min_bounds = [f64::MAX, f64::MAX];
        for point in &coordinates {
            if point.x > max_bounds[0] {
                max_bounds[0] = point.x;
            }
            if point.x < min_bounds[0] {
                min_bounds[0] = point.x;
            }
            if point.y > max_bounds[1] {
                max_bounds[1] = point.y;
            }
            if point.y < min_bounds[1] {
                min_bounds[1] = point.y;
            }
        }
        let bounds_hypotenuse =
            (max_bounds[0] - min_bounds[0]).hypot(max_bounds[1] - min_bounds[1]);
        Self {
            coordinates,
            max_bounds,
            min_bounds,
            bounds_hypotenuse,
        }
    }
}

/// A land of a map (`LandDefinition`): a feature of the GeoJSON with its names and rings.
#[derive(Clone, Debug, PartialEq)]
pub struct LandDefinition {
    /// The key of the land, the `shortName` property in lower case (the name without one).
    pub short_name: String,
    /// The `name` property in lower case, or `feature_<index>` without one.
    pub name: String,
    /// The `setOf` property in lower case, or `?`.
    pub set_of: String,
    pub max_bounds: [f64; 2],
    pub min_bounds: [f64; 2],
    /// The rings of the land, the largest bounds first.
    pub data: Vec<LandData>,
}

impl LandDefinition {
    pub fn new(
        short_name: impl Into<String>,
        name: impl Into<String>,
        set_of: impl Into<String>,
    ) -> Self {
        Self {
            short_name: short_name.into(),
            name: name.into(),
            set_of: set_of.into(),
            max_bounds: [f64::MIN, f64::MIN],
            min_bounds: [f64::MAX, f64::MAX],
            data: Vec::new(),
        }
    }
}

/// A named layer of lands (`MapLayer`). The stroke and fill of a layer draw it when the chart has
/// no stroke or fill of its own; the visible layers are drawn by descending `process_index`.
#[derive(Clone, Debug, PartialEq)]
pub struct MapLayer {
    pub name: String,
    pub process_index: i32,
    pub is_visible: bool,
    pub stroke: Option<Color>,
    pub fill: Option<Color>,
    /// The lands in the order they were read, with their keys.
    lands: Vec<(String, LandDefinition)>,
    index: HashMap<String, usize>,
}

impl MapLayer {
    pub fn new(name: impl Into<String>, stroke: Option<Color>, fill: Option<Color>) -> Self {
        Self {
            name: name.into(),
            process_index: 0,
            is_visible: true,
            stroke,
            fill,
            lands: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// The lands and their keys, in the order they were read.
    pub fn lands(&self) -> impl Iterator<Item = (&str, &LandDefinition)> {
        self.lands.iter().map(|(key, land)| (key.as_str(), land))
    }
    /// The number of lands.
    pub fn land_count(&self) -> usize {
        self.lands.len()
    }
    /// The land of the given key (its short name, or the short name with the index of its
    /// feature when two features share it); the key is case-sensitive, as in the source.
    pub fn land(&self, key: &str) -> Option<&LandDefinition> {
        self.index.get(key).map(|&i| &self.lands[i].1)
    }
    /// The land at the given position in the order the lands were read.
    pub fn land_at(&self, index: usize) -> Option<&LandDefinition> {
        self.lands.get(index).map(|(_, land)| land)
    }

    /// Reads the polygon features of a GeoJSON feature collection into the layer (`AddFile`).
    /// `Polygon` and `MultiPolygon` geometries are read, other geometries and features without
    /// coordinates are skipped. A second feature with a short name already read takes the key
    /// `<short name>_<feature index>`. Malformed JSON, a file without `features` or a malformed
    /// polygon is an error that leaves the layer unchanged. Returns the number of lands read.
    pub fn add_geojson(&mut self, json: &str) -> Result<usize, ChartError> {
        let file: Value = serde_json::from_str(json)
            .map_err(|error| ChartError::InvalidData(format!("invalid GeoJSON: {error}")))?;
        let features = match file.get("features") {
            Some(Value::Array(features)) => features,
            Some(Value::Null) | None => {
                return Err(ChartError::InvalidData(
                    "the features property is required to build a map".into(),
                ))
            }
            Some(_) => return Err(invalid("the features of a GeoJSON file must be an array")),
        };
        let mut read = Vec::new();
        for (feature_index, feature) in features.iter().enumerate() {
            let feature = feature
                .as_object()
                .ok_or_else(|| invalid("a GeoJSON feature must be an object"))?;
            let properties = match feature.get("properties") {
                Some(Value::Object(properties)) => Some(properties),
                Some(Value::Null) | None => None,
                Some(_) => return Err(invalid("the properties of a feature must be an object")),
            };
            let geometry = match feature.get("geometry") {
                Some(Value::Object(geometry)) => geometry,
                Some(Value::Null) | None => continue,
                Some(_) => return Err(invalid("the geometry of a feature must be an object")),
            };
            let Some(coordinates) = geometry.get("coordinates").filter(|c| !c.is_null()) else {
                continue;
            };
            let Some(polygons) = read_polygons(geometry.get("type"), coordinates, feature_index)?
            else {
                continue;
            };
            let name = property(properties, "name")
                .unwrap_or_else(|| format!("feature_{feature_index}"));
            let short_name = property(properties, "shortName").unwrap_or_else(|| name.clone());
            let set_of = property(properties, "setOf").unwrap_or_else(|| "?".into());
            let mut definition = LandDefinition::new(short_name, name, set_of);
            for ring in polygons.into_iter().flatten() {
                let data = LandData::new(ring);
                if data.max_bounds[0] > definition.max_bounds[0] {
                    definition.max_bounds[0] = data.max_bounds[0];
                }
                if data.min_bounds[0] < definition.min_bounds[0] {
                    definition.min_bounds[0] = data.min_bounds[0];
                }
                if data.max_bounds[1] > definition.max_bounds[1] {
                    definition.max_bounds[1] = data.max_bounds[1];
                }
                if data.min_bounds[1] < definition.min_bounds[1] {
                    definition.min_bounds[1] = data.min_bounds[1];
                }
                definition.data.push(data);
            }
            // A stable sort, as OrderByDescending.
            definition
                .data
                .sort_by(|a, b| b.bounds_hypotenuse.total_cmp(&a.bounds_hypotenuse));
            read.push((feature_index, definition));
        }
        // The keys are resolved before anything is added, so that an error leaves the layer as
        // it was (the source throws on a key that is taken twice, halfway through the file).
        let mut keys: HashMap<String, usize> = HashMap::new();
        let mut entries = Vec::with_capacity(read.len());
        for (feature_index, definition) in read {
            let taken = |key: &str| self.index.contains_key(key) || keys.contains_key(key);
            let key = if taken(&definition.short_name) {
                format!("{}_{feature_index}", definition.short_name)
            } else {
                definition.short_name.clone()
            };
            if taken(&key) {
                return Err(ChartError::InvalidData(format!(
                    "the map already has a land with the key {key}"
                )));
            }
            keys.insert(key.clone(), self.lands.len() + entries.len());
            entries.push((key, definition));
        }
        let count = entries.len();
        self.index.extend(keys);
        self.lands.extend(entries);
        Ok(count)
    }
}

/// A map of named layers (`DrawnMap`), in the order they were added.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawnMap {
    layers: Vec<MapLayer>,
}

impl DrawnMap {
    /// The name of the layer of a map read from one file.
    pub const DEFAULT_LAYER: &'static str = "default";

    /// An empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// A map of one layer, `default`, with the default layer paints.
    pub fn from_geojson(json: &str) -> Result<Self, ChartError> {
        let mut map = Self::new();
        map.add_layer_from_geojson(
            json,
            Self::DEFAULT_LAYER,
            Some(DEFAULT_LAYER_STROKE),
            Some(DEFAULT_LAYER_FILL),
        )?;
        Ok(map)
    }

    /// Reads a GeoJSON file into the layer of the given name (`AddLayerFromStreamReader`); a
    /// new layer takes the given paints, an existing one keeps its own.
    pub fn add_layer_from_geojson(
        &mut self,
        json: &str,
        layer_name: &str,
        stroke: Option<Color>,
        fill: Option<Color>,
    ) -> Result<&mut MapLayer, ChartError> {
        let index = match self.layers.iter().position(|layer| layer.name == layer_name) {
            Some(index) => index,
            None => {
                let mut layer = MapLayer::new(layer_name, stroke, fill);
                layer.add_geojson(json)?;
                self.layers.push(layer);
                return Ok(self.layers.last_mut().expect("the layer was just added"));
            }
        };
        self.layers[index].add_geojson(json)?;
        Ok(&mut self.layers[index])
    }

    /// The layers, in the order they were added.
    pub fn layers(&self) -> &[MapLayer] {
        &self.layers
    }
    pub fn layer(&self, name: &str) -> Option<&MapLayer> {
        self.layers.iter().find(|layer| layer.name == name)
    }
    pub fn layer_mut(&mut self, name: &str) -> Option<&mut MapLayer> {
        self.layers.iter_mut().find(|layer| layer.name == name)
    }
    /// Adds a layer, or replaces the layer of the same name.
    pub fn insert_layer(&mut self, layer: MapLayer) {
        match self.layers.iter_mut().find(|existing| existing.name == layer.name) {
            Some(existing) => *existing = layer,
            None => self.layers.push(layer),
        }
    }
    pub fn remove_layer(&mut self, name: &str) -> Option<MapLayer> {
        let index = self.layers.iter().position(|layer| layer.name == name)?;
        Some(self.layers.remove(index))
    }

    /// The land of the given key in the `default` layer (`FindLand`).
    pub fn find_land(&self, short_name: &str) -> Option<&LandDefinition> {
        self.find_land_in(short_name, Self::DEFAULT_LAYER)
    }
    /// The land of the given key in the given layer.
    pub fn find_land_in(&self, short_name: &str, layer_name: &str) -> Option<&LandDefinition> {
        self.layer(layer_name)?.land(short_name)
    }
}

fn invalid(message: &str) -> ChartError {
    ChartError::InvalidData(format!("invalid GeoJSON: {message}"))
}

/// The polygons of a geometry by its RFC 7946 type (`TryReadPolygons`): a polygon is a list of
/// one. Other types, and coordinates that are not an array, are skipped (`None`).
fn read_polygons(
    kind: Option<&Value>,
    coordinates: &Value,
    feature: usize,
) -> Result<Option<Vec<Vec<Vec<Point>>>>, ChartError> {
    let Value::Array(items) = coordinates else {
        return Ok(None);
    };
    match kind.and_then(Value::as_str) {
        Some("Polygon") => Ok(Some(vec![read_polygon(items, feature)?])),
        Some("MultiPolygon") => items
            .iter()
            .map(|polygon| {
                polygon
                    .as_array()
                    .ok_or_else(|| coordinate_error(feature))
                    .and_then(|rings| read_polygon(rings, feature))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        _ => Ok(None),
    }
}

/// The rings of a polygon (`ReadPolygon`); each position takes its first two numbers, longitude
/// and latitude (an altitude is ignored).
fn read_polygon(rings: &[Value], feature: usize) -> Result<Vec<Vec<Point>>, ChartError> {
    rings
        .iter()
        .map(|ring| -> Result<Vec<Point>, ChartError> {
            ring.as_array()
                .ok_or_else(|| coordinate_error(feature))?
                .iter()
                .map(|position| -> Result<Point, ChartError> {
                    let values = position.as_array().ok_or_else(|| coordinate_error(feature))?;
                    let mut numbers = values.iter().map(Value::as_f64);
                    match (numbers.next().flatten(), numbers.next().flatten()) {
                        (Some(longitude), Some(latitude))
                            if values.iter().all(Value::is_number) =>
                        {
                            Ok(Point::new(longitude, latitude))
                        }
                        _ => Err(coordinate_error(feature)),
                    }
                })
                .collect()
        })
        .collect()
}

fn coordinate_error(feature: usize) -> ChartError {
    ChartError::InvalidData(format!(
        "invalid GeoJSON: the coordinates of feature {feature} are not positions of numbers"
    ))
}

/// A property of a feature (`GetProperty`): the exact key, else the first key equal to it
/// ignoring case. A string is lower-cased, a number is its text, a boolean `true` or `false`.
fn property(properties: Option<&Map<String, Value>>, key: &str) -> Option<String> {
    let properties = properties?;
    let value = properties.get(key).or_else(|| {
        properties
            .iter()
            .find(|(candidate, _)| candidate.eq_ignore_ascii_case(key))
            .map(|(_, value)| value)
    })?;
    match value {
        Value::String(text) => Some(text.to_lowercase()),
        Value::Number(number) => Some(number.to_string().to_lowercase()),
        Value::Bool(true) => Some("true".into()),
        Value::Bool(false) => Some("false".into()),
        _ => None,
    }
}
