use super::*;
use crate::{DesignerWorkspace, LayerCommand, PreviewStatus};
use ferroui_base::Rect as NativeRect;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Border, ContentControl, Control, Window};

struct Fixture {
    session: DesignerSession,
    window: Ref<Window>,
}

impl Fixture {
    fn new(source: &str) -> Self {
        let host = ContentControl::new();
        let session = DesignerSession::new(source, host.clone()).unwrap();
        let window = Window::new();
        window.set_width(700.0);
        window.set_height(500.0);
        window.set_content(Some(Control::boxed(host)));
        window.show();
        let mut result = Self { session, window };
        result.layout();
        result
    }

    fn layout(&mut self) {
        self.window.update_layout();
        self.session.refresh_runtime_mapping();
        assert!(
            matches!(self.session.preview_status(), PreviewStatus::Ready { revision }
            if *revision == self.session.document().revision()),
            "{:?}",
            self.session.preview_status()
        );
    }

    fn id(&self, name: &str) -> NodeId {
        self.session.document().ast().find_name(name).unwrap()
    }

    fn bounds(&self, name: &str) -> NativeRect {
        self.session.preview.visual(self.id(name)).unwrap().bounds()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 0.01, "{actual} != {expected}");
}

#[test]
fn selective_reset_exposes_styles_and_is_one_undo_without_recording() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui" Width="500" Height="300">
  <Canvas.Styles><Style Selector="Border"><Setter Property="Width" Value="80" /></Style></Canvas.Styles>
  <!-- retained exactly -->
  <Border Name="First" Width='160' Height="40" Margin="5,6,7,8" Background="Red" />
  <Border Name="Second" Width="200" Height="60" Canvas.Left="240" Background="Blue" />
</Canvas>"#;
    let mut fixture = Fixture::new(source);
    let nodes = [fixture.id("First"), fixture.id("Second")];
    fixture.session.select(nodes);
    fixture.session.set_recording(true);
    let revision = fixture.session.document().revision();
    fixture
        .session
        .reset_selection_layout(LayoutReset::Width)
        .unwrap();
    fixture.layout();
    assert_eq!(fixture.session.document().revision(), revision + 1);
    assert_eq!(fixture.session.selection(), &nodes);
    assert!(fixture.session.timeline().tracks.is_empty());
    close(fixture.bounds("First").width, 80.0);
    close(fixture.bounds("Second").width, 80.0);
    assert!(fixture
        .session
        .document()
        .source()
        .contains("<!-- retained exactly -->"));
    assert!(fixture
        .session
        .document()
        .source()
        .contains("Margin=\"5,6,7,8\""));
    let revision = fixture.session.document().revision();
    fixture
        .session
        .reset_selection_layout(LayoutReset::Width)
        .unwrap();
    assert_eq!(
        fixture.session.document().revision(),
        revision,
        "resetting no local values is a no-op"
    );
    fixture.session.undo().unwrap();
    assert_eq!(fixture.session.document().source(), source);
    assert!(!fixture.session.can_undo());
}

#[test]
fn reset_aliases_and_base_owner_property_elements_use_native_property_identity() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::register();
    let source = r#"<Border xmlns="https://github.com/ferroui" xmlns:ui="using:FerroUI.Controls"
        xmlns:l="clr-namespace:FerroUI.Layout;assembly=FerroUI.Base" xmlns:foreign="using:Foreign.Controls"
        l:Layoutable.Width='140' Height="40" foreign:Layoutable.Width="foreign-width"
        foreign:Width="foreign-ordinary" Padding="8" Background="Red">
  <ui:Border.Margin><Thickness>4</Thickness></ui:Border.Margin>
  <l:Layoutable.HorizontalAlignment>Left</l:Layoutable.HorizontalAlignment>
  <foreign:Border.Margin><foreign:Thickness Value="keep" /></foreign:Border.Margin>
  <!-- keep content and spelling -->
  <TextBlock Text="Value" />
</Border>"#;
    let mut document = Document::new(source).unwrap();
    let element = document.ast().node(document.ast().root()).unwrap();
    let expected = LayoutReset::All
        .properties()
        .iter()
        .map(|name| native_property(name).unwrap())
        .collect::<Vec<_>>();
    let edits = reset_element(&document, element, Border::TYPE, &expected);
    assert_eq!(edits.len(), 4);
    document.apply("Reset native layout", edits).unwrap();
    let after = document.source();
    assert!(!after.contains("l:Layoutable.Width="));
    assert!(!after.contains("Height=\"40\""));
    assert!(!after.contains("<ui:Border.Margin>"));
    assert!(!after.contains("<l:Layoutable.HorizontalAlignment>"));
    for retained in [
        "foreign:Layoutable.Width=\"foreign-width\"",
        "foreign:Width=\"foreign-ordinary\"",
        "<foreign:Border.Margin><foreign:Thickness Value=\"keep\" /></foreign:Border.Margin>",
        "Padding=\"8\"",
        "Background=\"Red\"",
        "<!-- keep content and spelling -->",
        "<TextBlock Text=\"Value\" />",
    ] {
        assert!(after.contains(retained), "lost {retained}");
    }
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}

