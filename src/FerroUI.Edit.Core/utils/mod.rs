mod events;
mod file_reader;
mod file_text_reader;
mod pattern_occurrences;
mod rope;
mod rope_text_reader;
mod text_encoding;
mod utf16_memory;

pub use events::DocumentSubscription;
pub(crate) use events::Event;
pub use file_reader::FileReader;
pub use file_text_reader::{FileTextReader, Utf8FileTextReader};
pub(crate) use pattern_occurrences::pattern_occurrences;
pub use rope::{RopeChunks, Utf16Rope};
pub use rope_text_reader::{RopeTextReader, Utf8TextReader};
pub use text_encoding::{DecodingErrorPolicy, FileReaderOptions, TextEncoding};
pub use utf16_memory::Utf16Memory;
