//! The engine cases of the pinned `CoreTests/ChartTests/GeoMapTests.cs`,
//! `GeoVisualElementTests.cs`, `CollapsedDrawMarginAllChartsTests.GeoMap_HidesThenResets` and
//! `OtherTests/ChartInteractiveApiTests.GeoMap_FindLandAtReturnsNullOutsideAnyLand`: the source's
//! chart sizes, projections, lands and coordinates, measured on the bundled world map; and the
//! orthographic horizon, viewport and rotation rules of `MapFactory` and `GeoMapChart`.

use super::*;
use crate::geo::Maps;

fn world() -> &'static DrawnMap {
    Maps::world_map()
}

fn heat(lands: &[(&str, f64)]) -> HeatLandSeries {
    HeatLandSeries::new(
        lands
            .iter()
            .map(|&(name, value)| HeatLand::new(name, value))
            .collect(),
    )
}

fn mercator() -> GeoMapOptions {
    GeoMapOptions {
        projection: MapProjection::Mercator,
        ..GeoMapOptions::default()
    }
}

fn measure(size: f64, series: &[HeatLandSeries], options: &GeoMapOptions) -> GeoMapMeasurement {
    GeoMapChartEngine::measure(Size::new(size, size), world(), series, &[], options).unwrap()
}

/// The pixel of a coordinate with the projector of the source tests,
/// `Maps.BuildProjector(MapProjection.Mercator, [width, height])`.
fn mercator_pixel(width: f64, height: f64, longitude: f64, latitude: f64) -> Point {
    Maps::build_projector(
        MapProjection::Mercator,
        [width, height],
        [0.0, 0.0],
        (0.0, 0.0),
        GeographicBounds::default(),
    )
    .to_map(longitude, latitude)
}

fn land_fills(measurement: &GeoMapMeasurement, land: &GeoLand) -> Vec<Option<Color>> {
    measurement
        .scene
        .primitives
        .iter()
        .filter(|p| p.id == Some(land.id))
        .map(|p| p.style.fill)
        .collect()
}

/// GeoMap_SwappingSeries_PreservesFillOnSharedLands (issue 962): a land the old and the new
/// series share keeps its heat; a land of the old series only takes the chart's fill again.
#[test]
fn geo_map_swapping_series_preserves_fill_on_shared_lands() {
    let first = measure(400.0, &[heat(&[("fra", 10.0), ("usa", 5.0)])], &GeoMapOptions::default());
    let land = |m: &GeoMapMeasurement, name: &str| {
        m.lands
            .iter()
            .find(|l| l.short_name == name)
            .cloned()
            .unwrap_or_else(|| panic!("{name} is a land of the world map"))
    };
    let fra = land(&first, "fra");
    assert!(land_fills(&first, &fra).iter().all(|f| *f == Some(HEAT_LAND_MAP[1])));
    let usa = land(&first, "usa");
    assert!(land_fills(&first, &usa).iter().all(|f| *f == Some(HEAT_LAND_MAP[0])));
    let second = measure(400.0, &[heat(&[("fra", 1.0), ("bra", 99.0)])], &GeoMapOptions::default());
    assert!(land_fills(&second, &fra).iter().all(|f| *f == Some(HEAT_LAND_MAP[0])));
    assert!(land_fills(&second, &usa).iter().all(|f| *f == Some(MAP_FILL)));
}

/// GeoMap_TooltipAnchor_PicksLargestContour_NotUnionBbox: Russia is several rings, across the
/// antimeridian; the anchor of its tooltip is in its mainland, not between its parts.
#[test]
fn geo_map_tooltip_anchor_picks_largest_contour_not_union_bbox() {
    let measured = measure(600.0, &[heat(&[("rus", 10.0)])], &mercator());
    assert!(world().find_land("rus").unwrap().data.len() > 1);
    let moscow = mercator_pixel(600.0, 600.0, 37.62, 55.75);
    let hit = measured.find_land_at(moscow).expect("Moscow hits Russia");
    assert_eq!(hit.short_name, "rus");
    let region = measured.scene.hits.iter().find(|h| h.id == hit.id).unwrap();
    assert!(
        region.shape.contains(hit.center),
        "the anchor {:?} is inside a ring of Russia",
        hit.center
    );
    assert_eq!(region.center, hit.center);
}

