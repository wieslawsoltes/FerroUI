use super::*;
use ferroui_base::Size;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{ContentControl, Control, Window};
use ferroui_designer_tools::{LayoutAdapter, LayoutCapabilities, LayoutContext, ToolError};

const SOURCE: &str = r#"<Canvas xmlns="https://github.com/ferroui" Width="720" Height="420" UseLayoutRounding="False">
  <StackPanel Name="Flow" Canvas.Left="20" Canvas.Top="20" Width="180" Height="220" Spacing="0">
    <Button Name="First" Content="First" Height="40" Margin="2" />
    <Button Name="Second" Content="Second" Height="40" />
    <Button Name="Third" Content="Third" Height="40" />
  </StackPanel>
  <Border Name="Empty" Canvas.Left="240" Canvas.Top="20" Width="160" Height="140" Padding="12" />
  <Button Name="Occupied" Content="Existing content" Canvas.Left="430" Canvas.Top="20" Width="200" Height="100" />
  <Canvas Name="Destination" Canvas.Left="240" Canvas.Top="230" Width="220" Height="120" RenderTransform="scale(1.25)" />
</Canvas>"#;

struct Fixture {
    session: DesignerSession,
    window: Ref<Window>,
}
impl Fixture {
    fn new(source: &str) -> Self {
        let host = ContentControl::new();
        let session = DesignerSession::with_uri(
            source,
            host.clone(),
            "ferres://FerroUI.Designer/LayoutHelpers.xaml".into(),
        )
        .unwrap();
        let window = Window::new();
        window.set_width(760.0);
        window.set_height(460.0);
        window.set_content(Some(Control::boxed(host)));
        window.show();
        let mut fixture = Self { session, window };
        fixture.refresh();
        fixture
    }
    fn id(&self, name: &str) -> NodeId {
        self.session.document().ast().find_name(name).unwrap()
    }
    fn at(&self, name: &str, x: f64, y: f64) -> Point {
        self.session
            .preview
            .visual(self.id(name))
            .unwrap()
            .translate_point(Point::new(x, y), &self.session.preview.host)
            .unwrap()
    }
    fn center(&self, name: &str) -> Point {
        let visual = self.session.preview.visual(self.id(name)).unwrap();
        self.at(
            name,
            visual.bounds().width / 2.0,
            visual.bounds().height / 2.0,
        )
    }
    fn bounds(&self, name: &str) -> ferroui_base::Rect {
        self.session
            .preview
            .bounds(self.id(name), &self.session.preview.host.clone().upcast())
            .unwrap()
    }
    fn refresh(&mut self) {
        self.window.update_layout();
        self.session.preview_host().measure(Size::new(720.0, 420.0));
        self.session.refresh_runtime_mapping();
        assert!(
            self.session.can_manipulate(),
            "{:?}",
            self.session.preview_status()
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

#[test]
fn selecting_a_leaf_button_exposes_parent_flow_and_margin_helpers() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new(SOURCE);
    let first = fixture.id("First");
    fixture.session.select([first]);
    let snapshot = fixture.session.selection_layout_helpers().unwrap();
    assert_eq!(snapshot.scene.panel, fixture.id("Flow"));
    assert!(
        snapshot
            .scene
            .handles
            .iter()
            .any(|handle| matches!(&handle.action,
        ferroui_designer_tools::LayoutHandleAction::ThicknessSide { node, property, .. }
            if *node == first && property == "Margin"))
    );
    assert!(!fixture.session.can_undo());
}

#[test]
fn zero_gap_flow_edges_reorder_existing_nodes_with_one_undo() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new(SOURCE);
    let first = fixture.id("First");
    let second = fixture.id("Second");
    let third = fixture.id("Third");
    fixture.session.select([first]);
    let third_bounds = fixture.session.preview.visual(third).unwrap().bounds();
    let end = fixture.at("Third", third_bounds.width / 2.0, third_bounds.height - 1.0);
    let start = fixture.center("First");
    let preview = fixture
        .session
        .preview_layout_move(&[first], end, Point::new(end.x - start.x, end.y - start.y))
        .unwrap()
        .unwrap();
    assert!(preview.is_accepted(), "{:?}", preview.rejection());
    assert_eq!(preview.target(), fixture.id("Flow"));
    assert_eq!(fixture.session.document().source(), SOURCE);
    fixture.session.commit_layout_preview(&preview).unwrap();
    assert_eq!(
        fixture
            .session
            .document()
            .ast()
            .element_children(fixture.id("Flow")),
        vec![second, third, first]
    );
    assert_eq!(fixture.session.selection(), &[first]);
    assert_eq!(fixture.session.document().revision(), 1);
    fixture.session.undo().unwrap();
    assert_eq!(fixture.session.document().source(), SOURCE);
}

#[test]
fn empty_content_holder_accepts_move_and_occupied_interior_rejects_without_fallback() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new(SOURCE);
    let first = fixture.id("First");
    fixture.session.select([first]);
    let occupied = fixture
        .session
        .preview_layout_move(&[first], fixture.center("Occupied"), Point::new(300.0, 0.0))
        .unwrap()
        .unwrap();
    assert!(!occupied.is_accepted());
    assert_eq!(occupied.target(), fixture.id("Occupied"));
    assert!(fixture.session.commit_layout_preview(&occupied).is_err());
    assert_eq!(fixture.session.document().source(), SOURCE);
    let empty = fixture
        .session
        .preview_layout_move(&[first], fixture.center("Empty"), Point::new(220.0, 0.0))
        .unwrap()
        .unwrap();
    assert!(empty.is_accepted(), "{:?}", empty.rejection());
    fixture.session.commit_layout_preview(&empty).unwrap();
    assert_eq!(
        fixture.session.document().ast().node(first).unwrap().parent,
        Some(fixture.id("Empty"))
    );
    fixture.session.undo().unwrap();
    assert_eq!(fixture.session.document().source(), SOURCE);
}

