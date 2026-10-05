use crate::media::{TextDecoration, TextDecorationCollection, TextDecorationLocation};

/// Defines a set of commonly used text decorations.
pub struct TextDecorations;

fn create(location: TextDecorationLocation) -> TextDecorationCollection {
    let text_decoration = TextDecoration::new();
    text_decoration.set_location(location);
    TextDecorationCollection::from_items([text_decoration])
}

macro_rules! text_decorations {
    ($(#[$meta:meta])* $name:ident, $location:ident) => {
        $(#[$meta])*
        pub fn $name() -> TextDecorationCollection {
            thread_local! {
                static VALUE: TextDecorationCollection = create(TextDecorationLocation::$location);
            }
            VALUE.with(Clone::clone)
        }
    };
}

impl TextDecorations {
    text_decorations!(
        /// Gets a value containing a single underline.
        underline, Underline
    );

    text_decorations!(
        /// Gets a value containing a single strikethrough.
        strikethrough, Strikethrough
    );

    text_decorations!(
        /// Gets a value containing a single overline.
        overline, Overline
    );

    text_decorations!(
        /// Gets a value containing a single baseline.
        baseline, Baseline
    );
}
