use ferroui_build::{Build, TypeSystem, XamlGroup};
fn main() {
    println!("cargo:rerun-if-changed=ui/Editor.xaml");
    Build::from_env()
        .type_system(TypeSystem::Model)
        .compile_group(
            XamlGroup::new("compiled_xaml")
                .documents(&[("ui/Editor.xaml", include_str!("ui/Editor.xaml"))]),
        )
        .run();
}
