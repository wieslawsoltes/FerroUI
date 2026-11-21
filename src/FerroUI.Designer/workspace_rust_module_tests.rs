use super::*;
use crate::mcp::{DesignerMcp, RSX_STYLE_EDIT, RSX_STYLES};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Control, Window};
use ferroui_designer_project::{CargoTarget, ProjectDescriptor, Solution, file_uri};
use ferroui_mcp::{Permissions, UiToolProvider};
use serde_json::json;
use std::{fs, path::PathBuf};

const VIEW: &str = r#"use crate::brand::{INK, KEY};
fn overview() { rsx! { TextBlock { text: "Overview" } }; }
fn review() {
    rsx! { StackPanel {
        styles: [ferroui_view::style("TextBlock")
            .resource(KEY, (/* style reference */ INK))
            .set_resource(ferroui_controls::TextBlock::foreground_property(), KEY)],
        TextBlock { name: "Review", text: "Customer review" },
    } };
}"#;
const BRAND: &str = "use ferroui_base::media::Color as Paint;\r\npub(crate) const KEY: &str = \"Ink\";\r\npub(crate) const INK: Paint = Paint::from_rgb(37u8, /* 🎨 */ 74, 112);\r\n";

struct Files(PathBuf);

impl Drop for Files {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(feature = "render-smoke")]
#[test]
fn studio_replaces_a_cargo_relative_rsx_image_through_the_authored_source() {
    use base64::Engine;
    use ferroui_base::platform::{IFontManagerImpl, IPlatformRenderInterface};
    use ferroui_base::{FerroLocator, LocatorExtensions, Size};
    use ferroui_controls::Image;
    use ferroui_harfbuzz::HarfBuzzTextShaper;
    use ferroui_vello::{VelloOptions, VelloPlatform, VelloRenderingMode};

    let _locator = FerroLocator::enter_scope();
    VelloPlatform::initialize_with_options(VelloOptions::with_rendering_mode(
        VelloRenderingMode::Cpu,
    ));
    let locator = FerroLocator::current();
    let _app = UnitTestApplication::start(TestServices {
        render_interface: locator.get_service::<dyn IPlatformRenderInterface>(),
        font_manager_impl: locator.get_service::<dyn IFontManagerImpl>(),
        text_shaper_impl: Some(Rc::new(HarfBuzzTextShaper::new())),
        ..TestServices::styled_window()
    });
    let root =
        std::env::temp_dir().join(format!("ferroui-studio-rust-image-{}", std::process::id()));
    fs::create_dir_all(root.join("src")).unwrap();
    let files = Files(root.canonicalize().unwrap());
    fs::create_dir_all(files.0.join("Assets")).unwrap();
    let manifest = files.0.join("Cargo.toml");
    fs::write(
        &manifest,
        "[package]\nname='review-image'\nversion='0.0.0'\n",
    )
    .unwrap();
    let target = files.0.join("src/lib.rs");
    fs::write(&target, "mod review;").unwrap();
    let view = files.0.join("src/review.rs");
    let source = r##"fn view() { rsx! { Image {
        source(::ferroui_view::embedded_image(::core::include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"), /* paint 🧭 */ r#"/Assets/Card.png"#,
        )))),
    } }; }"##;
    fs::write(&view, source).unwrap();
    for (path, png) in [
        ("Assets/Card.png", "iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAE0lEQVR4nGNMjb3OAANMcBZeDgBQdAGhOsFZtwAAAABJRU5ErkJggg=="),
        ("Assets/Next Image.png", "iVBORw0KGgoAAAANSUhEUgAAAAYAAAADCAIAAAA/Y+msAAAAEUlEQVR4nGMwDpqKhhiIEgIALo0T1QA/I98AAAAASUVORK5CYII="),
    ] {
        let image = base64::engine::general_purpose::STANDARD.decode(png).unwrap();
        fs::write(files.0.join(path), image).unwrap();
    }
    let workspace = DesignerWorkspace::with_uri(source, file_uri(&view).unwrap()).unwrap();
    workspace
        .set_solution(Solution {
            format_version: 1,
            root: files.0.clone(),
            projects: vec![ProjectDescriptor {
                id: "review-image".into(),
                name: "review-image".into(),
                root: files.0.clone(),
                manifest_path: manifest,
                documents: Vec::new(),
                targets: vec![CargoTarget {
                    name: "review-image".into(),
                    kind: vec!["lib".into()],
                    source: target,
                }],
            }],
        })
        .unwrap();
    workspace.refresh(true);
    let session = workspace.session();
    let image_size = || {
        let session = session.borrow();
        assert!(session.can_manipulate(), "{:?}", session.preview_status());
        session
            .preview
            .root()
            .unwrap()
            .cast::<Image>()
            .unwrap()
            .source()
            .unwrap()
            .size()
    };
    assert_eq!(image_size(), Size::new(4.0, 4.0));
    let (node, revision) = {
        let session = session.borrow();
        assert!(session
            .document()
            .source()
            .contains("Source=\"../Assets/Card.png\""));
        (
            session.document().ast().root(),
            session.document().revision(),
        )
    };
    let provider = DesignerMcp::with_workspace(&workspace);
    let writes = Permissions {
        allow_property_writes: true,
        ..Default::default()
    };
    provider.call("designer_apply", &json!({
        "label": "Replace image", "expected_revision": revision,
        "edits": [{"SetAttribute":{"node": node, "name": "Source", "value": "../Assets/Next%20Image.png"}}],
    }), writes).unwrap();
    assert_eq!(
        session.borrow().authored_source(),
        source.replace("/Assets/Card.png", "/Assets/Next Image.png")
    );
    assert_eq!(image_size(), Size::new(6.0, 3.0));
    let revision = session.borrow().document().revision();
    provider
        .call(
            "designer_undo",
            &json!({"expected_revision": revision}),
            writes,
        )
        .unwrap();
    assert_eq!(session.borrow().authored_source(), source);
    assert_eq!(image_size(), Size::new(4.0, 4.0));
}

