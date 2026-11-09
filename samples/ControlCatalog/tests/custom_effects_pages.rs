//! Tests of the pages of the section "Custom effects" (#455, E8): one page per family of the custom
//! effects gallery, each its section with a tile per effect, shown in a window; the page
//! transitions sample photographs of the catalog's assets.
//!
//! Not ports: the upstream sample has no such pages. The tiles themselves are tested by the
//! gallery (`samples/CustomEffects/tests.rs`).

use super::support::*;
use crate::pages::custom_effects_pages::{
    colour_effects_page, image_effects_page, interactive_effects_page, page_transitions_page, photographs,
    shader_effects_page,
};
use custom_effects::{Family, TransitionImages};
use ferroui_base::media::effects::library::DissolveTransitionEffect;
use ferroui_base::media::effects::EffectExtensions;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ObjectType, Ref, Visual};
use ferroui_controls::{Border, Control, ContentPage, ScrollViewer, StackPanel, Window, WrapPanel};

/// The pages of the section with their families, in the order of the page list.
fn pages() -> [(fn() -> Ref<ContentPage>, Family); 5] {
    [
        (colour_effects_page, Family::Colour),
        (image_effects_page, Family::Image),
        (shader_effects_page, Family::Shader),
        (interactive_effects_page, Family::Interactive),
        (page_transitions_page, Family::Transitions),
    ]
}

/// The section a page shows: the content of its scroll viewer.
fn section_of(page: &ContentPage) -> Ref<StackPanel> {
    let scroller = page
        .content()
        .and_then(|content| Control::from_boxed(&content))
        .and_then(|content| content.cast::<ScrollViewer>())
        .expect("the scroll viewer of the page");
    scroller
        .content()
        .and_then(|content| Control::from_boxed(&content))
        .and_then(|content| content.cast::<StackPanel>())
        .expect("the section of the page")
}

fn descendants<T: ObjectType>(root: &Visual) -> Vec<Ref<T>> {
    root.get_visual_descendants().filter_map(|visual| visual.cast::<T>()).collect()
}

#[test]
fn each_page_is_the_section_of_its_family_with_a_tile_per_effect() {
    let _app = start_catalog_application();
    for (make, family) in pages() {
        let page = make();
        let header = page.header().and_then(|header| header.downcast_ref::<String>().cloned());
        assert_eq!(Some(family.title().to_owned()), header);
        let section = section_of(&page);
        let tiles = section.children().get(2).cast::<WrapPanel>().expect("the tiles of the section");
        assert_eq!(family.tiles(&TransitionImages::default()).len(), tiles.children().count(), "{}", family.title());

        // Shown in a window, the hosts of the tiles keep their custom effects.
        let window = Window::new();
        window.set_width(1100.0);
        window.set_height(800.0);
        window.set_content(Some(Control::boxed(page.clone())));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let custom = descendants::<Border>(&tiles)
            .iter()
            .filter(|border| border.effect().is_some_and(|effect| EffectExtensions::to_custom_effect(&*effect).is_some()))
            .count();
        assert!(custom >= family.tiles(&TransitionImages::default()).len().min(3), "{}: {custom} hosts", family.title());
        window.close();
        Dispatcher::ui_thread().run_jobs(None);
    }
}

#[test]
fn the_page_transitions_sample_photographs_of_the_assets() {
    let _app = start_catalog_application();
    let images = photographs();
    for image in [&images.first_page, &images.second_page, &images.pattern] {
        assert!(!image.is_empty(), "a photograph of the assets");
    }
    assert_ne!(images.first_page, images.second_page);
    // The water ripple bends one of them (#580).
    assert!(images.picture.image().is_some(), "the picture of the water ripple");

    let page = page_transitions_page();
    let tiles = section_of(&page).children().get(2).cast::<WrapPanel>().expect("the tiles");
    let dissolve = descendants::<Border>(&tiles)
        .iter()
        .filter_map(|border| border.effect())
        .filter_map(|effect| EffectExtensions::to_custom_effect(&*effect))
        .find_map(|effect| effect.downcast_ref::<DissolveTransitionEffect>().cloned())
        .expect("a dissolve");
    assert!(!dissolve.from_page.is_empty() && !dissolve.to_page.is_empty() && !dissolve.pattern.is_empty());
}
