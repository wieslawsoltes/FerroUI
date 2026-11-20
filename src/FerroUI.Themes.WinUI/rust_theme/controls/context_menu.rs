//! `Controls/ContextMenu.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
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
//! - WinUI has no context menu control: the context menu of an element (ContextFlyout) is a
//!   MenuFlyout, shown by a MenuFlyoutPresenter. FerroUI's ContextMenu is a menu in a popup of its
//!   own, so its theme is the theme of the menu presenter (Controls/MenuFlyoutPresenter.xaml): the
//!   same setters and template, with the items presenter as PART_ItemsPresenter and the scroll
//!   viewer MenuFlyoutPresenterScrollViewer.
//! - Focusable is set, as the other themes of FerroUI do, so that the open menu takes the
//!   keyboard. WindowManagerAddShadowHint is off: the shadow a window manager adds to a popup
//!   window does not follow the rounded corners of the presenter; the shadow of the menu flyout
//!   (a ThemeShadow at a Translation.Z of 32) is a ThemeShadow (the shared WinUIThemeShadow, Translation.Z as in WinUI) on the
//!   border ThemeShadowWrapper around the border of the menu (a border of its own), with the margin
//!   ThemeShadowFlyoutMargin as the room for it in the popup (Resources/ThemeShadows.xaml;
//!   DEVIATIONS.md, G-24).
//! - The SystemBackdrop setter is kept, as for MenuFlyoutPresenter (the property of the presenter,
//!   which ContextMenu shares). The MinWidth bound to TemplateSettings.FlyoutContentMinWidth, and
//!   the scroll, rail and zoom modes are left out, as for MenuFlyoutPresenter.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
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
use ferroui_controls::ContextMenu;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::ItemsControl;
use ferroui_controls::ScrollViewer;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<ContextMenu>(), context_menu_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `ContextMenu` of `context_menu_theme`.
fn context_menu_theme_template(t: &Template<ContextMenu>) -> Result<Ref<Control>> {
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

/// The default control theme of `ContextMenu`.
fn context_menu_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ContextMenu>();
    // The shadow is drawn outside the bounds of the control: a templated control clips to its
    //         bounds unless its theme says otherwise (G-24).
    theme.set(Visual::clip_to_bounds_property(), false);
    theme.dynamic(TemplatedControl::background_property(), "MenuFlyoutPresenterBackground");
    theme.static_resource(ContextMenu::system_backdrop_property(), "MenuFlyoutSystemBackdrop")?;
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
    theme.set(InputElement::focusable_property(), true);
    theme.set(ContextMenu::window_manager_add_shadow_hint_property(), false);
    theme.template(context_menu_theme_template);
    Ok(())
}
