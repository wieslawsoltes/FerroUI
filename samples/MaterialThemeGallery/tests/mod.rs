mod support;
mod compiled_markup;
mod documents { include!(concat!(env!("OUT_DIR"), "/document_tests.rs")); }
