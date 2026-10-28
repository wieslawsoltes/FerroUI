use super::*;

fn resize(source: &str, before: Rect, after: Rect) -> (Document, NodeId) {
    let mut document = Document::new(source).unwrap();
    let node = document.ast().find_name("Target").unwrap();
    let bounds = BTreeMap::from([
        (document.ast().root(), Rect::new(0.0, 0.0, 500.0, 300.0)),
        (node, before),
    ]);
    let edits = LayoutRegistry::built_in()
        .plan(
            &LayoutContext::new(&document, &bounds),
            LayoutAction::Resize {
                node,
                before,
                after,
            },
        )
        .unwrap();
    document.apply("Resize", edits).unwrap();
    (document, node)
}

#[test]
fn one_axis_resize_retains_opposite_bound_dimension_and_canvas_anchors() {
    for anchors in [
        "Canvas.Top='{Binding Y}'",
        "Canvas.Bottom='{Binding Bottom}'",
        "Canvas.Top='{Binding Y}' Canvas.Bottom='{Binding Bottom}'",
    ] {
        let source = format!("<Canvas><Border Name='Target' Width='60' Height='{{Binding H}}' Canvas.Left='20' {anchors}/></Canvas>");
        let (mut document, node) = resize(
            &source,
            Rect::new(20.0, 30.0, 60.0, 40.0),
            Rect::new(20.0, 30.0, 130.0, 40.0),
        );
        let element = document.ast().node(node).unwrap();
        assert_eq!(element.attribute("Width"), Some("130"));
        assert_eq!(element.attribute("Height"), Some("{Binding H}"));
        assert!(document.source().contains(anchors));
        document.undo().unwrap();
        assert_eq!(document.source(), source);
    }
    let source = "<Canvas><Border Name='Target' Width='{Binding W}' Height='40' Canvas.Left='{Binding X}' Canvas.Right='{Binding Right}' Canvas.Top='30'/></Canvas>";
    let (mut document, node) = resize(
        source,
        Rect::new(20.0, 30.0, 60.0, 40.0),
        Rect::new(20.0, 30.0, 60.0, 70.0),
    );
    let element = document.ast().node(node).unwrap();
    assert_eq!(element.attribute("Height"), Some("70"));
    assert_eq!(element.attribute("Width"), Some("{Binding W}"));
    assert_eq!(element.attribute("Canvas.Left"), Some("{Binding X}"));
    assert_eq!(element.attribute("Canvas.Right"), Some("{Binding Right}"));
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}

#[test]
fn resizing_one_dimension_does_not_materialize_the_other_automatic_dimension() {
    for panel in ["Canvas", "Grid", "StackPanel", "Border"] {
        let source = format!("<{panel}><Border Name='Target' Width='60'/></{panel}>");
        let (mut document, node) = resize(
            &source,
            Rect::new(0.0, 0.0, 60.0, 40.0),
            Rect::new(0.0, 0.0, 130.0, 40.0),
        );
        let element = document.ast().node(node).unwrap();
        assert_eq!(element.attribute("Width"), Some("130"));
        assert!(element.attribute("Height").is_none());
        document.undo().unwrap();
        assert_eq!(document.source(), source);
    }
}

#[test]
fn canvas_resize_keeps_native_aliases_and_removes_only_the_active_trailing_alias() {
    let source = "<ui:Canvas xmlns:ui='using:FerroUI.Controls' xmlns:app='urn:application'><ui:Border Name='Target' ui:Canvas.Left='20' ui:Canvas.Right='10' ui:Canvas.Bottom='{Binding Bottom}' app:Canvas.Right='keep' Width='60' Height='{Binding H}'/></ui:Canvas>";
    let (mut document, node) = resize(
        source,
        Rect::new(20.0, 30.0, 60.0, 40.0),
        Rect::new(25.0, 30.0, 130.0, 40.0),
    );
    let element = document.ast().node(node).unwrap();
    assert_eq!(element.attribute("ui:Canvas.Left"), Some("25"));
    assert!(element.attribute("Canvas.Left").is_none());
    assert!(element.attribute("ui:Canvas.Right").is_none());
    assert_eq!(element.attribute("app:Canvas.Right"), Some("keep"));
    assert_eq!(
        element.attribute("ui:Canvas.Bottom"),
        Some("{Binding Bottom}")
    );
    assert_eq!(element.attribute("Height"), Some("{Binding H}"));
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}

