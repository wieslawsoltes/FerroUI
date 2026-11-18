//! Compiles the document of each class of the crate (`documents.rs`) to `$OUT_DIR/xaml/` and writes
//! the `.xamlmeta` of the crate, with its type model, for the crate that constructs the classes in
//! its documents (`generated-rust-fixture-consumer`). Every document of a class merges another one,
//! so the compiled markup of every class is partitioned into units (#568).

use ferroui_build::{Build, TypeSystem, XamlGroup};

#[allow(dead_code)]
#[path = "documents.rs"]
mod documents;

fn main() {
    Build::from_env()
        .type_system(TypeSystem::Model)
        .input("documents.rs")
        .compile_group(XamlGroup::new("compiled_app").documents(documents::APP).class_document("App.xaml"))
        .compile_group(
            XamlGroup::new("compiled_parts_theme").documents(documents::PARTS_THEME).class_document("A_B/PartsTheme.xaml"),
        )
        .compile_group(
            XamlGroup::new("compiled_parts_theme_lower")
                .documents(documents::PARTS_THEME_LOWER)
                .class_document("A/B/Partstheme.xaml"),
        )
        .compile_group(
            XamlGroup::new("compiled_renamed_theme").documents(documents::RENAMED_THEME).class_document("RenamedTheme.xaml"),
        )
        .run();
}
