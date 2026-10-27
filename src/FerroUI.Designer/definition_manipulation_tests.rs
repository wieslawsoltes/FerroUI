use crate::{DesignGesture, DesignerSession};
use ferroui_base::{Point, Rect, Ref, Visual};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{ContentControl, Control, Window};
use ferroui_designer_core::NodeId;
use ferroui_designer_tools::{ComponentAction, LayoutAction};

const SOURCE: &str = r#"<Grid xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" UseLayoutRounding="False">
  <Grid.Resources>
    <ControlTemplate x:Key="Card" TargetType="Button">
      <Border Name="VisualRoot" Width="100" Height="60" HorizontalAlignment="Left" VerticalAlignment="Top" UseLayoutRounding="False" Background="Blue" />
    </ControlTemplate>
  </Grid.Resources>
</Grid>"#;

struct Fixture {
    session: DesignerSession,
    window: Ref<Window>,
}
impl Fixture {
    fn new(source: &str, key: &str) -> Self {
        let host = ContentControl::new();
        let mut session = DesignerSession::new(source, host.clone()).unwrap();
        let definition = session
            .document()
            .ast()
            .elements()
            .find(|node| node.attribute("x:Key") == Some(key))
            .unwrap()
            .id;
        session.enter_definition_scope(definition).unwrap();
        let window = Window::new();
        window.set_width(800.0);
        window.set_height(500.0);
        window.set_content(Some(Control::boxed(host)));
        window.show();
        let mut result = Self { session, window };
        result.refresh();
        result
    }
    fn id(&self, name: &str) -> NodeId {
        self.session.document().ast().find_name(name).unwrap()
    }
    fn refresh(&mut self) {
        self.window.update_layout();
        self.session.refresh_runtime_mapping();
        assert!(
            self.session.can_manipulate(),
            "{:?}",
            self.session.preview_status()
        );
    }
    fn bounds(&self, node: NodeId) -> Rect {
        self.session
            .preview
            .bounds(node, &self.session.preview.host.clone().upcast::<Visual>())
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}
fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 0.01, "{actual} != {expected}");
}

#[test]
fn definition_root_move_and_resize_use_native_parent_and_original_source_history() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new(SOURCE, "Card");
    let root = fixture.id("VisualRoot");
    let definition = fixture.session.definition_scope().unwrap().node;
    assert!(fixture.session.preview.visual(definition).is_none());
    let before = fixture.bounds(root);
    let revision = fixture.session.document().revision();
    crate::workspace_tools::commit(
        &mut fixture.session,
        DesignGesture::Move {
            nodes: vec![root],
            delta: Point::new(12.0, 8.0),
        },
    )
    .unwrap();
    fixture.refresh();
    let moved = fixture.bounds(root);
    near(moved.x, before.x + 12.0);
    near(moved.y, before.y + 8.0);
    near(moved.width, before.width);
    assert_eq!(fixture.session.document().revision(), revision + 1);
    assert_eq!(
        fixture
            .session
            .document()
            .ast()
            .node(root)
            .unwrap()
            .attribute("Margin"),
        Some("12,8,-12,-8")
    );
    let mut after = moved;
    after.x -= 4.0;
    after.y -= 3.0;
    after.width += 40.0;
    after.height += 20.0;
    crate::workspace_tools::commit(
        &mut fixture.session,
        DesignGesture::Resize {
            node: root,
            before: moved,
            after,
        },
    )
    .unwrap();
    fixture.refresh();
    let resized = fixture.bounds(root);
    near(resized.x, after.x);
    near(resized.y, after.y);
    near(resized.width, after.width);
    near(resized.height, after.height);
    assert_eq!(fixture.id("VisualRoot"), root);
    assert!(fixture.session.preview.visual(definition).is_none());
    fixture.session.undo().unwrap();
    fixture.session.undo().unwrap();
    assert_eq!(fixture.session.document().source(), SOURCE);
    assert!(!fixture.session.can_undo());
}

