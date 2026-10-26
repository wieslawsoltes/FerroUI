use super::*;
use ferroui_designer_core::{Document, Edit};
use std::collections::BTreeMap;
fn id(document: &Document, name: &str) -> u64 {
    document.ast().find_name(name).unwrap()
}
fn apply(document: &mut Document, bounds: &BTreeMap<u64, Rect>, action: LayoutAction) {
    let plan = LayoutRegistry::built_in()
        .plan(&LayoutContext::new(document, bounds), action)
        .unwrap();
    document.apply("Tool", plan).unwrap();
}
#[test]
fn canvas_movement_uses_measured_anchor_and_preserves_unrelated_source() {
    let mut document = Document::new(
        "<Canvas><!--keep--><Button Name='a' Canvas.Right='10' Width='20'/></Canvas>",
    )
    .unwrap();
    let node = id(&document, "a");
    let root = document.ast().root();
    let bounds = BTreeMap::from([
        (root, Rect::new(100.0, 200.0, 500.0, 400.0)),
        (node, Rect::new(570.0, 215.0, 20.0, 30.0)),
    ]);
    apply(
        &mut document,
        &bounds,
        LayoutAction::Move {
            node,
            delta: Point::new(5.0, 10.0),
        },
    );
    let node = document.ast().node(node).unwrap();
    assert_eq!(node.attribute("Canvas.Left"), Some("475"));
    assert_eq!(node.attribute("Canvas.Top"), Some("25"));
    assert_eq!(node.attribute("Canvas.Right"), None);
    assert!(document.source().contains("<!--keep-->"));
}
#[test]
fn canvas_and_resize_do_not_overwrite_bound_values() {
    let document=Document::new("<Canvas><Button Name='a' Canvas.Left='{Binding X}' Canvas.Top='0' Width='{Binding W}'/></Canvas>").unwrap();
    let node = id(&document, "a");
    let bounds = BTreeMap::new();
    let context = LayoutContext::new(&document, &bounds);
    let registry = LayoutRegistry::built_in();
    assert!(registry
        .plan(
            &context,
            LayoutAction::Move {
                node,
                delta: Point::new(5.0, 0.0)
            }
        )
        .is_err());
    assert!(registry
        .plan(
            &context,
            LayoutAction::Resize {
                node,
                before: Rect::new(0.0, 0.0, 10.0, 10.0),
                after: Rect::new(0.0, 0.0, 20.0, 20.0)
            }
        )
        .is_err());
    assert!(registry
        .plan(
            &context,
            LayoutAction::Move {
                node,
                delta: Point::new(f64::NAN, 0.0)
            }
        )
        .is_err());
}
#[test]
fn flow_panel_moves_preserve_the_opposite_margin_and_reorder_content_only() {
    for panel in [
        "Panel",
        "Grid",
        "StackPanel",
        "ReversibleStackPanel",
        "WrapPanel",
        "DockPanel",
        "UniformGrid",
        "RelativePanel",
        "FlexPanel",
    ] {
        let mut document = Document::new(format!(
            "<{panel}><Button Name='a' Margin='1,2,3,4'/><Button Name='b'/></{panel}>"
        ))
        .unwrap();
        let node = id(&document, "a");
        let bounds = BTreeMap::new();
        apply(
            &mut document,
            &bounds,
            LayoutAction::Move {
                node,
                delta: Point::new(10.0, 20.0),
            },
        );
        assert_eq!(
            document.ast().node(node).unwrap().attribute("Margin"),
            Some("11,22,-7,-16")
        );
        apply(
            &mut document,
            &bounds,
            LayoutAction::Reorder { node, index: 1 },
        );
        if panel == "FlexPanel" {
            assert_eq!(
                document.ast().node(node).unwrap().attribute("Flex.Order"),
                Some("1")
            );
        } else {
            assert_eq!(
                document
                    .ast()
                    .element_children(document.ast().root())
                    .last(),
                Some(&node)
            );
        }
    }
    let mut document=Document::new("<Grid><Grid.RowDefinitions><RowDefinition Height='*'/></Grid.RowDefinitions><Grid.Children><Button Name='a'/><Button Name='b'/></Grid.Children></Grid>").unwrap();
    let node = id(&document, "a");
    apply(
        &mut document,
        &BTreeMap::new(),
        LayoutAction::Reorder { node, index: 1 },
    );
    assert!(document.source().contains("<Grid.Children>"));
    assert!(document.source().contains("<RowDefinition Height='*'/>"));
}
#[test]
fn grid_cell_and_track_editing_preserve_constraints() {
    let mut document=Document::new("<Grid ColumnDefinitions='100,*'><Grid.RowDefinitions><RowDefinition Height='Auto' MinHeight='20'/><RowDefinition Height='*'/></Grid.RowDefinitions><Button Name='a'/></Grid>").unwrap();
    let root = document.ast().root();
    let node = id(&document, "a");
    apply(
        &mut document,
        &BTreeMap::new(),
        LayoutAction::GridTrack {
            panel: root,
            axis: Axis::Vertical,
            index: 0,
            length: GridLength::Pixels(48.0),
        },
    );
    assert!(document.source().contains("Height='48' MinHeight='20'"));
    apply(
        &mut document,
        &BTreeMap::new(),
        LayoutAction::GridCell {
            node,
            row: 0,
            column: 1,
            row_span: 2,
            column_span: 1,
        },
    );
    assert_eq!(
        document.ast().node(node).unwrap().attribute("Grid.RowSpan"),
        Some("2")
    );
    assert!(LayoutRegistry::built_in()
        .plan(
            &LayoutContext::new(&document, &BTreeMap::new()),
            LayoutAction::GridCell {
                node,
                row: 1,
                column: 1,
                row_span: 2,
                column_span: 1
            }
        )
        .is_err());
}
#[test]
fn grid_drop_uses_native_tracks_and_requires_live_auto_boundaries() {
    let mut document =
        Document::new("<Grid RowDefinitions='50,*' ColumnDefinitions='100,2*,*'/>").unwrap();
    let root = document.ast().root();
    let bounds = BTreeMap::from([(root, Rect::new(10.0, 20.0, 400.0, 300.0))]);
    apply(
        &mut document,
        &bounds,
        LayoutAction::Drop {
            parent: root,
            source: "<Button Name='new'/>".into(),
            position: Point::new(350.0, 100.0),
        },
    );
    let new = document.ast().node(id(&document, "new")).unwrap();
    assert_eq!(new.attribute("Grid.Row"), Some("1"));
    assert_eq!(new.attribute("Grid.Column"), Some("2"));
    let document = Document::new("<Grid RowDefinitions='Auto,*'/>").unwrap();
    let root = document.ast().root();
    let bounds = BTreeMap::from([(root, Rect::new(0.0, 0.0, 300.0, 300.0))]);
    let action = LayoutAction::Drop {
        parent: root,
        source: "<Button/>".into(),
        position: Point::new(10.0, 80.0),
    };
    assert!(LayoutRegistry::built_in()
        .plan(&LayoutContext::new(&document, &bounds), action.clone())
        .is_err());
    let lines = BTreeMap::from([(
        root,
        GridLines {
            rows: vec![0.0, 60.0, 300.0],
            columns: vec![0.0, 300.0],
        },
    )]);
    assert!(LayoutRegistry::built_in()
        .plan(
            &LayoutContext::new(&document, &bounds).with_grid_lines(&lines),
            action
        )
        .is_ok());
}
#[test]
fn drag_drop_preserves_inherited_namespaces_and_dock_fill_child() {
    let mut document=Document::new("<DockPanel xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Button Name='fill'/></DockPanel>").unwrap();
    let root = document.ast().root();
    let bounds = BTreeMap::from([(root, Rect::new(0.0, 0.0, 300.0, 200.0))]);
    apply(
        &mut document,
        &bounds,
        LayoutAction::Drop {
            parent: root,
            source: "<!--asset--><Button x:Name='tool'/>".into(),
            position: Point::new(1.0, 80.0),
        },
    );
    assert_eq!(
        document.ast().element_children(root).last(),
        Some(&id(&document, "fill"))
    );
    assert_eq!(
        document
            .ast()
            .node(id(&document, "tool"))
            .unwrap()
            .attribute("DockPanel.Dock"),
        Some("Left")
    );
    assert!(document.source().contains("<!--asset-->"));
}
#[test]
fn relative_constraints_reject_cycles_and_cross_panel_references() {
    let mut document = Document::new(
        "<RelativePanel><Button Name='a'/><Button Name='b'/><Button Name='c'/></RelativePanel>",
    )
    .unwrap();
    let a = id(&document, "a");
    let b = id(&document, "b");
    let c = id(&document, "c");
    apply(
        &mut document,
        &BTreeMap::new(),
        LayoutAction::Relative {
            node: b,
            relation: RelativeRelation::RightOf,
            target: Some(a),
        },
    );
    apply(
        &mut document,
        &BTreeMap::new(),
        LayoutAction::Relative {
            node: c,
            relation: RelativeRelation::Below,
            target: Some(b),
        },
    );
    assert!(LayoutRegistry::built_in()
        .plan(
            &LayoutContext::new(&document, &BTreeMap::new()),
            LayoutAction::Relative {
                node: a,
                relation: RelativeRelation::LeftOf,
                target: Some(c)
            }
        )
        .is_err());
    assert!(LayoutRegistry::built_in()
        .plan(
            &LayoutContext::new(&document, &BTreeMap::new()),
            LayoutAction::Relative {
                node: a,
                relation: RelativeRelation::Below,
                target: Some(a)
            }
        )
        .is_err());
}
#[test]
fn all_panel_families_expose_capabilities_and_generated_containers_stay_owned() {
    let registry = LayoutRegistry::built_in();
    assert_eq!(registry.panel_types().count(), 30);
    for panel in [
        "VirtualizingStackPanel",
        "VirtualizingCarouselPanel",
        "VirtualizingPanel",
        "DateTimePickerPanel",
        "TextSelectionHandleCanvas",
        "TableViewColumnHeadersPresenter",
        "TableViewCellsPresenter",
        "AdornerLayer",
        "OverlayLayer",
        "PopupOverlayLayer",
        "TextSelectorLayer",
    ] {
        let adapter = registry.adapter(panel).unwrap();
        assert!(adapter.capabilities().generated_items);
        assert!(!adapter.capabilities().drop_items);
        let document = Document::new(format!("<{panel}/>")).unwrap();
        assert!(registry
            .plan(
                &LayoutContext::new(&document, &BTreeMap::new()),
                LayoutAction::Drop {
                    parent: document.ast().root(),
                    source: "<Button/>".into(),
                    position: Point::default()
                }
            )
            .is_err());
    }
}
#[test]
fn property_metadata_validates_multi_edit_ranges_and_expressions() {
    let document=Document::new("<StackPanel><Button Name='a' Opacity='.5'/><Button Name='b' Opacity='.8' Width='{Binding W}'/></StackPanel>").unwrap();
    let a = id(&document, "a");
    let b = id(&document, "b");
    let registry = PropertyRegistry::built_in();
    assert!(registry.state(&document, &[a, b], "Opacity").unwrap().mixed);
    assert!(registry
        .plan(
            &document,
            &[a, b],
            "Opacity",
            Some("2"),
            ExpressionPolicy::PreserveExpressions
        )
        .is_err());
    assert!(registry
        .plan(
            &document,
            &[b],
            "Width",
            Some("200"),
            ExpressionPolicy::PreserveExpressions
        )
        .is_err());
    assert!(registry
        .plan(
            &document,
            &[b],
            "Width",
            Some("200"),
            ExpressionPolicy::ReplaceExpressions
        )
        .is_ok());
    assert!(registry
        .plan(
            &document,
            &[a, b],
            "Opacity",
            Some("0.7"),
            ExpressionPolicy::PreserveExpressions
        )
        .is_ok());
}
#[test]
fn geometry_snaps_aligns_and_distributes_with_finite_guards() {
    let snap = SnapSettings {
        grid: None,
        tolerance: 4.0,
        guides_x: vec![100.0],
        guides_y: vec![],
    };
    let result = snap.snap(Rect::new(96.0, 5.0, 20.0, 10.0), &[]).unwrap();
    assert_eq!(result.bounds.x, 100.0);
    let rects = [
        (1, Rect::new(0.0, 0.0, 10.0, 10.0)),
        (2, Rect::new(20.0, 8.0, 20.0, 10.0)),
        (3, Rect::new(100.0, 20.0, 10.0, 10.0)),
    ];
    assert_eq!(align(&rects, Alignment::Left).unwrap()[&3].x, -100.0);
    assert_eq!(distribute(&rects, Axis::Horizontal).unwrap()[&2].x, 25.0);
    assert!(SnapSettings {
        grid: Some(0.0),
        ..Default::default()
    }
    .snap(rects[0].1, &[])
    .is_err());
    let transform = Affine::rotation(90.0, Point::new(10.0, 10.0)).unwrap();
    let p = Point::new(20.0, 10.0);
    let rotated = transform.transform(p).unwrap();
    assert!((rotated.x - 10.0).abs() < 1e-10);
    assert!((rotated.y - 20.0).abs() < 1e-10);
    let restored = transform.inverse().unwrap().transform(rotated).unwrap();
    assert!((restored.x - p.x).abs() < 1e-10);
    assert!(Affine::scale(0.0, 1.0, Point::default())
        .unwrap()
        .inverse()
        .is_err());
}
#[test]
fn nested_selection_moves_once_and_stale_gestures_do_not_commit() {
    let mut document=Document::new("<Canvas><Canvas Name='group' Canvas.Left='0' Canvas.Top='0'><Button Name='child'/></Canvas></Canvas>").unwrap();
    let root = document.ast().root();
    let group = id(&document, "group");
    let child = id(&document, "child");
    assert_eq!(
        outermost(&document, &[group, child, group]).unwrap(),
        vec![group]
    );
    let bounds = BTreeMap::from([
        (root, Rect::new(0.0, 0.0, 500.0, 500.0)),
        (group, Rect::new(0.0, 0.0, 100.0, 100.0)),
        (child, Rect::new(10.0, 10.0, 20.0, 20.0)),
    ]);
    let gesture = MoveGesture::begin(
        &document,
        &[group, child],
        &bounds,
        Point::default(),
        SnapSettings {
            grid: None,
            tolerance: 0.0,
            guides_x: vec![],
            guides_y: vec![],
        },
    )
    .unwrap();
    let edits = gesture
        .commit(
            &LayoutContext::new(&document, &bounds),
            &LayoutRegistry::built_in(),
            Point::new(15.0, 20.0),
        )
        .unwrap();
    assert_eq!(edits.len(), 2);
    document
        .apply(
            "Concurrent",
            vec![Edit::SetAttribute {
                node: root,
                name: "Tag".into(),
                value: "changed".into(),
            }],
        )
        .unwrap();
    assert!(gesture
        .commit(
            &LayoutContext::new(&document, &bounds),
            &LayoutRegistry::built_in(),
            Point::new(15.0, 20.0)
        )
        .is_err());
}
#[test]
fn selection_hit_testing_respects_paint_order_lock_and_marquee() {
    let document = Document::new("<Canvas><Button Name='a'/><Button Name='b'/></Canvas>").unwrap();
    let a = id(&document, "a");
    let b = id(&document, "b");
    let paint = [
        (a, Rect::new(0.0, 0.0, 20.0, 20.0)),
        (b, Rect::new(0.0, 0.0, 20.0, 20.0)),
    ];
    let mut selection = Selection::default();
    assert_eq!(
        selection
            .hit_test(&document, Point::new(10.0, 10.0), &paint)
            .unwrap(),
        Some(b)
    );
    selection.set_locked(b, true);
    assert_eq!(
        selection
            .hit_test(&document, Point::new(10.0, 10.0), &paint)
            .unwrap(),
        Some(a)
    );
    selection
        .marquee(
            &document,
            Rect::new(0.0, 0.0, 20.0, 20.0),
            &paint,
            true,
            SelectionMode::Replace,
        )
        .unwrap();
    assert_eq!(selection.nodes(), vec![a]);
}
#[test]
fn uniform_and_reversed_flow_drops_use_the_visual_slot() {
    let mut document=Document::new("<UniformGrid Rows='2' Columns='2'><Button Name='a'/><Button Name='b'/><Button Name='c'/></UniformGrid>").unwrap();
    let root = document.ast().root();
    let bounds = BTreeMap::from([(root, Rect::new(0.0, 0.0, 200.0, 200.0))]);
    apply(
        &mut document,
        &bounds,
        LayoutAction::Drop {
            parent: root,
            source: "<Button Name='new'/>".into(),
            position: Point::new(150.0, 20.0),
        },
    );
    assert_eq!(
        document.ast().element_children(root)[1],
        id(&document, "new")
    );
    let mut document=Document::new("<ReversibleStackPanel ReverseOrder='True'><Button Name='a'/><Button Name='b'/></ReversibleStackPanel>").unwrap();
    let root = document.ast().root();
    let a = id(&document, "a");
    let b = id(&document, "b");
    let bounds = BTreeMap::from([
        (root, Rect::new(0.0, 0.0, 100.0, 200.0)),
        (a, Rect::new(0.0, 100.0, 100.0, 100.0)),
        (b, Rect::new(0.0, 0.0, 100.0, 100.0)),
    ]);
    apply(
        &mut document,
        &bounds,
        LayoutAction::Drop {
            parent: root,
            source: "<Button Name='new'/>".into(),
            position: Point::new(50.0, 180.0),
        },
    );
    assert_eq!(
        document.ast().element_children(root)[0],
        id(&document, "new")
    );
}
#[test]
fn custom_property_editors_and_layout_adapters_are_replaceable() {
    struct TokenEditor;
    impl ValueEditor for TokenEditor {
        fn id(&self) -> &str {
            "token"
        }
        fn supports(&self, p: &PropertyDescriptor) -> bool {
            p.name == "Token"
        }
        fn validate(&self, _: &PropertyDescriptor, v: &str) -> Result<()> {
            if v.starts_with("app:") {
                Ok(())
            } else {
                Err(ToolError("Use an app token".into()))
            }
        }
    }
    let mut properties = PropertyRegistry::built_in();
    properties.register(
        "Button",
        PropertyDescriptor::new("Token", "Custom", PropertyKind::Text),
    );
    let mut editors = EditorRegistry::built_in();
    editors.register(Box::new(TokenEditor));
    let document = Document::new("<Button/>").unwrap();
    let root = document.ast().root();
    assert!(properties
        .plan_with(
            &document,
            &[root],
            "Token",
            Some("invalid"),
            ExpressionPolicy::ReplaceExpressions,
            &editors
        )
        .is_err());
    assert!(properties
        .plan_with(
            &document,
            &[root],
            "Token",
            Some("app:ok"),
            ExpressionPolicy::ReplaceExpressions,
            &editors
        )
        .is_ok());
    struct CustomPanel;
    impl LayoutAdapter for CustomPanel {
        fn panel_type(&self) -> &str {
            "CustomPanel"
        }
        fn capabilities(&self) -> LayoutCapabilities {
            LayoutCapabilities {
                move_items: true,
                resize_items: false,
                reorder: false,
                drop_items: false,
                generated_items: false,
            }
        }
        fn panel_properties(&self) -> &[&str] {
            &[]
        }
        fn plan(&self, _: &LayoutContext<'_>, action: &LayoutAction) -> Result<Vec<Edit>> {
            if let LayoutAction::Move { node, delta } = action {
                Ok(vec![Edit::SetAttribute {
                    node: *node,
                    name: "Custom.X".into(),
                    value: delta.x.to_string(),
                }])
            } else {
                Err(ToolError("Unsupported custom action".into()))
            }
        }
    }
    let mut registry = LayoutRegistry::built_in();
    registry.register(Box::new(CustomPanel));
    let document = Document::new("<CustomPanel><Button Name='a'/></CustomPanel>").unwrap();
    assert!(registry
        .plan(
            &LayoutContext::new(&document, &BTreeMap::new()),
            LayoutAction::Move {
                node: id(&document, "a"),
                delta: Point::new(5.0, 0.0)
            }
        )
        .is_ok());
}
#[test]
fn empty_content_hosts_accept_one_drop_and_preserve_explicit_content_properties() {
    for host in [
        "Border",
        "Decorator",
        "ContentControl",
        "UserControl",
        "Window",
        "ScrollViewer",
        "Viewbox",
        "Button",
        "ToggleButton",
    ] {
        let mut document = Document::new(format!("<{host}/>")).unwrap();
        let root = document.ast().root();
        let action = LayoutAction::Drop {
            parent: root,
            source: "<Grid Name='content'/>".into(),
            position: Point::default(),
        };
        apply(&mut document, &BTreeMap::new(), action.clone());
        assert!(LayoutRegistry::built_in()
            .plan(&LayoutContext::new(&document, &BTreeMap::new()), action)
            .is_err());
    }
    let mut document = Document::new("<Border><Border.Child/></Border>").unwrap();
    let root = document.ast().root();
    apply(
        &mut document,
        &BTreeMap::new(),
        LayoutAction::Drop {
            parent: root,
            source: "<Grid Name='content'/>".into(),
            position: Point::default(),
        },
    );
    assert!(document.source().contains("<Border.Child><Grid"));
    for source in [
        "<Button Content='Save'/>",
        "<Button>Save</Button>",
        "<Border Child='{Binding Child}'/>",
    ] {
        let document = Document::new(source).unwrap();
        assert!(LayoutRegistry::built_in()
            .plan(
                &LayoutContext::new(&document, &BTreeMap::new()),
                LayoutAction::Drop {
                    parent: document.ast().root(),
                    source: "<Grid/>".into(),
                    position: Point::default()
                }
            )
            .is_err());
    }
}