/// GeoMap_GetPointsAt_ReturnsHitLand and GeoMap_FindLandAtReturnsNullOutsideAnyLand: the land
/// under a position, and none in the ocean or far outside the map.
#[test]
fn geo_map_get_points_at_returns_hit_land() {
    let measured = measure(600.0, &[], &mercator());
    let moscow = mercator_pixel(600.0, 600.0, 37.62, 55.75);
    let hits = measured
        .scene
        .hit_test(moscow, measured.scene.automatic_finding_strategy, 0.0);
    assert_eq!(hits.len(), 1);
    assert_eq!(measured.land(hits[0].id).unwrap().short_name, "rus");
    assert!(measured.find_land_at(Point::new(1.0, 1.0)).is_none());
    assert!(measured.find_land_at(Point::new(-1000.0, -1000.0)).is_none());
    assert!(measured
        .scene
        .hit_test(Point::new(1.0, 1.0), FindingStrategy::Automatic, 0.0)
        .is_empty());
}

/// GeoMap_MultipleHeatSeries_TooltipValuesPreserveDeclarationOrder: every series with a value
/// for the land, in the order of the series, and the tooltip lines of the source.
#[test]
fn geo_map_multiple_heat_series_tooltip_values_preserve_declaration_order() {
    let mut population = heat(&[("bra", 213.0)]);
    population.series.name = "Population".into();
    let mut gdp = heat(&[("bra", 1839.0)]);
    gdp.series.name = "GDP".into();
    let measured = measure(600.0, &[population, gdp], &mercator());
    let brasilia = mercator_pixel(600.0, 600.0, -47.92, -15.79);
    let hit = measured.find_land_at(brasilia).expect("Brasilia hits Brazil");
    assert_eq!(hit.short_name, "bra");
    assert_eq!(hit.values.len(), 2);
    assert_eq!((hit.values[0].series_name.as_str(), hit.values[0].value), ("Population", 213.0));
    assert_eq!((hit.values[1].series_name.as_str(), hit.values[1].value), ("GDP", 1839.0));
    assert_eq!(hit.value(), 213.0);
    let region = measured.scene.hits.iter().find(|h| h.id == hit.id).unwrap();
    assert_eq!(region.label, "Brazil\nPopulation: 213.00\nGDP: 1,839.00");
}

/// GeoMap_ProjectUnproject_RoundTrips.
#[test]
fn geo_map_project_unproject_round_trips() {
    let measured = GeoMapChartEngine::measure(
        Size::new(800.0, 600.0),
        world(),
        &[],
        &[],
        &mercator(),
    )
    .unwrap();
    for (lon, lat) in [(0.0, 0.0), (-74.0, 40.7), (139.69, 35.69), (-3.7, 40.4)] {
        let pixel = measured.project(lon, lat).expect("Mercator projects every coordinate");
        let (lon2, lat2) = measured.unproject(pixel).unwrap();
        assert!((lon - lon2).abs() < 1e-3, "lon round trip at ({lon}, {lat})");
        assert!((lat - lat2).abs() < 1e-3, "lat round trip at ({lon}, {lat})");
    }
}

/// GeoMap_Project_ReturnsNullForOrthographicBackHemisphere and
/// GeoMap_Unproject_ReturnsNullForOrthographicOffDisc.
#[test]
fn geo_map_orthographic_back_hemisphere_and_off_disc() {
    let options = GeoMapOptions {
        projection: MapProjection::Orthographic,
        ..GeoMapOptions::default()
    };
    let measured = measure(600.0, &[], &options);
    assert!(measured.project(180.0, 0.0).is_none());
    assert!(measured.unproject(Point::new(0.0, 0.0)).is_none());
    assert!(measured.project(0.0, 0.0).is_some());
}

