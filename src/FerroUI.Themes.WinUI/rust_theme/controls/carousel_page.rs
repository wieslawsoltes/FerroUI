//! `Controls/CarouselPage.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a style: WinUI has no page of pages that flip in microsoft-ui-xaml
//! (https://github.com/microsoft/microsoft-ui-xaml, commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0);
//! an application shows pages one at a time with a FlipView. The template is the one FerroUI's
//! CarouselPage requires (src/FerroUI.Themes.Fluent/Controls/CarouselPage.xaml: the Carousel
//! PART_Carousel with the page template, the page transition and the items panel of the page),
//! and the Carousel is drawn by the theme of Controls/Carousel.xaml, the port of FlipView
//! (DefaultFlipViewStyle of controls/dev/CommonStyles/FlipView_themeresources.xaml). The
//! background of the page is ApplicationPageBackgroundThemeBrush, the background a WinUI page is
//! given (as Controls/ContentPage.xaml).
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Carousel;
use ferroui_controls::CarouselPage;
use ferroui_controls::Control;
use ferroui_controls::ItemsControl;
use ferroui_controls::MultiPage;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<CarouselPage>(), carousel_page_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`: the document included
/// (`ResourceInclude`) rather than merged.
#[allow(dead_code)]
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `CarouselPage` of `carousel_page_theme`.
fn carousel_page_theme_template(t: &Template<CarouselPage>) -> Result<Ref<Control>> {
    Ok(t.make(Carousel::new())
        .name("PART_Carousel")?
        .template_bind(
            TemplatedControl::background_property(),
            TemplatedControl::background_property(),
        )
        .template_bind(
            ItemsControl::item_template_property(),
            MultiPage::page_template_property(),
        )
        .template_bind(
            Carousel::page_transition_property(),
            CarouselPage::page_transition_property(),
        )
        .template_bind(
            ItemsControl::items_panel_property(),
            CarouselPage::items_panel_property(),
        )
        .set(
            Layoutable::horizontal_alignment_property(),
            HorizontalAlignment::Stretch,
        )
        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
        .control())
}

/// The default control theme of `CarouselPage`.
fn carousel_page_theme(theme: &StyleBuilder) -> Result {
    theme.target::<CarouselPage>();
    theme.dynamic(
        TemplatedControl::background_property(),
        "ApplicationPageBackgroundThemeBrush",
    );
    theme.template(carousel_page_theme_template);
    Ok(())
}