#[test]
fn reset_all_preserves_parent_placement_content_layout_and_resources() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Grid xmlns="https://github.com/ferroui" RowDefinitions="*,*">
  <Grid Name="Target" Grid.Row="1" Width="200" Height="140" MinWidth="40" MinHeight="20"
        MaxWidth="400" MaxHeight="300" Margin="8" HorizontalAlignment="Left" VerticalAlignment="Top"
        RowDefinitions="Auto,*" ColumnDefinitions="*,2*" RowSpacing="4" RenderTransform="rotate(8deg)">
    <Border Name="Content" Padding="12" Background="Red" />
  </Grid>
</Grid>"#;
    let mut fixture = Fixture::new(source);
    let target = fixture.id("Target");
    fixture.session.select([target]);
    fixture
        .session
        .reset_selection_layout(LayoutReset::All)
        .unwrap();
    let element = fixture.session.document().ast().node(target).unwrap();
    for property in LayoutReset::All.properties() {
        assert!(
            element.attribute(property).is_none(),
            "{property} was not reset"
        );
    }
    for (property, value) in [
        ("Grid.Row", "1"),
        ("RowDefinitions", "Auto,*"),
        ("ColumnDefinitions", "*,2*"),
        ("RowSpacing", "4"),
        ("RenderTransform", "rotate(8deg)"),
    ] {
        assert_eq!(element.attribute(property), Some(value));
    }
    assert!(fixture
        .session
        .document()
        .source()
        .contains("Padding=\"12\""));
    fixture.session.undo().unwrap();
    assert_eq!(fixture.session.document().source(), source);
}

#[test]
fn reset_removes_a_bound_property_element_atomically() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui"><Border Name="Target" Height="40"><Border.Width><Binding Path="Size" FallbackValue="120" /></Border.Width></Border></Canvas>"#;
    let mut fixture = Fixture::new(source);
    let target = fixture.id("Target");
    fixture.session.select([target]);
    fixture
        .session
        .reset_selection_layout(LayoutReset::Width)
        .unwrap();
    assert!(!fixture
        .session
        .document()
        .source()
        .contains("<Border.Width>"));
    fixture.session.undo().unwrap();
    assert_eq!(fixture.session.document().source(), source);
}

#[test]
fn reset_rejects_locked_hidden_readonly_invalid_and_outside_scope_targets_without_edits() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><Canvas.Resources><ControlTemplate x:Key="Template" TargetType="Button"><Border Name="Part" Width="90" /></ControlTemplate></Canvas.Resources><Border Name="Target" Width="120" Height="40" /></Canvas>"#;
    let mut fixture = Fixture::new(source);
    let target = fixture.id("Target");
    fixture.session.select([target]);
    for command in [LayerCommand::ToggleLock, LayerCommand::ToggleHidden] {
        fixture.session.apply_layer_command(command).unwrap();
        assert!(fixture
            .session
            .reset_selection_layout(LayoutReset::All)
            .is_err());
        assert_eq!(fixture.session.document().source(), source);
        fixture.session.apply_layer_command(command).unwrap();
    }
    fixture.session.set_read_only(true);
    assert!(fixture
        .session
        .reset_selection_layout(LayoutReset::All)
        .is_err());
    fixture.session.set_read_only(false);
    let definition = fixture
        .session
        .document()
        .ast()
        .elements()
        .find(|node| node.local_name() == "ControlTemplate")
        .unwrap()
        .id;
    fixture.session.enter_definition_scope(definition).unwrap();
    fixture.session.select([target]);
    assert!(fixture
        .session
        .reset_selection_layout(LayoutReset::All)
        .is_err());
    assert_eq!(fixture.session.document().source(), source);
    assert!(!fixture.session.can_undo());
    fixture.session.update_source("<invalid");
    assert!(fixture
        .session
        .reset_selection_layout(LayoutReset::All)
        .is_err());
    assert_eq!(fixture.session.document().source(), "<invalid");
}

