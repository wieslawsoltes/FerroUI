//! The same model-based native markup build as the framework's small samples.
//!
//! The compiler reads the dependency `.xamlmeta` files and the source model of
//! this package. No runtime type registration or managed runtime is needed by
//! the build script.

use ferroui_build::{Build, TypeSystem, XamlGroup};

fn main() {
    println!("cargo::rerun-if-changed=MainView.xaml");
    Build::from_env()
        .type_system(TypeSystem::Model)
        .compile_group(
            XamlGroup::new("compiled_xaml")
                .documents(&[("MainView.xaml", include_str!("MainView.xaml"))]),
        )
        .run();
}
