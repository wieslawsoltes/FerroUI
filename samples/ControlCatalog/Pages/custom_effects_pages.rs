//! The pages of the section "Custom effects" of the catalog (#455, E8; not from upstream;
//! `docs/porting/custom-effects.md`, section 3.7): one page per family of the custom effects
//! FerroUI ships (`ferroui_base::media::effects::library`), as the catalog has one page per
//! control. Each page is the section of its family of the custom effects gallery
//! (`samples/CustomEffects`, the crate `custom-effects`): a tile per effect on a sample, with the
//! code that makes it, the effect string markup writes for it (`Effect="tint(...)"`), its sliders
//! and colours, and its animations (the transition of the effect of the host, key frames of
//! `Visual.Effect`).
//!
//! The page transitions sample photographs of the catalog's assets as their pages and their
//! pattern (decoded once; the backend uploads each once), and the water ripple bends one of them
//! (#580: a ripple shows on image content, not on a flat panel); the gallery application captures
//! pages and a picture of its own instead.
//!
//! The headers here are the ones of the entries of the page list
//! (`ViewModels/main_window_view_model_page_list.rs`), the titles of the families. The module names
//! no other page of the catalog, so that the browser host fetches only the photographs for these
//! pages (`build/page_files.rs`).

use custom_effects::{Family, Picture, TransitionImages};
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::effects::EffectImage;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::IImage;
use ferroui_base::platform::AssetLoader;
use ferroui_base::utilities::UriRef;
use ferroui_base::{BoxedValue, Ref, Thickness};
use ferroui_controls::{ContentPage, Control, ScrollViewer};
use std::rc::Rc;

/// The bitmap of the asset at `uri`, decoded; `None` when it does not load.
fn asset_bitmap(uri: &str) -> Option<Bitmap> {
    UriRef::parse(uri)
        .ok()
        .and_then(|uri| AssetLoader::open(&uri, None).ok())
        .and_then(|mut stream| Bitmap::from_stream(&mut stream).ok())
}

/// A bitmap as the handle of an effect image (the handle holds it); the empty image, which the
/// transitions draw transparent, when it did not load.
fn effect_image(bitmap: Option<&Bitmap>) -> EffectImage {
    bitmap.map(|bitmap| EffectImage::new(bitmap)).unwrap_or_default()
}

/// The images of the gallery: two photographs as the pages of the transitions, the luminance of
/// a third one as the pattern of the dissolve and the mask of the reveal, and the first one,
/// decoded once, as the picture the water ripple bends.
pub(crate) fn photographs() -> TransitionImages {
    let arch = asset_bitmap("ferres://ControlCatalog/Assets/delicate-arch-896885_640.jpg");
    let deer = asset_bitmap("ferres://ControlCatalog/Assets/hirsch-899118_640.jpg");
    let leaf = asset_bitmap("ferres://ControlCatalog/Assets/maple-leaf-888807_640.jpg");
    TransitionImages {
        first_page: effect_image(arch.as_ref()),
        second_page: effect_image(deer.as_ref()),
        pattern: effect_image(leaf.as_ref()),
        picture: arch.map(|bitmap| Picture::new(Rc::new(bitmap) as Rc<dyn IImage>)).unwrap_or_default(),
    }
}

/// The page of `family`: its section (its title, its description and its tiles) in a scroll
/// viewer, under its title, as the pages of the section "Composition" are.
fn family_page(family: Family) -> Ref<ContentPage> {
    // Only the transitions and the water ripple show images: the other pages decode none.
    let images = if matches!(family, Family::Transitions | Family::Interactive) {
        photographs()
    } else {
        TransitionImages::default()
    };
    let content = family.section(&images);
    content.set_margin(Thickness::uniform(16.0));
    content.set_vertical_alignment(VerticalAlignment::Top);
    let scroller = ScrollViewer::new();
    scroller.set_content(Some(Control::boxed(content)));
    let page = ContentPage::new();
    page.set_header(Some(Rc::new(family.title().to_owned()) as BoxedValue));
    page.set_content(Some(Control::boxed(scroller)));
    page
}

/// The page "Colour effects".
pub fn colour_effects_page() -> Ref<ContentPage> {
    family_page(Family::Colour)
}

/// The page "Image effects".
pub fn image_effects_page() -> Ref<ContentPage> {
    family_page(Family::Image)
}

/// The page "Shader effects".
pub fn shader_effects_page() -> Ref<ContentPage> {
    family_page(Family::Shader)
}

/// The page "Interactive effects".
pub fn interactive_effects_page() -> Ref<ContentPage> {
    family_page(Family::Interactive)
}

/// The page "Page transitions".
pub fn page_transitions_page() -> Ref<ContentPage> {
    family_page(Family::Transitions)
}
