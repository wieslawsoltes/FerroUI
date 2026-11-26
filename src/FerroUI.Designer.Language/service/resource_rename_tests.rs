use super::*;
use ferroui_designer_project::{Solution, file_uri};
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
    let source = format!(
        "<Grid {NS} {EXTENSIONS} xmlns:app='using:Application'>\r\n  <Grid.Resources><ResourceInclude Source='Colors.xaml'/></Grid.Resources>\r\n  <!-- 🧭 --> <Button Background='{{r:StaticResource ResourceKey=Accent}}' Tag='{{app:StaticResource Accent}}'/>\r\n  <Border><Border.Background><r:StaticResource ResourceKey='Accent'/></Border.Background></Border>\r\n  <Border><Border.Background><DynamicResource><DynamicResource.ResourceKey><!-- Keep --> A&#x63;cent </DynamicResource.ResourceKey></DynamicResource></Border.Background></Border>\r\n  <Border><Border.Background><DynamicResource><DynamicResource.ResourceKey><![CDATA[ Accent ]]></DynamicResource.ResourceKey></DynamicResource></Border.Background></Border>\r\n</Grid>"
    );
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
    let source = format!(
        "<Grid {NS}><Grid.Resources><ResourceInclude Source='Colors.xaml'/></Grid.Resources><Border Background=\"{{StaticResource 'Brand &amp; Ink'}}\"/></Grid>"
    );
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
    assert!(
        service
            .plan_resource_rename(
                &owner,
                &SourceReviewRequest {
                    version: 3,
                    ..request
                },
                "Accent"
            )
            .is_err()
    );
}

#[test]
fn theme_family_follows_inline_and_included_layers_and_preserves_local_shadows() {
    let project = Project::new();
    let outside = Project::new();
    let palette = format!(
        "<ResourceDictionary {NS}><ResourceDictionary.ThemeDictionaries>\r\n<ResourceDictionary x:Key='Default'><Color x:Key='Accent'>Gray</Color></ResourceDictionary>\r\n<ResourceInclude x:Key='{{x:Static ThemeVariant.Light}}' Source='Day.xaml'/>\r\n<ResourceDictionary x:Key='Dark'><Color x:Key='Accent'>Black</Color></ResourceDictionary>\r\n</ResourceDictionary.ThemeDictionaries></ResourceDictionary>"
    );
    let light = format!(
        "<ResourceDictionary {NS}><Color x:Key='Accent'>White</Color><Color x:Key='Taken'>Pink</Color></ResourceDictionary>"
    );
    let consumer = format!(
        "<Grid {NS}>\r\n<Grid.Resources><ResourceInclude Source='Palette.xaml'/></Grid.Resources>\r\n<!-- 🪴 --> <Border Background='{{DynamicResource Accent}}'/>\r\n<Border><Border.Resources><Color x:Key='Accent'>Purple</Color></Border.Resources><Button Background='{{StaticResource Accent}}'/></Border></Grid>"
    );
    let palette_uri = project.write("Palette.xaml", &palette);
    let day_uri = project.write("Day.xaml", &light);
    let view_uri = project.write("View.xaml", &consumer);
    outside.write("Day.xaml", &light);
    outside.write("Palette.xaml", &palette);
    outside.write("View.xaml", &consumer);
    let mut service = LanguageService::default();
    project.index(&mut service);
    outside.index(&mut service);
    service.open(&view_uri, &consumer, 2).unwrap();
    let selection = request(&consumer, "Accent", 2);
    let find = service
        .resource_rename_target(&view_uri, &selection)
        .unwrap();
    let family = find.theme_family.unwrap();
    assert_eq!(family.owner.uri, palette_uri);
    assert_eq!(family.members.len(), 3);
    assert!(
        family
            .members
            .iter()
            .any(|member| member.layers == ["Light"] && member.declaration.uri == day_uri)
    );
    assert!(
        service
            .plan_resource_rename(&view_uri, &selection, "Taken")
            .unwrap_err()
            .0
            .contains("already resolves")
    );
    let plan = service
        .plan_resource_rename(&view_uri, &selection, "Window Accent 🪴")
        .unwrap();
    assert_eq!(plan.documents.len(), 3);
    assert!(plan.theme_family.is_some());
    let view = plan
        .documents
        .iter()
        .find(|entry| entry.edit.uri == view_uri)
        .unwrap();
    assert_eq!(view.edit.edits.len(), 1);
    assert!(view.after.contains("StaticResource Accent"));
    assert!(view.after.contains("x:Key='Accent'>Purple"));
    assert_eq!(
        view.after.matches("\r\n").count(),
        consumer.matches("\r\n").count()
    );
    let plans = plan.documents.into_iter().map(|entry| entry.edit).collect();
    service
        .apply_workspace_edit(&crate::WorkspaceEdit {
            document_changes: plans,
        })
        .unwrap();
    let updated = service.document(&view_uri).unwrap().source();
    let at = updated.find("Window Accent").unwrap();
    assert_eq!(service.definitions(&view_uri, at).unwrap().len(), 3);
}