#[test]
fn canvas_resize_keeps_leading_edges_when_normalizing_margin_and_opposite_anchors() {
    let mut document = Document::new(
        "<Canvas><Button Name='a' Canvas.Right='10' Canvas.Bottom='15' Margin='5,7,3,2' Width='20' Height='30'/></Canvas>",
    )
    .unwrap();
    let node = id(&document, "a");
    let root = document.ast().root();
    let before = Rect::new(167.0, 103.0, 20.0, 30.0);
    let bounds = BTreeMap::from([(root, Rect::new(0.0, 0.0, 200.0, 150.0)), (node, before)]);
    apply(
        &mut document,
        &bounds,
        LayoutAction::Resize {
            node,
            before,
            after: Rect::new(before.x, before.y, 40.0, 50.0),
        },
    );
    let element = document.ast().node(node).unwrap();
    assert_eq!(element.attribute("Canvas.Left"), Some("162"));
    assert_eq!(element.attribute("Canvas.Top"), Some("96"));
    assert_eq!(element.attribute("Canvas.Right"), None);
    assert_eq!(element.attribute("Canvas.Bottom"), None);
    assert_eq!(element.attribute("Margin"), Some("5,7,3,2"));
    assert_eq!(element.attribute("Width"), Some("40"));
    assert_eq!(element.attribute("Height"), Some("50"));
}

