use super::*;
use crate::{mcp::DesignerMcp, PreviewStatus};
use ferroui_base::{interactivity::RoutedEventArgs, Ref};
use ferroui_controls::{
    automation::AutomationProperties,
    testing::{TestServices, UnitTestApplication},
    Button, Control, Flyout, TextBox, Window,
};
use ferroui_designer_project::Solution;
use ferroui_markup_xaml::FromXamlObject;
use ferroui_mcp::{Permissions, UiToolProvider};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

const NS: &str =
    "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

#[path = "workspace_file_move_rust_tests.rs"]
mod rust;

#[test]
fn studio_mcp_moves_unopened_references_and_replays_the_physical_file_from_a_consumer() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = format!("<ResourceDictionary {NS}>\r\n  <!-- 🎨 -->\r\n  <ResourceDictionary.MergedDictionaries><ResourceInclude Source='Base.xaml'/></ResourceDictionary.MergedDictionaries>\r\n  <SolidColorBrush x:Key='Card' Color='{{StaticResource Ink}}'/>\r\n</ResourceDictionary>\r\n");
    let view = format!("<Border {NS} Background='{{StaticResource Card}}'><Border.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source='Palette.xaml'/></ResourceDictionary.MergedDictionaries></ResourceDictionary></Border.Resources><TextBlock Text='Customer accounts'/></Border>");
    let (files, workspace) = Files::open("Palette.xaml", &source);
    fs::write(
        files.0.join("Base.xaml"),
        format!("<ResourceDictionary {NS}><Color x:Key='Ink'>#245783</Color></ResourceDictionary>"),
    )
    .unwrap();
    fs::write(files.0.join("View.xaml"), &view).unwrap();
    let destination = "Archive/Palette & cards.xaml";
    let session = workspace.session();
    let revision = || session.borrow().document().revision();
    let provider = DesignerMcp::with_workspace(&workspace);
    let write = Permissions {
        allow_property_writes: true,
        allow_input: false,
    };
    let plan = provider
        .call(
            "designer_plan_xaml_move",
            &json!({"destination":destination, "expected_revision":revision()}),
            Permissions::default(),
        )
        .unwrap();
    assert_eq!(plan["documents"].as_array().unwrap().len(), 2);
    assert!(!files.0.join(destination).exists());
    let args =
        json!({"destination":destination,"review":plan["review"],"expected_revision":revision()});
    assert!(provider
        .call("designer_move_xaml", &args, Permissions::default())
        .is_err());
    assert!(provider
        .call(
            "designer_move_xaml",
            &json!({"destination":destination,"review":"stale","expected_revision":revision()}),
            write
        )
        .is_err());
    assert_eq!(session.borrow().authored_source(), source);
    provider.call("designer_move_xaml", &args, write).unwrap();
    assert_eq!(session.borrow().document_uri(), files.uri(destination));
    assert!(session
        .borrow()
        .authored_source()
        .contains("Source='../Base.xaml'"));
    assert!(!files.0.join("Palette.xaml").exists());
    assert_eq!(
        fs::read_to_string(files.0.join(destination)).unwrap(),
        source
    );
    assert_eq!(fs::read_to_string(files.0.join("View.xaml")).unwrap(), view);
    workspace
        .activate_document(&files.uri("View.xaml"))
        .unwrap();
    let assert_ready = || {
        assert!(
            matches!(
                session.borrow().preview_status(),
                PreviewStatus::Ready { .. }
            ),
            "{:?}",
            session.borrow().preview_status()
        )
    };
    assert!(session
        .borrow()
        .authored_source()
        .contains("Archive/Palette%20%26%20cards.xaml"));
    assert_ready();
    provider
        .call(
            "designer_undo",
            &json!({"expected_revision":revision()}),
            write,
        )
        .unwrap();
    assert_eq!(session.borrow().authored_source(), view);
    assert!(files.0.join("Palette.xaml").exists());
    assert!(!files.0.join(destination).exists());
    assert_ready();
    fs::write(files.0.join(destination), "Occupied outside Studio").unwrap();
    assert!(provider
        .call(
            "designer_redo",
            &json!({"expected_revision":revision()}),
            write
        )
        .is_err());
    assert_eq!(session.borrow().authored_source(), view);
    assert_eq!(
        fs::read_to_string(files.0.join(destination)).unwrap(),
        "Occupied outside Studio"
    );
    fs::remove_file(files.0.join(destination)).unwrap();
    provider
        .call(
            "designer_redo",
            &json!({"expected_revision":revision()}),
            write,
        )
        .unwrap();
    assert!(session
        .borrow()
        .authored_source()
        .contains("Archive/Palette%20%26%20cards.xaml"));
    assert_ready();
    workspace.save_all_documents().unwrap();
    assert!(fs::read_to_string(files.0.join(destination))
        .unwrap()
        .contains("Source='../Base.xaml'"));
    provider
        .call(
            "designer_undo",
            &json!({"expected_revision":revision()}),
            write,
        )
        .unwrap();
    assert!(files.0.join("Palette.xaml").is_file());
    assert_eq!(session.borrow().authored_source(), view);
    assert_ready();
    provider
        .call(
            "designer_redo",
            &json!({"expected_revision":revision()}),
            write,
        )
        .unwrap();
    assert!(files.0.join(destination).is_file());
    fs::write(files.0.join(destination), "Replaced outside Studio").unwrap();
    assert!(provider
        .call(
            "designer_undo",
            &json!({"expected_revision":revision()}),
            write
        )
        .is_err());
    assert!(!files.0.join("Palette.xaml").exists());
}

