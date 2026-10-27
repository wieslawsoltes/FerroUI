use super::*;
use ferroui_base::{Point as NativePoint, Ref, Size};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{ContentControl, Control, Window};
use ferroui_designer_tools::LayoutHandleAction;

const SCALED_CONTENT: &str = r#"<Canvas xmlns="https://github.com/ferroui" Width="480" Height="300"><Viewbox Name="ScaleHost" Canvas.Left="40" Canvas.Top="30" Width="240" Height="120"><Button Name="Target" Width="100" Height="40" Margin="10" Content="Target" /></Viewbox></Canvas>"#;

struct Fixture {
    session: DesignerSession,
    window: Ref<Window>,
}
impl Fixture {
    fn new(source: &str) -> Self {
        let host = ContentControl::new();
        let mut session = DesignerSession::new(source, host.clone()).unwrap();
        let window = Window::new();
        window.set_width(520.0);
        window.set_height(340.0);
        window.set_content(Some(Control::boxed(host)));
        window.show();
        window.update_layout();
        session.preview_host().measure(Size::new(480.0, 300.0));
        session.refresh_runtime_mapping();
        assert!(session.can_manipulate());
        Self { session, window }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

fn near(actual: Point, expected: Point) -> bool {
    (actual.x - expected.x).abs() < 0.01 && (actual.y - expected.y).abs() < 0.01
}

#[test]
fn composed_margin_regions_do_not_fill_outside_rotated_or_sheared_bounds() {
    let document = ferroui_designer_core::Document::new("<Border/>").unwrap();
    let panel = document.ast().root();
    for (transform, filled) in [
        (
            Affine {
                m11: 2.0,
                m22: 3.0,
                ..Affine::IDENTITY
            },
            true,
        ),
        (
            Affine {
                m12: 0.5,
                m21: 0.3,
                ..Affine::IDENTITY
            },
            false,
        ),
    ] {
        let mut scene = LayoutHelperScene {
            panel,
            title: String::new(),
            primitives: vec![],
            handles: vec![],
            placement: None,
        };
        let bounds = ferroui_designer_tools::Rect::new(10.0, 20.0, 5.0, 60.0);
        append_primitives(
            &mut scene,
            vec![ferroui_designer_tools::LayoutGuide {
                role: LayoutGuideRole::Spacing,
                geometry: LayoutGuideGeometry::Rectangle {
                    bounds,
                    filled: true,
                },
            }],
            transform,
        )
        .unwrap();
        assert_eq!(
            scene.primitives.iter().any(|guide| matches!(
                guide.geometry,
                LayoutGuideGeometry::Rectangle { filled: true, .. }
            )),
            filled
        );
        if !filled {
            assert_eq!(scene.primitives.len(), 4);
            let corners = [
                Point::new(10.0, 20.0),
                Point::new(15.0, 20.0),
                Point::new(15.0, 80.0),
                Point::new(10.0, 80.0),
            ];
            for (index, guide) in scene.primitives.iter().enumerate() {
                let LayoutGuideGeometry::Line { start, end } = guide.geometry else {
                    panic!("Skewed margin regions require exact outline edges")
                };
                assert!(near(start, transform.transform(corners[index]).unwrap()));
                assert!(near(
                    end,
                    transform.transform(corners[(index + 1) % 4]).unwrap()
                ));
            }
        }
    }
}

#[test]
fn scaled_viewbox_margin_helpers_keep_outer_panel_geometry_and_child_authored_units() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new(SCALED_CONTENT);
    let panel = fixture
        .session
        .document()
        .ast()
        .find_name("ScaleHost")
        .unwrap();
    let child = fixture
        .session
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    fixture.session.select([child]);
    let snapshot = fixture.session.selection_layout_helpers().unwrap();
    assert_eq!(snapshot.scene.panel, panel);
    let outer_bounds = fixture.session.preview.visual(panel).unwrap().bounds();
    assert!(snapshot.scene.primitives.iter().any(|guide| {
        matches!(guide.geometry, LayoutGuideGeometry::Rectangle { bounds, .. }
            if guide.role == LayoutGuideRole::Container
                && (bounds.width-outer_bounds.width).abs()<0.01
                && (bounds.height-outer_bounds.height).abs()<0.01)
    }));
    let index = snapshot.scene.handles.iter().position(|handle| {
        matches!(&handle.action, LayoutHandleAction::ThicknessSide { node, property, side:0, .. }
            if *node==child && property=="Margin")
    }).unwrap();
    let handle = &snapshot.scene.handles[index];
    assert_eq!(handle.value, 10.0);

    let visual = fixture.session.preview.visual(child).unwrap();
    let layout_parent = visual.visual_parent().unwrap();
    let bounds = visual.bounds();
    let local_start = NativePoint::new(bounds.x - 10.0, bounds.y + bounds.height * 0.5);
    let local_end = NativePoint::new(bounds.x, local_start.y);
    let artboard = fixture.session.preview.host.clone();
    let start = layout_parent
        .translate_point(local_start, &artboard)
        .unwrap();
    let end = layout_parent.translate_point(local_end, &artboard).unwrap();
    assert!(
        (end.x - start.x - 20.0).abs() < 0.01,
        "fixture must have a 2x internal presenter"
    );
    let expected_start = Point::new(start.x, start.y);
    let expected_end = Point::new(end.x, end.y);
    assert!(
        snapshot.scene.primitives.iter().any(|guide| {
            if let LayoutGuideGeometry::Line { start, end } = guide.geometry {
                guide.role == LayoutGuideRole::Spacing
                    && near(
                        snapshot.to_artboard.transform(start).unwrap(),
                        expected_start,
                    )
                    && near(snapshot.to_artboard.transform(end).unwrap(), expected_end)
            } else {
                false
            }
        }),
        "margin bands must use presenter-scaled geometry"
    );
    let anchor = snapshot.to_artboard.transform(handle.position).unwrap();
    assert!(near(anchor, Point::new((start.x + end.x) * 0.5, start.y)));
    assert!(
        near(
            snapshot
                .handle_delta_to_artboard(index, Point::new(1.0, 0.0))
                .unwrap(),
            Point::new(2.0, 0.0)
        ),
        "keyboard steps must retain authored margin units"
    );

    let preview = fixture
        .session
        .preview_layout_handle(&snapshot, index, NativePoint::new(10.0, 0.0))
        .unwrap();
    assert!(preview.is_accepted(), "{:?}", preview.rejection());
    assert_eq!(fixture.session.document().source(), SCALED_CONTENT);
    assert_eq!(
        preview.helpers.scene.handles[index].value, 15.0,
        "ten artboard units must become five authored units"
    );
    let moved = preview
        .helpers
        .to_artboard
        .transform(preview.helpers.scene.handles[index].position)
        .unwrap();
    assert!(near(moved, Point::new(anchor.x + 10.0, anchor.y)));
    fixture.session.commit_layout_preview(&preview).unwrap();
    assert_eq!(
        fixture
            .session
            .document()
            .ast()
            .node(child)
            .unwrap()
            .attribute("Margin"),
        Some("15,10,10,10")
    );
    fixture.session.undo().unwrap();
    assert_eq!(fixture.session.document().source(), SCALED_CONTENT);
}