#[test]
fn transformed_canvas_reparent_preserves_grab_offset_and_world_size() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new(SOURCE);
    let first = fixture.id("First");
    fixture.session.select([first]);
    let before = fixture.bounds("First");
    let start = fixture.at("First", 11.0, 9.0);
    let end = fixture.at("Destination", 38.0, 36.0);
    let delta = Point::new(end.x - start.x, end.y - start.y);
    let preview = fixture
        .session
        .preview_layout_move(&[first], end, delta)
        .unwrap()
        .unwrap();
    assert!(preview.is_accepted(), "{:?}", preview.rejection());
    assert_eq!(preview.target(), fixture.id("Destination"));
    let landing = preview
        .helpers
        .to_artboard
        .bounds(preview.helpers.scene.placement.as_ref().unwrap().bounds)
        .unwrap();
    for error in [
        landing.x - before.x - delta.x,
        landing.y - before.y - delta.y,
        landing.width - before.width,
        landing.height - before.height,
    ] {
        assert!(
            error.abs() < 0.01,
            "Incorrect Canvas landing bounds: {landing:?}"
        );
    }
    fixture.session.commit_layout_preview(&preview).unwrap();
    fixture.refresh();
    let after = fixture.bounds("First");
    for error in [
        after.x - before.x - delta.x,
        after.y - before.y - delta.y,
        after.width - before.width,
        after.height - before.height,
    ] {
        assert!(
            error.abs() < 0.01,
            "world geometry changed by {error}: {before:?} -> {after:?}"
        );
    }
    assert_eq!(
        fixture.session.document().ast().node(first).unwrap().parent,
        Some(fixture.id("Destination"))
    );
    fixture.session.undo().unwrap();
    assert_eq!(fixture.session.document().source(), SOURCE);
}

#[test]
fn helper_drafts_apply_once_cancel_freely_and_reject_changed_context() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new(SOURCE);
    let flow = fixture.id("Flow");
    fixture.session.select([flow]);
    let snapshot = fixture.session.selection_layout_helpers().unwrap();
    let index = snapshot
        .scene
        .handles
        .iter()
        .position(|handle| {
            matches!(&handle.action,
        ferroui_designer_tools::LayoutHandleAction::PanelProperty { name, .. } if name == "Spacing")
        })
        .unwrap();
    let draft = fixture
        .session
        .preview_layout_handle(&snapshot, index, Point::new(0.0, 17.0))
        .unwrap();
    assert!(draft.is_accepted(), "{:?}", draft.rejection());
    assert_eq!(fixture.session.document().source(), SOURCE);
    assert!(!fixture.session.can_undo());
    fixture.session.commit_layout_preview(&draft).unwrap();
    assert_eq!(
        fixture
            .session
            .document()
            .ast()
            .node(flow)
            .unwrap()
            .attribute("Spacing"),
        Some("17")
    );
    assert!(fixture.session.commit_layout_preview(&draft).is_err());
    fixture.session.undo().unwrap();
    assert_eq!(fixture.session.document().source(), SOURCE);
    assert!(
        fixture
            .session
            .preview_layout_handle(&snapshot, index, Point::new(0.0, 20.0))
            .is_err()
    );
}

