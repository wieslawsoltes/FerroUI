use super::{ResourceColorMode, RsxStyleEdit};
use crate::RsxCatalog;
use ferroui_designer_core::TextPatch;
use ferroui_designer_project::{file_uri, PreviewSourceBuffer, RustModuleSources};
use std::fs;
use std::path::PathBuf;

struct Fixture(PathBuf);

impl Fixture {
    fn create() -> Self {
        let path = std::env::temp_dir().join(format!(
            "ferroui-rsx-palette-module-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, text: &str) -> String {
        let path = self.0.join(name);
        fs::write(&path, text).unwrap();
        ferroui_designer_project::canonical_file_uri(&file_uri(&path).unwrap()).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn resolves_the_packed_integer_through_a_color_constructor_in_another_module() {
    let fixture = Fixture::create();
    fixture.write("application.rs", "mod tokens; mod brand; mod screen;\n");
    let disk = "pub(crate) const INK: u32 = 0xFF0C_1520;\r\n";
    let tokens = fixture.write("tokens.rs", disk);
    let brand = "use ferroui_base::media::Color;\r\nuse crate::tokens::INK as PACKED;\r\npub(crate) const PANEL: Color = Color::from_uint32(PACKED);\r\n";
    fixture.write("brand.rs", brand);
    let screen = r#"use crate::brand::PANEL;
    fn view() { rsx! { StackPanel {
        styles: [ferroui_view::style("TextBlock")
            .resource("Ink", (/* use 🧭 */ PANEL))
            .set_resource(ferroui_controls::TextBlock::foreground_property(), "Ink")],
        TextBlock { text: "Review" },
    } }; }"#;
    let uri = fixture.write("screen.rs", screen);
    let draft = "pub(crate) const INK: u32 = /* warm 🧭 */ 0xA012_3456_u32;\r\n";
    let modules = RustModuleSources::capture(
        &fixture.0,
        &fixture.0.join("application.rs"),
        &uri,
        screen,
        &[PreviewSourceBuffer {
            uri: tokens.clone(),
            source: Some(draft.into()),
        }],
    )
    .unwrap();
    let catalog = RsxCatalog::built_in();
    let caret = screen.find("StackPanel").unwrap();
    let projection = catalog
        .project_with_modules(screen, caret, Some(&modules))
        .unwrap();
    let style = projection.style_definitions()[0].1;
    let resource = &style.resources[0];
    assert_eq!(resource.color().as_deref(), Some("#A0123456"));
    let shared = resource.shared.as_ref().unwrap();
    assert_eq!(shared.name, "INK");
    assert_eq!(shared.uri.as_deref(), Some(tokens.as_str()));
    assert_eq!(&draft[shared.value.range()], "0xA012_3456_u32");
    let edit = |mode| RsxStyleEdit::ColorResource {
        style: style.declaration.start,
        resource: resource.declaration.start,
        value: "#D0102030".into(),
        mode,
    };
    let plan = projection
        .plan_style_source(&edit(ResourceColorMode::Declaration))
        .unwrap();
    assert_eq!(plan.source_uri.as_deref(), Some(tokens.as_str()));
    assert_eq!(
        TextPatch::apply_all(draft, plan.patches).unwrap(),
        draft.replace("0xA012_3456_u32", "0xD010_2030_u32"),
    );
    let local = projection
        .plan_style_source(&edit(ResourceColorMode::Local))
        .unwrap();
    assert!(local.source_uri.is_none());
    let after = TextPatch::apply_all(screen, local.patches).unwrap();
    assert!(after.contains("/* use 🧭 */"));
    assert!(after.contains("Color::from_argb(208, 16, 32, 48)"));
    assert_eq!(
        fs::read_to_string(fixture.0.join("brand.rs")).unwrap(),
        brand
    );
    assert_eq!(
        fs::read_to_string(fixture.0.join("tokens.rs")).unwrap(),
        disk
    );

    let other_type = draft.replace("INK: u32", "INK: i64");
    let modules = modules.with_file_text(&tokens, &other_type).unwrap();
    let projection = catalog
        .project_with_modules(screen, caret, Some(&modules))
        .unwrap();
    let resource = &projection.style_definitions()[0].1.resources[0];
    assert!(resource.shared.is_none());
    assert!(resource.color().is_none());
    assert!(projection
        .plan_style_source(&edit(ResourceColorMode::Declaration))
        .is_err());
}

#[test]
fn checked_color_edit_targets_the_rust_module_or_the_active_invocation() {
    let fixture = Fixture::create();
    let root = fixture.write(
        "application.rs",
        "mod brand; mod screen; pub(crate) use brand::REVIEW as ACCENT;\n",
    );
    let disk = "use ferroui_base::media::Color as Paint;\r\npub(crate) const KEY: &str = \"Ink\";\r\npub(crate) const REVIEW: Paint = Paint::from_uint32((/* 🧭 */ 0xFF0C_1520_u32));\r\n";
    let brand = fixture.write("brand.rs", disk);
    let source = r#"
    use super::{ACCENT as INK, brand::KEY};
    fn first() { rsx! { TextBlock { text: "first" } }; }
    fn selected() { rsx! { StackPanel {
        styles: [ferroui_view::style("TextBlock")
            .resource(KEY, (/* local reference */ INK))
            .set_resource(ferroui_controls::TextBlock::foreground_property(), KEY)],
        TextBlock { text: "selected" },
    } }; }
    "#
    .replace('\n', "\r\n");
    let screen = fixture.write("screen.rs", &source);
    let draft = disk.replace("0xFF0C", "0xFF7B");
    let buffers = [PreviewSourceBuffer {
        uri: brand.clone(),
        source: Some(draft.clone()),
    }];
    let modules = RustModuleSources::capture(
        &fixture.0,
        &fixture.0.join("application.rs"),
        &screen,
        &source,
        &buffers,
    )
    .unwrap();
    assert_eq!(modules.target().uri, root);
    let caret = source.find("StackPanel").unwrap();
    let catalog = RsxCatalog::built_in();
    let projection = catalog
        .project_with_modules(&source, caret, Some(&modules))
        .unwrap();
    let heads = projection.style_definitions();
    let rule = heads[0].1;
    let color = &rule.resources[0];
    assert_eq!(color.color().as_deref(), Some("#FF7B1520"));
    assert_eq!(rule.setters[0].resource_key(), Some("Ink"));
    assert_eq!(
        &source[color.color_source().range()],
        "(/* local reference */ INK)"
    );
    let shared = color.shared.as_ref().unwrap();
    assert_eq!(shared.name, "REVIEW");
    assert_eq!(shared.uri.as_deref(), Some(brand.as_str()));
    assert_eq!(
        &draft[shared.value.range()],
        "Paint::from_uint32((/* 🧭 */ 0xFF7B_1520_u32))"
    );

    let edit = |mode| RsxStyleEdit::ColorResource {
        style: rule.declaration.start,
        resource: color.declaration.start,
        value: "#A0185060".into(),
        mode,
    };
    let declaration = edit(ResourceColorMode::Declaration);
    assert!(projection.plan_style(&declaration).is_err());
    let plan = projection.plan_style_source(&declaration).unwrap();
    assert_eq!(plan.source_uri.as_deref(), Some(brand.as_str()));
    assert_eq!(plan.shared_color, Some(shared.value));
    let after = TextPatch::apply_all(&draft, plan.patches).unwrap();
    assert_eq!(after, draft.replace("0xFF7B_1520_u32", "0xA018_5060_u32"));
    assert_eq!(
        fs::read_to_string(fixture.0.join("brand.rs")).unwrap(),
        disk
    );
    let next = modules.with_file_text(&brand, &after).unwrap();
    let updated = catalog
        .project_with_modules(&source, caret, Some(&next))
        .unwrap();
    assert_eq!(updated.invocation, projection.invocation);
    assert!(updated.xaml().contains("#A0185060"));

    let plan = projection
        .plan_style_source(&edit(ResourceColorMode::Local))
        .unwrap();
    assert!(plan.source_uri.is_none() && plan.shared_color.is_none());
    let local = TextPatch::apply_all(&source, plan.patches).unwrap();
    assert!(local.contains("/* local reference */"));
    assert!(local.contains(".resource(KEY,"));
    let next = modules.with_file_text(&screen, &local).unwrap();
    let updated = catalog
        .project_with_modules(&local, caret, Some(&next))
        .unwrap();
    assert!(updated.xaml().contains("#A0185060"));
    assert!(modules.with_file_text(&root, "mod screen;").is_none());
}
