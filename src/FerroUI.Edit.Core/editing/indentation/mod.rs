mod csharp;
mod default_indentation_strategy;
mod indentation_strategy;

pub use csharp::{CSharpIndentationStrategy, IDocumentAccessor, TextDocumentAccessor};
pub use default_indentation_strategy::DefaultIndentationStrategy;
pub use indentation_strategy::IIndentationStrategy as IndentationStrategy;
pub use indentation_strategy::{IIndentationStrategy, IndentationError};