struct Replacement;
impl LayoutAdapter for Replacement {
    fn panel_type(&self) -> &str {
        "StackPanel"
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
    fn plan(&self, _: &LayoutContext<'_>, _: &LayoutAction) -> Result<Vec<Edit>, ToolError> {
        Ok(Vec::new())
    }
}

#[test]
fn placement_snapshots_reject_provider_selection_permission_and_document_replacement() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    for change in 0..4 {
        let mut fixture = Fixture::new(SOURCE);
        let first = fixture.id("First");
        fixture.session.select([first]);
        let preview = fixture
            .session
            .preview_layout_move(&[first], fixture.center("Empty"), Point::new(220.0, 0.0))
            .unwrap()
            .unwrap();
        assert!(preview.is_accepted());
        match change {
            0 => {
                fixture
                    .session
                    .layout_tools_mut()
                    .register(Box::new(Replacement));
            }
            1 => fixture.session.select([fixture.id("Second")]),
            2 => fixture.session.set_read_only(true),
            3 => {
                let uri = fixture.session.document_uri().to_owned();
                fixture
                    .session
                    .set_document_uri(format!("{uri}.retired"))
                    .unwrap();
                fixture.session =
                    DesignerSession::with_uri(SOURCE, ContentControl::new(), uri).unwrap();
                fixture.session.select([first]);
            }
            _ => unreachable!(),
        }
        assert!(fixture.session.commit_layout_preview(&preview).is_err());
        assert_eq!(fixture.session.document().source(), SOURCE);
        assert!(!fixture.session.can_undo());
    }
}

#[test]
fn native_asset_preview_targets_transparent_nested_content_and_selects_inserted_identity() {
    const NESTED: &str = r#"<Canvas xmlns="https://github.com/ferroui" Width="500" Height="300"><Border Name="Holder" Canvas.Left="80" Canvas.Top="40" Width="220" Height="160" Padding="12"><Grid Name="Inner" /></Border></Canvas>"#;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new(NESTED);
    let preview = fixture
        .session
        .preview_layout_insert(
            r#"<Button Name="Inserted" Content="New" />"#,
            fixture.center("Inner"),
        )
        .unwrap();
    assert!(preview.is_accepted(), "{:?}", preview.rejection());
    assert_eq!(preview.target(), fixture.id("Inner"));
    assert_eq!(fixture.session.document().source(), NESTED);
    fixture.session.commit_layout_preview(&preview).unwrap();
    let inserted = fixture.id("Inserted");
    assert_eq!(
        fixture
            .session
            .document()
            .ast()
            .node(inserted)
            .unwrap()
            .parent,
        Some(fixture.id("Inner"))
    );
    assert_eq!(fixture.session.selection(), &[inserted]);
    fixture.session.undo().unwrap();
    assert_eq!(fixture.session.document().source(), NESTED);
}