/// GeoMap_HeatLandSeriesWeightBounds_AvailableBeforeMeasure.
#[test]
fn geo_map_heat_land_series_weight_bounds_available_before_measure() {
    let mut series = heat(&[("rus", 3.0), ("usa", 17.0)]);
    assert_eq!(series.weight_bounds().min, 3.0);
    assert_eq!(series.weight_bounds().max, 17.0);
    series.min_value = Some(0.0);
    series.max_value = Some(100.0);
    assert_eq!(series.weight_bounds().min, 0.0);
    assert_eq!(series.weight_bounds().max, 100.0);
    assert_eq!(series.try_get_value("RUS"), Some(3.0));
    assert_eq!(series.try_get_value("fra"), None);
}

/// GeoMap_EnumerateHeatLegendSources_ReturnsHeatLandSeries: the legend of the map lists its heat
/// land series.
#[test]
fn geo_map_legend_lists_the_heat_land_series() {
    let mut series = heat(&[("rus", 7.0)]);
    series.series.name = "Population".into();
    let measured = measure(400.0, &[series], &GeoMapOptions::default());
    assert_eq!(measured.scene.legend.len(), 1);
    assert_eq!(measured.scene.legend[0].name, "Population");
    assert_eq!(measured.scene.legend[0].color, HEAT_LAND_MAP[1]);
}

/// CollapsedDrawMarginAllChartsTests.GeoMap_HidesThenResets: a draw margin collapsed past the
/// control hides the map; a valid one draws it again, clipped to the projection's area.
#[test]
fn geo_map_hides_then_resets() {
    let series = [heat(&[("fra", 10.0)])];
    let collapsed = GeoMapOptions {
        draw_margin: Some(Rect::new(0.0, 300.0, 300.0, 0.0)),
        ..GeoMapOptions::default()
    };
    let hidden = measure(300.0, &series, &collapsed);
    assert!(hidden.scene.primitives.is_empty() && hidden.scene.hits.is_empty());
    assert!(hidden.scene.plot_bounds.width <= 0.0 || hidden.scene.plot_bounds.height <= 0.0);
    let restored = measure(300.0, &series, &GeoMapOptions::default());
    let plot = restored.scene.plot_bounds;
    assert!(plot.width > 0.0 && plot.height > 0.0);
    assert!(!restored.scene.hits.is_empty());
    assert!(restored
        .scene
        .primitives
        .iter()
        .filter(|p| p.id.is_some())
        .all(|p| p.clip == Some(plot)));
}

fn marker_center(measurement: &GeoMapMeasurement, index: usize) -> Point {
    let markers: Vec<_> = measurement
        .scene
        .primitives
        .iter()
        .filter(|p| p.id.is_none() && p.clip.is_none())
        .collect();
    match &markers[index].geometry {
        Geometry::Ellipse(rect) => Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0),
        other => panic!("a marker is a circle, not {other:?}"),
    }
}

/// GeoVisualElementTests: a marker is drawn at the projection of its coordinate, moved from the
/// origin, and two coordinates are two pixels.
#[test]
fn geo_markers_are_projected_to_their_coordinates() {
    const NEW_YORK: (f64, f64) = (-74.00, 40.71);
    let markers = [
        GeoMarker::new(NEW_YORK.0, NEW_YORK.1),
        GeoMarker::new(139.69, 35.69),
    ];
    let measured = GeoMapChartEngine::measure(
        Size::new(800.0, 600.0),
        world(),
        &[heat(&[("usa", 15.0)])],
        &markers,
        &mercator(),
    )
    .unwrap();
    let new_york = marker_center(&measured, 0);
    let expected = measured.project(NEW_YORK.0, NEW_YORK.1).unwrap();
    assert!((new_york.x - expected.x).abs() < 1e-9 && (new_york.y - expected.y).abs() < 1e-9);
    assert!(new_york.x != 0.0 && new_york.y != 0.0);
    assert_ne!(new_york.x, marker_center(&measured, 1).x);
    // A marker on the far side of a globe is hidden.
    let globe = GeoMapOptions {
        projection: MapProjection::Orthographic,
        ..GeoMapOptions::default()
    };
    let far = GeoMapChartEngine::measure(
        Size::new(600.0, 600.0),
        world(),
        &[],
        &[GeoMarker::new(180.0, 0.0)],
        &globe,
    )
    .unwrap();
    assert!(far.scene.primitives.iter().all(|p| p.id.is_some() || p.clip.is_some()));
}