#[test]
fn studio_moves_a_view_and_undoes_its_rust_reference_from_the_code_tab() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let xaml = format!("<Border {NS}><TextBlock Text='Customer card'/></Border>");
    let (files, workspace) = Files::open("View.xaml", &xaml);
    let rust = "pub const LABEL: &str = \"View.xaml\";\r\npub const VIEW: &str = { /* 🎨 */ include_str!(r#\"View.xaml\"#) };\r\n";
    let rust_uri = files.uri("compiled_view.rs");
    fs::write(files.0.join("compiled_view.rs"), rust).unwrap();
    let plan = workspace
        .plan_project_file_move("Archive/Customer card.xaml", true)
        .unwrap()
        .unwrap();
    let edited = plan
        .documents
        .iter()
        .find(|item| item.edit.uri == rust_uri)
        .unwrap();
    assert_eq!(
        edited.after,
        rust.replace("r#\"View.xaml\"#", "r#\"Archive/Customer card.xaml\"#")
    );
    workspace
        .apply_project_file_move("Archive/Customer card.xaml", &plan.review)
        .unwrap();
    workspace.activate_document(&rust_uri).unwrap();
    let session = workspace.session();
    assert_eq!(session.borrow().authored_source(), edited.after);
    workspace.save_all_documents().unwrap();
    assert_eq!(
        fs::read_to_string(files.0.join("compiled_view.rs")).unwrap(),
        edited.after
    );
    let provider = DesignerMcp::with_workspace(&workspace);
    let writes = Permissions {
        allow_property_writes: true,
        allow_input: false,
    };
    let revision = session.borrow().document().revision();
    provider
        .call(
            "designer_undo",
            &json!({"expected_revision":revision}),
            writes,
        )
        .unwrap();
    assert_eq!(session.borrow().authored_source(), rust);
    assert!(files.0.join("View.xaml").is_file());
    assert!(!files.0.join("Archive/Customer card.xaml").exists());
    let revision = session.borrow().document().revision();
    provider
        .call(
            "designer_redo",
            &json!({"expected_revision":revision}),
            writes,
        )
        .unwrap();
    assert_eq!(session.borrow().authored_source(), edited.after);
    assert!(files.0.join("Archive/Customer card.xaml").is_file());
}

