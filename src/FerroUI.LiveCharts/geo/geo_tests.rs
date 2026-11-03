//! Ports of `CoreTests/CoreObjectsTests/GeoProjectorsTesting.cs` and
//! `CoreTests/OtherTests/GeoJsonImportTests.cs` at the pinned source: the same projector sizes,
//! offsets, centres and coordinates, and the same GeoJSON documents.

use super::*;
use ferroui_base::Point;

fn bounds(min_lat: f64, max_lat: f64, min_lon: f64, max_lon: f64) -> GeographicBounds {
    GeographicBounds::new(min_lat, max_lat, min_lon, max_lon)
}

fn mercator(width: f64, height: f64, x: f64, y: f64) -> MercatorProjector {
    MercatorProjector::new(width, height, x, y, GeographicBounds::default())
}

const NAN: GeographicBounds = GeographicBounds::new(f64::NAN, f64::NAN, f64::NAN, f64::NAN);

// GeoProjectorsTesting.

#[test]
fn mercator_projector_projects_origin_to_center() {
    // The default crop is asymmetric; the symmetric 85 degrees put the equator in the middle.
    let projector = MercatorProjector::new(360.0, 180.0, 0.0, 0.0, bounds(-85.0, 85.0, -180.0, 180.0));
    let result = projector.to_map(0.0, 0.0);
    assert!((result.x - 180.0).abs() < 0.1);
    assert!((result.y - 90.0).abs() < 0.1);
}

#[test]
fn mercator_projector_projects_top_left() {
    assert!(mercator(360.0, 180.0, 0.0, 0.0).to_map(-180.0, 0.0).x.abs() < 0.1);
}

#[test]
fn mercator_projector_projects_top_right() {
    assert!((mercator(360.0, 180.0, 0.0, 0.0).to_map(180.0, 0.0).x - 360.0).abs() < 0.1);
}

#[test]
fn mercator_projector_respects_offset() {
    assert!((mercator(360.0, 180.0, 50.0, 25.0).to_map(-180.0, 0.0).x - 50.0).abs() < 0.1);
}

#[test]
fn mercator_projector_properties() {
    let projector = mercator(800.0, 600.0, 10.0, 20.0);
    assert_eq!(projector.map_width(), 800.0);
    assert_eq!(projector.map_height(), 600.0);
    assert_eq!(projector.x_offset(), 10.0);
    assert_eq!(projector.y_offset(), 20.0);
}

#[test]
fn mercator_projector_preferred_ratio() {
    // The default crop, -65 to 85 degrees: 2 pi / (mercN(85) - mercN(-65)), about 1.355.
    let ratio = MercatorProjector::preferred_ratio(GeographicBounds::default());
    assert!((ratio[0] - 1.355).abs() < 0.01, "{}", ratio[0]);
    assert!((ratio[1] - 1.0).abs() < 0.001);
    // The symmetric 85 degrees is about square.
    let full_earth = MercatorProjector::preferred_ratio(bounds(-85.0, 85.0, -180.0, 180.0));
    assert!((full_earth[0] - 1.0).abs() < 0.01, "{}", full_earth[0]);
    assert!((full_earth[1] - 1.0).abs() < 0.001);
    // NaN bounds take the defaults.
    assert!((MercatorProjector::preferred_ratio(NAN)[0] - 1.355).abs() < 0.01);
}

#[test]
fn mercator_projector_is_always_visible() {
    let projector = MapProjector::Mercator(mercator(360.0, 180.0, 0.0, 0.0));
    assert!(projector.is_visible(0.0, 0.0));
    assert!(projector.is_visible(180.0, 90.0));
    assert!(projector.is_visible(-180.0, -90.0));
}

#[test]
fn mercator_projector_normalizes_degenerate_and_inverted_bounds() {
    // An inverted latitude pair is swapped.
    let inverted = MercatorProjector::preferred_ratio(bounds(72.0, 35.0, -25.0, 45.0));
    let normal = MercatorProjector::preferred_ratio(bounds(35.0, 72.0, -25.0, 45.0));
    assert!((normal[0] - inverted[0]).abs() < 0.001);
    assert!((normal[1] - inverted[1]).abs() < 0.001);
    // Equal latitudes take the defaults: no NaN or infinity.
    let degenerate = MercatorProjector::preferred_ratio(bounds(50.0, 50.0, -25.0, 45.0));
    assert!(degenerate[0].is_finite() && degenerate[1].is_finite());
    let projector = MercatorProjector::new(360.0, 180.0, 0.0, 0.0, bounds(50.0, 50.0, -25.0, 45.0));
    let point = projector.to_map(0.0, 0.0);
    assert!(point.x.is_finite() && point.y.is_finite());
}

