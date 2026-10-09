//! The emitter's output for the class documents of the corpus
//! (`corpus::CLASS_DOCUMENTS`), one file per document, CHECKED IN: the file of a class as
//! a build writes it (`rust_emitter::generate_class_file_with`), with `populate` for an
//! instance of the class. `event_handlers.rs` keeps the files current and runs them.

pub mod attached_event;
pub mod delegate_property;
pub mod plain_event;
pub mod routed_event;
pub mod template_event;