#[test]
fn theme_template_nested_panel_children_keep_their_panel_layout_semantics() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new(
        r#"<Grid xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><Grid.Resources><ControlTheme x:Key="CardTheme" TargetType="Button"><Setter Property="Template"><ControlTemplate><Canvas Name="VisualRoot" Width="200" Height="120"><Border Name="Child" Width="40" Height="30" Canvas.Left="10" Canvas.Top="12" Background="Blue"/></Canvas></ControlTemplate></Setter></ControlTheme></Grid.Resources></Grid>"#,
        "CardTheme",
    );
    let child = fixture.id("Child");
    let before = fixture.bounds(child);
    fixture
        .session
        .apply_layout_action(
            LayoutAction::Move {
                node: child,
                delta: ferroui_designer_tools::Point::new(16.0, 6.0),
            },
            "Move nested template child",
        )
        .unwrap();
    fixture.refresh();
    let moved = fixture.bounds(child);
    near(moved.x, before.x + 16.0);
    near(moved.y, before.y + 6.0);
    let child_source = fixture.session.document().ast().node(child).unwrap();
    assert_eq!(child_source.attribute("Canvas.Left"), Some("26"));
    assert_eq!(child_source.attribute("Canvas.Top"), Some("18"));
    assert!(child_source.attribute("Margin").is_none());
    crate::workspace_tools::commit(
        &mut fixture.session,
        DesignGesture::ResizeLocal {
            node: child,
            after: Rect::new(0.0, 0.0, 60.0, 45.0),
        },
    )
    .unwrap();
    fixture.refresh();
    near(fixture.bounds(child).width, 60.0);
    near(fixture.bounds(child).height, 45.0);
    let root = fixture.id("VisualRoot");
    crate::workspace_tools::commit(
        &mut fixture.session,
        DesignGesture::Move {
            nodes: vec![root],
            delta: Point::new(5.0, 3.0),
        },
    )
    .unwrap();
    assert_eq!(
        fixture
            .session
            .document()
            .ast()
            .node(root)
            .unwrap()
            .attribute("Margin"),
        Some("5,3,-5,-3")
    );
}

#[test]
fn data_template_root_preserves_resource_expressions_until_explicit_property_edit() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Grid xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><Grid.Resources><Thickness x:Key="Spacing">4</Thickness><DataTemplate x:Key="Item"><Border Name="VisualRoot" Width="100" Height="60" Margin="{StaticResource Spacing}" Background="Blue"/></DataTemplate></Grid.Resources></Grid>"#;
    let mut fixture = Fixture::new(source, "Item");
    let root = fixture.id("VisualRoot");
    let action = || LayoutAction::Move {
        node: root,
        delta: ferroui_designer_tools::Point::new(8.0, 4.0),
    };
    assert!(fixture.session.plan_layout_action(action()).is_err());
    assert_eq!(fixture.session.document().source(), source);
    fixture
        .session
        .edit_component(ComponentAction::SetProperty {
            node: root,
            property: "Margin".into(),
            value: Some("0".into()),
        })
        .unwrap();
    fixture.refresh();
    let before = fixture.bounds(root);
    fixture
        .session
        .apply_layout_action(action(), "Move item template root")
        .unwrap();
    fixture.refresh();
    let after = fixture.bounds(root);
    near(after.x, before.x + 8.0);
    near(after.y, before.y + 4.0);
    let authored = fixture.session.document().source().to_owned();
    fixture.session.set_read_only(true);
    assert!(
        fixture
            .session
            .apply_layout_action(action(), "Blocked root move")
            .is_err()
    );
    assert_eq!(fixture.session.document().source(), authored);
}

