//! Compiles a consumer against dependency-exported FerroDock metadata.
#[path = "../support/json_actual_model_projection.rs"]
mod actual_model_projection;
use ferrodock_serializer_system_text_json_generators_rust as native_adapter;
mod native_json_models;
mod native_json_properties;
mod native_model_schemas;
fn main() {
    native_json_properties::emit();
    ferroui_build::Build::from_env()
        .type_system(ferroui_build::TypeSystem::Model)
        .compile_xaml()
        .run();
}
