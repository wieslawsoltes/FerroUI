//! `Controls/Carousel.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/FlipView_themeresources.xaml (DefaultFlipViewStyle and the implicit
//!   style of FlipView; the resources are Resources/FlipView_themeresources.xaml)
//!   controls/dev/CommonStyles/FlipViewItem_themeresources.xaml (DefaultFlipViewItemStyle and the
//!   implicit style of FlipViewItem; the resources are Resources/FlipViewItem_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! FerroUI's Carousel is WinUI's FlipView: a selector that shows one item at a time.
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The Grid of the template (background, border, corner radius) is a Border.
//! - The ScrollViewer ScrollingHost is the PART_ScrollViewer FerroUI's Carousel requires, with the
//!   ItemsPresenter PART_ItemsPresenter; the padding of the scroll viewer is the margin of the
//!   presenter. The items panel is the default panel of FerroUI's Carousel (its virtualizing
//!   carousel panel, which lays out and moves the pages), not the VirtualizingStackPanel of the
//!   source with its regular snap points, and the snap point types of the scroll viewer are left
//!   out with it. BringIntoViewOnFocusChange of the scroll viewer is False (the panel brings the
//!   selected page into view).
//! - The previous and next buttons (PreviousButtonHorizontal, NextButtonHorizontal,
//!   PreviousButtonVertical, NextButtonVertical with the template-local templates
//!   HorizontalPreviousTemplate, ...) are left out: FlipView wires them in code and shows them
//!   while the pointer moves over it, and FerroUI's Carousel has no commands a template could bind
//!   them to; its pages change with the keyboard, swipes and its Next and Previous methods.
//! - FlipViewItem is the container of an item: the containers of FerroUI's Carousel are content
//!   presenters, so DefaultFlipViewItemStyle is a control theme of ContentPresenter that the
//!   Carousel sets as its ItemContainerTheme (items that are controls are their own containers and
//!   are not themed by it, as in WinUI an item that is a FlipViewItem keeps its own style).
//! - TabNavigation="Once" is KeyboardNavigation.TabNavigation; the rails, the scroll chaining and
//!   the deferred scrolling of the scroll viewer are left out.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::input::KeyboardNavigation;
use ferroui_base::input::KeyboardNavigationMode;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::Thickness;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::Carousel;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::ItemsControl;
use ferroui_controls::ScrollViewer;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultFlipViewItemStyle", default_flip_view_item_style);
    resources.control_theme("DefaultFlipViewStyle", default_flip_view_style);
    resources.control_theme(type_key::<Carousel>(), carousel_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// `DefaultFlipViewItemStyle`: a control theme of `ContentPresenter`.
fn default_flip_view_item_style(theme: &StyleBuilder) -> Result {
    theme.target::<ContentPresenter>();
    theme.dynamic(ContentPresenter::background_property(), "FlipViewItemBackground");
    theme.set(
        ContentPresenter::horizontal_content_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(
        ContentPresenter::vertical_content_alignment_property(),
        VerticalAlignment::Stretch,
    );
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Local,
    );
    Ok(())
}

/// The template of `Carousel` of `default_flip_view_style`.
fn default_flip_view_style_template(t: &Template<Carousel>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
        .template_bind(
            Border::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(ScrollViewer::new())
                .name("PART_ScrollViewer")?
                .template_bind(
                    ScrollViewer::horizontal_scroll_bar_visibility_property(),
                    ScrollViewer::horizontal_scroll_bar_visibility_property(),
                )
                .template_bind(
                    ScrollViewer::vertical_scroll_bar_visibility_property(),
                    ScrollViewer::vertical_scroll_bar_visibility_property(),
                )
                .set(ScrollViewer::bring_into_view_on_focus_change_property(), false)
                .set(InputElement::is_tab_stop_property(), false)
                .content(
                    ContentControl::content_property(),
                    t.make(ItemsPresenter::new())
                        .name("PART_ItemsPresenter")?
                        .template_bind(
                            ItemsPresenter::items_panel_property(),
                            ItemsControl::items_panel_property(),
                        )
                        .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property()),
                )?,
        )
        .control())
}

/// `DefaultFlipViewStyle`: a control theme of `Carousel`.
fn default_flip_view_style(theme: &StyleBuilder) -> Result {
    theme.target::<Carousel>();
    theme.dynamic(TemplatedControl::background_property(), "FlipViewBackground");
    theme.set(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0));
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Once,
    );
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(
        ScrollViewer::horizontal_scroll_bar_visibility_property(),
        ScrollBarVisibility::Hidden,
    );
    theme.set(
        ScrollViewer::vertical_scroll_bar_visibility_property(),
        ScrollBarVisibility::Hidden,
    );
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.static_resource(
        ItemsControl::item_container_theme_property(),
        "DefaultFlipViewItemStyle",
    )?;
    theme.template(default_flip_view_style_template);
    Ok(())
}

/// The default control theme of `Carousel`.
fn carousel_theme(theme: &StyleBuilder) -> Result {
    theme.target::<Carousel>();
    theme.based_on("DefaultFlipViewStyle")?;
    Ok(())
}
