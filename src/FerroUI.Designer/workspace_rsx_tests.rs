use super::{apply_keyframe, click, keyframe_editor, keyframe_input};
use crate::{DesignerWorkspace, PreviewStatus};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Border, Button, ComboBox, Control, Grid, StackPanel, TextBox, Window};
use ferroui_designer_animation::{Easing, KeyframeGroupEdit, KeyframeLocation};
use ferroui_designer_language::Schema;
use ferroui_designer_project::{MarkupCatalog, Solution, file_uri};
use ferroui_designer_rsx::RsxCatalog;
use std::path::PathBuf;
use std::sync::Arc;

const XAML: &str = r#"<Canvas xmlns="https://github.com/ferroui" />"#;
const GETTER: &str = "crate::radius";
const SOURCE: &str = r###"// Keep both views 🧭
fn first() { rsx! { TextBlock { text: "First" } } }
fn review() {
    rsx! {
        StackPanel {
            styles: [ferroui_view::style("Border#Review")
                .animation(ferroui_view::animation(ferroui_view::ms(2000))
                    .key_frame(0.0, |key| key.set(crate::radius(), ferroui_base::CornerRadius::uniform(0.0)))
                    .key_frame(1.0, |key| key.set(crate::radius(),
                        ferroui_base::CornerRadius::new(32.0, /* Keep 🪴 */ 32.0, 32.0, 32.0)))
                    .fill_mode(ferroui_base::animation::FillMode::Both))
                .animation(compute_motion(model))],
            Border { name: "Review", width: 180.0, height: 100.0, background: "#234976" }
        }
    }
}"###;

const STATE_SOURCE: &str = r###"// Review routes 🧭
fn review() {
    rsx! { StackPanel {
        visual_states: [ferroui_view::visual_state_group("Review")
            .state(ferroui_view::visual_state("Idle")
                .set_on("Card", ferroui_base::layout::Layoutable::width_property(), 120.0))
            .state(ferroui_view::visual_state("Ready")
                .set_on("Card", ferroui_base::layout::Layoutable::width_property(), 260.0))
            .transition(None, "Ready", ferroui_view::ms(350), "LinearEasing")
            .transition(Some("Idle"), "Ready", ferroui_view::ms(/* keep */ 640), "CubicEaseIn")
            .transition(Some("Ready"), "Idle", ferroui_view::ms(370), "LinearEasing")],
        Border { name: "Card", width: 100.0, height: 80.0, background: "#234976" }
    } }
}"###;

