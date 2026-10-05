use ferroui_base::media::fonts::{EmbeddedFontCollection, FontCollectionBase, FontCollectionBaseImpl};
use ferroui_base::utilities::Uri;

/// The font collection of the embedded Inter family, with the key
/// `fonts:Inter`.
pub struct InterFontCollection {
    base: EmbeddedFontCollection,
}

impl InterFontCollection {
    /// Creates the collection and loads the embedded fonts into it.
    pub fn new() -> Self {
        crate::assets::register();
        Self {
            base: EmbeddedFontCollection::new(
                Uri::absolute("fonts:Inter").expect("a valid key"),
                Uri::absolute(&format!("ferres://{}/Assets", crate::ASSEMBLY_NAME)).expect("a valid source"),
            ),
        }
    }
}

impl Default for InterFontCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl FontCollectionBaseImpl for InterFontCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        EmbeddedFontCollection::base(&this.base)
    }

    fn key(this: &Self) -> Uri {
        EmbeddedFontCollection::key(&this.base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::media::fonts::IFontCollection;
    use ferroui_base::media::{FontFamily, FontManager, FontWeight, Typeface};
    use ferroui_base::platform::{IAssetLoader, IFontManagerImpl, StandardAssetLoader};
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::FerroLocator;
    use std::rc::Rc;

    /// A locator scope with the Skia font manager and the asset loader that
    /// serves the embedded assets.
    fn start() -> Rc<dyn IDisposable> {
        let scope = FerroLocator::enter_scope();
        let locator = FerroLocator::current_mutable();
        let font_manager: Rc<dyn IFontManagerImpl> = Rc::new(ferroui_skia::FontManagerImpl::new());
        locator.bind::<dyn IFontManagerImpl>().to_constant(font_manager);
        let asset_loader: Rc<dyn IAssetLoader> = Rc::new(StandardAssetLoader::new(None));
        locator.bind::<dyn IAssetLoader>().to_constant(asset_loader);
        scope
    }

    #[test]
    fn the_collection_is_keyed_as_the_inter_fonts() {
        let scope = start();

        let collection = InterFontCollection::new();

        assert_eq!("fonts:Inter", IFontCollection::key(&collection).original_string());
        scope.dispose();
    }

    #[test]
    fn the_collection_holds_the_families_of_the_six_font_files() {
        let scope = start();

        let collection = InterFontCollection::new();

        // The regular and the bold face share the family name; the other weights carry theirs in
        // the name, with "Inter" as their typographic family.
        let mut names: Vec<String> = (0..IFontCollection::count(&collection))
            .map(|index| IFontCollection::get(&collection, index).name().to_string())
            .collect();
        names.sort();
        assert_eq!(vec!["Inter", "Inter Light", "Inter Medium", "Inter SemiBold", "Inter Thin"], names);

        let typefaces = collection.try_get_family_typefaces("Inter").expect("the family is present");
        let weights: Vec<i32> = typefaces.iter().map(|typeface| typeface.weight().0).collect();
        assert!(weights.contains(&FontWeight::Normal.0), "{weights:?}");
        assert!(weights.contains(&FontWeight::Bold.0), "{weights:?}");
        scope.dispose();
    }

    #[test]
    fn the_collection_can_be_added_to_the_font_manager() {
        let scope = start();

        FontManager::current().add_font_collection(Rc::new(InterFontCollection::new()));

        let typeface = Typeface::new(FontFamily::parse("fonts:Inter#Inter").unwrap());
        let glyph_typeface =
            FontManager::current().try_get_glyph_typeface(&typeface).expect("the family resolves through its key");
        assert_eq!("Inter", glyph_typeface.family_name());
        scope.dispose();
    }
}
