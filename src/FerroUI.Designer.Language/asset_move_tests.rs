use crate::{FileMoveDraft, LanguageService};
use ferroui_designer_project::{Solution, file_uri};
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
    let rust = "fn view() { rsx! { Image { source: ::ferroui_view::embedded_image(/* art 🎨 */ ::core::include_bytes!(r#\"../Assets/Logo Ł.png\"#)) } }; }\r\n";
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
    assert!(
        xaml.after
            .contains("Source='../Icons/New%20%23.png#sample'")
    );
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
            .replace("../Assets/Logo Ł.png", "../Icons/New #.png")
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
    assert!(
        language
            .plan_asset_move(&from, &to, &[draft])
            .unwrap_err()
            .0
            .contains("Repair the Rust draft")
    );
    assert!(
        language
            .plan_asset_move(&from, &uri("Icons/New.svg"), &[])
            .is_err()
    );
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