#[test]
fn control_coordinates_projector_normalizes_degenerate_and_inverted_bounds() {
    let inverted = ControlCoordinatesProjector::preferred_ratio(bounds(72.0, 35.0, 45.0, -25.0));
    let normal = ControlCoordinatesProjector::preferred_ratio(bounds(35.0, 72.0, -25.0, 45.0));
    assert!((normal[0] - inverted[0]).abs() < 0.001);
    assert!((normal[1] - inverted[1]).abs() < 0.001);
    let degenerate = ControlCoordinatesProjector::preferred_ratio(bounds(50.0, 50.0, -25.0, 45.0));
    assert!(degenerate[0].is_finite() && degenerate[1].is_finite());
    let projector =
        ControlCoordinatesProjector::new(360.0, 180.0, 0.0, 0.0, bounds(50.0, 50.0, -25.0, 45.0));
    let point = projector.to_map(0.0, 0.0);
    assert!(point.x.is_finite() && point.y.is_finite());
}

fn globe(width: f64, height: f64, x: f64, y: f64, lon: f64, lat: f64) -> OrthographicProjector {
    OrthographicProjector::new(width, height, x, y, lon, lat)
}

#[test]
fn orthographic_projector_projects_center_to_screen_center() {
    let result = globe(500.0, 500.0, 0.0, 0.0, 0.0, 0.0).to_map(0.0, 0.0);
    assert!((result.x - 250.0).abs() < 0.1);
    assert!((result.y - 250.0).abs() < 0.1);
}

#[test]
fn orthographic_projector_center_point_is_visible() {
    assert!(globe(500.0, 500.0, 0.0, 0.0, 0.0, 0.0).is_visible(0.0, 0.0));
}

#[test]
fn orthographic_projector_opposite_point_is_not_visible() {
    assert!(!globe(500.0, 500.0, 0.0, 0.0, 0.0, 0.0).is_visible(180.0, 0.0));
}

#[test]
fn orthographic_projector_point_just_beyond_horizon_is_not_visible() {
    assert!(!globe(500.0, 500.0, 0.0, 0.0, 0.0, 0.0).is_visible(91.0, 0.0));
}

#[test]
fn orthographic_projector_point_just_before_horizon_is_visible() {
    assert!(globe(500.0, 500.0, 0.0, 0.0, 0.0, 0.0).is_visible(89.0, 0.0));
}

#[test]
fn orthographic_projector_properties() {
    let projector = globe(800.0, 600.0, 10.0, 20.0, 45.0, 30.0);
    assert_eq!(projector.map_width(), 800.0);
    assert_eq!(projector.map_height(), 600.0);
    assert_eq!(projector.x_offset(), 10.0);
    assert_eq!(projector.y_offset(), 20.0);
    assert_eq!(projector.center_longitude(), 45.0);
    assert_eq!(projector.center_latitude(), 30.0);
    assert!(projector.radius() > 0.0);
}

#[test]
fn orthographic_projector_preferred_ratio() {
    assert_eq!(OrthographicProjector::PREFERRED_RATIO, [1.0, 1.0]);
}

#[test]
fn orthographic_projector_radius_is_half_min_dimension() {
    assert_eq!(globe(800.0, 600.0, 0.0, 0.0, 0.0, 0.0).radius(), 300.0);
}

#[test]
fn orthographic_projector_screen_center() {
    let projector = globe(800.0, 600.0, 50.0, 30.0, 0.0, 0.0);
    assert_eq!(projector.screen_center_x(), 450.0);
    assert_eq!(projector.screen_center_y(), 330.0);
}

#[test]
fn orthographic_projector_visibility_with_different_center() {
    // Facing New York; Tokyo is on the far side.
    let projector = globe(500.0, 500.0, 0.0, 0.0, -74.0, 40.7);
    assert!(projector.is_visible(-74.0, 40.7));
    assert!(projector.is_visible(-80.0, 35.0));
    assert!(!projector.is_visible(139.7, 35.7));
}

#[test]
fn orthographic_projector_symmetry() {
    let projector = globe(500.0, 500.0, 0.0, 0.0, 0.0, 0.0);
    let left = projector.to_map(-45.0, 0.0);
    let right = projector.to_map(45.0, 0.0);
    assert!(((left.x - 250.0).abs() - (right.x - 250.0).abs()).abs() < 0.1);
    assert!((left.y - right.y).abs() < 0.1);
}

