//! `Controls/MenuScrollViewer.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/MenuFlyout_themeresources.xaml (the ScrollViewer
//!   MenuFlyoutPresenterScrollViewer of the template of DefaultMenuFlyoutPresenterStyle; the
//!   resources of the document are Resources/MenuFlyout_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The scroll viewer of an open menu is a plain ScrollViewer of WinUI (DefaultScrollViewerStyle)
//!   with the scroll settings of the menu presenter: no horizontal scrolling, a vertical scroll bar
//!   when the items do not fit. It is the keyed theme MenuFlyoutPresenterScrollViewer (the name of
//!   the element in the source), which the templates of MenuFlyoutPresenter, ContextMenu and the
//!   submenus of MenuItem use. WinUI has no scroll buttons at the ends of a menu: the scroll bar
//!   scrolls it, as the scroll bar of any scroll viewer.
//! - HorizontalScrollMode, VerticalScrollMode, IsHorizontalRailEnabled, IsVerticalRailEnabled and
//!   ZoomMode, which the presenter passes to its scroll viewer, have no counterpart.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::ScrollViewer;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("MenuFlyoutPresenterScrollViewer", menu_flyout_presenter_scroll_viewer);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// `MenuFlyoutPresenterScrollViewer`: a control theme of `ScrollViewer`.
fn menu_flyout_presenter_scroll_viewer(theme: &StyleBuilder) -> Result {
    theme.target::<ScrollViewer>();
    theme.based_on("DefaultScrollViewerStyle")?;
    theme.set(
        ScrollViewer::horizontal_scroll_bar_visibility_property(),
        ScrollBarVisibility::Disabled,
    );
    theme.set(
        ScrollViewer::vertical_scroll_bar_visibility_property(),
        ScrollBarVisibility::Auto,
    );
    Ok(())
}