#[test]
fn live_layout_preserves_styled_margins_and_exact_arrange_slots() {
    let mut document = Document::new("<Canvas><Button Name='a'/></Canvas>").unwrap();
    let root = document.ast().root();
    let node = id(&document, "a");
    let bounds = BTreeMap::from([
        (root, Rect::new(100.0, 200.0, 300.0, 300.0)),
        (node, Rect::new(122.0, 224.0, 20.0, 30.0)),
    ]);
    let layout = BTreeMap::from([(
        node,
        ResolvedLayout {
            margin: [3.0, 4.0, 5.0, 6.0],
            slot: Some(Rect::new(110.0, 212.0, 40.0, 60.0)),
            ..ResolvedLayout::default()
        },
    )]);
    let registry = LayoutRegistry::built_in();
    let edits = registry
        .plan(
            &LayoutContext::new(&document, &bounds).with_resolved_layout(&layout),
            LayoutAction::Move {
                node,
                delta: Point::new(7.0, 8.0),
            },
        )
        .unwrap();
    document.apply("Move", edits).unwrap();
    let element = document.ast().node(node).unwrap();
    assert_eq!(element.attribute("Canvas.Left"), Some("17"));
    assert_eq!(element.attribute("Canvas.Top"), Some("20"));
    assert!(element.attribute("Margin").is_none());

    let mut document = Document::new("<Grid><Button Name='a'/></Grid>").unwrap();
    let node = id(&document, "a");
    let layout = BTreeMap::from([(
        node,
        ResolvedLayout {
            margin: [3.0, 4.0, 5.0, 6.0],
            ..ResolvedLayout::default()
        },
    )]);
    let edits = registry
        .plan(
            &LayoutContext::new(&document, &BTreeMap::new()).with_resolved_layout(&layout),
            LayoutAction::Move {
                node,
                delta: Point::new(7.0, 8.0),
            },
        )
        .unwrap();
    document.apply("Move", edits).unwrap();
    assert_eq!(
        document.ast().node(node).unwrap().attribute("Margin"),
        Some("10,12,-2,-2")
    );
    let invalid = BTreeMap::from([(
        node,
        ResolvedLayout {
            margin: [f64::NAN, 0.0, 0.0, 0.0],
            ..ResolvedLayout::default()
        },
    )]);
    assert!(registry
        .plan(
            &LayoutContext::new(&document, &BTreeMap::new()).with_resolved_layout(&invalid),
            LayoutAction::Move {
                node,
                delta: Point::new(1.0, 0.0)
            },
        )
        .is_err());
}