const MATCH_SOURCE: &str = r#"<Canvas xmlns="https://github.com/ferroui" Width="500" Height="300" UseLayoutRounding="False">
  <Border Name="Reference" Width="130" Height="70" Canvas.Left="20" Canvas.Top="15" RenderTransform="rotate(25deg)" UseLayoutRounding="False" />
  <Border Name="First" Width="60" Height="40" Canvas.Right="30" Canvas.Bottom="20" UseLayoutRounding="False" />
  <Border Name="Second" Width="90" Height="50" Canvas.Left="180" Canvas.Top="100" UseLayoutRounding="False" />
</Canvas>"#;

#[test]
fn match_size_uses_logical_reference_dimensions_and_normalizes_canvas_anchors_once() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new(MATCH_SOURCE);
    let nodes = [
        fixture.id("Reference"),
        fixture.id("First"),
        fixture.id("Second"),
    ];
    let first = fixture.bounds("First");
    let second = fixture.bounds("Second");
    fixture.session.select(nodes);
    fixture.session.set_recording(true);
    fixture
        .session
        .match_selection_size(LayoutMatch::Size)
        .unwrap();
    fixture.layout();
    for (name, before) in [("First", first), ("Second", second)] {
        let after = fixture.bounds(name);
        close(after.width, 130.0);
        close(after.height, 70.0);
        close(after.x, before.x);
        close(after.y, before.y);
    }
    assert_eq!(fixture.session.selection(), &nodes);
    assert!(fixture.session.timeline().tracks.is_empty());
    let reference = fixture.session.document().ast().node(nodes[0]).unwrap();
    assert_eq!(
        reference.attribute("RenderTransform"),
        Some("rotate(25deg)")
    );
    let first = fixture.session.document().ast().node(nodes[1]).unwrap();
    assert!(first.attribute("Canvas.Right").is_none());
    assert!(first.attribute("Canvas.Bottom").is_none());
    fixture.session.undo().unwrap();
    assert_eq!(fixture.session.document().source(), MATCH_SOURCE);
    assert!(!fixture.session.can_undo());
}

#[test]
fn single_axis_match_preserves_other_dimension_and_canvas_anchor() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    for mode in [LayoutMatch::Width, LayoutMatch::Height] {
        let mut fixture = Fixture::new(MATCH_SOURCE);
        let reference = fixture.id("Reference");
        let target = fixture.id("First");
        let before = fixture.bounds("First");
        fixture.session.select([reference, target]);
        fixture.session.match_selection_size(mode).unwrap();
        fixture.layout();
        let after = fixture.bounds("First");
        let element = fixture.session.document().ast().node(target).unwrap();
        if mode == LayoutMatch::Width {
            close(after.width, 130.0);
            close(after.height, before.height);
            assert_eq!(element.attribute("Height"), Some("40"));
            assert_eq!(element.attribute("Canvas.Bottom"), Some("20"));
            assert!(element.attribute("Canvas.Top").is_none());
        } else {
            close(after.height, 70.0);
            close(after.width, before.width);
            assert_eq!(element.attribute("Width"), Some("60"));
            assert_eq!(element.attribute("Canvas.Right"), Some("30"));
            assert!(element.attribute("Canvas.Left").is_none());
        }
        close(after.x, before.x);
        close(after.y, before.y);
        fixture.session.undo().unwrap();
        assert_eq!(fixture.session.document().source(), MATCH_SOURCE);
    }
}