/// The draw order of the source: the globe in the chart's fill, every land fill, then the
/// stroke of the lands over them.
#[test]
fn orthographic_maps_draw_the_globe_then_fills_then_the_stroke() {
    let options = GeoMapOptions {
        projection: MapProjection::Orthographic,
        center_longitude: 80.0,
        center_latitude: 20.0,
        ..GeoMapOptions::default()
    };
    let measured = measure(600.0, &[heat(&[("ind", 12.0), ("chn", 14.0)])], &options);
    let primitives = &measured.scene.primitives;
    assert!(matches!(primitives[0].geometry, Geometry::Ellipse(_)));
    assert_eq!(primitives[0].style.fill, Some(MAP_FILL));
    let last = primitives.last().unwrap();
    assert!(matches!(last.geometry, Geometry::Path(_)));
    assert_eq!(last.style.stroke, Some(MAP_STROKE));
    assert!(primitives[1..primitives.len() - 1]
        .iter()
        .all(|p| matches!(p.geometry, Geometry::Polygon(_)) && p.style.stroke.is_none()));
    // Every vertex is inside the disc: the horizon is walked along the rim.
    let globe = measured.projector.as_orthographic().unwrap();
    let (cx, cy, r) = (globe.screen_center_x(), globe.screen_center_y(), globe.radius());
    for primitive in &primitives[1..primitives.len() - 1] {
        if let Geometry::Polygon(points) = &primitive.geometry {
            assert!(points.iter().all(|p| (p.x - cx).hypot(p.y - cy) <= r + 1e-6));
        }
    }
    // The far side of the globe has no land.
    assert!(measured.lands.iter().all(|land| land.short_name != "bra"));
}

/// `BuildOrthographicPath` closes a ring that leaves the visible side along the rim of the
/// disc, not with a chord, and drops a ring on the far side.
#[test]
fn horizon_closes_along_the_rim_instead_of_a_chord() {
    let globe = OrthographicProjector::new(500.0, 500.0, 0.0, 0.0, 0.0, 0.0);
    let ring = [
        Point::new(60.0, -40.0),
        Point::new(120.0, -40.0),
        Point::new(120.0, 40.0),
        Point::new(60.0, 40.0),
    ];
    let result = orthographic_ring(&globe, &ring).unwrap();
    let on_rim = result
        .iter()
        .filter(|p| ((p.x - 250.0).hypot(p.y - 250.0) - 250.0).abs() < 1e-3)
        .count();
    assert!(on_rim > 20, "{on_rim} rim points");
    assert!(result
        .iter()
        .all(|p| (p.x - 250.0).hypot(p.y - 250.0) <= 250.0 + 1e-6));
    let hidden = [
        Point::new(150.0, -10.0),
        Point::new(170.0, -10.0),
        Point::new(170.0, 10.0),
    ];
    assert!(orthographic_ring(&globe, &hidden).is_none());
}

