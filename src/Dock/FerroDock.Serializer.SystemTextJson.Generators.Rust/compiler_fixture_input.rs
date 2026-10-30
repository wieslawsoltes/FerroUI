// Test-only access to the original compiler's public facts; imported once per crate.
use ferrodock_serializer_system_text_json_generators::*;
#[path = "../../../tests/Dock/support/json_compiler_input.rs"]
mod facts;
pub(crate) use facts::compilation;