#[test]
fn studio_edits_and_undoes_a_palette_module_from_either_tab() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let root =
        std::env::temp_dir().join(format!("ferroui-studio-rust-module-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let files = Files(root.canonicalize().unwrap());
    let manifest = files.0.join("Cargo.toml");
    let main = files.0.join("main.rs");
    let view_path = files.0.join("review.rs");
    let brand_path = files.0.join("brand.rs");
    fs::write(&manifest, "[package]\nname='review'\nversion='0.0.0'\n").unwrap();
    fs::write(&main, "mod brand; mod review; fn main() {}\n").unwrap();
    fs::write(&view_path, VIEW).unwrap();
    fs::write(&brand_path, BRAND).unwrap();
    let view = file_uri(&view_path).unwrap();
    let brand = file_uri(&brand_path).unwrap();
    let workspace = DesignerWorkspace::with_uri(VIEW, view.clone()).unwrap();
    workspace
        .set_solution(Solution {
            format_version: 1,
            root: files.0.clone(),
            projects: vec![ProjectDescriptor {
                id: "review".into(),
                name: "review".into(),
                root: files.0.clone(),
                manifest_path: manifest,
                documents: Vec::new(),
                targets: vec![CargoTarget {
                    name: "review".into(),
                    kind: vec!["bin".into()],
                    source: main,
                }],
            }],
        })
        .unwrap();
    let session = workspace.session();
    let revision = || session.borrow().document().revision();
    let caret = VIEW.find("rsx! { StackPanel").unwrap();
    session.borrow_mut().select_rust_invocation(caret).unwrap();
    workspace.refresh(true);
    let window = Window::new();
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let provider = DesignerMcp::with_workspace(&workspace);
    let read = |tool, args| provider.call(tool, &args, Permissions::default()).unwrap();
    let writes = Permissions {
        allow_property_writes: true,
        ..Default::default()
    };
    let color = || {
        let node = session
            .borrow()
            .document()
            .ast()
            .find_name("Review")
            .unwrap();
        read(
            "designer_property_trace",
            json!({"node":node,"property":"TextElement.Foreground"}),
        )["effective"]
            .clone()
    };
    assert_eq!(color(), "#FF254A70");
    let style = read(RSX_STYLES, json!({}))["definitions"][0]["node"].clone();
    let resource = read(RSX_STYLES, json!({"node":style}))["color_resources"][0].clone();
    assert_eq!(resource["sourceAddress"], brand);
    let edit = json!({"action":"color_resource","style":style,"resource":resource["node"],
        "mode":"declaration","value":"#FF224064","expected_revision":revision()});
    provider.call(RSX_STYLE_EDIT, &edit, writes).unwrap();
    assert_eq!(session.borrow().authored_source(), VIEW);
    assert_eq!(color(), "#FF224064");
    assert!(provider.call(RSX_STYLE_EDIT, &edit, writes).is_err());

    let current = read(RSX_STYLES, json!({"node":style}));
    read(
        "designer_rsx_style_source",
        json!({"address":brand,"range":current["color_resources"][0]["range"],"expected_revision":revision()}),
    );
    assert_eq!(session.borrow().document_uri(), brand);
    assert_eq!(
        session.borrow().authored_source(),
        BRAND.replace("37u8, /* 🎨 */ 74, 112", "34u8, /* 🎨 */ 64, 100")
    );
    assert_eq!(fs::read_to_string(&brand_path).unwrap(), BRAND);
    provider
        .call(
            "designer_undo",
            &json!({"expected_revision":revision()}),
            writes,
        )
        .unwrap();
    assert_eq!(session.borrow().authored_source(), BRAND);
    workspace.activate_document(&view).unwrap();
    assert_eq!(color(), "#FF254A70");
    provider
        .call(
            "designer_redo",
            &json!({"expected_revision":revision()}),
            writes,
        )
        .unwrap();
    assert_eq!(color(), "#FF224064");
    provider
        .call(
            "designer_undo",
            &json!({"expected_revision":revision()}),
            writes,
        )
        .unwrap();
    assert_eq!(color(), "#FF254A70");
    window.close();
}
