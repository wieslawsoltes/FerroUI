//! Direct assertion ports of CoreTests/CoreObjectsTests/GeoProjectorsTesting.cs.
use super::*;
use ferroui_base::{Point, Rect};

fn close(a: f64, b: f64, tolerance: f64) {
    assert!((a - b).abs() < tolerance, "{a} != {b}");
}
fn mercator() -> MercatorProjector {
    MercatorProjector::new(
        Rect::new(0.0, 0.0, 360.0, 180.0),
        GeographicBounds::default(),
    )
}
fn ortho() -> OrthographicProjector {
    OrthographicProjector::new(Rect::new(0.0, 0.0, 500.0, 500.0), GeoCoordinate::default())
}

#[test]
fn mercator_projector_projects_origin_to_center() {
    let projector = MercatorProjector::new(
        Rect::new(0.0, 0.0, 360.0, 180.0),
        GeographicBounds::new(-85.0, 85.0, -180.0, 180.0),
    );
    let result = projector.to_map(0.0, 0.0);
    close(result.x, 180.0, 0.1);
    close(result.y, 90.0, 0.1);
}
#[test]
fn mercator_projector_projects_top_left() {
    close(mercator().to_map(-180.0, 0.0).x, 0.0, 0.1);
}
#[test]
fn mercator_projector_projects_top_right() {
    close(mercator().to_map(180.0, 0.0).x, 360.0, 0.1);
}
#[test]
fn mercator_projector_respects_offset() {
    let projector = MercatorProjector::new(
        Rect::new(50.0, 25.0, 360.0, 180.0),
        GeographicBounds::default(),
    );
    close(projector.to_map(-180.0, 0.0).x, 50.0, 0.1);
}
#[test]
fn mercator_projector_properties() {
    let bounds = Rect::new(10.0, 20.0, 800.0, 600.0);
    assert_eq!(
        MercatorProjector::new(bounds, GeographicBounds::default()).map_bounds(),
        bounds
    );
}
#[test]
fn mercator_projector_preferred_ratio() {
    close(
        MercatorProjector::preferred_ratio(GeographicBounds::default()),
        1.355,
        0.01,
    );
    close(
        MercatorProjector::preferred_ratio(GeographicBounds::new(-85.0, 85.0, -180.0, 180.0)),
        1.0,
        0.01,
    );
    close(
        MercatorProjector::preferred_ratio(GeographicBounds::new(
            f64::NAN,
            f64::NAN,
            f64::NAN,
            f64::NAN,
        )),
        1.355,
        0.01,
    );
}
#[test]
fn mercator_projector_is_always_visible() {
    for (lon, lat) in [(0.0, 0.0), (180.0, 90.0), (-180.0, -90.0)] {
        assert!(mercator().is_visible(lon, lat));
    }
}
#[test]
fn mercator_projector_normalizes_degenerate_and_inverted_bounds() {
    close(
        MercatorProjector::preferred_ratio(GeographicBounds::new(72.0, 35.0, -25.0, 45.0)),
        MercatorProjector::preferred_ratio(GeographicBounds::new(35.0, 72.0, -25.0, 45.0)),
        0.001,
    );
    let bounds = GeographicBounds::new(50.0, 50.0, -25.0, 45.0);
    assert!(MercatorProjector::preferred_ratio(bounds).is_finite());
    let point = MercatorProjector::new(Rect::new(0.0, 0.0, 360.0, 180.0), bounds).to_map(0.0, 0.0);
    assert!(point.x.is_finite() && point.y.is_finite());
}
#[test]
fn control_coordinates_projector_normalizes_degenerate_and_inverted_bounds() {
    close(
        ControlCoordinatesProjector::preferred_ratio(GeographicBounds::new(
            72.0, 35.0, 45.0, -25.0,
        )),
        ControlCoordinatesProjector::preferred_ratio(GeographicBounds::new(
            35.0, 72.0, -25.0, 45.0,
        )),
        0.001,
    );
    let bounds = GeographicBounds::new(50.0, 50.0, -25.0, 45.0);
    assert!(ControlCoordinatesProjector::preferred_ratio(bounds).is_finite());
    let point = ControlCoordinatesProjector::new(Rect::new(0.0, 0.0, 360.0, 180.0), bounds)
        .to_map(0.0, 0.0);
    assert!(point.x.is_finite() && point.y.is_finite());
}
#[test]
fn orthographic_projector_projects_center_to_screen_center() {
    let point = ortho().to_map(0.0, 0.0);
    close(point.x, 250.0, 0.1);
    close(point.y, 250.0, 0.1);
}
#[test]
fn orthographic_projector_center_point_is_visible() {
    assert!(ortho().is_visible(0.0, 0.0));
}
#[test]
fn orthographic_projector_opposite_point_is_not_visible() {
    assert!(!ortho().is_visible(180.0, 0.0));
}
#[test]
fn orthographic_projector_point_just_beyond_horizon_is_not_visible() {
    assert!(!ortho().is_visible(91.0, 0.0));
}
#[test]
fn orthographic_projector_point_just_before_horizon_is_visible() {
    assert!(ortho().is_visible(89.0, 0.0));
}
#[test]
fn orthographic_projector_properties() {
    let bounds = Rect::new(10.0, 20.0, 800.0, 600.0);
    let center = GeoCoordinate::new(45.0, 30.0);
    let projector = OrthographicProjector::new(bounds, center);
    assert_eq!(projector.map_bounds(), bounds);
    assert_eq!(projector.center(), center);
    assert!(projector.radius() > 0.0);
}
#[test]
fn orthographic_projector_preferred_ratio() {
    let projector = Projector::fit(
        MapProjection::Orthographic,
        Rect::new(0.0, 0.0, 800.0, 600.0),
        GeographicBounds::default(),
        GeoCoordinate::default(),
    );
    close(
        projector.map_bounds().width / projector.map_bounds().height,
        1.0,
        0.001,
    );
}
#[test]
fn orthographic_projector_radius_is_half_min_dimension() {
    close(
        OrthographicProjector::new(Rect::new(0.0, 0.0, 800.0, 600.0), GeoCoordinate::default())
            .radius(),
        300.0,
        0.001,
    );
}
#[test]
fn orthographic_projector_screen_center() {
    assert_eq!(
        OrthographicProjector::new(
            Rect::new(50.0, 30.0, 800.0, 600.0),
            GeoCoordinate::default()
        )
        .screen_center(),
        Point::new(450.0, 330.0)
    );
}
#[test]
fn orthographic_projector_visibility_with_different_center() {
    let projector = OrthographicProjector::new(
        Rect::new(0.0, 0.0, 500.0, 500.0),
        GeoCoordinate::new(-74.0, 40.7),
    );
    assert!(projector.is_visible(-74.0, 40.7));
    assert!(projector.is_visible(-80.0, 35.0));
    assert!(!projector.is_visible(139.7, 35.7));
}
#[test]
fn orthographic_projector_symmetry() {
    let left = ortho().to_map(-45.0, 0.0);
    let right = ortho().to_map(45.0, 0.0);
    close((left.x - 250.0).abs(), (right.x - 250.0).abs(), 0.1);
    close(left.y, right.y, 0.1);
}
fn round_trips(projector: &impl MapProjector) {
    for (lon, lat) in [
        (0.0, 0.0),
        (-74.0, 40.7),
        (139.69, 35.69),
        (-3.7, 40.4),
        (45.0, -25.0),
    ] {
        let roundtrip = projector
            .to_coordinates(projector.to_map(lon, lat))
            .unwrap();
        close(lon, roundtrip.longitude, 1e-4);
        close(lat, roundtrip.latitude, 1e-4);
    }
}
#[test]
fn mercator_projector_round_trips() {
    round_trips(&MercatorProjector::new(
        Rect::new(50.0, 30.0, 800.0, 600.0),
        GeographicBounds::default(),
    ));
}
#[test]
fn control_coordinates_projector_round_trips() {
    round_trips(&ControlCoordinatesProjector::new(
        Rect::new(50.0, 30.0, 800.0, 600.0),
        GeographicBounds::default(),
    ));
}
#[test]
fn orthographic_projector_round_trips_on_visible_hemisphere() {
    let projector = OrthographicProjector::new(
        Rect::new(0.0, 0.0, 500.0, 500.0),
        GeoCoordinate::new(-30.0, 20.0),
    );
    for (lon, lat) in [(-30.0, 20.0), (-74.0, 40.7), (-3.7, 40.4), (15.0, 50.0)] {
        assert!(projector.is_visible(lon, lat));
        let roundtrip = projector
            .to_coordinates(projector.to_map(lon, lat))
            .unwrap();
        close(lon, roundtrip.longitude, 1e-2);
        close(lat, roundtrip.latitude, 1e-2);
    }
}
#[test]
fn orthographic_projector_rejects_clicks_outside_disc() {
    assert!(ortho().to_coordinates(Point::new(0.0, 0.0)).is_none());
    assert!(ortho().to_coordinates(Point::new(500.0, 500.0)).is_none());
    assert_eq!(
        ortho().to_coordinates(Point::new(250.0, 250.0)),
        Some(GeoCoordinate::default())
    );
}
