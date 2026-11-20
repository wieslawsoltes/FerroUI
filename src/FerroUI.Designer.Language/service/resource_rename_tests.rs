use super::*;
use ferroui_designer_project::{file_uri, Solution};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

const NS: &str =
    "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";
const EXTENSIONS: &str = "xmlns:r='using:FerroUI.Markup.Xaml.MarkupExtensions'";

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "ferroui-resource-review-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root.canonicalize().unwrap())
    }

    fn write(&self, path: &str, source: &str) -> String {
        let file = self.0.join(path);
        fs::write(&file, source).unwrap();
        file_uri(file).unwrap()
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

fn request(source: &str, at: &str, version: i64) -> SourceReviewRequest {
    SourceReviewRequest {
        version,
        position: crate::LineIndex::new(source)
            .position(source.find(at).unwrap())
            .unwrap(),
    }
}

#[test]
fn resource_review_rewrites_unopened_consumers_with_exact_markup_and_object_spans() {
    let project = Project::new();
    let outside = Project::new();
    let dictionary = format!(
        "<ResourceDictionary {NS}><Color x:Key='Accent'>#FF215080</Color></ResourceDictionary>"
    );
    let owner = project.write("Colors.xaml", &dictionary);
    let source = format!("<Grid {NS} {EXTENSIONS} xmlns:app='using:Application'>\r\n  <Grid.Resources><ResourceInclude Source='Colors.xaml'/></Grid.Resources>\r\n  <!-- 🧭 --> <Button Background='{{r:StaticResource ResourceKey=Accent}}' Tag='{{app:StaticResource Accent}}'/>\r\n  <Border><Border.Background><r:StaticResource ResourceKey='Accent'/></Border.Background></Border>\r\n  <Border><Border.Background><DynamicResource><DynamicResource.ResourceKey><!-- Keep --> A&#x63;cent </DynamicResource.ResourceKey></DynamicResource></Border.Background></Border>\r\n  <Border><Border.Background><DynamicResource><DynamicResource.ResourceKey><![CDATA[ Accent ]]></DynamicResource.ResourceKey></DynamicResource></Border.Background></Border>\r\n</Grid>");
    let consumer = project.write("Consumer.xaml", &source);
    outside.write("Colors.xaml", &dictionary);
    outside.write("Consumer.xaml", &source);
    let mut service = LanguageService::default();
    project.index(&mut service);
    outside.index(&mut service);
    service.open(&owner, &dictionary, 4).unwrap();
    let request = request(&dictionary, "Color x:Key", 4);
    let target = service.resource_rename_target(&owner, &request).unwrap();
    assert_eq!(target.name, "Accent");
    assert_eq!(target.declaration.uri, owner);
    let plan = service
        .plan_resource_rename(&owner, &request, "Theme 🧭")
        .unwrap();
    assert_eq!(plan.documents.len(), 2);
    let edit = plan
        .documents
        .iter()
        .find(|document| document.edit.uri == consumer)
        .unwrap();
    assert!(edit.edit.version < 0);
    assert_eq!(edit.edit.edits.len(), 4);
    assert!(edit.after.contains("ResourceKey=&apos;Theme 🧭&apos;"));
    assert!(edit.after.contains("Tag='{app:StaticResource Accent}'"));
    assert!(edit.after.contains("ResourceKey='Theme 🧭'"));
    assert!(edit.after.contains("<!-- Keep --> Theme 🧭 "));
    assert!(edit.after.contains("<![CDATA[ Theme 🧭 ]]>"));
    assert_eq!(
        edit.after.matches("\r\n").count(),
        source.matches("\r\n").count()
    );
    let planned = crate::WorkspaceEdit {
        document_changes: plan
            .documents
            .into_iter()
            .map(|document| document.edit)
            .collect(),
    };
    service.apply_workspace_edit(&planned).unwrap();
    let updated = service.document(&consumer).unwrap().source();
    for (offset, _) in updated.match_indices("Theme 🧭") {
        assert_eq!(
            service.definitions(&consumer, offset).unwrap()[0].uri,
            owner
        );
    }
}

#[test]
fn resource_review_rechecks_disk_changes_and_drafts_that_hide_an_encoded_key() {
    let project = Project::new();
    let dictionary =
        format!("<ResourceDictionary {NS}><Color x:Key='Brand &amp; Ink'/></ResourceDictionary>");
    let owner = project.write("Colors.xaml", &dictionary);
    let source = format!("<Grid {NS}><Grid.Resources><ResourceInclude Source='Colors.xaml'/></Grid.Resources><Border Background=\"{{StaticResource 'Brand &amp; Ink'}}\"/></Grid>");
    let consumer = project.write("Consumer.xaml", &source);
    let mut service = LanguageService::default();
    project.index(&mut service);
    service.open(&owner, &dictionary, 4).unwrap();
    let request = request(&dictionary, "Brand", 4);
    let plan = service
        .plan_resource_rename(&owner, &request, "Accent")
        .unwrap();
    project.write(
        "Consumer.xaml",
        &format!("<!-- saved elsewhere -->{source}"),
    );
    let refreshed = service
        .plan_resource_rename(&owner, &request, "Accent")
        .unwrap();
    assert_ne!(refreshed.review, plan.review);
    service.open(&consumer, "<Grid", 6).unwrap();
    let error = service
        .plan_resource_rename(&owner, &request, "Accent")
        .unwrap_err();
    assert!(error.to_string().contains("incomplete XAML"));
    assert_eq!(service.document(&owner).unwrap().source(), dictionary);
    assert!(service
        .plan_resource_rename(
            &owner,
            &SourceReviewRequest {
                version: 3,
                ..request
            },
            "Accent"
        )
        .is_err());
}