const ROUND_TRIPS: [(f64, f64); 5] = [
    (0.0, 0.0),
    (-74.0, 40.7),
    (139.69, 35.69),
    (-3.7, 40.4),
    (45.0, -25.0),
];

#[test]
fn mercator_projector_round_trips() {
    let projector = mercator(800.0, 600.0, 50.0, 30.0);
    for (lon, lat) in ROUND_TRIPS {
        let point = projector.to_map(lon, lat);
        let (lon2, lat2) = projector.to_coordinates(point).unwrap();
        assert!((lon - lon2).abs() < 1e-4, "lon round trip at ({lon}, {lat})");
        assert!((lat - lat2).abs() < 1e-4, "lat round trip at ({lon}, {lat})");
    }
}

#[test]
fn control_coordinates_projector_round_trips() {
    let projector =
        ControlCoordinatesProjector::new(800.0, 600.0, 50.0, 30.0, GeographicBounds::default());
    for (lon, lat) in ROUND_TRIPS {
        let (lon2, lat2) = projector.to_coordinates(projector.to_map(lon, lat)).unwrap();
        assert!((lon - lon2).abs() < 1e-4);
        assert!((lat - lat2).abs() < 1e-4);
    }
}

#[test]
fn orthographic_projector_round_trips_on_visible_hemisphere() {
    let projector = globe(500.0, 500.0, 0.0, 0.0, -30.0, 20.0);
    for (lon, lat) in [(-30.0, 20.0), (-74.0, 40.7), (-3.7, 40.4), (15.0, 50.0)] {
        assert!(projector.is_visible(lon, lat), "({lon}, {lat}) should be visible");
        let (lon2, lat2) = projector.to_coordinates(projector.to_map(lon, lat)).unwrap();
        assert!((lon - lon2).abs() < 1e-2, "lon round trip at ({lon}, {lat})");
        assert!((lat - lat2).abs() < 1e-2, "lat round trip at ({lon}, {lat})");
    }
}

#[test]
fn orthographic_projector_rejects_clicks_outside_disc() {
    let projector = globe(500.0, 500.0, 0.0, 0.0, 0.0, 0.0);
    assert!(projector.to_coordinates(Point::new(0.0, 0.0)).is_none());
    assert!(projector.to_coordinates(Point::new(500.0, 500.0)).is_none());
    let (lon, lat) = projector.to_coordinates(Point::new(250.0, 250.0)).unwrap();
    assert!(lon.abs() < 1e-6 && lat.abs() < 1e-6);
}

/// `Maps.BuildProjector` keeps the aspect ratio of the projection and centres the shorter side.
#[test]
fn build_projector_fits_the_preferred_ratio_into_the_map_area() {
    let default = GeographicBounds::default();
    let mercator =
        Maps::build_projector(MapProjection::Mercator, [600.0, 600.0], [0.0, 0.0], (0.0, 0.0), default);
    let ratio = MercatorProjector::preferred_ratio(default);
    assert_eq!(mercator.map_width(), 600.0);
    assert!((mercator.map_height() - 600.0 / ratio[0]).abs() < 1e-9);
    assert!((mercator.y_offset() - (600.0 - mercator.map_height()) / 2.0).abs() < 1e-9);
    let globe =
        Maps::build_projector(MapProjection::Orthographic, [800.0, 600.0], [10.0, 20.0], (15.0, 20.0), default);
    assert_eq!((globe.map_width(), globe.map_height()), (600.0, 600.0));
    assert_eq!((globe.x_offset(), globe.y_offset()), (110.0, 20.0));
    assert_eq!(globe.as_orthographic().unwrap().center_longitude(), 15.0);
    let flat =
        Maps::build_projector(MapProjection::Default, [800.0, 800.0], [0.0, 0.0], (0.0, 0.0), default);
    assert_eq!((flat.map_width(), flat.map_height()), (800.0, 400.0));
}

// GeoJsonImportTests.

/// RFC 7946 data: mixed Polygon and MultiPolygon geometries, numeric and string properties and
/// none of the LiveCharts keys (regression of issue 1426).
const CUSTOM_FEATURE_COLLECTION: &str = r#"{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "properties": { "name": "Piemonte", "code_num": 1 },
      "geometry": {
        "type": "Polygon",
        "coordinates": [[[0,0],[1,0],[1,1],[0,1],[0,0]]]
      }
    },
    {
      "type": "Feature",
      "properties": { "name": "Sicilia", "code_num": 19 },
      "geometry": {
        "type": "MultiPolygon",
        "coordinates": [[[[10,10],[11,10],[11,11],[10,11],[10,10]]]]
      }
    },
    {
      "type": "Feature",
      "properties": { "label": "no-name-key" },
      "geometry": {
        "type": "Polygon",
        "coordinates": [[[5,5],[6,5],[6,6],[5,5]]]
      }
    },
    {
      "type": "Feature",
      "properties": { "name": "Capital" },
      "geometry": { "type": "Point", "coordinates": [0,0] }
    },
    {
      "type": "Feature",
      "properties": { "name": "Highway" },
      "geometry": {
        "type": "MultiLineString",
        "coordinates": [[[20,20],[21,21]],[[22,22],[23,23]]]
      }
    }
  ]
}"#;