#[test]
fn canvas_resize_materializes_a_native_leading_anchor_under_a_foreign_default_namespace() {
    let source = "<ui:Canvas xmlns='urn:application' xmlns:ui='using:FerroUI.Controls'><ui:Border Name='Target' ui:Canvas.Right='10' Canvas.Right='keep' Width='60' Height='40'/></ui:Canvas>";
    let (mut document, node) = resize(
        source,
        Rect::new(430.0, 30.0, 60.0, 40.0),
        Rect::new(430.0, 30.0, 130.0, 40.0),
    );
    let element = document.ast().node(node).unwrap();
    assert_eq!(element.attribute("ui:Canvas.Left"), Some("430"));
    assert!(element.attribute("ui:Canvas.Right").is_none());
    assert_eq!(element.attribute("Canvas.Right"), Some("keep"));
    assert!(element.attribute("Canvas.Left").is_none());
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}

#[test]
fn canvas_resize_respects_effective_leading_anchors_and_nan_defaults() {
    for left in ["25", "NaN"] {
        let document =
            Document::new("<Canvas><Border Name='Target' Width='60' Height='40'/></Canvas>")
                .unwrap();
        let node = document.ast().find_name("Target").unwrap();
        let before = Rect::new(25.0, 30.0, 60.0, 40.0);
        let bounds = BTreeMap::from([
            (document.ast().root(), Rect::new(0.0, 0.0, 500.0, 300.0)),
            (node, before),
        ]);
        let properties = BTreeMap::from([(
            node,
            BTreeMap::from([
                ("Canvas.Left".into(), left.into()),
                ("Canvas.Top".into(), "NaN".into()),
            ]),
        )]);
        let edits = LayoutRegistry::built_in()
            .plan(
                &LayoutContext::new(&document, &bounds).with_resolved_properties(&properties),
                LayoutAction::Resize {
                    node,
                    before,
                    after: Rect::new(25.0, 30.0, 130.0, 70.0),
                },
            )
            .unwrap();
        let mut candidate = document.clone();
        candidate.apply("Resize", edits).unwrap();
        let element = candidate.ast().node(node).unwrap();
        assert_eq!(element.attribute("Canvas.Left"), Some("25"));
        assert_eq!(element.attribute("Canvas.Top"), Some("30"));
    }
}

#[test]
fn moving_still_protects_both_axes_and_resizing_protects_the_changed_axis() {
    let document = Document::new("<Canvas xmlns:ui='using:FerroUI.Controls'><Border Name='Target' Width='60' Height='40' ui:Canvas.Right='{Binding R}' Canvas.Top='{Binding Y}'/></Canvas>").unwrap();
    let node = document.ast().find_name("Target").unwrap();
    let before = Rect::new(20.0, 30.0, 60.0, 40.0);
    let bounds = BTreeMap::from([
        (document.ast().root(), Rect::new(0.0, 0.0, 500.0, 300.0)),
        (node, before),
    ]);
    let context = LayoutContext::new(&document, &bounds);
    for action in [
        LayoutAction::Move {
            node,
            delta: Point::new(5.0, 0.0),
        },
        LayoutAction::Resize {
            node,
            before,
            after: Rect::new(20.0, 30.0, 130.0, 40.0),
        },
        LayoutAction::Resize {
            node,
            before,
            after: Rect::new(20.0, 30.0, 60.0, 70.0),
        },
    ] {
        assert!(LayoutRegistry::built_in().plan(&context, action).is_err());
    }
    assert!(!document.can_undo());
}