#[test]
fn resize_preserves_dragged_rectangle_for_start_center_stretch_and_end_alignment() {
    for (horizontal, hx) in [
        ("Left", 0.0),
        ("Center", 0.5),
        ("Stretch", 0.5),
        ("Right", 1.0),
    ] {
        for (vertical, vy) in [
            ("Top", 0.0),
            ("Center", 0.5),
            ("Stretch", 0.5),
            ("Bottom", 1.0),
        ] {
            let mut document = Document::new(format!(
                "<Grid><!--retain--><Button Name='a' Margin='1,2,3,4' Width='20' Height='30' HorizontalAlignment='{horizontal}' VerticalAlignment='{vertical}'/></Grid>"
            )).unwrap();
            let original = document.source().to_owned();
            let node = id(&document, "a");
            let before = Rect::new(1.0 + hx * 76.0, 2.0 + vy * 44.0, 20.0, 30.0);
            let after = Rect::new(before.x + 5.0, before.y + 7.0, 40.0, 15.0);
            apply(
                &mut document,
                &BTreeMap::new(),
                LayoutAction::Resize {
                    node,
                    before,
                    after,
                },
            );
            let element = document.ast().node(node).unwrap();
            let margin = crate::layout::thickness(element.attribute("Margin").unwrap()).unwrap();
            let width: f64 = element.attribute("Width").unwrap().parse().unwrap();
            let height: f64 = element.attribute("Height").unwrap().parse().unwrap();
            // Evaluate the runtime's layout contract against the unchanged 100 x 80 slot.
            let x = margin[0] + hx * (100.0 - margin[0] - margin[2] - width);
            let y = margin[1] + vy * (80.0 - margin[1] - margin[3] - height);
            assert_eq!(Rect::new(x, y, width, height), after);
            assert_eq!(width + margin[0] + margin[2], 24.0);
            assert_eq!(height + margin[1] + margin[3], 36.0);
            assert!(document.source().contains("<!--retain-->"));
            document.undo().unwrap();
            assert_eq!(document.source(), original);
        }
    }
}