#[test]
fn single_axis_match_keeps_untouched_native_bindings_active() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    for anchor in [
        "Canvas.Top='{Binding Y, FallbackValue=50}'",
        "Canvas.Bottom='{Binding Bottom, FallbackValue=20}'",
    ] {
        let source = format!("<Canvas xmlns='https://github.com/ferroui' Width='500' Height='300'><Border Name='Reference' Width='130' Height='70'/><Border Name='Target' Width='60' Height='{{Binding H, FallbackValue=40}}' Canvas.Left='20' {anchor}/></Canvas>");
        let mut fixture = Fixture::new(&source);
        let reference = fixture.id("Reference");
        let target = fixture.id("Target");
        let before = fixture.bounds("Target");
        fixture.session.select([reference, target]);
        fixture
            .session
            .match_selection_size(LayoutMatch::Width)
            .unwrap();
        fixture.layout();
        let after = fixture.bounds("Target");
        close(after.width, 130.0);
        close(after.height, before.height);
        close(after.y, before.y);
        assert!(fixture.session.document().source().contains(anchor));
        let layout = fixture
            .session
            .preview
            .visual(target)
            .unwrap()
            .cast::<Layoutable>()
            .unwrap();
        assert!(BindingOperations::get_binding_expression_base(
            &layout,
            Layoutable::height_property().as_property()
        )
        .is_some());
        fixture.session.undo().unwrap();
        assert_eq!(fixture.session.document().source(), source);
    }
    let source = "<Canvas xmlns='https://github.com/ferroui' Width='500' Height='300'><Border Name='Reference' Width='130' Height='70'/><Border Name='Target' Width='{Binding W, FallbackValue=60}' Height='40' Canvas.Right='{Binding Right, FallbackValue=20}' Canvas.Top='30'/></Canvas>";
    let mut fixture = Fixture::new(source);
    let reference = fixture.id("Reference");
    let target = fixture.id("Target");
    fixture.session.select([reference, target]);
    fixture
        .session
        .match_selection_size(LayoutMatch::Height)
        .unwrap();
    fixture.layout();
    close(fixture.bounds("Target").height, 70.0);
    assert!(fixture
        .session
        .document()
        .source()
        .contains("Canvas.Right='{Binding Right, FallbackValue=20}'"));
    let layout = fixture
        .session
        .preview
        .visual(target)
        .unwrap()
        .cast::<Layoutable>()
        .unwrap();
    assert!(BindingOperations::get_binding_expression_base(
        &layout,
        Layoutable::width_property().as_property()
    )
    .is_some());
    fixture.session.undo().unwrap();
    assert_eq!(fixture.session.document().source(), source);
}

#[test]
fn match_size_removes_aliased_trailing_canvas_anchors_in_the_same_transaction() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = "<Canvas xmlns='https://github.com/ferroui' xmlns:ui='using:FerroUI.Controls' Width='500' Height='300'><Border Name='Reference' Width='130' Height='70'/><Border Name='Target' Width='60' Height='40' ui:Canvas.Right='20' ui:Canvas.Bottom='30'/></Canvas>";
    let mut fixture = Fixture::new(source);
    let reference = fixture.id("Reference");
    let target = fixture.id("Target");
    let before = fixture.bounds("Target");
    fixture.session.select([reference, target]);
    fixture
        .session
        .match_selection_size(LayoutMatch::Size)
        .unwrap();
    fixture.layout();
    let after = fixture.bounds("Target");
    close(after.x, before.x);
    close(after.y, before.y);
    close(after.width, 130.0);
    close(after.height, 70.0);
    let element = fixture.session.document().ast().node(target).unwrap();
    assert!(element.attribute("ui:Canvas.Right").is_none());
    assert!(element.attribute("ui:Canvas.Bottom").is_none());
    fixture.session.undo().unwrap();
    assert_eq!(fixture.session.document().source(), source);
}

#[test]
fn match_size_keeps_grid_alignment_allocation_and_preserves_aliased_dimensions() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Grid xmlns="https://github.com/ferroui" xmlns:l="using:FerroUI.Layout" Width="500" Height="300" UseLayoutRounding="False">
      <Border Name="Reference" Width="140" Height="80" HorizontalAlignment="Left" VerticalAlignment="Top" />
      <Border Name="Target" l:Layoutable.Width="70" Height="50" HorizontalAlignment="Center" VerticalAlignment="Center" Margin="20,30,40,50" />
    </Grid>"#;
    let mut fixture = Fixture::new(source);
    let before = fixture.bounds("Target");
    let reference = fixture.id("Reference");
    let target = fixture.id("Target");
    fixture.session.select([reference, target]);
    fixture
        .session
        .match_selection_size(LayoutMatch::Size)
        .unwrap();
    fixture.layout();
    let after = fixture.bounds("Target");
    close(after.width, 140.0);
    close(after.height, 80.0);
    close(after.x, before.x);
    close(after.y, before.y);
    let element = fixture.session.document().ast().node(target).unwrap();
    assert_eq!(element.attribute("l:Layoutable.Width"), Some("140"));
    assert!(element.attribute("Width").is_none());
    assert_eq!(element.attribute("HorizontalAlignment"), Some("Center"));
    fixture.session.undo().unwrap();
    assert_eq!(fixture.session.document().source(), source);
}

#[test]
fn match_rejects_any_constrained_or_bound_target_before_changing_the_selection() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    for attribute in [
        "Width=\"{Binding Width, FallbackValue=60}\"",
        "Width=\"60\" MaxWidth=\"100\"",
        "Width=\"60\" MinWidth=\"160\"",
    ] {
        let source = format!(
            r#"<Canvas xmlns="https://github.com/ferroui" Width="500" Height="300"><Border Name="Reference" Width="130" Height="70" /><Border Name="Valid" Width="40" Height="20" /><Border Name="Blocked" {attribute} Height="40" /></Canvas>"#
        );
        let mut fixture = Fixture::new(&source);
        let nodes = [
            fixture.id("Reference"),
            fixture.id("Valid"),
            fixture.id("Blocked"),
        ];
        fixture.session.select(nodes);
        assert!(fixture
            .session
            .match_selection_size(LayoutMatch::Size)
            .is_err());
        assert_eq!(fixture.session.document().source(), source);
        assert!(!fixture.session.can_undo());
    }
}