struct CustomHelpers;
impl LayoutAdapter for CustomHelpers {
    fn panel_type(&self) -> &str {
        "{https://github.com/ferroui}StackPanel"
    }
    fn capabilities(&self) -> LayoutCapabilities {
        Replacement.capabilities()
    }
    fn panel_properties(&self) -> &[&str] {
        &["Spacing"]
    }
    fn plan(&self, _: &LayoutContext<'_>, _: &LayoutAction) -> Result<Vec<Edit>, ToolError> {
        Err(ToolError(
            "This sample exposes its own helper action".into(),
        ))
    }
    fn design_helpers(
        &self,
        context: &LayoutContext<'_>,
        request: &LayoutHelperRequest,
    ) -> Result<LayoutHelperScene, ToolError> {
        let mut scene = LayoutHelperScene::generic(context, request)?;
        scene.title = "Custom rhythm".into();
        scene.handles.push(ferroui_designer_tools::LayoutHandle {
            id: "rhythm".into(),
            label: "Rhythm".into(),
            position: LayoutPoint::new(70.0, 18.0),
            axis: Axis::Vertical,
            value: 0.0,
            minimum: 0.0,
            maximum: Some(40.0),
            direction: 1.0,
            action: ferroui_designer_tools::LayoutHandleAction::Custom {
                key: request.panel.to_string(),
            },
            disabled_reason: None,
        });
        Ok(scene)
    }
    fn plan_helper(
        &self,
        context: &LayoutContext<'_>,
        action: &ferroui_designer_tools::LayoutHandleAction,
        value: f64,
    ) -> Result<Vec<Edit>, ToolError> {
        let ferroui_designer_tools::LayoutHandleAction::Custom { key } = action else {
            return Err(ToolError("Unexpected action".into()));
        };
        let node = key
            .parse()
            .map_err(|_| ToolError("Invalid helper owner".into()))?;
        ferroui_designer_tools::plan_layout_attributes(
            context.document,
            node,
            &[("Spacing".into(), value.to_string())],
        )
    }
}

#[test]
fn canonical_custom_provider_supplies_native_helpers_and_transaction_planning() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = SOURCE
        .replace(
            "<StackPanel",
            "<ui:StackPanel xmlns:ui=\"https://github.com/ferroui\"",
        )
        .replace("</StackPanel>", "</ui:StackPanel>");
    let mut fixture = Fixture::new(&source);
    fixture
        .session
        .layout_tools_mut()
        .register(Box::new(CustomHelpers));
    let flow = fixture.id("Flow");
    fixture.session.select([flow]);
    let snapshot = fixture.session.selection_layout_helpers().unwrap();
    assert_eq!(snapshot.scene.title, "Custom rhythm");
    assert_eq!(snapshot.scene.handles.len(), 1);
    let preview = fixture
        .session
        .preview_layout_handle(&snapshot, 0, Point::new(0.0, 9.0))
        .unwrap();
    assert!(preview.is_accepted(), "{:?}", preview.rejection());
    assert_eq!(fixture.session.document().source(), source);
    fixture.session.commit_layout_preview(&preview).unwrap();
    assert_eq!(
        fixture
            .session
            .document()
            .ast()
            .node(flow)
            .unwrap()
            .attribute("Spacing"),
        Some("9")
    );
    fixture.session.undo().unwrap();
    assert_eq!(fixture.session.document().source(), source);
}

#[test]
fn native_grid_placement_preserves_attached_aliases_and_rejects_binding_replacement() {
    const ALIASED: &str = r#"<ui:Canvas xmlns="urn:project" xmlns:ui="https://github.com/ferroui" Width="720" Height="420"><ui:Button Name="Moving" ui:Canvas.Left="20" ui:Canvas.Top="20" Width="80" Height="40" ui:Grid.Row="{ui:Binding SelectedRow}" /><ui:Grid Name="GridTarget" ui:Canvas.Left="260" ui:Canvas.Top="180" Width="240" Height="180" RowDefinitions="*,*" ColumnDefinitions="*,*" /></ui:Canvas>"#;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    for bound in [true, false] {
        let source = if bound {
            ALIASED.to_owned()
        } else {
            ALIASED.replace("{ui:Binding SelectedRow}", "0")
        };
        let mut fixture = Fixture::new(&source);
        let moving = fixture.id("Moving");
        fixture.session.select([moving]);
        let end = fixture.at("GridTarget", 180.0, 130.0);
        let preview = fixture
            .session
            .preview_layout_move(&[moving], end, Point::new(300.0, 220.0))
            .unwrap()
            .unwrap();
        if bound {
            assert!(!preview.is_accepted());
            assert!(preview.rejection().unwrap().contains("expression"));
            assert!(fixture.session.commit_layout_preview(&preview).is_err());
            assert_eq!(fixture.session.document().source(), source);
        } else {
            assert!(preview.is_accepted(), "{:?}", preview.rejection());
            fixture.session.commit_layout_preview(&preview).unwrap();
            let moved = fixture.session.document().ast().node(moving).unwrap();
            assert_eq!(moved.parent, Some(fixture.id("GridTarget")));
            assert_eq!(moved.attribute("ui:Grid.Row"), Some("1"));
            assert_eq!(moved.attribute("ui:Grid.Column"), Some("1"));
            assert_eq!(moved.attribute("Grid.Row"), None);
            fixture.session.undo().unwrap();
            assert_eq!(fixture.session.document().source(), source);
        }
    }
}

