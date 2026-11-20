use crate::{
    DictionaryExtractionRequest, LanguageService, LineIndex, apply_text_edits, lsp::LspServer,
};
use ferroui_designer_project::{Solution, file_path, file_uri};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

const NS: &str =
    "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";
const VERSION: i64 = 16;

#[test]
fn moves_lossless_resources_and_resolves_the_consumer_to_the_new_dictionary() {
    let source = format!(
        "<Grid {NS} xmlns:ui='using:FerroUI.Controls'>\r\n  <Grid.Resources>\r\n    <!-- 🎨 Keep this comment -->\r\n    <SolidColorBrush x:Key='Accent' Color='#124578' />\r\n    <DataTemplate x:Key='Card'><ui:Border Background='{{StaticResource Accent}}'><TextBlock x:Name='Inner' Text='A &amp; B' /></ui:Border></DataTemplate>\r\n  </Grid.Resources>\r\n  <!-- Source comment --> <ListBox ItemTemplate='{{StaticResource Card}}'/>\r\n</Grid>\r\n"
    );
    let fixture = Fixture::new(&source);
    let mut service = fixture.service(&source);
    let request = request(&source, "SolidColorBrush");
    let target = service
        .dictionary_extraction_target(&fixture.uri, request)
        .unwrap();
    assert_eq!(target.entries, 2);
    assert_eq!(target.suggested_file, "View.Resources.xaml");
    let name = "Accent & cards 🎨.xaml";
    let plan = service
        .plan_dictionary_extraction(&fixture.uri, request, name)
        .unwrap();
    let next = apply_text_edits(&source, &plan.edit.edits).unwrap();
    assert_eq!(plan.edit.edits.len(), 1);
    assert!(
        next.contains("<ResourceInclude Source=\"Accent%20%26%20cards%20%F0%9F%8E%A8.xaml\" />")
    );
    assert!(
        next.contains("<!-- Source comment --> <ListBox ItemTemplate='{StaticResource Card}'/>")
    );
    assert!(plan.file.source.contains(
        "<!-- 🎨 Keep this comment -->\r\n    <SolidColorBrush x:Key='Accent' Color='#124578' />"
    ));
    assert!(
        plan.file
            .source
            .contains("xmlns:ui=\"using:FerroUI.Controls\"")
    );
    assert!(plan.file.source.contains("x:Name='Inner' Text='A &amp; B'"));
    assert!(!next.replace("\r\n", "").contains('\n'));
    assert!(!plan.file.source.replace("\r\n", "").contains('\n'));
    assert_eq!(
        fs::read_to_string(file_path(&fixture.uri).unwrap()).unwrap(),
        source
    );
    assert!(!fixture.root.join(name).exists());
    assert_eq!(
        service
            .plan_dictionary_extraction(&fixture.uri, request, name)
            .unwrap()
            .review,
        plan.review
    );
    service.open(&plan.file.uri, &plan.file.source, 1).unwrap();
    service.replace(&fixture.uri, &next, VERSION + 1).unwrap();
    let definitions = service
        .definitions(&fixture.uri, next.rfind("Card").unwrap())
        .unwrap();
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].uri, plan.file.uri);
}

#[test]
fn replaces_a_merged_dictionary_in_its_original_slot_and_keeps_relative_sources() {
    let source = format!(
        "<ResourceDictionary {NS}><ResourceDictionary.MergedDictionaries><!-- keep order --><ResourceInclude Source='First.xaml'/><ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source='First.xaml'/></ResourceDictionary.MergedDictionaries><SolidColorBrush x:Key='Card' Color='{{StaticResource Blue}}'/></ResourceDictionary><ResourceInclude Source='Last.xaml'/></ResourceDictionary.MergedDictionaries></ResourceDictionary>"
    );
    let fixture = Fixture::new(&source);
    fs::write(
        fixture.root.join("First.xaml"),
        format!("<ResourceDictionary {NS}><Color x:Key='Blue'>Blue</Color></ResourceDictionary>"),
    )
    .unwrap();
    fs::write(
        fixture.root.join("Last.xaml"),
        format!("<ResourceDictionary {NS}><Color x:Key='Red'>Red</Color></ResourceDictionary>"),
    )
    .unwrap();
    let service = fixture.service(&source);
    let plan = service
        .plan_dictionary_extraction(
            &fixture.uri,
            request(&source, "SolidColorBrush"),
            "Cards.xaml",
        )
        .unwrap();
    let next = apply_text_edits(&source, &plan.edit.edits).unwrap();
    assert!(next.contains("<!-- keep order --><ResourceInclude Source='First.xaml'/><ResourceInclude Source=\"Cards.xaml\" /><ResourceInclude Source='Last.xaml'/>"));
    assert!(
        plan.file
            .source
            .contains("<ResourceInclude Source='First.xaml'/>")
    );
    assert!(plan.file.source.contains("Color='{StaticResource Blue}'"));
}