#[test]
fn studio_timeline_authors_whole_rust_clips_and_keeps_source_history() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let files = Fixture::new();
    let source = "fn view() { rsx! { Border { name: \"Card\", width: 180.0 } } }\r\n";
    let uri = files.file("clip.rs", source);
    let workspace = DesignerWorkspace::with_uri(XAML, files.file("Main.xaml", XAML)).unwrap();
    workspace.set_solution(Solution::discover(&files.0).unwrap()).unwrap();
    workspace.activate_document(&uri).unwrap();
    let root = workspace.control();
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();
    let session = workspace.session();
    let card = session.borrow().document().ast().root();
    session.borrow_mut().select([card]);
    workspace.refresh(true);

    click(&root.get_control::<Button>("NewAnimation"));
    let (created_source, identifier) = {
        let session = session.borrow();
        assert_eq!(session.animation_programs().len(), 1);
        let clip = &session.animation_programs()[0];
        assert_eq!(clip.name, "Animation 1");
        assert_eq!(clip.targets, [card]);
        assert_eq!(session.active_animation_program(), Some(clip.id));
        (session.authored_source().to_owned(), clip.source_key.clone().unwrap())
    };
    click(&root.get_control::<Button>("TimelineDetails"));
    root.get_control::<TextBox>("AnimationName").set_text(Some("Card reveal"));
    click(&root.get_control::<Button>("RenameAnimation"));
    let renamed_source = session.borrow().authored_source().to_owned();
    assert_eq!(session.borrow().animation_programs()[0].name, "Card reveal");
    assert_eq!(session.borrow().animation_programs()[0].source_key.as_deref(), Some(identifier.as_str()));

    click(&root.get_control::<Button>("DuplicateAnimation"));
    let duplicated_source = {
        let session = session.borrow();
        let copy = &session.animation_programs()[1];
        assert_eq!(copy.name, "Card reveal copy");
        assert_ne!(copy.source_key.as_deref(), Some(identifier.as_str()));
        assert_eq!(session.active_animation_program(), Some(copy.id));
        session.authored_source().to_owned()
    };
    click(&root.get_control::<Button>("DeleteAnimation"));
    assert_eq!(session.borrow().animation_programs().len(), 1);
    assert_eq!(session.borrow().authored_source(), renamed_source);
    for expected in [&duplicated_source, &renamed_source, &created_source, source] {
        session.borrow_mut().undo().unwrap();
        assert_eq!(session.borrow().authored_source(), expected);
    }
    assert!(!session.borrow().can_undo());
}

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "ferroui-studio-rsx-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }

    fn file(&self, path: &str, source: &str) -> String {
        let path = self.0.join(path);
        std::fs::write(&path, source).unwrap();
        file_uri(&path).unwrap()
    }

    fn catalog(&self) -> Arc<RsxCatalog> {
        self.file(
            "motion.xamlmeta",
            &serde_json::json!({
                "format": 2, "name": "motion", "crate_name": "motion",
                "metadata": [[format!("FerroUI.Designer.RsxAnimation.v1:{GETTER}"),
                    "::ferroui_controls::Border::corner_radius_property"]]
            })
            .to_string(),
        );
        self.file(
            "controls.xamlmeta",
            &serde_json::json!({
                "format": 2, "name": "FerroUI.Controls", "crate_name": "ferroui_controls",
                "types": [{ "name": "Border", "namespace": "FerroUI.Controls", "kind": "class",
                    "rust_path": "::ferroui_controls::Border", "object_model": true,
                    "registered": [{ "name": "CornerRadius", "kind": "styled",
                        "accessor": "corner_radius_property", "registration": "declared",
                        "value_type": "::ferroui_base::corner_radius::CornerRadius" }] }]
            })
            .to_string(),
        );
        let catalog = MarkupCatalog::load(std::slice::from_ref(&self.0)).unwrap();
        let mut schema = Schema::ferroui();
        schema.register_project(&catalog);
        Arc::new(RsxCatalog::from_project(&catalog, &schema, Some("motion")))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn studio_rust_states_edit_and_seek_the_original_source() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let files = Fixture::new();
    let source = STATE_SOURCE.replace('\n', "\r\n");
    let uri = files.file("review_states.rs", &source);
    let workspace = DesignerWorkspace::with_uri(XAML, files.file("Main.xaml", XAML)).unwrap();
    workspace.set_solution(Solution::discover(&files.0).unwrap()).unwrap();
    workspace.activate_document(&uri).unwrap();
    let window = Window::new();
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();

    let session = workspace.session();
    let mut group = session.borrow().state_groups().remove(0);
    let owner = group.owner;
    session.borrow_mut().select([owner]);
    workspace.refresh(true);
    let states = workspace.control().get_control::<Button>("StatesMode");
    assert!(states.is_visible());
    click(&states);
    let panel = workspace.control().get_control::<StackPanel>("AuthoringPanel");
    let picker = panel.children().to_vec().into_iter()
        .filter_map(|control| control.cast::<Grid>())
        .flat_map(|row| row.children().to_vec())
        .filter_map(|control| control.cast::<ComboBox>())
        .find(|picker| picker.name().as_deref() == Some("RustStateRoutes"))
        .expect("the Rust states panel offers its authored routes");
    assert_eq!(picker.items().count(), 3);
    let revision = session.borrow().document().revision();
    workspace.reveal_rust_state_source(revision, owner, "Review", Some("Idle"), Some("Ready")).unwrap();
    let editor = workspace.control().get_control::<TextBox>("Source");
    let selection = editor.selection_start().min(editor.selection_end()) as usize
        ..editor.selection_start().max(editor.selection_end()) as usize;
    let utf16 = source.encode_utf16().collect::<Vec<_>>();
    assert!(String::from_utf16(&utf16[selection]).unwrap().starts_with("transition(Some(\"Idle\")"));
    let target = ferroui_designer_animation::resolve_state_target(session.borrow().document(), owner, "Card").unwrap();
    let width = || session.borrow().preview.visual(target).unwrap().cast::<Border>().unwrap().width();
    for (from, to, time, expected) in [
        (None, "Ready", 0.175, 180.0),
        (Some("Ready"), "Idle", 0.185, 190.0),
        (Some("Idle"), "Ready", 0.32, 137.5),
    ] {
        session.borrow_mut().preview_state_transition(owner, "Review", from, to, time).unwrap();
        assert!((width() - expected).abs() < 0.000_01);
    }
    group.transitions[1].duration = 0.8;
    group.transitions[1].easing = Easing::Linear;
    group.states[1].setters[0].value = "280".to_owned();
    session.borrow_mut().upsert_state_group(group.clone()).unwrap();
    let edited = source.replace("640", "800").replace("CubicEaseIn", "LinearEasing").replace("260.0", "280.0");
    assert_eq!(session.borrow().authored_source(), edited);
    session.borrow_mut().preview_state_transition(owner, "Review", Some("Idle"), "Ready", 0.4).unwrap();
    assert!((width() - 200.0).abs() < 0.000_01);
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().authored_source(), source);
    assert!(!session.borrow().can_undo());
    let mut highlights = ferroui_designer_animation::VisualStateGroup {
        name: "Highlight".into(), owner,
        states: vec![ferroui_designer_animation::VisualState { name: "Focused".into(), setters: vec![
            ferroui_designer_animation::StateSetter { target: "Card".into(), property: "Width".into(), value: "160".into() },
        ] }],
        transitions: vec![],
    };
    assert!(session.borrow().can_create_rust_state_group(owner));
    session.borrow_mut().upsert_state_group(highlights.clone()).unwrap();
    let added = session.borrow().authored_source().to_owned();
    assert!(added.contains(r#"::ferroui_view::visual_state_group("Highlight")"#));
    session.borrow_mut().preview_state(owner, "Highlight", Some("Focused")).unwrap();
    assert!((width() - 160.0).abs() < 0.000_01);
    highlights.states[0].setters.clear();
    session.borrow_mut().upsert_state_group(highlights).unwrap();
    assert!(!session.borrow().authored_source().contains("160.0"));
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().authored_source(), added);
    session.borrow_mut().remove_state_group(owner, "Highlight").unwrap();
    assert_eq!(session.borrow().authored_source(), source);
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().authored_source(), added);
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().authored_source(), source);
    assert!(!session.borrow_mut().update_source(source.replace("width: 100.0", "width: [")).valid);
    assert!(session.borrow_mut().upsert_state_group(group).is_err());
    window.close();
}

