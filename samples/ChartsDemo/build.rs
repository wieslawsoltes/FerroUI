//! Compile the same declarative Charts surface that the runtime-loader tests use.
use ferroui_build::{Build, TypeSystem, XamlGroup};

fn main() {
    println!("cargo::rerun-if-changed=MainView.xaml");
    Build::from_env()
        .type_system(TypeSystem::Model)
        .compile_group(
            XamlGroup::new("compiled_view")
                .documents(&[("MainView.xaml", include_str!("MainView.xaml"))]),
        )
        .run();
}
