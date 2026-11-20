//! `Controls/ListBox.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/ListBox_themeresources.xaml (the style DefaultListBoxStyle; the
//!   resources of the document are Resources/ListBox_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - Template parts: the ScrollViewer of the source is FerroUI's PART_ScrollViewer, the
//!   ItemsPresenter PART_ItemsPresenter.
//! - The scroll bar visibilities, IsDeferredScrollingEnabled and BringIntoViewOnFocusChange are
//!   bound as in the source; the scroll, rail and zoom modes have no counterpart. The scroll
//!   chaining, inertia, snap points and automatic hiding of the bars, which FerroUI's list box
//!   passes to its scroll viewer (as its other themes do), are bound as well.
//! - TabNavigation="Once" is KeyboardNavigation.TabNavigation; UseSystemFocusVisuals is the
//!   default focus adorner.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::input::KeyboardNavigation;
use ferroui_base::input::KeyboardNavigationMode;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::Visual;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::ItemsControl;
use ferroui_controls::ListBox;
use ferroui_controls::Panel;
use ferroui_controls::ScrollViewer;
use ferroui_controls::VirtualizingStackPanel;
use ferroui_markup_xaml::rust_theme::items_panel;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultListBoxStyle", default_list_box_style);
    resources.control_theme(type_key::<ListBox>(), list_box_theme);
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

/// The items panel of `default_list_box_style`.
fn default_list_box_style_items_panel(t: &Template<Control>) -> Result<Ref<Panel>> {
    Ok(t.make(VirtualizingStackPanel::new()).end().upcast())
}

/// The template of `ListBox` of `default_list_box_style`.
fn default_list_box_style_template(t: &Template<ListBox>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("LayoutRoot")?
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
        .template_bind(Visual::clip_to_bounds_property(), Visual::clip_to_bounds_property())
        .child(
            Decorator::child_property(),
            t.make(ScrollViewer::new())
                .name("PART_ScrollViewer")?
                .template_bind(
                    TemplatedControl::padding_property(),
                    TemplatedControl::padding_property(),
                )
                .template_bind(
                    ScrollViewer::horizontal_scroll_bar_visibility_property(),
                    ScrollViewer::horizontal_scroll_bar_visibility_property(),
                )
                .template_bind(
                    ScrollViewer::vertical_scroll_bar_visibility_property(),
                    ScrollViewer::vertical_scroll_bar_visibility_property(),
                )
                .template_bind(
                    ScrollViewer::is_deferred_scrolling_enabled_property(),
                    ScrollViewer::is_deferred_scrolling_enabled_property(),
                )
                .template_bind(
                    ScrollViewer::bring_into_view_on_focus_change_property(),
                    ScrollViewer::bring_into_view_on_focus_change_property(),
                )
                .template_bind(
                    ScrollViewer::is_scroll_chaining_enabled_property(),
                    ScrollViewer::is_scroll_chaining_enabled_property(),
                )
                .template_bind(
                    ScrollViewer::is_scroll_inertia_enabled_property(),
                    ScrollViewer::is_scroll_inertia_enabled_property(),
                )
                .template_bind(
                    ScrollViewer::vertical_snap_points_type_property(),
                    ScrollViewer::vertical_snap_points_type_property(),
                )
                .template_bind(
                    ScrollViewer::horizontal_snap_points_type_property(),
                    ScrollViewer::horizontal_snap_points_type_property(),
                )
                .template_bind(
                    ScrollViewer::allow_auto_hide_property(),
                    ScrollViewer::allow_auto_hide_property(),
                )
                .content(
                    ContentControl::content_property(),
                    t.make(ItemsPresenter::new())
                        .name("PART_ItemsPresenter")?
                        .template_bind(
                            ItemsPresenter::items_panel_property(),
                            ItemsControl::items_panel_property(),
                        ),
                )?,
        )
        .control())
}

/// `DefaultListBoxStyle`: a control theme of `ListBox`.
fn default_list_box_style(theme: &StyleBuilder) -> Result {
    theme.target::<ListBox>();
    theme.dynamic(TemplatedControl::foreground_property(), "ListBoxForeground");
    theme.dynamic(TemplatedControl::background_property(), "ListBoxBackground");
    theme.dynamic(TemplatedControl::border_brush_property(), "ListBoxBorder");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "ListBoxBorderThemeThickness",
    );
    theme.set(
        ScrollViewer::horizontal_scroll_bar_visibility_property(),
        ScrollBarVisibility::Disabled,
    );
    theme.set(
        ScrollViewer::vertical_scroll_bar_visibility_property(),
        ScrollBarVisibility::Auto,
    );
    theme.set(ScrollViewer::is_deferred_scrolling_enabled_property(), false);
    theme.set(ScrollViewer::bring_into_view_on_focus_change_property(), true);
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Once,
    );
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.set(
        ItemsControl::items_panel_property(),
        items_panel(theme.scope(), default_list_box_style_items_panel),
    );
    theme.template(default_list_box_style_template);
    Ok(())
}

/// The default control theme of `ListBox`.
fn list_box_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ListBox>();
    theme.based_on("DefaultListBoxStyle")?;
    Ok(())
}
