use crate::{FileMoveDraft, LanguageService};
use ferroui_designer_project::{file_uri, Solution};
use std::fs;

#[test]
fn asset_move_rewrites_xaml_and_rsx_operands_with_live_draft_precedence() {
    let files = crate::project_navigation_tests::CatalogFixture::create();
    let root = &files.0;
    for subdir in ["Assets", "Icons", "Views"] {
        fs::create_dir(root.join(subdir)).unwrap();
    }
    fs::write(root.join("Assets/Logo Ł.png"), [0, 19, 254, 42]).unwrap();
    let authored = concat!(
        "<StackPanel xmlns='https://github.com/ferroui'>\r\n",
        "  <StackPanel.Styles><Style Selector='Image.hero'><Setter Property='Source' Value='../Assets/Logo%20%C5%81.png'/></Style></StackPanel.Styles>\r\n",
        "  <Image Source='../Assets/Logo%20%C5%81.png#sample'/>\r\n",
        "  <!-- 🪴 Logo stays here --> <TextBlock Text='../Assets/Logo%20%C5%81.png'/>\r\n",
        "</StackPanel>",
    );
    files.write("Views/View.xaml", authored);
    let rust = "fn view() { rsx! { Image { source: ::ferroui_view::embedded_image(::core::include_bytes!(concat!(r#\"../Assets/\"#, /* art 🎨 */ r#\"Logo Ł\"#, \".png\"))) } }; }\r\n";
    files.write("Views/Card.rs", rust);
    let mut language = LanguageService::default();
    language
        .index_solution(&Solution::discover(root).unwrap())
        .unwrap();
    let uri = |path: &str| file_uri(root.join(path)).unwrap();
    let from = uri("Assets/Logo Ł.png");
    let to = uri("Icons/New #.png");
    let mut draft = FileMoveDraft {
        uri: uri("Views/Card.rs"),
        version: 7,
        source: rust.replace("art", "draft"),
    };
    let plan = language
        .plan_asset_move(&from, &to, std::slice::from_ref(&draft))
        .unwrap();
    assert_eq!(plan.documents.len(), 2);
    let xaml = plan
        .documents
        .iter()
        .find(|doc| doc.edit.uri.ends_with("View.xaml"))
        .unwrap();
    assert_eq!(xaml.edit.edits.len(), 2);
    assert!(xaml.after.contains("Value='../Icons/New%20%23.png'"));
    assert!(xaml
        .after
        .contains("Source='../Icons/New%20%23.png#sample'"));
    assert!(xaml.after.contains("Text='../Assets/Logo%20%C5%81.png'"));
    let rsx = plan
        .documents
        .iter()
        .find(|doc| doc.edit.uri.ends_with("Card.rs"))
        .unwrap();
    assert_eq!(rsx.edit.version, 7);
    assert_eq!(
        rsx.after,
        draft
            .source
            .replace("../Assets/", "../Icons/")
            .replace("Logo Ł", "New #")
    );
    assert_eq!(
        plan,
        language
            .plan_asset_move(&from, &to, std::slice::from_ref(&draft))
            .unwrap()
    );

    fs::write(root.join("Assets/Logo Ł.png"), [0, 19, 254, 18]).unwrap();
    let rereview = language
        .plan_asset_move(&from, &to, std::slice::from_ref(&draft))
        .unwrap();
    assert_ne!(plan.review, rereview.review);
    draft.source = "fn incomplete(".into();
    assert!(language
        .plan_asset_move(&from, &to, &[draft])
        .unwrap_err()
        .0
        .contains("Repair the Rust draft"));
    assert!(language
        .plan_asset_move(&from, &uri("Icons/New.svg"), &[])
        .is_err());
}

#[test]
fn asset_move_preserves_font_family_fallbacks_and_typed_relative_source() {
    let files = crate::project_navigation_tests::CatalogFixture::create();
    let root = &files.0;
    fs::create_dir(root.join("Assets")).unwrap();
    fs::write(root.join("Assets/Display.otf"), [0, 14, 55]).unwrap();
    let source = "<TextBlock xmlns='https://github.com/ferroui' FontFamily='Arial, Assets/Display.otf#Display &amp; Text, Default'/>";
    files.write("View.xaml", source);
    let uri = |path: &str| file_uri(root.join(path)).unwrap();
    let mut language = LanguageService::default();
    language
        .index_solution(&Solution::discover(root).unwrap())
        .unwrap();
    let plan = language
        .plan_asset_move(&uri("Assets/Display.otf"), &uri("Copy.otf"), &[])
        .unwrap();
    assert_eq!(
        plan.documents[0].after,
        source.replace("Assets/Display.otf", "./Copy.otf")
    );
}

