#[path = "windows/resources.rs"]
mod windows_resources;

fn main() {
    println!("cargo::rerun-if-env-changed=FERRORIBBON_BUILD_FINGERPRINT");
    if let Ok(value) = std::env::var("FERRORIBBON_BUILD_FINGERPRINT") {
        assert!(
            value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()),
            "invalid package source fingerprint"
        );
        println!("cargo::rustc-env=FERRORIBBON_BUILD_FINGERPRINT={value}");
    }
    let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    windows_resources::link(&root);
    let mut files: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "xaml")
        })
        .collect();
    files.sort();
    let documents: Vec<_> = files
        .iter()
        .map(|path| {
            (
                path.file_name().unwrap().to_str().unwrap().to_owned(),
                std::fs::read_to_string(path).unwrap(),
            )
        })
        .collect();
    let ordinary: Vec<_> = documents
        .iter()
        .filter(|(name, _)| {
            !matches!(
                name.as_str(),
                "CadPage.xaml"
                    | "SettingsPage.xaml"
                    | "WordPage.xaml"
                    | "PowerPointPage.xaml"
                    | "ToolsPage.xaml"
                    | "ExcelPage.xaml"
                    | "ShellPage.xaml"
            )
        })
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();
    let page = documents
        .iter()
        .find(|(name, _)| name == "CadPage.xaml")
        .unwrap();
    let settings = documents
        .iter()
        .find(|(name, _)| name == "SettingsPage.xaml")
        .unwrap();
    let word = documents
        .iter()
        .find(|(name, _)| name == "WordPage.xaml")
        .unwrap();
    let asset_consumer = std::fs::read_to_string(root.join("test_markup/AppAssets.xaml")).unwrap();
    let mut build = ferroui_build::Build::from_env()
        .type_system(ferroui_build::TypeSystem::Model)
        .embed_assets(&["Assets"])
        .input("test_markup/AppAssets.xaml")
        .compile_group(
            ferroui_build::XamlGroup::new("compiled_asset_consumer")
                .documents(&[("AppAssets.xaml", asset_consumer.as_str())]),
        )
        .default_compile_bindings(true)
        .compile_group(ferroui_build::XamlGroup::new("compiled_xaml").documents(&ordinary))
        .compile_group(
            ferroui_build::XamlGroup::new("compiled_cad_page")
                .documents(&[(page.0.as_str(), page.1.as_str())])
                .class_document("CadPage.xaml"),
        )
        .compile_group(
            ferroui_build::XamlGroup::new("compiled_settings_page")
                .documents(&[(settings.0.as_str(), settings.1.as_str())])
                .class_document("SettingsPage.xaml"),
        )
        .compile_group(
            ferroui_build::XamlGroup::new("compiled_word_page")
                .documents(&[(word.0.as_str(), word.1.as_str())])
                .class_document("WordPage.xaml"),
        );
    for (group, name) in [
        ("compiled_powerpoint_page", "PowerPointPage.xaml"),
        ("compiled_tools_page", "ToolsPage.xaml"),
        ("compiled_excel_page", "ExcelPage.xaml"),
        ("compiled_shell_page", "ShellPage.xaml"),
    ] {
        let page = documents.iter().find(|(n, _)| n == name).unwrap();
        build = build.compile_group(
            ferroui_build::XamlGroup::new(group)
                .documents(&[(page.0.as_str(), page.1.as_str())])
                .class_document(name),
        );
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        build = build
            .theme_roots(&["ToggleButton"])
            .input("windows/PinResources.xaml")
            .compile_group(
                ferroui_build::XamlGroup::new("compiled_windows_pin_resources").documents(&[(
                    "windows/PinResources.xaml",
                    include_str!("windows/PinResources.xaml"),
                )]),
            );
    }
    for path in &files {
        build = build.input(path.to_str().unwrap());
    }
    build.run();
}