#[test]
fn studio_rust_keyframe_editor_preserves_authored_history_and_the_native_preview() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let files = Fixture::new();
    let source = SOURCE.replace('\n', "\r\n");
    let uri = files.file("motion.rs", &source);
    let xaml = files.file("Main.xaml", XAML);
    let workspace = DesignerWorkspace::with_uri(XAML, xaml.clone()).unwrap();
    workspace
        .set_solution(Solution::discover(&files.0).unwrap())
        .unwrap();
    workspace.activate_document(&uri).unwrap();
    workspace
        .session()
        .borrow_mut()
        .set_rust_catalog(Some(files.catalog()))
        .unwrap();
    workspace.refresh(true);
    let window = Window::new();
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();

    let session = workspace.session();
    let picker = workspace
        .control()
        .get_control::<ComboBox>("RustInvocations");
    assert_eq!(picker.items().count(), 2);
    assert!(session.borrow().animation_programs().is_empty());
    picker.set_selected_index(1);
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    assert!(!session.borrow().can_undo());
    let (program, target) = {
        let session = session.borrow();
        assert_eq!(session.rust_map().unwrap().issues.len(), 1);
        let program = &session.animation_programs()[0];
        (program.id, program.primary_target())
    };
    let root = workspace.control();
    session.borrow_mut().select([target]);
    workspace.reveal_property_source("Width").unwrap();
    let width = session
        .borrow()
        .rust_property_span(target, "Width")
        .unwrap();
    assert_eq!(&source[width.range()], "180.0");
    session
        .borrow_mut()
        .set_property(&[target], "Width", "220")
        .unwrap();
    assert_eq!(
        session.borrow().authored_source(),
        source.replace("width: 180.0", "width: 220.0")
    );
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().authored_source(), source);
    workspace.refresh(true);
    click(&root.get_control::<Button>("AnimationSource"));
    let editor = root.get_control::<TextBox>("Source");
    let selection = editor.selection_start().min(editor.selection_end()) as usize
        ..editor.selection_start().max(editor.selection_end()) as usize;
    let utf16 = source.encode_utf16().collect::<Vec<_>>();
    assert!(
        String::from_utf16(&utf16[selection])
            .unwrap()
            .starts_with("ferroui_view::animation(")
    );

    workspace
        .select_keyframe(target, "CornerRadius", 2.0)
        .unwrap();
    session
        .borrow_mut()
        .scrub_animation_program(program, 1.0)
        .unwrap();
    let inspector = keyframe_editor(&workspace);
    keyframe_input(&inspector, "KeyframeTime").set_text(Some("1.6"));
    keyframe_input(&inspector, "KeyframeValue").set_text(Some("48, 48, 48, 48"));
    apply_keyframe(&inspector);
    let edited = source
        .replace(".key_frame(1.0", ".key_frame(0.8")
        .replace("32.0", "48.0");
    assert_eq!(session.borrow().authored_source(), edited);
    assert_eq!(session.borrow().time(), 1.0);
    let native = session
        .borrow()
        .preview
        .visual(target)
        .unwrap()
        .cast::<Border>()
        .unwrap();
    let radius = native.corner_radius().top_left;
    assert!(
        (radius - 30.0).abs() < 1e-9,
        "The retained 1 s pose has radius {radius}"
    );
    {
        let mut session = session.borrow_mut();
        session
            .start_animation_playback(program, Some(0.75), false)
            .unwrap();
        session.advance(0.25).unwrap();
        assert!(session.is_playing());
        let revised = edited.replace("Keep both views", "Keep both Rust views");
        assert!(session.update_source(revised).valid);
        assert!(session.is_playing());
        assert_eq!(session.time(), 1.0);
        session.pause();
        session.undo().unwrap();
        assert_eq!(session.authored_source(), edited);
    }

    let revision = session.borrow().document().revision();
    let original_pose = session.borrow().preview.visual(target).unwrap();
    assert!(
        !session
            .borrow_mut()
            .update_source(format!("{edited}\n{{"))
            .valid
    );
    assert!(matches!(
        session.borrow().preview_status(),
        PreviewStatus::InvalidSource { .. }
    ));
    assert_eq!(
        session.borrow().preview.visual(target).unwrap(),
        original_pose
    );
    assert!(
        session
            .borrow_mut()
            .apply_at(revision, "Stale", Vec::new())
            .is_err()
    );
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().authored_source(), edited);
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().authored_source(), source);
    assert!(!session.borrow().can_undo());
    #[cfg(feature = "mcp")]
    verify_rust_mcp(&workspace, program, target, &source, &edited, revision);
    session.borrow_mut().redo().unwrap();

    workspace.refresh(true);
    session.borrow_mut().select([target]);
    workspace.select_keyframe(target, "CornerRadius", 1.6).unwrap();
    session.borrow_mut().scrub_animation_program(program, 1.0).unwrap();
    let add = root.get_control::<Button>("Keyframe");
    assert!(add.is_visible());
    click(&add);
    let recorded = session.borrow().authored_source().to_owned();
    let timeline = session.borrow().timeline().clone();
    let track = timeline.track(target, "CornerRadius").unwrap();
    assert_eq!(track.keyframes.len(), 3);
    let key = track.keyframes.iter().find(|key| key.time == 1.0).unwrap();
    assert_eq!(key.value.to_xaml(), "30,30,30,30");
    assert!(recorded.contains(".key_frame(0.5, |key| key.set(crate::radius(), ::ferroui_base::CornerRadius::new"));
    let selected = KeyframeLocation { target, property: "CornerRadius".into(), time: 1.0 };
    session.borrow_mut().edit_program_keyframes(program, std::slice::from_ref(&selected),
        KeyframeGroupEdit::Duplicate(0.25)).unwrap();
    let duplicated = session.borrow().authored_source().to_owned();
    let copied = KeyframeLocation { time: 1.25, ..selected };
    session.borrow_mut().edit_program_keyframes(program, &[copied], KeyframeGroupEdit::Remove).unwrap();
    assert_eq!(session.borrow().authored_source(), recorded);
    for expected in [&duplicated, &recorded, &edited] {
        session.borrow_mut().undo().unwrap();
        assert_eq!(session.borrow().authored_source(), expected);
    }

    let keys = [KeyframeLocation {
        target,
        property: "CornerRadius".into(),
        time: 1.6,
    }];
    session
        .borrow_mut()
        .edit_program_keyframes(program, &keys, KeyframeGroupEdit::Move(-0.4))
        .unwrap();
    let moved = session.borrow().authored_source().to_owned();
    assert!(moved.contains(".key_frame(0.6"));
    workspace.activate_document(&xaml).unwrap();
    workspace.save_all_documents().unwrap();
    assert_eq!(
        std::fs::read_to_string(files.0.join("motion.rs")).unwrap(),
        moved
    );
    workspace.activate_document(&uri).unwrap();
    assert_eq!(session.borrow().authored_source(), moved);
    assert!(!session.borrow().authored_is_dirty());
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().authored_source(), edited);
}

