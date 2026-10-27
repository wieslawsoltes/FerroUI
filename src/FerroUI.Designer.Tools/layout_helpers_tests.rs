use super::*;
use crate::{LayoutAdapter, LayoutCapabilities, LayoutRegistry};
use ferroui_designer_core::Document;

fn request(panel: NodeId, pointer: Point) -> LayoutHelperRequest {
    let mut request = LayoutHelperRequest::new(panel);
    request.pointer = Some(pointer);
    request
}
fn node(document: &Document, name: &str) -> NodeId {
    document.ast().find_name(name).unwrap()
}
fn inserted(edits: &[Edit]) -> (NodeId, usize, &str) {
    edits
        .iter()
        .find_map(|edit| {
            if let Edit::InsertChild {
                parent,
                index,
                source,
            } = edit
            {
                Some((*parent, *index, source.as_str()))
            } else {
                None
            }
        })
        .unwrap()
}
#[test]
fn stack_insertion_preview_uses_the_authoring_boundary_and_preserves_source() {
    let document=Document::new("<StackPanel Name='stack' Orientation='Horizontal' Spacing='12'><StackPanel.Children><Button Name='a'/><Button Name='b'/></StackPanel.Children></StackPanel>").unwrap();
    let panel = document.ast().root();
    let a = node(&document, "a");
    let b = node(&document, "b");
    let bounds = BTreeMap::from([
        (panel, Rect::new(0.0, 0.0, 220.0, 80.0)),
        (a, Rect::new(0.0, 0.0, 80.0, 60.0)),
        (b, Rect::new(92.0, 0.0, 80.0, 60.0)),
    ]);
    let context = LayoutContext::new(&document, &bounds);
    let registry = LayoutRegistry::built_in();
    let before = document.source().to_owned();
    let scene = registry
        .design_helpers(&context, &request(panel, Point::new(87.0, 25.0)))
        .unwrap();
    let placement = scene.placement.unwrap();
    let edits = registry
        .plan(
            &context,
            LayoutAction::Drop {
                parent: panel,
                source: "<Button Name='new'/>".into(),
                position: Point::new(87.0, 25.0),
            },
        )
        .unwrap();
    let (owner, index, _) = inserted(&edits);
    assert_eq!((placement.container, placement.index), (owner, index));
    assert_ne!(
        owner, panel,
        "the explicit Children property remains the AST owner"
    );
    assert!(
        scene
            .primitives
            .iter()
            .any(|guide| guide.role == LayoutGuideRole::Spacing)
    );
    let spacing = scene
        .handles
        .iter()
        .find(|handle| handle.id == "Spacing")
        .unwrap();
    assert_eq!(spacing.value, 12.0);
    assert_eq!(
        spacing.value_after_delta(Point::new(5.0, 90.0)).unwrap(),
        17.0
    );
    assert_eq!(document.source(), before, "hover never authors source");
}
#[test]
fn flow_helpers_share_reversal_wrap_and_flex_order_with_drop_planners() {
    for (tag, attributes) in [
        (
            "ReversibleStackPanel",
            "Orientation='Horizontal' ReverseOrder='True'",
        ),
        (
            "WrapPanel",
            "Orientation='Horizontal' ItemSpacing='8' LineSpacing='16'",
        ),
        (
            "FlexPanel",
            "Direction='RowReverse' RowSpacing='16' ColumnSpacing='8'",
        ),
    ] {
        let document = Document::new(format!(
            "<{tag} {attributes}><Button Name='a'/><Button Name='b'/><Button Name='c'/></{tag}>"
        ))
        .unwrap();
        let panel = document.ast().root();
        let bounds = BTreeMap::from([
            (panel, Rect::new(0.0, 0.0, 200.0, 140.0)),
            (node(&document, "a"), Rect::new(100.0, 0.0, 80.0, 50.0)),
            (node(&document, "b"), Rect::new(12.0, 0.0, 80.0, 50.0)),
            (node(&document, "c"), Rect::new(12.0, 66.0, 80.0, 50.0)),
        ]);
        let context = LayoutContext::new(&document, &bounds);
        let registry = LayoutRegistry::built_in();
        for position in [Point::new(105.0, 25.0), Point::new(80.0, 90.0)] {
            let proposal = registry
                .placement(&context, panel, position, &[])
                .unwrap()
                .unwrap();
            let edits = registry
                .plan(
                    &context,
                    LayoutAction::Drop {
                        parent: panel,
                        source: "<Button/>".into(),
                        position,
                    },
                )
                .unwrap();
            let (owner, index, _) = inserted(&edits);
            assert_eq!(
                (proposal.container, proposal.index),
                (owner, index),
                "{tag}"
            );
        }
        let scene = registry
            .design_helpers(&context, &LayoutHelperRequest::new(panel))
            .unwrap();
        assert!(
            scene
                .primitives
                .iter()
                .any(|guide| guide.role == LayoutGuideRole::Flow)
        );
        if tag == "WrapPanel" {
            assert!(
                scene
                    .handles
                    .iter()
                    .any(|handle| handle.id == "LineSpacing")
            );
        }
    }
}
#[test]
fn grid_helpers_use_measured_tracks_and_preserve_star_units_and_definition_metadata() {
    let mut document=Document::new("<Grid><Grid.RowDefinitions><!-- retained --><RowDefinition Height='2*' MinHeight='20'/><RowDefinition Height='Auto' SharedSizeGroup='header'/></Grid.RowDefinitions><Grid.ColumnDefinitions><ColumnDefinition Width='{Binding Width}'/></Grid.ColumnDefinitions></Grid>").unwrap();
    let panel = document.ast().root();
    let bounds = BTreeMap::from([(panel, Rect::new(0.0, 0.0, 300.0, 160.0))]);
    let measured = BTreeMap::from([(
        panel,
        crate::GridLines {
            rows: vec![0.0, 100.0, 160.0],
            columns: vec![0.0, 300.0],
        },
    )]);
    let registry = LayoutRegistry::built_in();
    let scene = registry
        .design_helpers(
            &LayoutContext::new(&document, &bounds).with_grid_lines(&measured),
            &LayoutHelperRequest::new(panel),
        )
        .unwrap();
    let first = scene
        .handles
        .iter()
        .find(|handle| handle.id == "R0")
        .unwrap();
    assert_eq!(first.value, 100.0);
    let edits = registry
        .plan_helper(
            &LayoutContext::new(&document, &bounds).with_grid_lines(&measured),
            panel,
            &first.action,
            150.0,
        )
        .unwrap();
    document.apply("Resize star track", edits).unwrap();
    assert!(document.source().contains("Height='3*'"));
    assert!(document.source().contains("MinHeight='20'"));
    assert!(document.source().contains("<!-- retained -->"));
    let auto = scene
        .handles
        .iter()
        .find(|handle| handle.id == "R1")
        .unwrap();
    let edits = registry
        .plan_helper(
            &LayoutContext::new(&document, &bounds).with_grid_lines(&measured),
            panel,
            &auto.action,
            72.5,
        )
        .unwrap();
    document.apply("Resize Auto track", edits).unwrap();
    assert!(document.source().contains("Height='72.5'"));
    assert!(document.source().contains("SharedSizeGroup='header'"));
    let bound = scene
        .handles
        .iter()
        .find(|handle| handle.id == "C0")
        .unwrap();
    assert!(bound.disabled_reason.is_some());
    assert!(
        registry
            .plan_helper(
                &LayoutContext::new(&document, &bounds).with_grid_lines(&measured),
                panel,
                &bound.action,
                280.0
            )
            .is_err()
    );
}
#[test]
fn single_content_rejection_and_excluded_self_are_explicit() {
    for source in [
        "<Border><Button Name='old'/></Border>",
        "<ContentControl Content='title'/>",
        "<Button>Save</Button>",
    ] {
        let document = Document::new(source).unwrap();
        let panel = document.ast().root();
        let bounds = BTreeMap::from([(panel, Rect::new(0.0, 0.0, 100.0, 80.0))]);
        let context = LayoutContext::new(&document, &bounds);
        let registry = LayoutRegistry::built_in();
        let placement = registry
            .placement(&context, panel, Point::new(10.0, 10.0), &[])
            .unwrap()
            .unwrap();
        assert!(placement.rejection.is_some());
        let scene = registry
            .design_helpers(&context, &request(panel, Point::new(10.0, 10.0)))
            .unwrap();
        assert!(scene.placement.unwrap().rejection.is_some());
        if let Some(old) = document.ast().find_name("old") {
            assert!(
                registry
                    .placement(&context, panel, Point::new(10.0, 10.0), &[old])
                    .unwrap()
                    .unwrap()
                    .rejection
                    .is_none()
            );
        }
    }
}
#[test]
fn generated_layouts_are_visible_but_never_offer_placement_or_edit_handles() {
    let document = Document::new("<VirtualizingStackPanel Orientation='Vertical'/>").unwrap();
    let panel = document.ast().root();
    let bounds = BTreeMap::from([(panel, Rect::new(0.0, 0.0, 100.0, 200.0))]);
    let context = LayoutContext::new(&document, &bounds);
    let registry = LayoutRegistry::built_in();
    let scene = registry
        .design_helpers(&context, &request(panel, Point::new(10.0, 10.0)))
        .unwrap();
    assert!(scene.handles.is_empty());
    assert!(scene.placement.unwrap().rejection.is_some());
    assert!(
        registry
            .plan_helper(
                &context,
                panel,
                &LayoutHandleAction::PanelProperty {
                    panel,
                    name: "Spacing".into()
                },
                8.0
            )
            .is_err()
    );
}
#[test]
fn bound_spacing_is_disabled_even_when_effective_geometry_is_available() {
    let document = Document::new(
        "<StackPanel Spacing='{Binding Gap}'><Button Name='a'/><Button Name='b'/></StackPanel>",
    )
    .unwrap();
    let panel = document.ast().root();
    let bounds = BTreeMap::from([
        (panel, Rect::new(0.0, 0.0, 80.0, 160.0)),
        (node(&document, "a"), Rect::new(0.0, 0.0, 80.0, 40.0)),
        (node(&document, "b"), Rect::new(0.0, 52.0, 80.0, 40.0)),
    ]);
    let properties = BTreeMap::from([(panel, BTreeMap::from([("Spacing".into(), "12".into())]))]);
    let context = LayoutContext::new(&document, &bounds).with_resolved_properties(&properties);
    let registry = LayoutRegistry::built_in();
    let scene = registry
        .design_helpers(&context, &LayoutHelperRequest::new(panel))
        .unwrap();
    let handle = scene.handles.iter().find(|h| h.id == "Spacing").unwrap();
    assert_eq!(handle.value, 12.0);
    assert!(handle.disabled_reason.is_some());
    assert!(
        registry
            .plan_helper(&context, panel, &handle.action, 20.0)
            .is_err()
    );
}
#[test]
fn selected_margin_handles_preserve_negative_values_and_other_components_precision() {
    let mut document =
        Document::new("<Grid><Button Name='child' Margin='-4,0.123456,7,8'/></Grid>").unwrap();
    let panel = document.ast().root();
    let child = node(&document, "child");
    let bounds = BTreeMap::from([
        (panel, Rect::new(0.0, 0.0, 200.0, 120.0)),
        (child, Rect::new(-4.0, 0.123456, 100.0, 30.0)),
    ]);
    let mut request = LayoutHelperRequest::new(panel);
    request.selected_nodes.push(child);
    let registry = LayoutRegistry::built_in();
    let scene = registry
        .design_helpers(&LayoutContext::new(&document, &bounds), &request)
        .unwrap();
    let handle = scene
        .handles
        .iter()
        .find(|handle| handle.id == format!("Margin.{child}.0"))
        .unwrap();
    assert_eq!(handle.value, -4.0);
    assert!(handle.minimum < 0.0);
    let edits = registry
        .plan_helper(
            &LayoutContext::new(&document, &bounds),
            panel,
            &handle.action,
            -9.25,
        )
        .unwrap();
    document.apply("Margin", edits).unwrap();
    assert_eq!(
        document.ast().node(child).unwrap().attribute("Margin"),
        Some("-9.25,0.123456,7,8")
    );
}
#[test]
fn padding_helpers_belong_to_content_hosts_and_preserve_bound_values() {
    for (source, expected) in [
        ("<Border Padding='8,12'/>", true),
        ("<StackPanel Padding='8,12'/>", false),
    ] {
        let document = Document::new(source).unwrap();
        let panel = document.ast().root();
        let bounds = BTreeMap::from([(panel, Rect::new(0.0, 0.0, 160.0, 100.0))]);
        let registry = LayoutRegistry::built_in();
        let scene = registry
            .design_helpers(
                &LayoutContext::new(&document, &bounds),
                &LayoutHelperRequest::new(panel),
            )
            .unwrap();
        assert_eq!(
            scene.handles.iter().any(|handle| handle.id == "Padding.0"),
            expected
        );
    }
}
#[test]
fn dedicated_uniform_dock_relative_helpers_expose_their_layout_semantics() {
    for source in [
        "<UniformGrid Rows='2' Columns='2' FirstColumn='1' RowSpacing='8' ColumnSpacing='10'><Button Name='a'/><Button Name='b'/></UniformGrid>",
        "<DockPanel><Button Name='a' DockPanel.Dock='Left'/><Button Name='b'/></DockPanel>",
        "<RelativePanel><Button Name='a'/><Button Name='b' RelativePanel.RightOf='a'/></RelativePanel>",
    ] {
        let document = Document::new(source).unwrap();
        let panel = document.ast().root();
        let bounds = BTreeMap::from([
            (panel, Rect::new(0.0, 0.0, 200.0, 100.0)),
            (node(&document, "a"), Rect::new(0.0, 0.0, 80.0, 40.0)),
            (node(&document, "b"), Rect::new(90.0, 0.0, 80.0, 40.0)),
        ]);
        let registry = LayoutRegistry::built_in();
        let context = LayoutContext::new(&document, &bounds);
        let scene = registry
            .design_helpers(&context, &request(panel, Point::new(160.0, 70.0)))
            .unwrap();
        let kind = document.ast().node(panel).unwrap().local_name();
        match kind {"UniformGrid"=>assert!(scene.primitives.iter().any(|guide|matches!(&guide.geometry,LayoutGuideGeometry::Label{text,..} if text=="Reserved"))),"DockPanel"=>assert!(scene.primitives.iter().any(|guide|matches!(&guide.geometry,LayoutGuideGeometry::Label{text,..} if text=="Fill"))),_=>assert!(scene.primitives.iter().any(|guide|guide.role==LayoutGuideRole::Relation))}
        let placement = scene.placement.unwrap();
        let edits = registry
            .plan(
                &context,
                LayoutAction::Drop {
                    parent: panel,
                    source: "<Button/>".into(),
                    position: Point::new(160.0, 70.0),
                },
            )
            .unwrap();
        let (owner, index, _) = inserted(&edits);
        assert_eq!((owner, index), (placement.container, placement.index));
    }
}
struct CustomProvider;
impl LayoutAdapter for CustomProvider {
    fn panel_type(&self) -> &str {
        "{urn:layout}StackPanel"
    }
    fn capabilities(&self) -> LayoutCapabilities {
        LayoutCapabilities {
            move_items: true,
            resize_items: true,
            reorder: true,
            drop_items: true,
            generated_items: false,
        }
    }
    fn panel_properties(&self) -> &[&str] {
        &[]
    }
    fn plan(&self, _: &LayoutContext<'_>, _: &LayoutAction) -> Result<Vec<Edit>> {
        Ok(Vec::new())
    }
    fn design_helpers(
        &self,
        context: &LayoutContext<'_>,
        request: &LayoutHelperRequest,
    ) -> Result<LayoutHelperScene> {
        let mut scene = LayoutHelperScene::generic(context, request)?;
        scene.title = "Custom radial layout".into();
        scene.line(
            Point::new(0.0, 0.0),
            Point::new(10.0, 10.0),
            LayoutGuideRole::Relation,
        );
        Ok(scene)
    }
    fn plan_helper(
        &self,
        context: &LayoutContext<'_>,
        action: &LayoutHandleAction,
        value: f64,
    ) -> Result<Vec<Edit>> {
        if let LayoutHandleAction::Custom { key } = action {
            Ok(vec![Edit::SetAttribute {
                node: context.document.ast().root(),
                name: key.clone(),
                value: value.to_string(),
            }])
        } else {
            Err(ToolError("Unsupported custom helper".into()))
        }
    }
}
#[test]
fn custom_panel_features_resolve_canonical_type_identity_across_prefix_aliases() {
    let mut registry = LayoutRegistry::built_in();
    let before = registry.revision();
    registry.register(Box::new(CustomProvider));
    assert_ne!(before, registry.revision());
    for source in [
        "<a:StackPanel xmlns:a='urn:layout'/>",
        "<b:StackPanel xmlns:b='urn:layout'/>",
        "<StackPanel xmlns='urn:layout'/>",
    ] {
        let document = Document::new(source).unwrap();
        let panel = document.ast().root();
        let bounds = BTreeMap::from([(panel, Rect::new(0.0, 0.0, 100.0, 100.0))]);
        let context = LayoutContext::new(&document, &bounds);
        assert_eq!(
            registry
                .design_helpers(&context, &LayoutHelperRequest::new(panel))
                .unwrap()
                .title,
            "Custom radial layout"
        );
        let edits = registry
            .plan_helper(
                &context,
                panel,
                &LayoutHandleAction::Custom {
                    key: "Radius".into(),
                },
                42.0,
            )
            .unwrap();
        assert!(
            matches!(&edits[0],Edit::SetAttribute{name,value,..} if name=="Radius" && value=="42")
        );
    }
    let unknown = Document::new("<StackPanel xmlns='urn:unregistered'/>").unwrap();
    assert!(
        registry
            .adapter_for(&unknown, unknown.ast().root())
            .is_none()
    );
    let framework = Document::new("<f:StackPanel xmlns:f='https://github.com/ferroui'/>").unwrap();
    assert_eq!(
        registry
            .adapter_for(&framework, framework.ast().root())
            .unwrap()
            .panel_type(),
        "StackPanel"
    );
}
#[test]
fn helper_scene_validation_rejects_nonfinite_geometry_and_duplicate_handle_identity() {
    let document = Document::new("<Border Padding='8'/>").unwrap();
    let panel = document.ast().root();
    let bounds = BTreeMap::from([(panel, Rect::new(0.0, 0.0, 100.0, 80.0))]);
    let mut scene = LayoutRegistry::built_in()
        .design_helpers(
            &LayoutContext::new(&document, &bounds),
            &LayoutHelperRequest::new(panel),
        )
        .unwrap();
    scene.handles.push(scene.handles[0].clone());
    assert!(scene.validate().is_err());
    scene.handles.pop();
    scene.line(
        Point::new(f64::NAN, 0.0),
        Point::new(1.0, 1.0),
        LayoutGuideRole::Track,
    );
    assert!(scene.validate().is_err());
}
