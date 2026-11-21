use super::*;
use crate::LineIndex;
use ferroui_designer_project::{Solution, file_uri};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const NS: &str =
    "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

struct ProjectFiles(PathBuf);

impl ProjectFiles {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "ferroui-resource-move-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, source: &str) -> String {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, source).unwrap();
        file_uri(path.canonicalize().unwrap()).unwrap()
    }

    fn service(&self) -> LanguageService {
        let mut service = LanguageService::default();
        service
            .index_solution(&Solution::discover(&self.0).unwrap())
            .unwrap();
        service
    }
}

impl Drop for ProjectFiles {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn request(source: &str, version: i64, needle: &str) -> SourceReviewRequest {
    SourceReviewRequest {
        version,
        position: LineIndex::new(source)
            .position(source.find(needle).unwrap() + 1)
            .unwrap(),
    }
}

#[test]
fn moving_one_resource_links_an_unopened_dictionary_and_rebases_typed_assets() {
    let files = ProjectFiles::new();
    let source = format!(
        "<Grid {NS} xmlns:local='using:Sample'>\r\n<Grid.Resources>\r\n<!-- keep compiler note -->\r\n<Image x:Key='Card' Source='Assets/Cover%20Art.png' Tag='🧭'/>\r\n<SolidColorBrush x:Key='LocalOnly' Color='Red'/>\r\n</Grid.Resources>\r\n<ContentControl Content='{{StaticResource Card}}'/>\r\n</Grid>"
    );
    let to_source = format!("<ResourceDictionary {NS} />");
    let from = files.write("Overview.xaml", &source);
    let to = files.write("Theme/Palette.xaml", &to_source);
    let _asset = files.write("Assets/Cover Art.png", "fixture");
    let mut service = files.service();
    service.open(&from, &source, 8).unwrap();
    let request = request(&source, 8, "<Image");
    let target = service.resource_move_target(&from, &request).unwrap();
    assert_eq!(target.name, "Card");
    let destination = target
        .destinations
        .iter()
        .find(|entry| entry.uri == to)
        .unwrap();
    assert!(destination.request.version < 0);
    let plan = service
        .plan_resource_move(&from, &request, &to, &destination.request)
        .unwrap();
    assert!(plan.linked_destination);
    assert_eq!(plan.documents.len(), 2);
    let old = &plan
        .documents
        .iter()
        .find(|edit| edit.edit.uri == from)
        .unwrap()
        .after;
    let new = &plan
        .documents
        .iter()
        .find(|edit| edit.edit.uri == to)
        .unwrap()
        .after;
    assert!(old.contains("Source=\"Theme/Palette.xaml\""));
    assert!(old.contains("<!-- keep compiler note -->"));
    assert!(old.contains("x:Key='LocalOnly'"));
    assert!(!old.contains("x:Key='Card'"));
    assert!(new.contains("x:Key='Card'"));
    assert!(new.contains("../Assets/Cover%20Art.png"));
    assert!(new.contains("Tag='🧭'"));
    assert!(new.contains("xmlns:local=\"using:Sample\""));
    assert_eq!(service.state(&from).unwrap().document.source(), source);
    service.open(&to, &to_source, 3).unwrap();
    assert!(
        service
            .plan_resource_move(&from, &request, &to, &destination.request)
            .is_err()
    );
}

#[test]
fn moving_within_the_view_keeps_local_shadows_and_a_static_dependency() {
    let files = ProjectFiles::new();
    let source = format!(
        "<Grid {NS}><Grid.Resources><SolidColorBrush x:Key='Ink' Color='Red'/></Grid.Resources><StackPanel Name='Card'><StackPanel.Resources><SolidColorBrush x:Key='Accent' Color='{{StaticResource Ink}}'/></StackPanel.Resources><Border Background='{{StaticResource Accent}}'/><StackPanel><StackPanel.Resources><SolidColorBrush x:Key='Accent' Color='Blue'/></StackPanel.Resources><Border Background='{{DynamicResource Accent}}'/></StackPanel></StackPanel></Grid>"
    );
    let uri = files.write("Overview.xaml", &source);
    let mut service = files.service();
    service.open(&uri, &source, 2).unwrap();
    let request = request(&source, 2, "<SolidColorBrush x:Key='Accent'");
    let destination = super::tests::request(&source, 2, "<Grid.Resources");
    let plan = service
        .plan_resource_move(&uri, &request, &uri, &destination)
        .unwrap();
    assert!(!plan.linked_destination);
    assert_eq!(plan.documents.len(), 1);
    let after = &plan.documents[0].after;
    assert!(after.contains("x:Key='Accent' Color='{StaticResource Ink}'"));
    assert!(after.contains("x:Key='Accent' Color='Blue'"));
    assert!(
        after
            .find("x:Key='Accent' Color='{StaticResource Ink}'")
            .unwrap()
            < after.find("</Grid.Resources>").unwrap()
    );
}

#[test]
fn move_refuses_lost_dependencies_and_cross_scope_shadows() {
    let files = ProjectFiles::new();
    let source = format!(
        "<Grid {NS}><Grid.Resources><SolidColorBrush x:Key='Ink' Color='Red'/></Grid.Resources><StackPanel><StackPanel.Resources><SolidColorBrush x:Key='Ink' Color='Green'/><SolidColorBrush x:Key='Accent' Color='{{StaticResource Ink}}'/></StackPanel.Resources><Border Background='{{StaticResource Accent}}'/></StackPanel></Grid>"
    );
    let uri = files.write("Overview.xaml", &source);
    let mut service = files.service();
    service.open(&uri, &source, 1).unwrap();
    let selected = request(&source, 1, "<SolidColorBrush x:Key='Accent'");
    let destination = request(&source, 1, "<Grid.Resources");
    let error = service
        .plan_resource_move(&uri, &selected, &uri, &destination)
        .unwrap_err();
    assert!(error.to_string().contains("resolution of 'Ink'"), "{error}");
}

#[test]
fn move_keeps_theme_precedence_and_refuses_another_theme_scope() {
    let files = ProjectFiles::new();
    let source = format!(
        r#"<ResourceDictionary {NS}>
  <ResourceDictionary.ThemeDictionaries>
    <ResourceDictionary x:Key='Light'><SolidColorBrush x:Key='Accent' Color='Orange'/></ResourceDictionary>
    <ResourceDictionary x:Key='Dark'><SolidColorBrush x:Key='Accent' Color='Blue'/></ResourceDictionary>
    <ResourceDictionary x:Key='Default'/>
  </ResourceDictionary.ThemeDictionaries>
</ResourceDictionary>"#,
    );
    let target = format!("<ResourceDictionary {NS}/>");
    let uri = files.write("Styles.xaml", &source);
    let external = files.write("Light.xaml", &target);
    files.write(
        "View.xaml",
        &format!(
            r#"<Border {NS} Background='{{DynamicResource Accent}}'><Border.Resources>
          <ResourceDictionary><ResourceDictionary.MergedDictionaries>
            <ResourceInclude Source='Styles.xaml'/>
          </ResourceDictionary.MergedDictionaries></ResourceDictionary>
        </Border.Resources></Border>"#,
        ),
    );
    let mut service = files.service();
    service.open(&uri, &source, 1).unwrap();
    let selected = request(&source, 1, "<SolidColorBrush");
    let target = service.resource_move_target(&uri, &selected).unwrap();
    let option = target
        .destinations
        .iter()
        .find(|item| item.uri == external)
        .unwrap();
    let plan = service
        .plan_resource_move(&uri, &selected, &external, &option.request)
        .unwrap();
    assert!(plan.linked_destination);
    assert!(plan.documents[0].after.contains("Source=\"Light.xaml\""));
    assert!(plan.documents[0].after.contains("Color='Blue'"));
    let default = target
        .destinations
        .iter()
        .find(|item| item.label == "Default dictionary")
        .unwrap();
    assert!(
        service
            .plan_resource_move(&uri, &selected, &uri, &default.request)
            .is_err()
    );
}

#[test]
fn a_move_rejects_new_include_cycles_and_unrelated_shadowing() {
    let files = ProjectFiles::new();
    let source = format!(
        "<ResourceDictionary {NS}><SolidColorBrush x:Key='Accent' Color='Red'/></ResourceDictionary>"
    );
    let cyclic = format!(
        "<ResourceDictionary {NS}><ResourceDictionary.MergedDictionaries><ResourceInclude Source='Original.xaml'/></ResourceDictionary.MergedDictionaries></ResourceDictionary>"
    );
    let target = format!(
        "<ResourceDictionary {NS}><SolidColorBrush x:Key='Ink' Color='Gold'/></ResourceDictionary>"
    );
    let uri = files.write("Original.xaml", &source);
    let cycle = files.write("Cycle.xaml", &cyclic);
    let occupied = files.write("Occupied.xaml", &target);
    files.write(
        "View.xaml",
        &format!(
            r#"<Border {NS} Background='{{DynamicResource Ink}}'><Border.Resources>
          <ResourceDictionary><ResourceDictionary.MergedDictionaries>
            <ResourceInclude Source='Original.xaml'/>
          </ResourceDictionary.MergedDictionaries></ResourceDictionary>
        </Border.Resources></Border>"#,
        ),
    );
    let mut service = files.service();
    service.open(&uri, &source, 1).unwrap();
    let selected = request(&source, 1, "<SolidColorBrush");
    let targets = service.resource_move_target(&uri, &selected).unwrap();
    let failed = |service: &mut LanguageService, file: &str| {
        let target = targets
            .destinations
            .iter()
            .find(|item| item.uri == file)
            .unwrap();
        service
            .plan_resource_move(&uri, &selected, file, &target.request)
            .unwrap_err()
            .to_string()
    };
    assert!(failed(&mut service, &cycle).contains("include cycle"));
    assert!(failed(&mut service, &occupied).contains("resolution of 'Ink'"));
}

#[test]
fn an_existing_import_is_not_repeated_to_change_merge_precedence() {
    let files = ProjectFiles::new();
    let source = format!(
        r#"<ResourceDictionary {NS}>
  <ResourceDictionary.MergedDictionaries>
    <ResourceInclude Source='Target.xaml'/>
    <ResourceInclude Source='HigherPriority.xaml'/>
  </ResourceDictionary.MergedDictionaries>
  <SolidColorBrush x:Key='Accent' Color='Red'/>
</ResourceDictionary>"#,
    );
    let uri = files.write("Original.xaml", &source);
    let destination = files.write("Target.xaml", &format!("<ResourceDictionary {NS}/>"));
    files.write("HigherPriority.xaml", &format!(
        "<ResourceDictionary {NS}><SolidColorBrush x:Key='Accent' Color='Blue'/></ResourceDictionary>"
    ));
    let mut service = files.service();
    service.open(&uri, &source, 1).unwrap();
    let selected = request(&source, 1, "<SolidColorBrush");
    let targets = service.resource_move_target(&uri, &selected).unwrap();
    let option = targets
        .destinations
        .iter()
        .find(|item| item.uri == destination)
        .unwrap();
    let error = service
        .plan_resource_move(&uri, &selected, &destination, &option.request)
        .unwrap_err();
    assert!(error.to_string().contains("already links this dictionary"));
}