#[test]
fn native_canvas_placement_preserves_aliases_and_protects_opposite_anchor_bindings() {
    const ALIASED: &str = r#"<ui:Canvas xmlns="urn:project" xmlns:ui="https://github.com/ferroui" Width="720" Height="420" UseLayoutRounding="False"><ui:Button Name="Moving" ui:Canvas.Right="{ui:Binding RightEdge}" ui:Canvas.Bottom="30" Width="80" Height="40" /><ui:Canvas Name="Destination" ui:Canvas.Left="260" ui:Canvas.Top="180" Width="240" Height="180" RenderTransform="scale(1.25)" /></ui:Canvas>"#;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    for bound in [true, false] {
        let source = if bound {
            ALIASED.to_owned()
        } else {
            ALIASED.replace("{ui:Binding RightEdge}", "20")
        };
        let mut fixture = Fixture::new(&source);
        let moving = fixture.id("Moving");
        fixture.session.select([moving]);
        let start = fixture.center("Moving");
        let end = fixture.center("Destination");
        let preview = fixture
            .session
            .preview_layout_move(&[moving], end, Point::new(end.x - start.x, end.y - start.y))
            .unwrap()
            .unwrap();
        if bound {
            assert!(!preview.is_accepted());
            assert!(preview.rejection().unwrap().contains("expression"));
            assert!(fixture.session.commit_layout_preview(&preview).is_err());
            assert_eq!(fixture.session.document().source(), source);
        } else {
            assert!(preview.is_accepted(), "{:?}", preview.rejection());
            fixture.session.commit_layout_preview(&preview).unwrap();
            let moved = fixture.session.document().ast().node(moving).unwrap();
            assert_eq!(moved.parent, Some(fixture.id("Destination")));
            assert!(moved.attribute("ui:Canvas.Left").is_some());
            assert!(moved.attribute("ui:Canvas.Top").is_some());
            assert_eq!(moved.attribute("Canvas.Left"), None);
            assert_eq!(moved.attribute("ui:Canvas.Right"), None);
            assert_eq!(moved.attribute("ui:Canvas.Bottom"), None);
            assert!(
                fixture.session.can_manipulate(),
                "{:?}",
                fixture.session.preview_status()
            );
            fixture.session.undo().unwrap();
            assert_eq!(fixture.session.document().source(), source);
        }
    }
}

#[test]
fn canvas_drag_displacement_survives_an_otherwise_unchanged_reparent_position() {
    const COINCIDENT: &str = r#"<Canvas xmlns="https://github.com/ferroui" Width="720" Height="420" UseLayoutRounding="False"><Button Name="Moving" Width="80" Height="40" Canvas.Left="80" Canvas.Top="80"/><Canvas Name="Destination" Width="720" Height="420"/></Canvas>"#;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new(COINCIDENT);
    let moving = fixture.id("Moving");
    fixture.session.select([moving]);
    let start = fixture.center("Moving");
    let preview = fixture
        .session
        .preview_layout_move(
            &[moving],
            Point::new(start.x + 100.0, start.y + 50.0),
            Point::new(100.0, 50.0),
        )
        .unwrap()
        .unwrap();
    assert!(preview.is_accepted(), "{:?}", preview.rejection());
    assert_eq!(preview.target(), fixture.id("Destination"));
    fixture.session.commit_layout_preview(&preview).unwrap();
    let moved = fixture.session.document().ast().node(moving).unwrap();
    assert_eq!(moved.attribute("Canvas.Left"), Some("180"));
    assert_eq!(moved.attribute("Canvas.Top"), Some("130"));
    fixture.session.undo().unwrap();
    assert_eq!(fixture.session.document().source(), COINCIDENT);
}