#[test]
fn attached_property_elements_are_never_overwritten_by_layout_gestures() {
    for (panel, property) in [
        ("Canvas", "Canvas.Left"),
        ("Grid", "Grid.Row"),
        ("DockPanel", "DockPanel.Dock"),
    ] {
        let document = Document::new(format!(
            "<{panel}><Button Name='a'><{property}>0</{property}></Button></{panel}>"
        ))
        .unwrap();
        let node = id(&document, "a");
        let action = match panel {
            "Canvas" => LayoutAction::Move {
                node,
                delta: Point::new(1.0, 1.0),
            },
            "Grid" => LayoutAction::GridCell {
                node,
                row: 0,
                column: 0,
                row_span: 1,
                column_span: 1,
            },
            _ => LayoutAction::Dock {
                node,
                side: DockSide::Right,
            },
        };
        let result = LayoutRegistry::built_in()
            .plan(&LayoutContext::new(&document, &BTreeMap::new()), action);
        assert!(result.unwrap_err().to_string().contains("property element"));
        assert!(PropertyRegistry::built_in()
            .plan(
                &document,
                &[node],
                property,
                Some("1"),
                ExpressionPolicy::ReplaceExpressions
            )
            .unwrap_err()
            .to_string()
            .contains("property element"));
    }
}