#[test]
fn import_custom_geo_json_parses_mixed_polygon_shapes() {
    let map = Maps::map_from_geojson(CUSTOM_FEATURE_COLLECTION).unwrap();
    let layer = map.layer("default").expect("default layer should exist");
    // The polygon and multipolygon lands and the one without a name; the point and the lines,
    // which share the depth of a polygon, are skipped by their type.
    assert_eq!(layer.land_count(), 3);
    assert!(map.find_land("piemonte").is_some());
    assert!(map.find_land("sicilia").is_some());
    assert!(layer.lands().any(|(key, _)| key.starts_with("feature_")));
}

#[test]
fn import_built_in_world_map_still_loads() {
    let world = Maps::world_map();
    assert!(world.layer("default").unwrap().land_count() > 100);
    assert!(world.find_land("bra").is_some());
    // Russia is several rings, its mainland first.
    let russia = world.find_land("rus").unwrap();
    assert!(russia.data.len() > 1);
    assert!(russia.data[0].bounds_hypotenuse >= russia.data[1].bounds_hypotenuse);
}

const MIXED_CASE_AND_DUPLICATE_NAMES_COLLECTION: &str = r#"{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "properties": { "Name": "Alpha", "SHORTNAME": "AL" },
      "geometry": {
        "type": "Polygon",
        "coordinates": [[[0,0],[1,0],[1,1],[0,1],[0,0]]]
      }
    },
    {
      "type": "Feature",
      "properties": { "name": "Duplicate" },
      "geometry": {
        "type": "Polygon",
        "coordinates": [[[2,2],[3,2],[3,3],[2,3],[2,2]]]
      }
    },
    {
      "type": "Feature",
      "properties": { "name": "Duplicate" },
      "geometry": {
        "type": "Polygon",
        "coordinates": [[[4,4],[5,4],[5,5],[4,5],[4,4]]]
      }
    }
  ]
}"#;

#[test]
fn import_mixed_case_property_keys_resolve_case_insensitively() {
    let map = Maps::map_from_geojson(MIXED_CASE_AND_DUPLICATE_NAMES_COLLECTION).unwrap();
    let land = map.find_land("al").expect("SHORTNAME resolves ignoring case");
    assert_eq!(land.name, "alpha");
}

#[test]
fn import_duplicate_short_names_do_not_abort_import() {
    let map = Maps::map_from_geojson(MIXED_CASE_AND_DUPLICATE_NAMES_COLLECTION).unwrap();
    let layer = map.layer("default").unwrap();
    assert_eq!(layer.land_count(), 3);
    assert!(map.find_land("duplicate").is_some());
    assert!(map.find_land("duplicate_2").is_some());
}

/// Not in the source, which throws halfway through a file: a malformed file is an error that
/// leaves the layer as it was; a file without features is an error too.
#[test]
fn malformed_geo_json_leaves_the_layer_unchanged() {
    let mut map = Maps::map_from_geojson(CUSTOM_FEATURE_COLLECTION).unwrap();
    let original = map.clone();
    let bad = r#"{"features":[
        {"properties":{"name":"Fine"},"geometry":{"type":"Polygon","coordinates":[[[0,0],[1,0],[0,1]]]}},
        {"geometry":{"type":"Polygon","coordinates":[[[0,"bad"],[1,1],[0,0]]]}}]}"#;
    assert!(map
        .add_layer_from_geojson(bad, "default", None, None)
        .is_err());
    assert_eq!(map, original);
    assert!(Maps::map_from_geojson(r#"{"features":null}"#).is_err());
    assert!(Maps::map_from_geojson("not json").is_err());
    // A second file joins the layer of the same name and keeps its paints.
    let layer = map
        .add_layer_from_geojson(MIXED_CASE_AND_DUPLICATE_NAMES_COLLECTION, "default", None, None)
        .unwrap();
    assert_eq!(layer.land_count(), 6);
    assert_eq!(layer.stroke, Some(DEFAULT_LAYER_STROKE));
}
