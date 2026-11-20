use crate::{apply_text_edits, lsp::LspServer, LanguageService};
use ferroui_designer_project::{file_uri, Solution};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

const NS: &str =
    "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";
static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "ferroui-file-move-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("Themes/Archive")).unwrap();
        fs::create_dir(root.join("Assets")).unwrap();
        Self(root.canonicalize().unwrap())
    }

    fn uri(&self, name: &str) -> String {
        file_uri(self.0.join(name)).unwrap()
    }
    fn write(&self, name: &str, source: &str) {
        fs::write(self.0.join(name), source).unwrap();
    }
    fn index(&self, service: &mut LanguageService) {
        service
            .index_solution(&Solution::discover(&self.0).unwrap())
            .unwrap();
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn moving_a_view_rebases_typed_uri_and_include_paths_with_exact_source_ranges() {
    let project = Project::new();
    let from = project.uri("View.xaml");
    let to = project.uri("Themes/Archive/Account cards & notes.xaml");
    let source = format!(
        "<Grid {NS}>\r\n  <!-- 🧭 Keep the path and suffix --> <Image Source='Assets/Brand%20lockup.png#large' />\r\n  <Grid.Styles><StyleInclude><StyleInclude.Source><!-- author --><![CDATA[ Themes/Styles.xaml ]]></StyleInclude.Source></StyleInclude></Grid.Styles>\r\n  <TextBlock Text='Assets/Brand%20lockup.png'/><Image Source='{{Binding Logo}}'/>\r\n</Grid>\r\n"
    );
    project.write("View.xaml", &source);
    let mut service = LanguageService::default();
    project.index(&mut service);
    service.open(&from, &source, 7).unwrap();
    let plan = service.plan_file_move(&from, &to).unwrap();
    let [moved] = plan.documents.as_slice() else {
        panic!("only the moved view is affected")
    };
    assert_eq!(plan.to, to);
    assert_eq!(moved.edit.version, 7);
    assert_eq!(moved.before, source);
    assert_eq!(moved.edit.edits.len(), 2);
    assert_eq!(
        apply_text_edits(&source, &moved.edit.edits).unwrap(),
        moved.after
    );
    assert!(moved.after.contains(
        "<!-- 🧭 Keep the path and suffix --> <Image Source='../../Assets/Brand%20lockup.png#large'"
    ));
    assert!(moved
        .after
        .contains("<!-- author --><![CDATA[ ../Styles.xaml ]]>"));
    assert!(moved
        .after
        .contains("<TextBlock Text='Assets/Brand%20lockup.png'/><Image Source='{Binding Logo}'/>"));
    assert!(!moved.after.replace("\r\n", "").contains('\n'));
    assert!(project.0.join("View.xaml").exists());
    assert!(!project
        .0
        .join("Themes/Archive/Account cards & notes.xaml")
        .exists());
}

#[test]
fn review_tracks_unopened_and_open_cross_workspace_consumers_by_resolved_identity() {
    let project = Project::new();
    let other = Project::new();
    let empty = format!("<ResourceDictionary {NS}/>");
    let from = project.uri("Palette.xaml");
    let to = project.uri("Themes/Archive/Brand & 🎨.xaml");
    project.write("Palette.xaml", &empty);
    other.write("Palette.xaml", &empty);
    let direct = format!(
        "<ResourceDictionary {NS}><ResourceDictionary.MergedDictionaries><ResourceInclude Source='./Palette.xaml'/></ResourceDictionary.MergedDictionaries></ResourceDictionary>"
    );
    project.write("Consumer.xaml", &direct);
    other.write("Unrelated.xaml", &direct);
    let package = project.0.file_name().unwrap().to_str().unwrap();
    let packaged = direct.replace(
        "./Palette.xaml",
        &format!("ferres://{package}/Palette.xaml"),
    );
    other.write("Other consumer.xaml", &packaged);
    let mut service = LanguageService::default();
    project.index(&mut service);
    other.index(&mut service);
    service
        .open(other.uri("Other consumer.xaml"), &packaged, 14)
        .unwrap();
    let first = service.plan_file_move(&from, &to).unwrap();
    assert_eq!(first.documents.len(), 3);
    assert!(first
        .documents
        .iter()
        .all(|doc| doc.edit.uri != other.uri("Unrelated.xaml")));
    let closed = first
        .documents
        .iter()
        .find(|doc| doc.edit.uri == project.uri("Consumer.xaml"))
        .unwrap();
    assert!(closed.edit.version < 0);
    assert!(closed
        .after
        .contains("'./Themes/Archive/Brand%20%26%20%F0%9F%8E%A8.xaml'"));
    let open = first
        .documents
        .iter()
        .find(|doc| doc.edit.uri == other.uri("Other consumer.xaml"))
        .unwrap();
    assert_eq!(open.edit.version, 14);
    assert!(open.after.contains(&format!(
        "ferres://{package}/Themes/Archive/Brand%20%26%20%F0%9F%8E%A8.xaml"
    )));
    assert_eq!(
        service.plan_file_move(&from, &to).unwrap().review,
        first.review
    );
    project.write("New consumer.xaml", &direct);
    let second = service.plan_file_move(&from, &to).unwrap();
    assert_ne!(first.review, second.review);
    assert_eq!(second.documents.len(), 4);
    project.write("Themes/Archive/Brand & 🎨.xaml", &empty);
    assert!(service.plan_file_move(&from, &to).is_err());
}

#[test]
fn file_move_lsp_rejects_a_referencing_draft_and_preserves_other_workspaces() {
    let project = Project::new();
    let other = Project::new();
    let from = project.uri("Palette.xaml");
    let to = project.uri("Themes/Palette.xaml");
    project.write("Palette.xaml", &format!("<ResourceDictionary {NS}/>"));
    let draft = project.uri("Draft.xaml");
    let mut lsp = LspServer::default();
    assert_eq!(lsp.handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":file_uri(&project.0).unwrap()}}))[0]["result"]["capabilities"]["experimental"]["ferrouiFileMove"], true);
    assert!(lsp
        .service
        .plan_file_move(&from, &other.uri("Palette.xaml"))
        .is_err());
    let request = json!({"jsonrpc":"2.0","id":2,"method":"ferroui/planFileMove","params":{"from":from,"to":to}});
    lsp.service
        .open(&draft, "<ResourceInclude Source='Palette.xaml'", 1)
        .unwrap();
    assert!(lsp.handle(request.clone())[0]["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Repair the draft"));
    lsp.service.replace(&draft, "<Grid", 2).unwrap();
    let result = lsp.handle(request);
    assert_eq!(result[0]["result"]["to"], to);
    assert_eq!(
        result[0]["result"]["documents"].as_array().unwrap().len(),
        1
    );
}