#[test]
fn uniform_drop_matches_runtime_square_auto_spacing_first_column_and_visibility() {
    for (source, position, expected_index) in [
        ("<UniformGrid><Button Name='a'/><Button Name='b'/><Button Name='c'/><Button Name='d'/></UniformGrid>", Point::new(10.0, 120.0), 3),
        ("<UniformGrid Rows='2' Columns='2' FirstColumn='4'><Button Name='a'/><Button Name='b'/></UniformGrid>", Point::new(180.0, 10.0), 1),
        ("<UniformGrid Rows='2' Columns='2' ColumnSpacing='100'><Button Name='a'/><Button Name='b'/></UniformGrid>", Point::new(180.0, 10.0), 0),
        ("<UniformGrid Rows='2' Columns='2'><Button Name='hidden' IsVisible='False'/><Button Name='a'/><Button Name='b'/></UniformGrid>", Point::new(10.0, 180.0), 3),
    ] {
        let mut document = Document::new(source).unwrap();
        let root = document.ast().root();
        let bounds = BTreeMap::from([(root, Rect::new(0.0, 0.0, 300.0, 300.0))]);
        apply(&mut document, &bounds, LayoutAction::Drop {
            parent: root, source: "<Button Name='new'/>".into(), position,
        });
        assert_eq!(document.ast().element_children(root)[expected_index], id(&document, "new"), "{source}");
    }
    let mut document = Document::new("<UniformGrid Rows='2' Columns='2'><Button Name='hidden' IsVisible='{Binding Hidden}'/><Button Name='a'/><Button Name='b'/></UniformGrid>").unwrap();
    let root = document.ast().root();
    let hidden = id(&document, "hidden");
    let bounds = BTreeMap::from([(root, Rect::new(0.0, 0.0, 300.0, 300.0))]);
    let layout = BTreeMap::from([(
        hidden,
        ResolvedLayout {
            visible: false,
            ..ResolvedLayout::default()
        },
    )]);
    let action = LayoutAction::Drop {
        parent: root,
        source: "<Button Name='new'/>".into(),
        position: Point::new(10.0, 180.0),
    };
    let registry = LayoutRegistry::built_in();
    assert!(registry
        .plan(&LayoutContext::new(&document, &bounds), action.clone())
        .is_err());
    let edits = registry
        .plan(
            &LayoutContext::new(&document, &bounds).with_resolved_layout(&layout),
            action,
        )
        .unwrap();
    document.apply("Drop", edits).unwrap();
    assert_eq!(
        document.ast().element_children(root)[3],
        id(&document, "new")
    );
}

