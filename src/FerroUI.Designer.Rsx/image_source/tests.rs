use crate::{project_rust, RsxCatalog};
use ferroui_designer_core::TextPatch;
use ferroui_designer_project::{file_uri, RustModuleSources};
use std::fs;
use std::path::PathBuf;

struct Package(PathBuf);

impl Drop for Package {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn packaged_image_edits_keep_the_cargo_address_and_authored_expression() {
    let first = "ferres://Photo.App/Assets/Old%20badge.png";
    let next = "ferres://Photo.App/Assets/%C5%81%C3%B3d%C5%BA%20%23%25.png";
    let source = format!(
        r##"use ::ferroui_view::asset_image as texture;
fn header() {{ rsx! {{ TextBlock {{ text: "Other invocation" }} }}; }}
fn badge() {{ rsx! {{ Image {{ source(texture(/* selected 🧭 */ r#"{first}"#,)) }} }}; }}"##
    )
    .replace('\n', "\r\n");
    let projection = project_rust(
        &source,
        source.find("fn badge").unwrap() + "fn badge() { rsx! {".len(),
    )
    .unwrap();
    assert_eq!(projection.omitted_count, 0);
    assert!(projection.xaml().contains(first));
    let xaml = ferroui_designer_core::Document::new(&projection.xaml()).unwrap();
    let rust = crate::xaml_to_rsx(&xaml).unwrap();
    assert!(rust.contains(&format!("::ferroui_view::asset_image({first:?})")));
    let patches = projection
        .apply_xaml(&projection.xaml().replace(first, next))
        .unwrap();
    assert_eq!(
        TextPatch::apply_all(&source, patches).unwrap(),
        source.replace(first, next)
    );

    let physical = projection
        .edit_property(0, "Source", Some("../Assets/Local%20badge.png"))
        .unwrap();
    let physical = TextPatch::apply_all(&source, vec![physical]).unwrap();
    assert!(physical.contains(r#"/* selected 🧭 */ ::ferroui_view::embedded_image(::core::include_bytes!("../Assets/Local badge.png"))"#));
    let cursor = physical.find("fn badge").unwrap() + "fn badge() { rsx! {".len();
    let projection = project_rust(&physical, cursor).unwrap();
    let publish = projection.edit_property(0, "Source", Some(next)).unwrap();
    let publish = TextPatch::apply_all(&physical, vec![publish]).unwrap();
    assert!(publish.contains(&format!("::ferroui_view::asset_image({next:?})")));
    assert!(publish.contains("/* selected 🧭 */"));
    assert!(publish.contains("text: \"Other invocation\""));
    assert!(project_rust(&publish, cursor)
        .unwrap()
        .xaml()
        .contains(next));
    let computed = source.replace(&format!(r##"r#"{first}"#"##), "active_badge()");
    let projection = project_rust(&computed, cursor).unwrap();
    assert!(!projection.nodes[0].can_set_image_source());
}

#[test]
fn cargo_include_projects_a_document_uri_and_keeps_its_authored_package_anchor() {
    let package = Package(std::env::temp_dir().join(format!(
        "ferroui-rsx-image-package-{}-{:?}",
        std::process::id(),
        std::thread::current().id(),
    )));
    fs::create_dir_all(package.0.join("src")).unwrap();
    let target = package.0.join("src/lib.rs");
    fs::write(&target, "mod screen;").unwrap();
    let screen = package.0.join("src/screen.rs");
    let source = r##"fn view() { rsx! {
        Image {
            source(::ferroui_view::embedded_image(::std::include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"), r#"/Assets/"#, /* keep 🎨 */ r#"Łódź #%.png"#,
            )))),
        }
    }; }"##
        .replace('\n', "\r\n");
    fs::write(&screen, &source).unwrap();
    let uri = file_uri(&screen).unwrap();
    let modules = RustModuleSources::capture(&package.0, &target, &uri, &source, &[]).unwrap();
    let catalog = RsxCatalog::built_in();
    let projection = catalog
        .project_with_modules(&source, 0, Some(&modules))
        .unwrap();
    assert_eq!(projection.omitted_count, 0);
    assert!(projection.nodes[0].can_set_image_source());
    let xaml = projection.xaml();
    let previous = "../Assets/%C5%81%C3%B3d%C5%BA%20%23%25.png";
    let replacement = "../Assets/New%20photo.png";
    assert!(xaml.contains(previous));
    let draft = xaml.replace(previous, replacement);
    let patches = projection.apply_xaml(&draft).unwrap();
    assert_eq!(patches.len(), 1);
    assert_eq!(
        TextPatch::apply_all(&source, patches).unwrap(),
        source.replace("Łódź #%.png", "New photo.png"),
    );
    assert!(!project_rust(&source, 0).unwrap().nodes[0].can_set_image_source());
}

#[test]
fn inferred_standard_relative_include_uses_the_verified_file_scope() {
    let source = r#"fn view() { rsx! {
        Image { source(::ferroui_view::embedded_image(include_bytes!("../assets/card.png"))) }
    }; }"#;
    let projection = project_rust(source, 0).unwrap();
    assert!(projection.nodes[0].can_set_image_source());
    let patch = projection
        .edit_property(0, "Source", Some("../assets/other.png"))
        .unwrap();
    assert_eq!(
        TextPatch::apply_all(source, vec![patch]).unwrap(),
        source.replace("card.png", "other.png"),
    );
    let shadowed =
        format!("macro_rules! include_bytes {{ ($path:literal) => {{ &[] }} }}\n{source}");
    let projection = project_rust(&shadowed, 0).unwrap();
    assert!(!projection.nodes[0].can_set_image_source());
}

#[test]
fn an_imported_image_helper_keeps_its_alias_and_assembled_path() {
    let source = r#"use ::ferroui_view::embedded_image as texture;
        fn view() { rsx! { Image { source: texture(include_bytes!(concat!("art/", "Logo", ".png"))) } }; }"#;
    let preview = project_rust(source, 0).unwrap();
    assert_eq!(preview.omitted_count, 0);
    assert!(preview.nodes[0].can_set_image_source());
    assert!(preview.xaml().contains("art/Logo.png"));
    let patches = preview
        .apply_xaml(&preview.xaml().replace("art/Logo.png", "icons/Mark.png"))
        .unwrap();
    assert_eq!(
        TextPatch::apply_all(source, patches).unwrap(),
        source
            .replace("\"art/\"", "\"icons/\"")
            .replace("\"Logo\"", "\"Mark\"")
    );
    assert!(!preview.can_replace_structure());
    assert!(preview
        .apply_xaml("<Button xmlns='https://github.com/ferroui' />")
        .is_err());

    for unrelated in [
        source.replace(
            "::ferroui_view::embedded_image",
            "::other_api::embedded_image",
        ),
        source.replace("fn view() {", "fn view() { let texture = custom_loader;"),
    ] {
        assert!(!project_rust(&unrelated, 0).unwrap().nodes[0].can_set_image_source());
    }
}
