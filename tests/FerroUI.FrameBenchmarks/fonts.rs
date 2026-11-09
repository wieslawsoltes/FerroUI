//! Roboto from files, as a font collection of its own: the scenes do not depend on the fonts of
//! the machine.

use ferroui_base::media::fonts::{FontCollectionBase, FontCollectionBaseImpl};
use ferroui_base::utilities::UriRef;
use std::path::PathBuf;

/// Roboto Regular and Bold from the folder `PERF_FONTS` names (default: the Material theme's assets
/// of this checkout).
pub struct Roboto {
    base: FontCollectionBase,
    key: url::Url,
}

impl Roboto {
    /// The family the scenes name.
    pub const FAMILY: &'static str = "fonts:Frames#Roboto";

    pub fn new() -> Self {
        let collection = Self {
            base: FontCollectionBase::new(),
            key: UriRef::parse_absolute("fonts:Frames").expect("the key of the collection"),
        };
        let folder = std::env::var_os("PERF_FONTS").map(PathBuf::from).unwrap_or_else(|| {
            PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../src/FerroUI.Themes.Material/Assets"))
        });
        for file in ["Roboto-Regular.ttf", "Roboto-Bold.ttf"] {
            let path = folder.join(file);
            let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            FontCollectionBase::try_add_glyph_typeface_from_stream(&collection, &mut std::io::Cursor::new(bytes));
        }
        collection
    }
}

impl Default for Roboto {
    fn default() -> Self {
        Self::new()
    }
}

impl FontCollectionBaseImpl for Roboto {
    fn base(this: &Self) -> &FontCollectionBase {
        &this.base
    }

    fn key(this: &Self) -> url::Url {
        this.key.clone()
    }
}
