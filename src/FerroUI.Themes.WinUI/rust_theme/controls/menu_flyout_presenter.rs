//! `Controls/MenuFlyoutPresenter.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/MenuFlyout_themeresources.xaml (the style
//!   DefaultMenuFlyoutPresenterStyle; the resources of the document are
//!   Resources/MenuFlyout_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - Template parts: the ItemsPresenter of the source is FerroUI's PART_ItemsPresenter. The scroll
//!   viewer MenuFlyoutPresenterScrollViewer keeps its name and has the keyed theme of the same
//!   name (Controls/MenuScrollViewer.xaml).
//! - The items presenter is a scope of shared sizes (Grid.IsSharedSizeScope): WinUI aligns the
//!   text of the items with the check placeholder and the icon placeholder states that the
//!   presenter computes from all of its items; the templates of MenuItem align them with shared
//!   column sizes instead (Controls/MenuItem.xaml).
//! - The SystemBackdrop setter (the resource MenuFlyoutSystemBackdrop, the desktop acrylic backdrop)
//!   is kept: the backdrop draws the acrylic as the background of the presenter where the menu is
//!   in a popup of the window, and its fallback colour where the menu is in a popup window of its
//!   own (docs/porting/winui-materials.md).
//! - The MinWidth of the scroll viewer, bound to TemplateSettings.FlyoutContentMinWidth (the width
//!   of the target of a flyout opened with a minimum width), has no counterpart.
//! - The scroll and rail modes and ZoomMode of the source are left out.
//! - The shadow WinUI's presenter gives itself in code (ApplyElevationEffect at the depth of the
//!   menu, MenuFlyoutPresenter_Partial.cpp: a ThemeShadow at a Translation.Z of 32 for a menu) is
//!   a ThemeShadow (the shared WinUIThemeShadow, Translation.Z 32) on the border ThemeShadowWrapper
//!   around the border of the presenter, with the margin ThemeShadowFlyoutMargin as the room for it
//!   in the popup (Resources/ThemeShadows.xaml; DEVIATIONS.md, G-24).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::KeyboardNavigation;
use ferroui_base::input::KeyboardNavigationMode;
use ferroui_base::layout::Layoutable;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::StyledElement;
use ferroui_base::Vector3D;
use ferroui_base::Visual;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::ItemsControl;
use ferroui_controls::MenuFlyoutPresenter;
use ferroui_controls::ScrollViewer;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultMenuFlyoutPresenterStyle", default_menu_flyout_presenter_style);
    resources.control_theme(type_key::<MenuFlyoutPresenter>(), menu_flyout_presenter_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `MenuFlyoutPresenter` of `default_menu_flyout_presenter_style`.
fn default_menu_flyout_presenter_style_template(t: &Template<MenuFlyoutPresenter>) -> Result<Ref<Control>> {
    // The theme shadow on a border of its own (G-24).
    Ok(t.make(Border::new())
        .name("ThemeShadowWrapper")?
        .set(Visual::translation_property(), literal(Vector3D::parse("0,0,32"))?)
        .static_resource(Visual::shadow_property(), "WinUIThemeShadow")?
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .template_bind(
                    Border::border_brush_property(),
                    TemplatedControl::border_brush_property(),
                )
                .template_bind(
                    Border::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(
                    Border::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                .child(
                    Decorator::child_property(),
                    t.make(ScrollViewer::new())
                        .name("MenuFlyoutPresenterScrollViewer")?
                        .static_resource(StyledElement::theme_property(), "MenuFlyoutPresenterScrollViewer")?
                        .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
                        .template_bind(
                            ScrollViewer::horizontal_scroll_bar_visibility_property(),
                            ScrollViewer::horizontal_scroll_bar_visibility_property(),
                        )
                        .template_bind(
                            ScrollViewer::vertical_scroll_bar_visibility_property(),
                            ScrollViewer::vertical_scroll_bar_visibility_property(),
                        )
                        .content(
                            ContentControl::content_property(),
                            t.make(ItemsPresenter::new())
                                .name("PART_ItemsPresenter")?
                                .template_bind(
                                    ItemsPresenter::items_panel_property(),
                                    ItemsControl::items_panel_property(),
                                )
                                .set(
                                    KeyboardNavigation::tab_navigation_property(),
                                    KeyboardNavigationMode::Continue,
                                )
                                .set(Grid::is_shared_size_scope_property(), true),
                        )?,
                ),
        )
        .control())
}

/// `DefaultMenuFlyoutPresenterStyle`: a control theme of `MenuFlyoutPresenter`.
fn default_menu_flyout_presenter_style(theme: &StyleBuilder) -> Result {
    theme.target::<MenuFlyoutPresenter>();
    // The shadow is drawn outside the bounds of the control: a templated control clips to its
    //         bounds unless its theme says otherwise (G-24).
    theme.set(Visual::clip_to_bounds_property(), false);
    theme.dynamic(TemplatedControl::background_property(), "MenuFlyoutPresenterBackground");
    theme.static_resource(
        MenuFlyoutPresenter::system_backdrop_property(),
        "MenuFlyoutSystemBackdrop",
    )?;
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "MenuFlyoutPresenterBorderBrush",
    );
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "MenuFlyoutPresenterBorderThemeThickness",
    );
    theme.static_resource(TemplatedControl::padding_property(), "MenuFlyoutPresenterThemePadding")?;
    theme.set(
        ScrollViewer::horizontal_scroll_bar_visibility_property(),
        ScrollBarVisibility::Disabled,
    );
    theme.set(
        ScrollViewer::vertical_scroll_bar_visibility_property(),
        ScrollBarVisibility::Auto,
    );
    theme.dynamic(Layoutable::min_width_property(), "FlyoutThemeMinWidth");
    theme.static_resource(Layoutable::min_height_property(), "MenuFlyoutThemeMinHeight")?;
    theme.dynamic(TemplatedControl::corner_radius_property(), "OverlayCornerRadius");
    // The room for the shadow in the popup (G-24).
    theme.static_resource(Layoutable::margin_property(), "ThemeShadowFlyoutMargin")?;
    theme.template(default_menu_flyout_presenter_style_template);
    Ok(())
}

/// The default control theme of `MenuFlyoutPresenter`.
fn menu_flyout_presenter_theme(theme: &StyleBuilder) -> Result {
    theme.target::<MenuFlyoutPresenter>();
    theme.based_on("DefaultMenuFlyoutPresenterStyle")?;
    Ok(())
}