#[test]
fn rejects_stale_or_occupied_destinations_and_scope_dependent_definitions() {
    let source = format!(
        "<Grid {NS}><Grid.Resources><Color x:Key='Outer'>Red</Color></Grid.Resources><Border><Border.Resources><SolidColorBrush x:Key='Inner' Color='{{StaticResource Outer}}'/></Border.Resources></Border></Grid>"
    );
    let fixture = Fixture::new(&source);
    let mut service = fixture.service(&source);
    let requested = request(&source, "SolidColorBrush");
    let plan = |service: &LanguageService, request, file| {
        service.plan_dictionary_extraction(&fixture.uri, request, file)
    };
    assert!(
        plan(&service, requested, "New.xaml")
            .unwrap_err()
            .0
            .contains("Static resource 'Outer'")
    );
    assert!(plan(&service, requested, "../New.xaml").is_err());
    assert!(
        plan(&service, requested, "View.xaml")
            .unwrap_err()
            .0
            .contains("already exists")
    );
    assert!(
        plan(
            &service,
            DictionaryExtractionRequest {
                version: VERSION - 1,
                ..requested
            },
            "New.xaml"
        )
        .unwrap_err()
        .0
        .contains("XAML changed")
    );
    let names = source.replace(
        "<SolidColorBrush x:Key='Inner' Color='{StaticResource Outer}'/>",
        "<Button x:Key='Named' x:Name='BoundToView' />",
    );
    service.replace(&fixture.uri, &names, VERSION + 1).unwrap();
    assert!(
        plan(
            &service,
            DictionaryExtractionRequest {
                version: VERSION + 1,
                ..request(&names, "Button")
            },
            "New.xaml"
        )
        .unwrap_err()
        .0
        .contains("namescope")
    );
    let recursive = source.replace("<SolidColorBrush x:Key='Inner' Color='{StaticResource Outer}'/>", "<ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source='View.xaml'/><ResourceInclude Source='New.xaml'/></ResourceDictionary.MergedDictionaries></ResourceDictionary>");
    service
        .replace(&fixture.uri, &recursive, VERSION + 2)
        .unwrap();
    assert!(
        plan(
            &service,
            DictionaryExtractionRequest {
                version: VERSION + 2,
                ..request(&recursive, "Source='View.xaml'")
            },
            "New.xaml"
        )
        .unwrap_err()
        .0
        .contains("Circular")
    );
}

#[test]
fn exposes_an_extract_refactoring_to_a_client_that_can_review_it() {
    let source = format!(
        "<Grid {NS}><Grid.Resources><Color x:Key='Ink'>Black</Color></Grid.Resources></Grid>"
    );
    let project = Fixture::new(&source);
    let root = file_uri(&project.root).unwrap();
    let mut lsp = LspServer::default();
    let send = |server: &mut LspServer, id, method, params| {
        server.handle(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
    };
    send(
        &mut lsp,
        1,
        "initialize",
        json!({ "rootUri": root, "initializationOptions": { "ferrouiExtractionCommands":true } }),
    );
    send(
        &mut lsp,
        2,
        "textDocument/didOpen",
        json!({ "textDocument": { "uri":project.uri, "text":source,"version":VERSION } }),
    );
    let caret = request(&source, "Color").position;
    let actions = send(
        &mut lsp,
        3,
        "textDocument/codeAction",
        json!({ "textDocument": {"uri":project.uri}, "range": {"start":caret,"end":caret}, "context": {"only":["refactor"],"diagnostics":[]} }),
    );
    assert_eq!(
        actions.last().unwrap()["result"][0]["kind"],
        "refactor.extract"
    );
    assert_eq!(
        actions.last().unwrap()["result"][0]["command"]["command"],
        "ferroui.extractResourceDictionary"
    );
}

fn request(source: &str, text: &str) -> DictionaryExtractionRequest {
    let position = LineIndex::new(source)
        .position(source.find(text).unwrap())
        .unwrap();
    DictionaryExtractionRequest {
        position,
        version: VERSION,
    }
}

struct Fixture {
    root: PathBuf,
    uri: String,
}

impl Fixture {
    fn new(source: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "ferroui-dictionary-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let file = root.join("View.xaml");
        fs::write(&file, source).unwrap();
        Self {
            root,
            uri: file_uri(file).unwrap(),
        }
    }

    fn service(&self, source: &str) -> LanguageService {
        let mut service = LanguageService::default();
        service
            .index_solution(&Solution::discover(&self.root).unwrap())
            .unwrap();
        service.open(&self.uri, source, VERSION).unwrap();
        service
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