#[test]
fn batch_plans_validate_one_atomic_source_transaction_across_coordinate_spaces() {
    struct NamespacePanel;
    impl LayoutAdapter for NamespacePanel {
        fn panel_type(&self) -> &str {
            "NamespacePanel"
        }
        fn capabilities(&self) -> LayoutCapabilities {
            LayoutCapabilities {
                move_items: true,
                resize_items: false,
                reorder: false,
                drop_items: false,
                generated_items: false,
            }
        }
        fn panel_properties(&self) -> &[&str] {
            &[]
        }
        fn plan(&self, context: &LayoutContext<'_>, action: &LayoutAction) -> Result<Vec<Edit>> {
            let LayoutAction::Move { node, delta } = action else {
                unreachable!()
            };
            Ok(if delta.x == 0.0 {
                vec![Edit::SetAttribute {
                    node: context.document.ast().root(),
                    name: "xmlns:layout".into(),
                    value: "urn:custom".into(),
                }]
            } else {
                vec![Edit::SetAttribute {
                    node: *node,
                    name: "layout:Offset".into(),
                    value: context.bounds(*node)?.x.to_string(),
                }]
            })
        }
    }
    let mut document =
        Document::new("<NamespacePanel><Button Name='a'/><Button Name='b'/></NamespacePanel>")
            .unwrap();
    let a = id(&document, "a");
    let b = id(&document, "b");
    let first_bounds = BTreeMap::from([(a, Rect::new(12.0, 0.0, 10.0, 10.0))]);
    let second_bounds = BTreeMap::new();
    let first = LayoutContext::new(&document, &first_bounds);
    let second = LayoutContext::new(&document, &second_bounds);
    let mut registry = LayoutRegistry::built_in();
    registry.register(Box::new(NamespacePanel));
    let offset = LayoutAction::Move {
        node: a,
        delta: Point::new(1.0, 0.0),
    };
    let namespace = LayoutAction::Move {
        node: b,
        delta: Point::default(),
    };
    assert!(registry.plan(&first, offset.clone()).is_err());
    let edits = registry
        .plan_batch([(&first, offset.clone()), (&second, namespace.clone())])
        .unwrap();
    let other = document.clone();
    let other_context = LayoutContext::new(&other, &second_bounds);
    assert!(registry
        .plan_batch([(&first, offset), (&other_context, namespace)])
        .is_err());
    document.apply("Atomic custom layout", edits).unwrap();
    assert_eq!(
        document.ast().node(a).unwrap().attribute("layout:Offset"),
        Some("12")
    );
}

