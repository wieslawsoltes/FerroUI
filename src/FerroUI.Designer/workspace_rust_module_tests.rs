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
    let source = r##"use ::ferroui_view::embedded_image as raster;
    fn view() { rsx! { Image {
        source(raster(::core::include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"), "/Assets/", /* paint 🧭 */ r#"Card"#, ".png",
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
        source.replace("r#\"Card\"#", "r#\"Next Image\"#")
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

#[test]
fn studio_column_edit_changes_the_companion_buffer_and_links_undo() {
    const VIEW: &str = r#"use crate::account_columns::shared;
fn view() { rsx! { TableView { name: "Accounts", columns: shared() } }; }
"#;
    const COLUMNS: &str = r#"use ferroui_view::{table_columns as list, table_column};
pub fn shared() -> Columns {
    list([table_column().header(/* 🧭 */ "Account")])
}
"#;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let root = std::env::temp_dir().join(format!("ferroui-studio-rust-columns-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let files = Files(root.canonicalize().unwrap());
    let manifest = files.0.join("Cargo.toml");
    let main = files.0.join("main.rs");
    let view_path = files.0.join("review.rs");
    let column_path = files.0.join("account_columns.rs");
    fs::write(&manifest, "[package]\nname='review'\nversion='0.0.0'\n").unwrap();
    fs::write(&main, "mod account_columns; mod review; fn main() {}\n").unwrap();
    fs::write(&view_path, VIEW).unwrap();
    fs::write(&column_path, COLUMNS).unwrap();
    let view_uri = file_uri(&view_path).unwrap();
    let column_uri = file_uri(&column_path).unwrap();
    let workspace = DesignerWorkspace::with_uri(VIEW, view_uri.clone()).unwrap();
    workspace.set_solution(Solution {
        format_version: 1, root: files.0.clone(),
        projects: vec![ProjectDescriptor {
            id: "review".into(), name: "review".into(), root: files.0.clone(), manifest_path: manifest,
            documents: Vec::new(),
            targets: vec![CargoTarget { name: "review".into(), kind: vec!["bin".into()], source: main }],
        }],
    }).unwrap();
    let window = Window::new();
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let session = workspace.session();
    let provider = DesignerMcp::with_workspace(&workspace);
    let read = || {
        let table = session.borrow().document().ast().find_name("Accounts").unwrap();
        provider.call("designer_columns", &json!({"node": table}), Permissions::default()).unwrap()
    };
    let writes = Permissions { allow_property_writes: true, ..Default::default() };
    let columns = read();
    assert_eq!(columns["sourceUri"], column_uri);
    assert_eq!(columns["factory"], "shared");
    assert_eq!(columns["columns"][0]["label"], "Account");
    let edit = json!({
        "table": columns["table"], "column": columns["columns"][0]["node"],
        "expected_model": columns["model"], "expected_revision": columns["revision"],
        "action": "set", "field": "Header", "value": "Customer",
    });
    provider.call("designer_columns_edit", &edit, writes).unwrap();
    assert!(provider.call("designer_columns_edit", &edit, writes).is_err());
    assert_eq!(session.borrow().authored_source(), VIEW);
    assert_eq!(read()["columns"][0]["label"], "Customer");
    workspace.activate_document(&column_uri).unwrap();
    assert_eq!(session.borrow().authored_source(), COLUMNS.replace("\"Account\"", "\"Customer\""));
    assert_eq!(fs::read_to_string(&column_path).unwrap(), COLUMNS);
    let revision = session.borrow().document().revision();
    provider.call("designer_undo", &json!({"expected_revision": revision}), writes).unwrap();
    assert_eq!(session.borrow().authored_source(), COLUMNS);
    workspace.activate_document(&view_uri).unwrap();
    assert_eq!(read()["columns"][0]["label"], "Account");
    window.close();
}

#[test]
fn studio_opens_a_shared_cell_body_and_undoes_the_assignment_from_its_module() {
    const VIEW: &str = r#"fn view() { rsx! { TableView { name: "Accounts", columns: crate::columns::account_columns() } }; }"#;
    const COLUMNS: &str = r#"use crate::templates::account_cell;
pub fn account_columns() -> Columns {
    ferroui_view::table_columns([ferroui_view::table_column().header("Account").cell_template(account_cell())])
}
"#;
    const TEMPLATE: &str = r#"fn earlier() { rsx! { TextBlock { name: "Earlier" } }; }
pub fn account_cell() -> Template {
    ferroui_view::template::<String, _>(|data| rsx! { StackPanel {
        TextBlock { name: "Caption", text: "Customer 🧭" }
        TextBlock { text: data }
    } })
}
"#;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let root = std::env::temp_dir().join(format!("ferroui-studio-rust-templates-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let files = Files(root.canonicalize().unwrap());
    let manifest = files.0.join("Cargo.toml");
    let main = files.0.join("main.rs");
    let view = files.0.join("review.rs");
    let columns = files.0.join("columns.rs");
    let template = files.0.join("templates.rs");
    fs::write(&manifest, "[package]\nname='review'\nversion='0.0.0'\n").unwrap();
    fs::write(&main, "mod columns; mod review; mod templates; fn main() {}\n").unwrap();
    fs::write(&view, VIEW).unwrap();
    fs::write(&columns, COLUMNS).unwrap();
    fs::write(&template, TEMPLATE).unwrap();
    let view_uri = file_uri(&view).unwrap();
    let columns_uri = file_uri(&columns).unwrap();
    let template_uri = file_uri(&template).unwrap();
    let workspace = DesignerWorkspace::with_uri(VIEW, view_uri.clone()).unwrap();
    workspace.set_solution(Solution {
        format_version: 1, root: files.0.clone(),
        projects: vec![ProjectDescriptor {
            id: "review".into(), name: "review".into(), root: files.0.clone(), manifest_path: manifest,
            documents: Vec::new(),
            targets: vec![CargoTarget { name: "review".into(), kind: vec!["bin".into()], source: main }],
        }],
    }).unwrap();
    let window = Window::new();
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let session = workspace.session();
    let provider = DesignerMcp::with_workspace(&workspace);
    let writes = Permissions { allow_property_writes: true, ..Default::default() };
    let read = || {
        let node = {
            let session = session.borrow();
            let document = session.document();
            let table = document.ast().find_name("Accounts").unwrap();
            ferroui_designer_tools::table_columns(document, table).unwrap().columns[0].node
        };
        provider.call("designer_item_templates", &json!({"node":node}), Permissions::default()).unwrap()
    };
    let apply = |report: serde_json::Value, action| provider.call("designer_item_template", &json!({
        "node":report["node"], "property":"CellTemplate", "action":action,
        "expected_model":report["model"], "expected_revision":report["revision"],
    }), writes).unwrap();
    let model = read();
    assert_eq!(model["slots"][0]["rustBodyUri"], template_uri);
    assert_eq!(model["slots"][0]["sourceUri"], columns_uri);
    apply(model, "preview");
    assert_eq!(session.borrow().document_uri(), template_uri);
    let caption = session.borrow().document().ast().find_name("Caption").unwrap();
    session.borrow_mut().set_property(&[caption], "Text", "Account 🧭").unwrap();
    assert_eq!(session.borrow().authored_source(), TEMPLATE.replace("Customer 🧭", "Account 🧭"));
    session.borrow_mut().undo().unwrap();
    workspace.return_to_document().unwrap();
    assert_eq!(session.borrow().document_uri(), view_uri);
    assert_eq!(session.borrow().authored_source(), VIEW);
    apply(read(), "reset");
    assert_eq!(read()["slots"][0]["canCreate"], true);
    workspace.activate_document(&columns_uri).unwrap();
    assert_eq!(session.borrow().authored_source(), COLUMNS.replace(".cell_template(account_cell())", ""));
    let revision = session.borrow().document().revision();
    provider.call("designer_undo", &json!({"expected_revision": revision}), writes).unwrap();
    assert_eq!(session.borrow().authored_source(), COLUMNS);
    workspace.activate_document(&view_uri).unwrap();
    assert_eq!(read()["slots"][0]["rustFactory"], "account_cell");
    window.close();
}