#[test]
fn matching_requires_measured_independent_controls_and_leaves_equal_sizes_unmodified() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui"><Border Name="Outer" Width="130" Height="70"><Border Name="Inner" Width="60" Height="40" /></Border><Border Name="Same" Width="130" Height="70" /></Canvas>"#;
    let mut fixture = Fixture::new(source);
    let outer = fixture.id("Outer");
    let inner = fixture.id("Inner");
    let same = fixture.id("Same");
    fixture.session.select([outer, inner]);
    assert!(fixture
        .session
        .match_selection_size(LayoutMatch::Size)
        .is_err());
    fixture.session.select([outer, same]);
    let revision = fixture.session.document().revision();
    fixture
        .session
        .match_selection_size(LayoutMatch::Size)
        .unwrap();
    assert_eq!(fixture.session.document().revision(), revision);
    assert!(!fixture.session.can_undo());
    fixture
        .session
        .preview
        .visual(same)
        .unwrap()
        .cast::<Layoutable>()
        .unwrap()
        .invalidate_measure();
    assert!(fixture
        .session
        .match_selection_size(LayoutMatch::Size)
        .is_err());
    assert_eq!(fixture.session.document().source(), source);
}

#[test]
fn viewer_can_plan_layout_but_both_mutators_reject_even_empty_plans() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui" Width="500" Height="300"><Border Name="Reference" Width="130" Height="70"/><Border Name="Same" Width="130" Height="70"/><Border Name="Smaller" Width="60" Height="40"/></Canvas>"#;
    let mut fixture = Fixture::new(source);
    let reference = fixture.id("Reference");
    let same = fixture.id("Same");
    let smaller = fixture.id("Smaller");
    fixture.session.select([reference, same]);
    fixture.session.set_read_only(true);
    let generation = fixture.session.generation();
    let revision = fixture.session.document().revision();
    assert!(!fixture
        .session
        .plan_layout_reset(&[reference, same], LayoutReset::Width)
        .unwrap()
        .is_empty());
    assert!(fixture
        .session
        .plan_layout_reset(&[reference, same], LayoutReset::Margin)
        .unwrap()
        .is_empty());
    assert!(fixture
        .session
        .plan_match_selection_size(LayoutMatch::Size)
        .unwrap()
        .is_empty());
    assert_eq!(fixture.session.generation(), generation);
    for reset in [LayoutReset::Width, LayoutReset::Margin] {
        assert!(fixture
            .session
            .reset_selection_layout(reset)
            .unwrap_err()
            .contains("read-only"));
    }
    assert!(fixture
        .session
        .match_selection_size(LayoutMatch::Size)
        .unwrap_err()
        .contains("read-only"));
    fixture.session.select([reference, smaller]);
    assert!(!fixture
        .session
        .plan_match_selection_size(LayoutMatch::Width)
        .unwrap()
        .is_empty());
    assert!(fixture
        .session
        .match_selection_size(LayoutMatch::Width)
        .unwrap_err()
        .contains("read-only"));
    assert_eq!(fixture.session.document().revision(), revision);
    assert_eq!(fixture.session.document().source(), source);
    assert!(!fixture.session.can_undo());
}

#[test]
fn workspace_commands_reject_pending_source_and_keep_single_undo() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui"><Border Name="Target" Width="120" Height="40" /></Canvas>"#;
    let workspace = DesignerWorkspace::new(source).unwrap();
    let session = workspace.session();
    let target = session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    session.borrow_mut().select([target]);
    workspace
        .reset_selection_layout(LayoutReset::Width)
        .unwrap();
    assert!(session
        .borrow()
        .document()
        .ast()
        .node(target)
        .unwrap()
        .attribute("Width")
        .is_none());
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().document().source(), source);
    // A source editor change is staged, never implicitly applied by an Arrange command.
    let source_editor = workspace.source_editor();
    source_editor.set_text(Some("<invalid"));
    let revision = session.borrow().document().revision();
    assert!(workspace.reset_selection_layout(LayoutReset::All).is_err());
    assert_eq!(session.borrow().document().source(), "<invalid");
    assert_eq!(session.borrow().document().revision(), revision);
}