#[test]
fn large_multiselection_retains_source_and_one_undo_boundary() {
    let mut source = "<Canvas><!--preserve whitespace and quoting-->\n".to_owned();
    for i in 0..256 {
        source.push_str(&format!("  <Button Name='item{i}' Canvas.Left='{i}' Canvas.Top='0' Content='text &amp; trivia'/>\n"));
    }
    source.push_str("</Canvas>");
    let mut document = Document::new(source.clone()).unwrap();
    let selection = document.ast().element_children(document.ast().root());
    let edits = LayoutRegistry::built_in()
        .move_selection(
            &LayoutContext::new(&document, &BTreeMap::new()),
            &selection,
            Point::new(5.0, 10.0),
        )
        .unwrap();
    document.apply("Move 256 controls", edits).unwrap();
    assert_eq!(document.revision(), 1);
    let last = document.ast().node(id(&document, "item255")).unwrap();
    assert_eq!(last.attribute("Canvas.Left"), Some("260"));
    assert_eq!(last.attribute("Canvas.Top"), Some("10"));
    assert!(document.source().contains("Content='text &amp; trivia'"));
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}

#[test]
fn batch_rejects_combined_relative_cycles_conflicting_tracks_and_multiple_content() {
    let registry = LayoutRegistry::built_in();
    let document =
        Document::new("<RelativePanel><Button Name='a'/><Button Name='b'/></RelativePanel>")
            .unwrap();
    let a = id(&document, "a");
    let b = id(&document, "b");
    let bounds = BTreeMap::new();
    let context = LayoutContext::new(&document, &bounds);
    let first = LayoutAction::Relative {
        node: a,
        relation: RelativeRelation::RightOf,
        target: Some(b),
    };
    let second = LayoutAction::Relative {
        node: b,
        relation: RelativeRelation::RightOf,
        target: Some(a),
    };
    assert!(registry.plan(&context, first.clone()).is_ok());
    assert!(registry.plan(&context, second.clone()).is_ok());
    assert!(registry
        .plan_many(&context, [first, second])
        .unwrap_err()
        .to_string()
        .contains("cycle"));

    let document = Document::new("<Grid RowDefinitions='*,*'/>").unwrap();
    let context = LayoutContext::new(&document, &bounds);
    let tracks = [0, 1].map(|index| LayoutAction::GridTrack {
        panel: document.ast().root(),
        axis: Axis::Vertical,
        index,
        length: GridLength::Pixels(40.0),
    });
    assert!(registry
        .plan_many(&context, tracks)
        .unwrap_err()
        .to_string()
        .contains("conflicting"));

    let document = Document::new("<Border/>").unwrap();
    let context = LayoutContext::new(&document, &bounds);
    let drops = ["<Button Name='a'/>", "<Button Name='b'/>"].map(|source| LayoutAction::Drop {
        parent: document.ast().root(),
        source: source.into(),
        position: Point::default(),
    });
    assert!(registry
        .plan_many(&context, drops)
        .unwrap_err()
        .to_string()
        .contains("multiple children"));
    assert_eq!(document.source(), "<Border/>");
}

#[test]
fn grid_cell_spans_use_effective_tracks_from_the_host() {
    let mut document = Document::new("<Grid><Button Name='a'/></Grid>").unwrap();
    let root = document.ast().root();
    let node = id(&document, "a");
    let bounds = BTreeMap::new();
    let lines = BTreeMap::from([(
        root,
        GridLines {
            rows: vec![0.0, 40.0, 100.0],
            columns: vec![0.0, 20.0, 50.0, 100.0],
        },
    )]);
    let registry = LayoutRegistry::built_in();
    let action = LayoutAction::GridCell {
        node,
        row: 1,
        column: 1,
        row_span: 1,
        column_span: 2,
    };
    assert!(registry
        .plan(&LayoutContext::new(&document, &bounds), action.clone())
        .is_err());
    let edits = registry
        .plan(
            &LayoutContext::new(&document, &bounds).with_grid_lines(&lines),
            action,
        )
        .unwrap();
    document.apply("Position in styled Grid", edits).unwrap();
    let element = document.ast().node(node).unwrap();
    assert_eq!(element.attribute("Grid.Row"), Some("1"));
    assert_eq!(element.attribute("Grid.ColumnSpan"), Some("2"));
    assert!(document
        .ast()
        .node(root)
        .unwrap()
        .attribute("RowDefinitions")
        .is_none());
}
