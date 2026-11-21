use super::*;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{ContentControl, Control, Window};

fn arrange(source: &str, rust: bool) -> (Ref<Window>, DesignerSession) {
    let host = ContentControl::new();
    let mut session = if rust {
        DesignerSession::with_rust_source(source, host.clone(), "file:///LayoutReview.rs".into())
    } else {
        DesignerSession::with_uri(source, host.clone(), "file:///LayoutReview.xaml".into())
    }
    .unwrap();
    let window = Window::new();
    window.set_width(720.0);
    window.set_height(460.0);
    window.set_content(Some(Control::boxed(host)));
    window.show();
    window.update_layout();
    session.refresh_runtime_mapping();
    (window, session)
}

fn issue<'a>(findings: &'a [PreviewFinding], name: &str, code: &str) -> &'a PreviewFinding {
    findings
        .iter()
        .find(|finding| finding.name.as_deref() == Some(name) && finding.code == code)
        .unwrap_or_else(|| panic!("Find {name} {code} in {findings:#?}"))
}

#[test]
fn diagnoses_actual_panel_slot_and_clip_edges_without_warning_on_scrollable_content() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui" Width="650" Height="400" UseLayoutRounding="False">
      <StackPanel Width="150" Height="45" Orientation="Horizontal" Canvas.Left="20">
        <Border Width="96" Height="30" />
        <Border Name="TooWide" Width="90" Height="30" />
      </StackPanel>
      <Grid Width="220" Height="50" ColumnDefinitions="80,*" Canvas.Top="60" Canvas.Left="20">
        <Border Name="WideSlot" Width="120" Height="35" HorizontalAlignment="Left" />
      </Grid>
      <Canvas Width="160" Height="60" ClipToBounds="True" Canvas.Top="130" Canvas.Left="20">
        <Border Name="Clipped" Width="55" Height="24" Canvas.Left="141" Canvas.Top="46" />
      </Canvas>
      <ScrollViewer Width="100" Height="80" Canvas.Top="220" Canvas.Left="20"
                    HorizontalScrollBarVisibility="Auto" VerticalScrollBarVisibility="Disabled">
        <Canvas Width="240" Height="40">
          <Border Name="Reachable" Width="38" Height="28" Canvas.Left="150" />
          <Border Name="OffAxis" Width="38" Height="38" Canvas.Left="150" Canvas.Top="66" />
        </Canvas>
      </ScrollViewer>
      <Canvas Width="120" Height="45" ClipToBounds="True" Canvas.Left="280">
        <Border Name="Translated" Width="50" Height="28" Canvas.Left="55" RenderTransform="translate(25px,0px)" />
      </Canvas>
    </Canvas>"#;
    let (window, mut session) = arrange(source, false);
    let report = session.preview_checks().unwrap();
    let stack = issue(&report, "TooWide", "layoutOverflow")
        .overflow
        .as_ref()
        .unwrap();
    assert_eq!(stack.boundary, "the StackPanel bounds");
    assert_eq!(stack.right, 36.0);
    let slot = issue(&report, "WideSlot", "layoutOverflow")
        .overflow
        .as_ref()
        .unwrap();
    assert_eq!(slot.boundary, "the Grid layout slot");
    assert_eq!(slot.right, 40.0);
    let clip = issue(&report, "Clipped", "clippedLayout")
        .overflow
        .as_ref()
        .unwrap();
    assert_eq!((clip.right, clip.bottom), (36.0, 10.0));
    let transformed = issue(&report, "Translated", "clippedLayout");
    assert_eq!(transformed.overflow.as_ref().unwrap().right, 10.0);
    assert!(
        !report
            .iter()
            .any(|finding| finding.name.as_deref() == Some("Reachable")),
        "{report:#?}"
    );
    let off_axis = issue(&report, "OffAxis", "clippedLayout")
        .overflow
        .as_ref()
        .unwrap();
    assert_eq!(off_axis.right, 0.0);
    assert!(off_axis.bottom > 0.0);
    assert_eq!(session.document().source(), source);
    assert!(!session.can_undo());
    session.update_source("<Canvas");
    assert!(session.preview_checks().is_err());
    window.close();
}