#[test]
fn theme_family_stops_when_a_variant_has_an_unrelated_fallback() {
    let project = Project::new();
    let palette = format!(
        "<ResourceDictionary {NS}><ResourceDictionary.ThemeDictionaries><ResourceDictionary x:Key='Light'><Color x:Key='Accent'>White</Color></ResourceDictionary><ResourceDictionary x:Key='Dark'><Color x:Key='Accent'>Black</Color></ResourceDictionary></ResourceDictionary.ThemeDictionaries><ResourceDictionary.MergedDictionaries><ResourceInclude Source='Fallback.xaml'/></ResourceDictionary.MergedDictionaries></ResourceDictionary>"
    );
    let owner = project.write("Palette.xaml", &palette);
    project.write(
        "Fallback.xaml",
        &format!(
            "<ResourceDictionary {NS}><Color x:Key='Accent'>Gray</Color></ResourceDictionary>"
        ),
    );
    project.write("Page.xaml", &format!("<Grid {NS}><Grid.Resources><ResourceInclude Source='Palette.xaml'/></Grid.Resources><Button Background='{{DynamicResource Accent}}'/></Grid>"));
    let mut service = LanguageService::default();
    project.index(&mut service);
    service.open(&owner, &palette, 4).unwrap();
    let selection = request(&palette, "Accent", 4);
    assert!(
        service
            .resource_rename_target(&owner, &selection)
            .unwrap()
            .theme_family
            .is_some()
    );
    let error = service
        .plan_resource_rename(&owner, &selection, "Surface")
        .unwrap_err();
    assert!(error.0.contains("change the resolution"), "{error}");
    assert_eq!(service.document(&owner).unwrap().source(), palette);
}

#[test]
fn scoped_theme_family_rename_reaches_nested_contrast_colors() {
    let project = Project::new();
    let palette = format!(
        r#"<ResourceDictionary {NS} xmlns:t="using:FerroUI.Themes.WinUI">
  <ResourceDictionary.ThemeDictionaries>
    <ResourceDictionary x:Key="Default"><Color x:Key="Accent">Gray</Color></ResourceDictionary>
    <ResourceDictionary x:Key="Dark"><Color x:Key="Accent">Black</Color></ResourceDictionary>
    <ResourceInclude x:Key="{{x:Static t:WinUITheme.HighContrast}}" Source="Contrast.xaml"/>
  </ResourceDictionary.ThemeDictionaries>
</ResourceDictionary>"#,
    );
    let contrast = format!(
        "<ResourceDictionary {NS}><ResourceDictionary.MergedDictionaries><ResourceInclude Source='ContrastColors.xaml'/></ResourceDictionary.MergedDictionaries></ResourceDictionary>"
    );
    let colors = format!(
        "<ResourceDictionary {NS}><Color x:Key='Accent'>Yellow</Color></ResourceDictionary>"
    );
    let consumer = format!(
        "<Grid {NS}>\r\n<Grid.Resources><ResourceInclude Source='Palette.xaml'/></Grid.Resources>\r\n<!-- 🧭 --> <Border Background='{{DynamicResource Accent}}'/>\r\n<Border><Border.Resources><Color x:Key='Accent'>Purple</Color></Border.Resources><Button Background='{{StaticResource Accent}}'/></Border>\r\n</Grid>"
    );
    let owner = project.write("Palette.xaml", &palette);
    let include = project.write("Contrast.xaml", &contrast);
    let contrast_colors = project.write("ContrastColors.xaml", &colors);
    let view = project.write("Review.xaml", &consumer);
    let mut service = LanguageService::default();
    project.index(&mut service);
    service.open(&view, &consumer, 1).unwrap();
    let origin = request(&consumer, "Accent", 1);
    let family = service
        .resource_rename_target(&view, &origin)
        .unwrap()
        .theme_family
        .unwrap();
    assert_eq!(family.owner.uri, owner);
    assert_eq!(family.members.len(), 3);
    assert!(family.members.iter().any(|entry| {
        entry.layers == ["HighContrast"] && entry.declaration.uri == contrast_colors
    }));
    let plan = service
        .plan_resource_rename(&view, &origin, "Review Accent 🧭")
        .unwrap();
    assert_eq!(plan.documents.len(), 3);
    assert!(plan.documents.iter().all(|draft| draft.edit.uri != include));
    let view_edit = plan
        .documents
        .iter()
        .find(|draft| draft.edit.uri == view)
        .unwrap();
    assert_eq!(
        view_edit.after,
        consumer.replace(
            "DynamicResource Accent",
            "DynamicResource &apos;Review Accent 🧭&apos;"
        )
    );
    service
        .apply_workspace_edit(&crate::WorkspaceEdit {
            document_changes: plan.documents.into_iter().map(|entry| entry.edit).collect(),
        })
        .unwrap();
    let updated = service.document(&view).unwrap().source();
    let key = updated.find("Review Accent 🧭").unwrap();
    assert_eq!(service.definitions(&view, key).unwrap().len(), 3);
}

