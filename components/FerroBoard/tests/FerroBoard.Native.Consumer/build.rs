fn main() {
    let java = std::path::PathBuf::from(
        std::env::var_os("DEP_FERROBOARD_CONTROLS_XAML_ANDROID_JAVA")
            .expect("library must export its Android Java asset root"),
    );
    assert!(java.join("org/ferroboard/controls/Haptics.java").is_file());
    println!("cargo::rerun-if-changed={}", java.display());

    ferroui_build::Build::from_env()
        .type_system(ferroui_build::TypeSystem::Model)
        .embed_assets(&["Runtime"])
        .compile_xaml()
        .run();
}