#[test]
fn studio_move_persists_saved_design_references_and_replays_them_from_source_history() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = format!("<Border {NS}/>");
    let (files, workspace) = Files::open("Customer.xaml", &source);
    let old = files.uri("Customer.xaml");
    let destination = "Archive/Customer & notes.xaml";
    let next = files.uri(destination);
    let configuration = crate::PreviewStageConfiguration::new(
        "Customer review", &old, crate::PreviewEnvironment::default()
    );
    let id = workspace.add_preview_stage(configuration.clone()).unwrap();
    let initial = workspace.preview_stage(id).unwrap();
    let stage_path = files.0.join("Review.ferrostages");
    let scenario_path = files.0.join("Review.ferroui-scenario.json");
    let stage = serde_json::to_string_pretty(&json!({
        "version":3, "active":0, "stages":[configuration],
        "prototype":{"flows":[{"name":"Review","start":0}],"links":[]}
    })).unwrap();
    let scenario = serde_json::to_string_pretty(&json!({
        "version":1,"name":"Review customer", "document":"./Customer.xaml",
        "prototype":{"stageSet":"Review.ferrostages","flow":"Review","sources":{(&old):{"path":"Customer.xaml"}}},
        "steps":[{"action":"expectStage","name":"Customer review"}]
    })).unwrap();
    fs::write(&stage_path, &stage).unwrap();
    fs::write(&scenario_path, &scenario).unwrap();
    let plan = workspace.plan_project_file_move(destination, true).unwrap().unwrap();
    assert_eq!(plan.documents.len(), 3);
    workspace.apply_project_file_move(destination, &plan.review).unwrap();
    let moved = workspace.preview_stage(id).unwrap();
    assert_eq!(moved.configuration.source_uri, next);
    assert!(moved.epoch > initial.epoch);
    assert!(matches!(moved.status, crate::PreviewStageStatus::Ready { .. }));
    let changed_stage = stage.replace(&old, &next);
    let changed_scenario = scenario.replace(&old, &next).replace("Customer.xaml", destination);
    assert_eq!(fs::read_to_string(&stage_path).unwrap(), changed_stage);
    assert_eq!(fs::read_to_string(&scenario_path).unwrap(), changed_scenario);
    assert!(files.0.join(destination).is_file());
    let provider = DesignerMcp::with_workspace(&workspace);
    let history = |action| provider.call(
        action,
        &json!({"expected_revision":workspace.session().borrow().document().revision()}),
        Permissions { allow_property_writes: true, ..Default::default() },
    );
    let retained = files.0.join("external-hold.json");
    fs::rename(&scenario_path, &retained).unwrap();
    fs::write(&scenario_path, "External scenario replacement").unwrap();
    assert!(history("designer_undo").is_err());
    let retained_stage = workspace.preview_stage(id).unwrap();
    assert_eq!(retained_stage.epoch, moved.epoch);
    assert_eq!(retained_stage.configuration.source_uri, next);
    assert!(files.0.join(destination).is_file());
    assert_eq!(fs::read_to_string(&stage_path).unwrap(), changed_stage);
    assert_eq!(fs::read_to_string(&scenario_path).unwrap(), "External scenario replacement");
    fs::remove_file(&scenario_path).unwrap();
    fs::rename(&retained, &scenario_path).unwrap();
    history("designer_undo").unwrap();
    let restored = workspace.preview_stage(id).unwrap();
    assert_eq!(restored.configuration.source_uri, old);
    assert!(restored.epoch > moved.epoch);
    assert!(files.0.join("Customer.xaml").is_file());
    assert_eq!(fs::read_to_string(&stage_path).unwrap(), stage);
    assert_eq!(fs::read_to_string(&scenario_path).unwrap(), scenario);
    history("designer_redo").unwrap();
    let reapplied = workspace.preview_stage(id).unwrap();
    assert_eq!(reapplied.configuration.source_uri, next);
    assert!(reapplied.epoch > restored.epoch);
    assert!(files.0.join(destination).is_file());
    assert_eq!(fs::read_to_string(&scenario_path).unwrap(), changed_scenario);
}

#[test]
fn studio_toolbar_reviews_and_records_history_for_a_move_with_no_source_edits() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = format!("<Border {NS}><TextBlock Text='Customer accounts'/></Border>");
    let (files, workspace) = Files::open("View.xaml", &source);
    let window = Window::new();
    window.set_width(1240.0);
    window.set_height(840.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let anchor = workspace.control().get_control::<Button>("MoveXamlFile");
    let flyout = anchor
        .flyout()
        .and_then(|value| value.cast::<Flyout>())
        .unwrap();
    let find = |name: &str| {
        let content = Ref::<Control>::from_xaml_object(&flyout.content().unwrap()).unwrap();
        content
            .get_logical_descendants()
            .find(|control| AutomationProperties::get_name(control).as_deref() == Some(name))
            .unwrap()
    };
    let press = |name: &str| {
        find(name)
            .cast::<Button>()
            .unwrap()
            .raise_event(&RoutedEventArgs::with_event(Button::click_event()))
    };
    let destination = "Archive/Customer.xaml";
    let choose = || {
        flyout.show_at(&anchor);
        find("Destination XAML path")
            .cast::<TextBox>()
            .unwrap()
            .set_text(Some(destination));
        press("Review file move");
        assert!(find("Original source")
            .cast::<TextBox>()
            .unwrap()
            .is_read_only());
        assert!(!files.0.join(destination).exists());
    };
    choose();
    press("Cancel file move");
    assert!(files.0.join("View.xaml").exists());
    choose();
    press("Apply file move");
    assert_eq!(
        workspace.session().borrow().document_uri(),
        files.uri(destination)
    );
    assert_eq!(workspace.session().borrow().document().source(), source);
    workspace.source_history(false).unwrap();
    assert!(files.0.join("View.xaml").exists());
    assert!(!files.0.join(destination).exists());
    assert_eq!(
        workspace.session().borrow().document_uri(),
        files.uri("View.xaml")
    );
    window.close();
}

struct Files(PathBuf);

impl Files {
    fn open(name: &str, source: &str) -> (Self, Rc<DesignerWorkspace>) {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "ferroui-studio-move-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("Archive")).unwrap();
        let root = root.canonicalize().unwrap();
        let files = Self(root);
        fs::write(files.0.join(name), source).unwrap();
        let workspace = DesignerWorkspace::with_uri(source, files.uri(name)).unwrap();
        workspace
            .set_solution(Solution::discover(&files.0).unwrap())
            .unwrap();
        (files, workspace)
    }
    fn uri(&self, name: &str) -> String {
        file_uri(self.0.join(name)).unwrap()
    }
}

impl Drop for Files {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
