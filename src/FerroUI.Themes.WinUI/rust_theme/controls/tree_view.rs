//! `Controls/TreeView.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/TreeView/TreeView.xaml (the style DefaultTreeViewStyle; the resources of
//!   controls/dev/TreeView/TreeView_themeresources.xaml are Resources/TreeView_themeresources.xaml)
//!   dxaml/xcp/dxaml/themes/generic.xaml (the implicit style of ListView, the style of the
//!   TreeViewList the template of TreeView consists of)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - WinUI's TreeView is a TreeViewList, a ListView that shows the visible nodes of the tree as a
//!   flat list. FerroUI's TreeView hosts its items, which host their own children, so the template
//!   is the template of that list: a Border around the ScrollViewer ScrollViewer and the items
//!   presenter PART_ItemsPresenter, with the setters of the list (no horizontal scrolling, a
//!   vertical scroll bar when needed, TabNavigation Once, not a tab stop of its own) and of
//!   DefaultTreeViewStyle.
//! - CanDragItems, CanReorderItems, AllowDrop and the ItemContainerTransitions (content, reorder
//!   and entrance theme transitions) have no counterpart: FerroUI's tree view does not reorder its
//!   items by drag and drop and has no theme transitions.
//! - The padding, border and corner radius of the tree view are drawn, as FerroUI's other themes
//!   do (the defaults draw nothing, as WinUI); the scroll chaining and automatic hiding of the bars
//!   are passed to the scroll viewer.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::input::KeyboardNavigation;
use ferroui_base::input::KeyboardNavigationMode;
use ferroui_base::layout::Layoutable;
use ferroui_base::media::BackgroundSizing;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::ItemsControl;
use ferroui_controls::ScrollViewer;
use ferroui_controls::TreeView;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultTreeViewStyle", default_tree_view_style);
    resources.control_theme(type_key::<TreeView>(), tree_view_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `TreeView` of `default_tree_view_style`.
fn default_tree_view_style_template(t: &Template<TreeView>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .template_bind(
            Border::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
        .template_bind(Border::background_property(), TemplatedControl::background_property())
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
                .name("ScrollViewer")?
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
                        )
                        .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property()),
                )?,
        )
        .control())
}

/// `DefaultTreeViewStyle`: a control theme of `TreeView`.
fn default_tree_view_style(theme: &StyleBuilder) -> Result {
    theme.target::<TreeView>();
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Once,
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
    theme.template(default_tree_view_style_template);
    Ok(())
}

/// The default control theme of `TreeView`.
fn tree_view_theme(theme: &StyleBuilder) -> Result {
    theme.target::<TreeView>();
    theme.based_on("DefaultTreeViewStyle")?;
    Ok(())
}
