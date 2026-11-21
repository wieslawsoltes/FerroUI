use super::{ResourceColorMode, RsxStyleEdit};
use crate::RsxCatalog;
use ferroui_designer_core::TextPatch;
use ferroui_designer_project::{PreviewSourceBuffer, RustModuleSources, file_uri};
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
fn checked_color_edit_targets_the_rust_module_or_the_active_invocation() {
    let fixture = Fixture::create();
    let root = fixture.write(
        "application.rs",
        "mod brand; mod screen; pub(crate) use brand::REVIEW as ACCENT;\n",
    );
    let disk = "use ferroui_base::media::Color as Paint;\r\npub(crate) const KEY: &str = \"Ink\";\r\npub(crate) const REVIEW: Paint = Paint::from_rgb(12u8, /* 🧭 */ 21, 32);\r\n";
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
    let draft = disk.replace("12u8", "123u8");
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
        "Paint::from_rgb(123u8, /* 🧭 */ 21, 32)"
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
    assert_eq!(
        after,
        draft.replace(
            "from_rgb(123u8, /* 🧭 */ 21, 32)",
            "from_argb(160, 24u8, /* 🧭 */ 80, 96)"
        )
    );
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