#[test]
fn mixed_parent_canvas_selection_moves_children_already_in_the_destination_too() {
    const MIXED: &str = r#"<Canvas xmlns="https://github.com/ferroui" Width="720" Height="420" UseLayoutRounding="False"><Button Name="Moving" Width="80" Height="40" Canvas.Left="80" Canvas.Top="80"/><Canvas Name="Destination" Width="720" Height="420"><Button Name="Resident" Width="60" Height="30" Canvas.Left="260" Canvas.Top="80"/></Canvas></Canvas>"#;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new(MIXED);
    let moving = fixture.id("Moving");
    let resident = fixture.id("Resident");
    fixture.session.select([moving, resident]);
    let start = fixture.center("Moving");
    let preview = fixture
        .session
        .preview_layout_move(
            &[moving, resident],
            Point::new(start.x + 100.0, start.y + 50.0),
            Point::new(100.0, 50.0),
        )
        .unwrap()
        .unwrap();
    assert!(preview.is_accepted(), "{:?}", preview.rejection());
    fixture.session.commit_layout_preview(&preview).unwrap();
    for (node, left) in [(moving, "180"), (resident, "360")] {
        let moved = fixture.session.document().ast().node(node).unwrap();
        assert_eq!(moved.parent, Some(fixture.id("Destination")));
        assert_eq!(moved.attribute("Canvas.Left"), Some(left));
        assert_eq!(moved.attribute("Canvas.Top"), Some("130"));
    }
    fixture.session.undo().unwrap();
    assert_eq!(fixture.session.document().source(), MIXED);
}

struct CustomCanvas;
impl LayoutAdapter for CustomCanvas {
    fn panel_type(&self) -> &str {
        "{https://github.com/ferroui}Canvas"
    }
    fn capabilities(&self) -> LayoutCapabilities {
        Replacement.capabilities()
    }
    fn panel_properties(&self) -> &[&str] {
        &[]
    }
    fn plan(&self, _: &LayoutContext<'_>, _: &LayoutAction) -> Result<Vec<Edit>, ToolError> {
        Err(ToolError("Use the custom semantic placement".into()))
    }
    fn placement(
        &self,
        context: &LayoutContext<'_>,
        panel: NodeId,
        _: LayoutPoint,
        _: &[NodeId],
    ) -> Result<Option<LayoutPlacement>, ToolError> {
        let container = context.content_container(panel)?;
        Ok(Some(LayoutPlacement {
            container,
            index: context.document.ast().element_children(container).len(),
            attributes: vec![
                ("Canvas.Left".into(), "22".into()),
                ("Canvas.Top".into(), "33".into()),
            ],
            bounds: LayoutRect::new(22.0, 33.0, 80.0, 90.0),
            description: "Custom Canvas slot".into(),
            rejection: None,
        }))
    }
}

#[test]
fn overriding_canvas_provider_controls_same_parent_and_cross_parent_placement() {
    const CUSTOM: &str = r#"<Canvas xmlns="https://github.com/ferroui" Width="720" Height="420"><Button Name="Moving" Width="80" Height="40" Canvas.Left="20" Canvas.Top="20"/><Canvas Name="Destination" Canvas.Left="260" Canvas.Top="180" Width="240" Height="180" RenderTransform="scale(1.25)" /></Canvas>"#;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    for cross_parent in [false, true] {
        let mut fixture = Fixture::new(CUSTOM);
        fixture
            .session
            .layout_tools_mut()
            .register(Box::new(CustomCanvas));
        let moving = fixture.id("Moving");
        fixture.session.select([moving]);
        let end = if cross_parent {
            fixture.center("Destination")
        } else {
            Point::new(620.0, 380.0)
        };
        let preview = fixture
            .session
            .preview_layout_move(&[moving], end, Point::new(100.0, 80.0))
            .unwrap()
            .expect("Custom same-parent placement must not become a free move");
        assert!(preview.is_accepted(), "{:?}", preview.rejection());
        assert_eq!(preview.description(), "Custom Canvas slot");
        assert_eq!(
            preview.helpers.scene.placement.as_ref().unwrap().bounds,
            LayoutRect::new(22.0, 33.0, 80.0, 90.0)
        );
        fixture.session.commit_layout_preview(&preview).unwrap();
        let moved = fixture.session.document().ast().node(moving).unwrap();
        assert_eq!(moved.attribute("Canvas.Left"), Some("22"));
        assert_eq!(moved.attribute("Canvas.Top"), Some("33"));
        assert_eq!(moved.attribute("RenderTransform"), None);
        fixture.session.undo().unwrap();
        assert_eq!(fixture.session.document().source(), CUSTOM);
    }
}