#[test]
fn lsp_refreshes_open_consumers_after_a_nested_theme_dictionary_changes_or_closes() {
    use serde_json::{json, Value};

    let project = Project::new();
    let view = format!(
        "<Grid {NS}><Grid.Resources><ResourceInclude Source='Palette.xaml'/></Grid.Resources><Border Background='{{DynamicResource Accent}}'/></Grid>"
    );
    let palette = format!(
        "<ResourceDictionary {NS} xmlns:t='using:FerroUI.Themes.WinUI'><ResourceDictionary.ThemeDictionaries><ResourceInclude x:Key='{{x:Static t:WinUITheme.HighContrast}}' Source='Colors.xaml'/></ResourceDictionary.ThemeDictionaries></ResourceDictionary>"
    );
    let colors = format!(
        "<ResourceDictionary {NS}><SolidColorBrush x:Key='Accent' Color='Yellow'/></ResourceDictionary>"
    );
    let unrelated = format!("<Grid {NS}/>");
    let view_uri = project.write("View.xaml", &view);
    let other_uri = project.write("Unrelated.xaml", &unrelated);
    let colors_uri = project.write("Colors.xaml", &colors);
    project.write("Palette.xaml", &palette);

    let mut server = crate::lsp::LspServer::default();
    project.index(&mut server.service);
    server.handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}));
    for (uri, source) in [
        (&view_uri, &view),
        (&other_uri, &unrelated),
        (&colors_uri, &colors),
    ] {
        server.handle(json!({"jsonrpc":"2.0","method":"textDocument/didOpen",
            "params":{"textDocument":{"uri":uri,"version":1,"text":source}}}));
    }

    let assert_consumer = |published: Vec<Value>, missing: bool| {
        assert!(published
            .iter()
            .all(|message| message["params"]["uri"] != other_uri));
        let view = published
            .iter()
            .find(|message| {
                message["method"] == "textDocument/publishDiagnostics"
                    && message["params"]["uri"] == view_uri
            })
            .expect("the open consumer must receive fresh diagnostics");
        assert_eq!(view["params"]["version"], 1);
        let entries = view["params"]["diagnostics"].as_array().unwrap();
        assert_eq!(
            entries
                .iter()
                .any(|entry| entry["code"] == "unresolved-reference"),
            missing
        );
    };
    let draft = colors.replace("Accent", "TemporaryAccent");
    for (version, source, unresolved) in [(2, &draft, true), (3, &colors, false), (4, &draft, true)]
    {
        let published = server.handle(json!({"jsonrpc":"2.0","method":"textDocument/didChange",
            "params":{"textDocument":{"uri":colors_uri,"version":version},"contentChanges":[{"text":source}]}}));
        assert_consumer(published, unresolved);
    }
    let published = server.handle(json!({"jsonrpc":"2.0","method":"textDocument/didClose",
        "params":{"textDocument":{"uri":colors_uri}}}));
    assert_consumer(published, false);
}