#[test]
fn published_move_rebases_the_closed_export_and_all_checked_consumers() {
    use ferroui_build_scan::packaged_assets::{
        AssetDeclaration, AssetPublication, PackagedAsset, PackagedAssetIndex,
    };
    use ferroui_designer_project::RustAssetReferences;

    let files = crate::project_navigation_tests::CatalogFixture::create();
    let metadata = files.build();
    let app = files.0.join("app");
    fs::create_dir_all(app.join("Assets/Archive")).unwrap();
    fs::create_dir(app.join("Outside")).unwrap();
    let previous = "Assets/Badge Ł.png";
    let target = "Assets/Archive/Mark #.png";
    let original = "ferres://Customer_App/Assets/Badge%20%C5%81.png";
    let relocated = "ferres://Customer_App/Assets/Archive/Mark%20%23.png";
    let old_registration = "/assets/badge ł.png";
    fs::write(app.join(previous), [0, 19, 254, 42]).unwrap();
    let build = format!(
        concat!(
            "use ferroui_build::{{Build, ExportPlan}};\r\n",
            "fn main() {{ /* 🧭 */ Build::from_env().embed_assets(&[\"Assets\"])\r\n",
            "    .export_plan(ExportPlan::closed().asset(/* selection */ r#\"{}\"#)).run(); }}\r\n",
        ),
        old_registration
    );
    files.write("app/build.rs", &build);
    let site = |method: &str, value: &str| {
        let index = build.find(method).unwrap();
        let (prefix, line) = build[..index].rsplit_once('\n').unwrap();
        AssetDeclaration {
            value: value.into(),
            rust_file: "app/build.rs".into(),
            line: (prefix.lines().count() + 1) as u32,
            column: (line.chars().count() + 1) as u32,
        }
    };
    let receipt = PackagedAssetIndex {
        manifest: metadata.manifest.clone(),
        assembly: "customer_app".into(),
        crate_name: "customer_app".into(),
        assets: vec![PackagedAsset {
            key: format!("/{previous}"),
            source: previous.into(),
        }],
    };
    let publication = AssetPublication {
        directories: vec![site("embed_assets", "Assets")],
        registrations: Some(vec![site("asset(/* selection", old_registration)]),
    };
    fs::write(
        metadata.directories[0].join(PackagedAssetIndex::FILE_NAME),
        receipt.to_json_with_publication(&publication),
    )
    .unwrap();

    let xaml = format!("<StackPanel xmlns='https://github.com/ferroui'><Image Source='{original}#preview'/><TextBlock Text='{original}'/></StackPanel>");
    files.write("app/View.xaml", &xaml);
    let rust = format!(
        r##"fn badge() {{ rsx! {{ Image {{ source: ::ferroui_view::asset_image(/* keep 🎨 */ r#"{original}"#) }} }}; }}
fn backup() {{ let _ = include_bytes!("Assets/Badge Ł.png"); }}
fn alternate() {{ let _ = concat!("ferres://Customer_App/Assets/", /* same suffix */ "Badge%20%C5%81", ".png"); }}"##
    );
    files.write("app/Banner.rs", &rust);
    let mut service = LanguageService::default();
    service
        .index_solution(&Solution::discover(&files.0).unwrap())
        .unwrap();
    service.set_build_metadata(&metadata).unwrap();
    let uri = |path: &str| file_uri(app.join(path)).unwrap();
    let from = uri(previous);
    let to = uri(target);
    let owner = uri("View.xaml");
    let movement = service.plan_asset_move(&from, &to, &[]).unwrap();
    assert_eq!(movement.published_assets.len(), 1);
    assert_eq!(movement.documents.len(), 3);
    let after = |path: &str| {
        movement
            .documents
            .iter()
            .find(|doc| doc.edit.uri == uri(path))
            .unwrap()
            .after
            .as_str()
    };
    assert_eq!(
        after("build.rs"),
        build.replace(old_registration, &format!("/{target}"))
    );
    assert_eq!(
        after("View.xaml"),
        xaml.replace(
            &format!("{original}#preview"),
            &format!("{relocated}#preview")
        )
    );
    let updated = after("Banner.rs");
    assert!(updated.contains("/* keep 🎨 */"));
    assert!(updated.contains("/* same suffix */"));
    let refs = RustAssetReferences::parse(updated, false).unwrap();
    assert_eq!(refs.includes[0].path, target);
    assert_eq!(refs.addresses.len(), 2);
    assert!(refs.addresses.iter().all(|site| site.path == relocated));

    let stale = FileMoveDraft {
        uri: uri("build.rs"),
        version: 8,
        source: format!("// shifted build declaration\n{build}"),
    };
    assert!(service
        .plan_asset_move(&from, &to, &[stale])
        .unwrap_err()
        .0
        .contains("Cannot review"));
    assert!(service
        .plan_asset_move(&from, &uri("Outside/Mark.png"), &[])
        .unwrap_err()
        .0
        .contains("embedded"));

    fs::rename(app.join(previous), app.join(target)).unwrap();
    service.rename_uri(&from, &to).unwrap();
    assert_eq!(service.resource_uri(&owner, relocated).unwrap(), to);
    fs::rename(app.join(target), app.join(previous)).unwrap();
    service.rename_uri(&to, &from).unwrap();
    assert_eq!(service.resource_uri(&owner, original).unwrap(), from);
}
