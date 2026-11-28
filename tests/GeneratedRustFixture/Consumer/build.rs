//! Compiles the documents of the crate (`documents.rs`) to `$OUT_DIR/xaml/` against the type models:
//! the scan of the sources of this crate and the models in the `.xamlmeta` of the crates it depends
//! on (the library of the fixture and the themes of the framework). rustc compiles what it writes
//! with the crate: the fixture is that the generated Rust compiles (#849).

use ferroui_build::{Build, TypeSystem, XamlGroup};

#[allow(dead_code)]
#[path = "documents.rs"]
mod documents;

fn main() {
    Build::from_env()
        .type_system(TypeSystem::Model)
        .input("documents.rs")
        .compile_group(XamlGroup::new("compiled_xaml").documents(documents::DOCUMENTS))
        .compile_group(
            XamlGroup::new("compiled_collision_styles")
                .documents(documents::COLLISION_STYLES)
                .class_document("CollisionStyles.xaml"),
        )
        .run();
}
