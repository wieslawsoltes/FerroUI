//! `Controls/ContentPage.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a style: WinUI's Page has no style and no template in microsoft-ui-xaml
//! (https://github.com/microsoft/microsoft-ui-xaml, commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0;
//! a Page is a UserControl whose content the application gives). The template is the one
//! FerroUI's ContentPage requires (src/FerroUI.Themes.Fluent/Controls/ContentPage.xaml: the
//! content and the top and bottom command bars, whose visibility the control sets); the values
//! are the resources a WinUI 3 page shows: ApplicationPageBackgroundThemeBrush (the background the
//! page templates of WinUI set on a Page, SolidBackgroundFillColorBase of
//! dxaml/xcp/dxaml/themes/generic.xaml and controls/dev/CommonStyles/Common_themeresources.xaml)
//! and the primary text colour (TextFillColorPrimaryBrush).
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::ContentPage;
use ferroui_controls::Control;
use ferroui_controls::Dock;
use ferroui_controls::DockPanel;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<ContentPage>(), content_page_theme);
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

/// The template of `ContentPage` of `content_page_theme`.
fn content_page_theme_template(t: &Template<ContentPage>) -> Result<Ref<Control>> {
    Ok(t.make(DockPanel::new())
        // The control shows the command bar presenters when it has command bars, and sets the
        // padding of the content presenter (its Padding with the safe area).
        .children([
            t.make(ContentPresenter::new())
                .name("PART_TopCommandBar")?
                .set(DockPanel::dock_property(), Dock::Top)
                .set(
                    ContentPresenter::horizontal_content_alignment_property(),
                    HorizontalAlignment::Stretch,
                )
                .set(Visual::is_visible_property(), false)
                .control(),
            t.make(ContentPresenter::new())
                .name("PART_BottomCommandBar")?
                .set(DockPanel::dock_property(), Dock::Bottom)
                .set(
                    ContentPresenter::horizontal_content_alignment_property(),
                    HorizontalAlignment::Stretch,
                )
                .set(Visual::is_visible_property(), false)
                .control(),
            t.make(ContentPresenter::new())
                .name("PART_ContentPresenter")?
                .template_bind(
                    ContentPresenter::horizontal_content_alignment_property(),
                    ContentPage::horizontal_content_alignment_property(),
                )
                .template_bind(
                    ContentPresenter::vertical_content_alignment_property(),
                    ContentPage::vertical_content_alignment_property(),
                )
                .template_bind(
                    ContentPresenter::background_property(),
                    TemplatedControl::background_property(),
                )
                .template_bind(
                    ContentPresenter::border_brush_property(),
                    TemplatedControl::border_brush_property(),
                )
                .set(
                    ContentPresenter::background_sizing_property(),
                    BackgroundSizing::InnerBorderEdge,
                )
                .template_bind(
                    ContentPresenter::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(ContentPresenter::content_property(), ContentPage::content_property())
                .template_bind(
                    ContentPresenter::content_template_property(),
                    ContentPage::content_template_property(),
                )
                .template_bind(
                    ContentPresenter::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .control(),
        ])
        .control())
}

/// The default control theme of `ContentPage`.
fn content_page_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ContentPage>();
    theme.dynamic(
        TemplatedControl::background_property(),
        "ApplicationPageBackgroundThemeBrush",
    );
    theme.dynamic(TemplatedControl::foreground_property(), "TextFillColorPrimaryBrush");
    theme.template(content_page_theme_template);
    Ok(())
}