#[test]
fn ineffective_values_link_to_authored_properties_and_use_native_precedence() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui" xmlns:ui="https://github.com/ferroui"
                            Width="650" Height="400" UseLayoutRounding="False">
      <Border Name="Anchored" Canvas.Left="20" ui:Canvas.Right="7" Canvas.Top="4" Canvas.Bottom="9"
              Width="42" MinWidth="64" Height="60" MaxHeight="40" />
      <DockPanel Width="280" Height="80" Canvas.Top="80" LastChildFill="True">
        <Border Name="Leading" DockPanel.Dock="Left" Width="40" />
        <Border Name="Fills" DockPanel.Dock="Right" />
      </DockPanel>
      <StackPanel Width="300" Height="60" Canvas.Top="180">
        <Border Name="WrongPanel" Canvas.Left="20" Height="38" />
      </StackPanel>
      <Grid Width="310" Height="64" Canvas.Top="260" RowDefinitions="Auto,*" ColumnDefinitions="*,*">
        <Border Name="Tracks" Grid.Row="7" Grid.Column="1" Grid.RowSpan="3" Grid.ColumnSpan="4" Height="18" />
      </Grid>
    </Canvas>"#;
    let (window, session) = arrange(source, false);
    let report = session.preview_checks().unwrap();
    let authored = |name, property| {
        let issue = report
            .iter()
            .find(|finding| {
                finding.name.as_deref() == Some(name)
                    && finding.property == Some(property)
                    && finding.code == "ineffectiveLayoutValue"
            })
            .unwrap_or_else(|| panic!("Find {name} {property} in {report:#?}"));
        &source[issue.source.unwrap().range()]
    };
    assert_eq!(
        authored("Anchored", "Canvas.Right"),
        "ui:Canvas.Right=\"7\""
    );
    assert_eq!(authored("Anchored", "Canvas.Bottom"), "Canvas.Bottom=\"9\"");
    assert_eq!(authored("Anchored", "Width"), "Width=\"42\"");
    assert_eq!(authored("Anchored", "Height"), "Height=\"60\"");
    assert_eq!(
        authored("Fills", "DockPanel.Dock"),
        "DockPanel.Dock=\"Right\""
    );
    assert!(
        !report
            .iter()
            .any(|finding| finding.name.as_deref() == Some("Leading"))
    );
    assert_eq!(authored("WrongPanel", "Canvas.Left"), "Canvas.Left=\"20\"");
    assert_eq!(authored("Tracks", "Grid.Row"), "Grid.Row=\"7\"");
    assert_eq!(authored("Tracks", "Grid.RowSpan"), "Grid.RowSpan=\"3\"");
    assert_eq!(
        authored("Tracks", "Grid.ColumnSpan"),
        "Grid.ColumnSpan=\"4\""
    );
    window.close();
}

#[test]
fn studio_checks_preserve_rust_value_spans_and_other_source() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = concat!(
        "fn queue() -> Element {\r\n",
        "    rsx! { Canvas { width: 360.0, height: 140.0,\r\n",
        "        // 🌿 Keep the comment and the neighboring expression.\r\n",
        "        Border { name: \"Queue\", width: 44.0, height: 40.0, Canvas.left: 338.0, Canvas.right: 14.0 }\r\n",
        "        TextBlock { text: status() }\r\n",
        "    } }\r\n",
        "}\r\n",
    );
    let (window, session) = arrange(source, true);
    let report = session.preview_checks().unwrap();
    let issue = issue(&report, "Queue", "ineffectiveLayoutValue");
    assert_eq!(issue.property, Some("Canvas.Right"));
    assert_eq!(&source[issue.source.unwrap().range()], "14.0");
    assert_eq!(session.authored_source(), source);
    assert!(!session.can_undo());
    window.close();
}
