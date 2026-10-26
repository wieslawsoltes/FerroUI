use super::*;
use ferroui_base::{Matrix, Point, Rect};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{ContentControl, Control, UserControl, Window};
use std::rc::Rc;

fn point_close(actual: Point, expected: Point) {
    assert!(
        (actual.x - expected.x).abs() < 1e-5 && (actual.y - expected.y).abs() < 1e-5,
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn affine_pointer_resize_and_rotation_match_actual_rendered_coordinates() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let host = ContentControl::new();
    let source = r#"<Canvas xmlns="https://github.com/ferroui" Width="700" Height="500" UseLayoutRounding="False"><Canvas Name="Group" Canvas.Left="300" Canvas.Top="70" Width="240" Height="200" RenderTransformOrigin="0,0" RenderTransform="matrix(-1.3,0.35,0.6,0.9,0,0)"><Button Name="Target" Width="80" Height="50" Canvas.Left="35" Canvas.Top="20" RenderTransformOrigin="20%,80%" RenderTransform="matrix(1.1,0.25,-0.3,0.75,0,0)" /></Canvas></Canvas>"#;
    let mut session = DesignerSession::new(source, host.clone()).unwrap();
    let window = Window::new();
    window.set_width(700.0);
    window.set_height(500.0);
    window.set_content(Some(Control::boxed(&host)));
    window.show();
    window.update_layout();
    let id = session.document().ast().find_name("Target").unwrap();
    let before = session
        .preview
        .visual(id)
        .unwrap()
        .transform_to_visual(&host)
        .unwrap();
    let resized = Rect::new(-10.0, -5.0, 110.0, 70.0);
    crate::workspace_tools::commit(
        &mut session,
        DesignGesture::ResizeLocal {
            node: id,
            after: resized,
        },
    )
    .unwrap();
    window.update_layout();
    let visual = session.preview.visual(id).unwrap();
    let after = visual.transform_to_visual(&host).unwrap();
    for point in [
        Point::default(),
        Point::new(110.0, 70.0),
        Point::new(110.0, 0.0),
    ] {
        point_close(
            point.transform(after),
            Point::new(point.x - 10.0, point.y - 5.0).transform(before),
        );
    }
    let geometry = crate::affine_manipulation::Geometry::capture(&visual, &host).unwrap();
    let rotation = Matrix::create_rotation_at(0.4, geometry.pivot);
    crate::workspace_tools::commit(
        &mut session,
        DesignGesture::Rotate {
            node: id,
            degrees: 0.4f64.to_degrees(),
        },
    )
    .unwrap();
    window.update_layout();
    let final_matrix = session
        .preview
        .visual(id)
        .unwrap()
        .transform_to_visual(&host)
        .unwrap();
    for point in [
        Point::default(),
        Point::new(110.0, 70.0),
        Point::new(25.0, 19.0),
    ] {
        point_close(
            point.transform(final_matrix),
            point.transform(after * rotation),
        );
    }
    assert!(matches!(
        session.preview_status(),
        PreviewStatus::Ready { .. }
    ));
    window.close();
}

#[test]
fn presence_pointer_coordinates_ignore_viewport_zoom_and_clear_on_exit_or_interaction() {
    use ferroui_base::input::{
        InputElement, KeyModifiers, Pointer, PointerEventArgs, PointerPointProperties, PointerType,
        PointerUpdateKind, RawInputModifiers,
    };
    use ferroui_base::interactivity::RoutedEvent;
    use ferroui_base::media::MatrixTransform;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let surface = DesignSurface::new();
    surface.set_width(200.0);
    surface.set_height(100.0);
    surface.set_zoom(2.0);
    let zoom = ferroui_controls::LayoutTransformControl::new();
    zoom.set_layout_transform(Some(
        MatrixTransform::with_matrix(Matrix::create_scale(2.0, 2.0)).into(),
    ));
    zoom.set_child(Some(surface.clone().upcast::<Control>()));
    let window = Window::new();
    window.set_width(640.0);
    window.set_height(480.0);
    window.set_content(Some(Control::boxed(zoom)));
    window.show();
    window.update_layout();
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let send = |event: &RoutedEvent<PointerEventArgs>, local: Point| {
        let position = local.transform(surface.transform_to_visual(&window).unwrap());
        surface.raise_event(&PointerEventArgs::new(
            Some(event),
            surface.clone().upcast::<ferroui_base::FerroObject>(),
            pointer.clone(),
            Some(&window),
            position,
            1,
            PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
            KeyModifiers::NONE,
        ));
    };
    let point = Point::new(33.5, 27.25);
    send(InputElement::pointer_moved_event(), point);
    point_close(surface.cursor_position().unwrap(), point);
    send(InputElement::pointer_exited_event(), point);
    assert_eq!(surface.cursor_position(), None);
    send(InputElement::pointer_moved_event(), point);
    surface.set_interactive(true);
    assert_eq!(surface.cursor_position(), None);
    send(InputElement::pointer_moved_event(), point);
    assert_eq!(surface.cursor_position(), None);
    window.close();
}

#[test]
fn rotation_preserves_property_element_transform_and_namespace_inheritance() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let host = ContentControl::new();
    let mut session=DesignerSession::new(r#"<Canvas xmlns="https://github.com/ferroui" Width="300" Height="200"><Button Name="Target" Width="80" Height="40"><Button.RenderTransform><ScaleTransform ScaleX="2" ScaleY="0.5" /></Button.RenderTransform></Button></Canvas>"#,host.clone()).unwrap();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&host)));
    window.show();
    window.update_layout();
    let id = session.document().ast().find_name("Target").unwrap();
    crate::workspace_tools::commit(
        &mut session,
        DesignGesture::Rotate {
            node: id,
            degrees: 22.0,
        },
    )
    .unwrap();
    assert!(
        session
            .document()
            .source()
            .contains("<ScaleTransform ScaleX=\"2\"")
    );
    assert!(session.document().source().contains("<TransformGroup>"));
    assert_eq!(session.document().source().matches("xmlns=").count(), 1);
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    window.close();
}

#[test]
fn registered_root_factory_populates_real_instances_and_survives_document_switches() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<UserControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" x:Class="Example.RealView"><Button Name="Target" Content="First" /></UserControl>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    let count = Rc::new(std::cell::Cell::new(0));
    let capture = count.clone();
    let clicks = Rc::new(std::cell::Cell::new(0));
    let click_capture = clicks.clone();
    let mut options = PreviewHostOptions::default();
    options.register_root(
        "Example.RealView",
        Rc::new(move |request| {
            capture.set(capture.get() + 1);
            let root = UserControl::new();
            let loaded = request.populate(Some(Control::boxed(root.clone())))?;
            let clicks = click_capture.clone();
            root.get_control::<ferroui_controls::Button>("Target")
                .click(move |_, _| clicks.set(clicks.get() + 1));
            Ok(loaded)
        }),
    );
    session.configure_preview(options).unwrap();
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    assert_eq!(count.get(), 1);
    let id = session.document().ast().find_name("Target").unwrap();
    session
        .preview
        .visual(id)
        .unwrap()
        .cast::<ferroui_controls::Button>()
        .unwrap()
        .raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(
            ferroui_controls::Button::click_event(),
        ));
    assert_eq!(clicks.get(), 1);
    session.set_property(&[id], "Content", "Second").unwrap();
    assert!(count.get() >= 2);
    session
        .open_document(source.into(), "file:///tmp/AnotherFactoryView.xaml".into())
        .unwrap();
    assert!(count.get() >= 3);
    // Reopening the current URI and changing factories must reuse its sole reload owner.
    session
        .open_document(source.into(), "file:///tmp/AnotherFactoryView.xaml".into())
        .unwrap();
    assert!(matches!(
        session.preview_status(),
        PreviewStatus::Ready { .. }
    ));
    let accepted = session.preview.root().unwrap();
    let mut rejected = PreviewHostOptions::default();
    rejected.register_root(
        "Example.RealView",
        Rc::new(|_| Err("factory rejected".into())),
    );
    assert!(
        session
            .configure_preview(rejected)
            .unwrap_err()
            .contains("factory rejected")
    );
    assert!(std::ptr::eq(&*accepted, &*session.preview.root().unwrap()));
    let id = session.document().ast().find_name("Target").unwrap();
    session.set_property(&[id], "Content", "Recovered").unwrap();
    assert!(matches!(
        session.preview_status(),
        PreviewStatus::Ready { .. }
    ));
    assert!(
        session
            .preview
            .root()
            .unwrap()
            .cast::<UserControl>()
            .is_some()
    );
}

