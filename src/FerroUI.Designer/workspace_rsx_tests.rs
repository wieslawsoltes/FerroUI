use super::{apply_keyframe, click, keyframe_editor, keyframe_input};
use crate::{DesignerWorkspace, PreviewStatus};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Border, Button, ComboBox, Control, TextBox, Window};
use ferroui_designer_animation::{KeyframeGroupEdit, KeyframeLocation};
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
    assert!((radius - 30.0).abs() < 1e-9, "The retained 1 s pose has radius {radius}");
    {
        let mut session = session.borrow_mut();
        session.start_animation_playback(program, Some(0.75), false).unwrap();
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
