//! Roboto from files, as a font collection of its own: the scenes do not depend on the fonts of
//! the machine.

use ferroui_base::media::fonts::{FontCollectionBase, FontCollectionBaseImpl};
use ferroui_base::utilities::UriRef;
use std::path::PathBuf;

const REGULAR: &[u8] = include_bytes!("../../src/FerroUI.Themes.Material/Assets/Roboto-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../../src/FerroUI.Themes.Material/Assets/Roboto-Bold.ttf");

/// Roboto Regular and Bold: the files of the Material theme's assets, which are part of the
/// program (an application of a device has no checkout beside it), or those of the folder
/// `PERF_FONTS` names.
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
        let folder = std::env::var_os("PERF_FONTS").map(PathBuf::from);
        for (file, built_in) in [("Roboto-Regular.ttf", REGULAR), ("Roboto-Bold.ttf", BOLD)] {
            let bytes = match &folder {
                Some(folder) => {
                    let path = folder.join(file);
                    std::fs::read(&path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
                }
                None => built_in.to_vec(),
            };
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