#[test]
fn native_window_preview_uses_real_title_and_window_properties() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session=DesignerSession::new(r#"<Window xmlns="https://github.com/ferroui" Title="Native title" Width="420" Height="270" CanResize="False"><Button Content="Native" /></Window>"#,ContentControl::new()).unwrap();
    let window = session.open_native_preview().unwrap();
    assert_eq!(window.title().as_deref(), Some("Native title"));
    assert!(!window.can_resize());
    assert!(window.is_visible());
    session.close_native_preview();
    assert!(!window.is_visible());
}

#[test]
fn resource_documents_use_native_design_preview_with() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let session=DesignerSession::new(r#"<ResourceDictionary xmlns="https://github.com/ferroui"><Design.PreviewWith><Button Content="Resource preview" Width="160" /></Design.PreviewWith></ResourceDictionary>"#,ContentControl::new()).unwrap();
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    let preview = session
        .preview
        .root()
        .unwrap()
        .cast::<ferroui_controls::Button>()
        .expect("Design.PreviewWith must create the authored preview control");
    assert_eq!(preview.width(), 160.0);
}

#[test]
fn restoring_invalid_tab_loads_its_own_last_valid_preview() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<TextBlock xmlns="https://github.com/ferroui" Text="Draft tab" />"#;
    let mut draft = ferroui_designer_core::Document::new(source).unwrap();
    draft.update_source("Incomplete edit", source.replace(" />", " "));
    let mut session = DesignerSession::new(
        r#"<TextBlock xmlns="https://github.com/ferroui" Text="Previous tab" />"#,
        ContentControl::new(),
    )
    .unwrap();
    for uri in [
        "file:///tmp/InvalidDraftTab.xaml",
        "file:///tmp/InvalidDraftTab.xaml",
    ] {
        session.restore_document(draft.clone(), uri.into()).unwrap();
        assert!(matches!(
            session.preview_status(),
            PreviewStatus::InvalidSource { .. }
        ));
        assert!(!session.can_manipulate());
        let text = session
            .preview
            .root()
            .unwrap()
            .cast::<ferroui_controls::TextBlock>()
            .unwrap();
        assert_eq!(text.text().as_deref(), Some("Draft tab"));
        assert_eq!(session.document().source(), draft.source());
    }
}