/// `MapFactory.Zoom` keeps the pivot in place, `Pan` moves the map and the viewport inverts;
/// the stroke follows the zoom.
#[test]
fn zoom_keeps_the_pivot_in_place_and_pan_inverts() {
    let center = Point::new(300.0, 300.0);
    let viewport = MapViewport::default().panned(Point::new(30.0, -20.0));
    let pivot = Point::new(450.0, 270.0);
    let base = viewport.inverse(pivot, center);
    let zoomed = viewport.zoomed(pivot, ZoomDirection::ZoomIn, 0.2, 1.0, 100.0, center);
    assert!((zoomed.zoom - 1.25).abs() < 1e-12);
    let again = zoomed.transform(base, center);
    assert!((again.x - pivot.x).abs() < 1e-9 && (again.y - pivot.y).abs() < 1e-9);
    // Zooming out stops at 80 % of the minimum zoom.
    let out = MapViewport::default().zoomed(pivot, ZoomDirection::ZoomOut, 0.9, 1.0, 100.0, center);
    assert!((out.zoom - 0.8).abs() < 1e-12);
    let options = GeoMapOptions {
        viewport: zoomed,
        ..mercator()
    };
    let measured = measure(600.0, &[], &options);
    let stroke = measured.scene.primitives.last().unwrap();
    assert!((stroke.style.stroke_width - 1.25).abs() < 1e-12);
    let pixel = measured.project(10.0, 20.0).unwrap();
    let (lon, lat) = measured.unproject(pixel).unwrap();
    assert!((lon - 10.0).abs() < 1e-9 && (lat - 20.0).abs() < 1e-9);
}

/// `BounceBack`: a map zoomed out below the minimum, or panned off the control, settles back.
#[test]
fn bounce_back_settles_the_zoom_and_the_pan() {
    let control = Size::new(600.0, 400.0);
    assert_eq!(MapViewport::default().bounce_back_target(control, 1.0), None);
    let small = MapViewport::new(0.8, Point::new(0.0, 0.0));
    assert_eq!(small.bounce_back_target(control, 1.0), Some(MapViewport::default()));
    let panned = MapViewport::new(2.0, Point::new(500.0, -500.0));
    // At zoom 2 the map spans twice the control: it may move by half the control at most.
    assert_eq!(
        panned.bounce_back_target(control, 1.0),
        Some(MapViewport::new(2.0, Point::new(300.0, -200.0)))
    );
    assert_eq!(MapViewport::default().bounce_back_target(Size::new(0.0, 0.0), 1.0), None);
}

/// `RotateTo` takes the short way round the globe.
#[test]
fn rotation_takes_the_short_way_round() {
    assert_eq!(shortest_rotation(170.0, -170.0), 190.0);
    assert_eq!(shortest_rotation(-170.0, 170.0), -190.0);
    assert_eq!(shortest_rotation(10.0, 80.0), 80.0);
}

/// The world map draws finite scenes in every projection.
#[test]
fn world_map_builds_finite_scenes_in_every_projection() {
    for projection in [
        MapProjection::Default,
        MapProjection::Mercator,
        MapProjection::Orthographic,
    ] {
        let options = GeoMapOptions {
            projection,
            ..GeoMapOptions::default()
        };
        let measured = GeoMapChartEngine::measure(
            Size::new(900.0, 600.0),
            world(),
            &[heat(&[("bra", 13.0), ("chn", 14.0)])],
            &[],
            &options,
        )
        .unwrap();
        assert!(measured.scene.hits.len() > 30, "{projection:?}");
        for primitive in &measured.scene.primitives {
            let points: Vec<Point> = match &primitive.geometry {
                Geometry::Polygon(points) => points.clone(),
                Geometry::Path(commands) => commands
                    .iter()
                    .filter_map(|c| match c {
                        PathCommand::MoveTo(p) | PathCommand::LineTo(p) => Some(*p),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            assert!(points.iter().all(|p| p.x.is_finite() && p.y.is_finite()), "{projection:?}");
        }
    }
}

/// A heat map of one colour is an error, as the source throws.
#[test]
fn heat_maps_need_two_colours() {
    let mut series = heat(&[("fra", 1.0)]);
    series.series.heat_map = vec![Color::from_rgb(0, 0, 0)];
    assert!(GeoMapChartEngine::measure(
        Size::new(400.0, 400.0),
        world(),
        &[series],
        &[],
        &GeoMapOptions::default()
    )
    .is_err());
}