#[test]
fn template_root_margin_handles_author_original_root_and_reject_a_changed_preview_scope() {
    use ferroui_designer_tools::LayoutHandleAction;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new(SOURCE, "Card");
    let root = fixture.id("VisualRoot");
    let definition = fixture.session.definition_scope().unwrap().node;
    fixture.session.select([root]);
    let snapshot = fixture
        .session
        .layout_helper_snapshot(definition, None, &[])
        .unwrap();
    let index = snapshot.scene.handles.iter().position(|handle| matches!(
        &handle.action,
        LayoutHandleAction::ThicknessSide { node, property, side: 0, .. } if *node == root && property == "Margin"
    )).expect("A nonvisual template owner exposes the actual authored root's margin handle");
    let preview = fixture
        .session
        .preview_layout_handle(&snapshot, index, Point::new(9.0, 0.0))
        .unwrap();
    assert!(preview.is_accepted());
    assert_eq!(fixture.session.document().source(), SOURCE);
    fixture.session.commit_layout_preview(&preview).unwrap();
    assert_eq!(
        fixture
            .session
            .document()
            .ast()
            .node(root)
            .unwrap()
            .attribute("Margin"),
        Some("9,0,0,0")
    );
    assert!(
        fixture
            .session
            .document()
            .ast()
            .node(definition)
            .unwrap()
            .attribute("Margin")
            .is_none()
    );
    fixture.refresh();
    let snapshot = fixture
        .session
        .layout_helper_snapshot(definition, None, &[])
        .unwrap();
    let index = snapshot.scene.handles.iter().position(|handle| matches!(
        &handle.action,
        LayoutHandleAction::ThicknessSide { node, property, side: 0, .. } if *node == root && property == "Margin"
    )).unwrap();
    let pending = fixture
        .session
        .preview_layout_handle(&snapshot, index, Point::new(3.0, 0.0))
        .unwrap();
    let source = fixture.session.document().source().to_owned();
    fixture.session.exit_definition_scope().unwrap();
    fixture.session.select([root]);
    assert!(fixture.session.commit_layout_preview(&pending).is_err());
    assert_eq!(fixture.session.document().source(), source);
}

#[test]
fn qualified_template_types_use_the_same_native_layout_provider() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = SOURCE
        .replace(
            "<ControlTemplate ",
            "<t:ControlTemplate xmlns:t=\"using:FerroUI.Markup.Xaml.Templates\" ",
        )
        .replace("</ControlTemplate>", "</t:ControlTemplate>");
    let mut fixture = Fixture::new(&source, "Card");
    let root = fixture.id("VisualRoot");
    let before = fixture.bounds(root);
    fixture
        .session
        .apply_layout_action(
            LayoutAction::Move {
                node: root,
                delta: ferroui_designer_tools::Point::new(6.0, 4.0),
            },
            "Move qualified template root",
        )
        .unwrap();
    fixture.refresh();
    near(fixture.bounds(root).x, before.x + 6.0);
    near(fixture.bounds(root).y, before.y + 4.0);
    fixture.session.undo().unwrap();
    assert_eq!(fixture.session.document().source(), source);
}

#[test]
fn definition_layout_providers_are_canonical_extensible_and_reject_second_roots() {
    use ferroui_designer_core::Document;
    let registry = super::layout_registry();
    let source = Document::new(
        "<ControlTemplate xmlns='https://github.com/ferroui'><Border/></ControlTemplate>",
    )
    .unwrap();
    let root = source.ast().root();
    let adapter = registry.adapter_for(&source, root).unwrap();
    assert!(adapter.capabilities().move_items);
    assert!(adapter.capabilities().resize_items);
    assert!(!adapter.capabilities().reorder);
    assert!(!adapter.capabilities().drop_items);
    let bounds = [(
        root,
        ferroui_designer_tools::Rect::new(0.0, 0.0, 200.0, 100.0),
    )]
    .into();
    let context = ferroui_designer_tools::LayoutContext::new(&source, &bounds);
    assert!(
        registry
            .plan(
                &context,
                LayoutAction::Drop {
                    parent: root,
                    source: "<Button/>".into(),
                    position: ferroui_designer_tools::Point::new(0.0, 0.0),
                }
            )
            .is_err()
    );
    let custom =
        Document::new("<ControlTemplate xmlns='urn:application'><Border/></ControlTemplate>")
            .unwrap();
    assert!(registry.adapter_for(&custom, custom.ast().root()).is_none());
}