#[test]
fn project_tabs_retain_history_and_source_completion_is_registered() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let path = std::env::temp_dir().join(format!(
        "ferroui-designer-project-ui-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).unwrap();
    let a = path.join("A.xaml");
    let b = path.join("B.xaml");
    let source = r#"<Button xmlns="https://github.com/ferroui" Name="Target" Width="100" />"#;
    std::fs::write(&a, source).unwrap();
    std::fs::write(&b, source).unwrap();
    let workspace = DesignerWorkspace::new(source).unwrap();
    workspace
        .set_solution(ferroui_designer_project::Solution::discover(&path).unwrap())
        .unwrap();
    let a_uri = ferroui_designer_project::file_uri(&a).unwrap();
    let b_uri = ferroui_designer_project::file_uri(&b).unwrap();
    workspace.activate_document(&a_uri).unwrap();
    let id = workspace
        .session()
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    workspace
        .session()
        .borrow_mut()
        .set_property(&[id], "Width", "150")
        .unwrap();
    workspace.activate_document(&b_uri).unwrap();
    workspace.activate_document(&a_uri).unwrap();
    assert!(workspace.session().borrow().can_undo());
    workspace.session().borrow_mut().undo().unwrap();
    assert_eq!(
        workspace
            .session()
            .borrow()
            .document()
            .ast()
            .node(id)
            .unwrap()
            .attribute("Width"),
        Some("100")
    );
    let mut language = workspace.language_service();
    language.open("untitled:completion", "<But", 1).unwrap();
    assert!(
        language
            .completions("untitled:completion", 4)
            .unwrap()
            .iter()
            .any(|item| item.label == "Button")
    );
    drop(language);
    drop(workspace);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn source_completion_handles_invalid_drafts_and_rejects_stale_or_read_only_edits() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(STARTER_XAML).unwrap();
    workspace.session().borrow_mut().stage_source("<But");
    let revision = workspace.session().borrow().document().revision();
    workspace
        .apply_source_completion(
            revision,
            ferroui_designer_core::SourceSpan::new(1, 4),
            "Button",
        )
        .unwrap();
    assert_eq!(workspace.session().borrow().document().source(), "<Button");
    assert!(!workspace.session().borrow().document().is_valid());
    assert!(
        workspace
            .apply_source_completion(revision, ferroui_designer_core::SourceSpan::new(0, 0), "x")
            .is_err()
    );
    let revision = workspace.session().borrow().document().revision();
    workspace.session().borrow_mut().set_read_only(true);
    assert!(
        workspace
            .apply_source_completion(
                revision,
                ferroui_designer_core::SourceSpan::new(7, 7),
                " />"
            )
            .is_err()
    );
    assert_eq!(workspace.session().borrow().document().source(), "<Button");
}

#[test]
fn workspace_refactoring_rejects_stale_buffers_atomically_and_close_restores_disk_index() {
    use ferroui_designer_language::{DocumentEdits, LineIndex, TextEdit, WorkspaceEdit};
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let directory = std::env::temp_dir().join(format!(
        "ferroui-designer-refactoring-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let source = r#"<Button xmlns="https://github.com/ferroui" Name="Target" Width="100" />"#;
    let a = directory.join("A.xaml");
    let b = directory.join("B.xaml");
    std::fs::write(&a, source).unwrap();
    std::fs::write(&b, source).unwrap();
    let a_uri = ferroui_designer_project::file_uri(&a).unwrap();
    let b_uri = ferroui_designer_project::file_uri(&b).unwrap();
    let workspace = DesignerWorkspace::new(STARTER_XAML).unwrap();
    workspace
        .set_solution(ferroui_designer_project::Solution::discover(&directory).unwrap())
        .unwrap();
    workspace.activate_document(&a_uri).unwrap();
    let (a_version, b_version) = {
        let language = workspace.language_service();
        (
            language.version(&a_uri).unwrap(),
            language.version(&b_uri).unwrap(),
        )
    };
    let start = source.find("Target").unwrap();
    let rename = TextEdit {
        range: LineIndex::new(source)
            .range(ferroui_designer_core::SourceSpan::new(start, start + 6))
            .unwrap(),
        new_text: "Renamed".into(),
    };
    let target = workspace
        .session()
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    workspace
        .session()
        .borrow_mut()
        .set_property(&[target], "Width", "200")
        .unwrap();
    let edit = WorkspaceEdit {
        document_changes: vec![
            DocumentEdits {
                uri: b_uri.clone(),
                version: b_version,
                edits: vec![rename.clone()],
            },
            DocumentEdits {
                uri: a_uri.clone(),
                version: a_version,
                edits: vec![rename],
            },
        ],
    };
    assert!(workspace.apply_language_edits(edit).is_err());
    assert!(
        !workspace.document_uris().contains(&b_uri),
        "A rejected refactoring must not open its candidate tabs"
    );
    assert!(
        workspace
            .session()
            .borrow()
            .document()
            .source()
            .contains("Width=\"200\"")
    );
    assert_eq!(
        workspace
            .language_service()
            .document(&b_uri)
            .unwrap()
            .source(),
        source
    );
    workspace.activate_document(&b_uri).unwrap();
    let dirty = source.replace("Target", "UnsavedName");
    workspace
        .session()
        .borrow_mut()
        .update_source(dirty.clone());
    workspace.synchronize_language_service().unwrap();
    assert_eq!(
        workspace
            .language_service()
            .document(&b_uri)
            .unwrap()
            .source(),
        dirty
    );
    workspace.close_document(&b_uri, true).unwrap();
    assert_eq!(
        workspace
            .language_service()
            .document(&b_uri)
            .unwrap()
            .source(),
        source
    );
    drop(workspace);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn sharing_panel_opens_without_starting_transport_or_mutating_the_document() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(STARTER_XAML).unwrap();
    let revision = workspace.session().borrow().document().revision();
    workspace.show_sharing();
    assert!(workspace.sharing_status().is_none());
    assert_eq!(workspace.session().borrow().document().revision(), revision);
    let host = workspace
        .control()
        .get_control::<ContentControl>("SharingHost");
    assert!(host.is_visible());
    assert!(host.content().is_some());
}

#[test]
fn styled_panel_orientation_drives_native_drop_without_becoming_a_local_value() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let host = ContentControl::new();
    let source = r#"<StackPanel xmlns="https://github.com/ferroui" Name="Flow" Width="400" Height="180"><StackPanel.Styles><Style Selector="StackPanel"><Setter Property="Orientation" Value="Horizontal" /></Style></StackPanel.Styles><Border Name="First" Width="80" Height="80" /><Border Name="Last" Width="80" Height="80" /></StackPanel>"#;
    let mut session = DesignerSession::new(source, host.clone()).unwrap();
    let window = Window::new();
    window.set_width(400.0);
    window.set_height(180.0);
    window.set_content(Some(Control::boxed(&host)));
    window.show();
    window.update_layout();
    crate::workspace_tools::commit(
        &mut session,
        DesignGesture::Insert {
            source: "<Border Name=\"Inserted\" Width=\"30\" Height=\"30\" />".into(),
            position: Point::new(80.0, 150.0),
        },
    )
    .unwrap();
    let document = session.document();
    let root = document.ast().root();
    assert_eq!(
        document.ast().node(root).unwrap().attribute("Orientation"),
        None
    );
    let names = document
        .ast()
        .element_children(root)
        .into_iter()
        .filter_map(|id| {
            document
                .ast()
                .node(id)
                .and_then(|node| node.xaml_name())
                .map(str::to_owned)
        })
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["First", "Inserted", "Last"]);
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    session.undo().unwrap();
    assert!(session.document().ast().find_name("Inserted").is_none());
    assert!(matches!(
        session.preview_status(),
        PreviewStatus::Ready { .. }
    ));
    session.redo().unwrap();
    assert!(session.document().ast().find_name("Inserted").is_some());
    assert!(matches!(
        session.preview_status(),
        PreviewStatus::Ready { .. }
    ));
    window.close();
}

#[test]
fn application_layout_factory_uses_runtime_fallback_for_move_resize_and_drop() {
    use ferroui_controls::StackPanel;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let host = ContentControl::new();
    let source = r#"<app:Flow xmlns="https://github.com/ferroui" xmlns:app="using:PreviewHost" Name="Flow" Orientation="Horizontal" Width="400" Height="180" UseLayoutRounding="False"><Button Name="Target" Width="80" Height="40" /></app:Flow>"#;
    let mut session = DesignerSession::new(source, host.clone()).unwrap();
    let mut options = PreviewHostOptions::default();
    options.register_root(
        "app:Flow",
        Rc::new(|request| {
            let request = PreviewRequest {
                source: request.source.replace("app:Flow", "StackPanel"),
                uri: request.uri.clone(),
                configuration: request.configuration.clone(),
            };
            request.populate(Some(Control::boxed(StackPanel::new())))
        }),
    );
    session.configure_preview(options).unwrap();
    let window = Window::new();
    window.set_width(400.0);
    window.set_height(180.0);
    window.set_content(Some(Control::boxed(&host)));
    window.show();
    window.update_layout();
    let root = session.document().ast().root();
    assert!(
        session
            .runtime_layout_capabilities(root)
            .unwrap()
            .drop_items
    );
    let target = session.document().ast().find_name("Target").unwrap();
    session.select(vec![target]);
    let before = session
        .preview
        .visual(target)
        .unwrap()
        .transform_to_visual(&host)
        .unwrap();
    crate::workspace_tools::commit(
        &mut session,
        DesignGesture::Move {
            nodes: vec![target],
            delta: Point::new(25.0, 15.0),
        },
    )
    .unwrap();
    window.update_layout();
    let after = session
        .preview
        .visual(target)
        .unwrap()
        .transform_to_visual(&host)
        .unwrap();
    point_close(
        Point::default().transform(after),
        Point::new(25.0, 15.0).transform(Matrix::create_translation(before.m31, before.m32)),
    );
    assert_eq!(
        session
            .document()
            .ast()
            .node(target)
            .unwrap()
            .attribute("Width"),
        Some("80")
    );
    crate::workspace_tools::commit(
        &mut session,
        DesignGesture::ResizeLocal {
            node: target,
            after: Rect::new(0.0, 0.0, 120.0, 60.0),
        },
    )
    .unwrap();
    window.update_layout();
    let resized = session
        .preview
        .visual(target)
        .unwrap()
        .transform_to_visual(&host)
        .unwrap();
    point_close(
        Point::new(80.0, 40.0).transform(resized),
        Point::new(120.0, 60.0).transform(after),
    );
    crate::workspace_tools::commit(
        &mut session,
        DesignGesture::Insert {
            source: "<Border Name=\"Added\" Width=\"20\" Height=\"20\" />".into(),
            position: Point::new(250.0, 120.0),
        },
    )
    .unwrap();
    assert!(session.document().ast().find_name("Added").is_some());
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    assert!(session.document().source().starts_with("<app:Flow"));
    window.close();
}