#[test]
fn native_grid_helpers_match_arranged_cells_without_counting_spacing_twice() {
    use ferroui_designer_tools::LayoutHandleAction;

    let _app = UnitTestApplication::start(TestServices::styled_window());
    for (rows, columns, row_gap, column_gap) in [
        (vec!["*"], vec!["*"], 8, 11),
        (vec!["*", "*"], vec!["*", "*"], 8, 8),
        (vec!["31", "*", "2*"], vec!["41", "2*", "*"], 13, 7),
        (vec!["*", "*"], vec!["*", "*", "*"], 0, 0),
    ] {
        let mut source = format!(
            r#"<Canvas xmlns="https://github.com/ferroui" Width="720" Height="420" UseLayoutRounding="False"><Grid Name="MeasuredGrid" Canvas.Left="60" Canvas.Top="40" Width="232" Height="158" RowDefinitions="{}" ColumnDefinitions="{}" RowSpacing="{row_gap}" ColumnSpacing="{column_gap}" RenderTransform="scale(1.1)">"#,
            rows.join(","),
            columns.join(",")
        );
        for row in 0..rows.len() {
            for column in 0..columns.len() {
                source.push_str(&format!(
                    r#"<Border Name="Cell{row}_{column}" Grid.Row="{row}" Grid.Column="{column}" />"#
                ));
            }
        }
        source.push_str("</Grid></Canvas>");
        let mut fixture = Fixture::new(&source);
        let grid = fixture.id("MeasuredGrid");
        fixture.session.select([grid]);
        let native_grid = fixture.session.preview.visual(grid).unwrap().bounds();
        for row in 0..rows.len() {
            for column in 0..columns.len() {
                let cell_name = format!("Cell{row}_{column}");
                let native_cell = fixture
                    .session
                    .preview
                    .visual(fixture.id(&cell_name))
                    .unwrap()
                    .bounds();
                let snapshot = fixture
                    .session
                    .layout_helper_snapshot(grid, Some(fixture.center(&cell_name)), &[])
                    .unwrap();
                let placement = snapshot.scene.placement.as_ref().unwrap();
                assert!(placement.rejection.is_none());
                assert!(
                    placement
                        .attributes
                        .contains(&("Grid.Row".into(), row.to_string()))
                );
                assert!(
                    placement
                        .attributes
                        .contains(&("Grid.Column".into(), column.to_string()))
                );
                for error in [
                    placement.bounds.x - native_cell.x,
                    placement.bounds.y - native_cell.y,
                    placement.bounds.width - native_cell.width,
                    placement.bounds.height - native_cell.height,
                ] {
                    assert!(
                        error.abs() < 1e-7,
                        "Cell {row}/{column}: native {native_cell:?}, helper {:?}",
                        placement.bounds
                    );
                }
                for (axis, index, extent) in [
                    (Axis::Horizontal, column, native_cell.width),
                    (Axis::Vertical, row, native_cell.height),
                ] {
                    let handle = snapshot.scene.handles.iter().find(|handle| matches!(
                        handle.action,
                        LayoutHandleAction::GridTrack { axis: actual_axis, index: actual_index, .. }
                            if actual_axis == axis && actual_index == index
                    )).unwrap();
                    assert!(
                        (handle.value - extent).abs() < 1e-7,
                        "{axis:?} track {index} includes a gutter: {} vs {extent}",
                        handle.value
                    );
                }
                for guide in &snapshot.scene.primitives {
                    if guide.role == LayoutGuideRole::Track {
                        if let LayoutGuideGeometry::Line { start, end } = guide.geometry {
                            for point in [start, end] {
                                assert!(
                                    point.x >= -1e-7 && point.x <= native_grid.width + 1e-7,
                                    "Grid track exceeds native width: {point:?} / {native_grid:?}"
                                );
                                assert!(
                                    point.y >= -1e-7 && point.y <= native_grid.height + 1e-7,
                                    "Grid track exceeds native height: {point:?} / {native_grid:?}"
                                );
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(fixture.session.document().source(), source);
        assert!(!fixture.session.can_undo());
    }
}