#[cfg(feature = "mcp")]
fn verify_rust_mcp(
    workspace: &std::rc::Rc<DesignerWorkspace>,
    program: u64,
    target: u64,
    source: &str,
    edited: &str,
    stale: u64,
) {
    use ferroui_mcp::{Permissions, UiToolProvider};
    use serde_json::json;

    let provider = crate::mcp::DesignerMcp::with_workspace(workspace);
    let read = Permissions::default();
    let write = Permissions {
        allow_property_writes: true,
        ..read
    };
    let document = provider
        .call("designer_document", &json!({}), read)
        .unwrap();
    assert_eq!(document["language"], "rust");
    assert_eq!(document["source"], source);
    let views = provider
        .call("designer_rsx_views", &json!({}), read)
        .unwrap();
    assert_eq!(views["invocations"].as_array().unwrap().len(), 2);
    let programs = provider
        .call("designer_programs", &json!({}), read)
        .unwrap();
    assert!(
        programs["programs"][0]["source"]
            .as_str()
            .unwrap()
            .starts_with("ferroui_view::animation(")
    );
    assert_eq!(programs["sourceIssues"].as_array().unwrap().len(), 1);
    let mut args = json!({"action":"upsert_keyframe", "program":program, "target":target,
        "property":"CornerRadius", "previous":2, "time":1.6, "value":"48,48,48,48",
        "easing":"LinearEasing", "expected_revision":stale});
    assert!(
        provider
            .call("designer_program", &args, write)
            .unwrap_err()
            .contains("revision")
    );
    args["expected_revision"] = document["revision"].clone();
    assert!(provider.call("designer_program", &args, read).is_err());
    provider.call("designer_program", &args, write).unwrap();
    assert_eq!(workspace.session().borrow().authored_source(), edited);
    let revision = workspace.session().borrow().document().revision();
    provider
        .call(
            "designer_undo",
            &json!({"expected_revision":revision}),
            write,
        )
        .unwrap();
    assert_eq!(workspace.session().borrow().authored_source(), source);
}
