// Actual compiled crate-scope registrations of the source application payloads.
use ferrodock_serializer_system_text_json::{
    dock_json_source_generation as generate_json,
    dock_json_serializable as register_json,
};
generate_json!();
register_json!(crate::payload_models::Holder);
register_json!(crate::payload_models::Payload);
register_json!(crate::payload_models::Generic<i32>);
register_json!(crate::payload_models::Generic<Option<String>>);
register_json!(crate::payload_models::TypeNamed);
register_json!(crate::payload_models::CustomHolder);
register_json!(crate::payload_models::ReadonlyHolder);
register_json!(crate::payload_models::ArrayPayload);
register_json!(crate::payload_models::WriteOnlyHolder);
