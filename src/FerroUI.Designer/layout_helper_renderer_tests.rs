use super::*;
use ferroui_designer_tools::{
    LayoutHandle, LayoutHandleAction, LayoutHelperScene, Point as ModelPoint,
};

pub(super) fn fixture_scene() -> LayoutHelperScene {
    let mut scene = LayoutHelperScene {
        panel: 1,
        title: "Custom panel · layout helpers".into(),
        primitives: Vec::new(),
        handles: Vec::new(),
        placement: None,
    };
    scene.rectangle(
        ferroui_designer_tools::Rect::new(0.0, 0.0, 280.0, 220.0),
        LayoutGuideRole::Container,
        false,
    );
    scene.rectangle(
        ferroui_designer_tools::Rect::new(12.0, 12.0, 256.0, 196.0),
        LayoutGuideRole::Content,
        false,
    );
    scene.line(
        ModelPoint::new(140.0, 0.0),
        ModelPoint::new(140.0, 220.0),
        LayoutGuideRole::Track,
    );
    scene.line(
        ModelPoint::new(0.0, 110.0),
        ModelPoint::new(280.0, 110.0),
        LayoutGuideRole::Track,
    );
    scene.label(
        ModelPoint::new(70.0, 8.0),
        "Column 1 · 140",
        LayoutGuideRole::Track,
    );
    scene.rectangle(
        ferroui_designer_tools::Rect::new(152.0, 122.0, 116.0, 86.0),
        LayoutGuideRole::Placement,
        true,
    );
    scene.label(
        ModelPoint::new(210.0, 164.0),
        "Row 2, column 2",
        LayoutGuideRole::Placement,
    );
    scene.handles.push(LayoutHandle {
        id: "spacing".into(),
        label: "Spacing".into(),
        position: ModelPoint::new(140.0, 60.0),
        axis: Axis::Horizontal,
        value: 12.0,
        minimum: 0.0,
        maximum: None,
        direction: 1.0,
        action: LayoutHandleAction::Custom {
            key: "spacing".into(),
        },
        disabled_reason: None,
    });
    scene
}

#[test]
fn handle_hit_targets_use_screen_space_after_affine_transform_and_exclude_disabled_grips() {
    let mut scene = fixture_scene();
    scene.handles[0].position = ModelPoint::new(4.0, 8.0);
    let transform = Affine {
        m11: 2.0,
        m12: 0.5,
        m21: 0.25,
        m22: 1.2,
        dx: 30.0,
        dy: 40.0,
    };
    let mut snapshot = LayoutHelperSnapshot::new(scene, transform);
    let center = Point::new(40.0, 51.6);
    assert_eq!(hit_test_layout_handle(&snapshot, center, 2.0), Some(0));
    assert_eq!(
        hit_test_layout_handle(&snapshot, Point::new(center.x + 3.9, center.y), 2.0),
        Some(0)
    );
    assert_eq!(
        hit_test_layout_handle(&snapshot, Point::new(center.x + 4.1, center.y), 2.0),
        None
    );
    assert_eq!(
        hit_test_layout_handle(&snapshot, Point::new(center.x + 15.9, center.y), 0.5),
        Some(0)
    );
    snapshot.scene.handles[0].disabled_reason = Some("Spacing comes from a binding".into());
    assert_eq!(hit_test_layout_handle(&snapshot, center, 1.0), None);
    assert_eq!(
        hover_layout_handle(&snapshot, center, 1.0),
        Some(0),
        "disabled grips explain their reason on hover without becoming editable"
    );
    assert_eq!(
        hover_layout_handle(&snapshot, Point::new(center.x + 4.1, center.y), 2.0),
        None,
        "hover uses the same screen-size target as pressing"
    );
    snapshot.scene.handles[0].disabled_reason = None;
    snapshot.to_artboard.m22 = 0.0625;
    assert_eq!(
        hit_test_layout_handle(&snapshot, center, 1.0),
        None,
        "singular transforms cannot author a drag"
    );
    assert_eq!(hit_test_layout_handle(&snapshot, center, f64::NAN), None);
}

#[test]
fn distance_feedback_uses_nearest_overlapping_neighbors_and_identifies_equal_gaps() {
    let moving = Rect::new(100.0, 100.0, 40.0, 40.0);
    let peers = [
        Rect::new(40.0, 110.0, 40.0, 20.0),
        Rect::new(160.0, 110.0, 40.0, 20.0),
        Rect::new(99.0, -100.0, 0.5, 10.0),
        Rect::new(-100.0, 110.0, 10.0, 20.0),
        Rect::new(f64::NAN, 110.0, 10.0, 20.0),
    ];
    let guides = metrics::manipulation_guides(moving, &peers, &[], 1.0);
    let labels = guides
        .iter()
        .filter_map(|guide| match &guide.geometry {
            LayoutGuideGeometry::Label { text, .. } => Some((guide.role, text.as_str())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        [
            (LayoutGuideRole::Spacing, "20 equal"),
            (LayoutGuideRole::Spacing, "20 equal")
        ]
    );
}

#[test]
fn alignment_guides_connect_nearest_matching_peer_and_remain_finite() {
    let moving = Rect::new(100.0, 100.0, 40.0, 40.0);
    let peers = [
        Rect::new(100.0, 20.0, 40.0, 10.0),
        Rect::new(100.0, 1000.0, 40.0, 40.0),
    ];
    let guides = metrics::manipulation_guides(
        moving,
        &peers,
        &[SnapGuide {
            axis: Axis::Horizontal,
            position: 100.0,
        }],
        1.0,
    );
    let alignment = guides
        .iter()
        .find(|guide| guide.role == LayoutGuideRole::Relation)
        .unwrap();
    assert_eq!(
        alignment.geometry,
        LayoutGuideGeometry::Line {
            start: ModelPoint::new(100.0, 14.0),
            end: ModelPoint::new(100.0, 146.0)
        }
    );
    let quad = transformed_rect(
        ferroui_designer_tools::Rect::new(0.0, 0.0, 40.0, 20.0),
        Affine {
            m11: 1.0,
            m12: 0.5,
            m21: 0.25,
            m22: 1.0,
            dx: 0.0,
            dy: 0.0,
        },
    )
    .unwrap();
    assert_eq!(quad[1], Point::new(40.0, 20.0));
    assert_eq!(quad[2], Point::new(45.0, 40.0));
    assert_eq!(
        quad[3],
        Point::new(5.0, 20.0),
        "a helper preserves its affine quadrilateral rather than using an AABB"
    );
}
