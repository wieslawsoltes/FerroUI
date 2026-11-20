//! `Resources/ThemeShadows.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a document: the theme shadow WinUI 3 applies in code under the elevated popups.
//! microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml, commit
//! 7e4b0c2096be402f51cc8f7d203ebf67cff183d0) gives a popup element a ThemeShadow and a Translation.Z
//! (dxaml/xcp/dxaml/lib/ElevationHelper.cpp, ApplyElevationEffect: 32 for a flyout, a menu flyout,
//! the drop-downs of ComboBox and AutoSuggestBox and the date and time picker flyouts, 8 more per
//! level of a submenu, 16 for a tool tip, ToolTip_Partial.cpp, 128 for a content dialog) and draws
//! it as a drop shadow by the recipe of dxaml/xcp/components/graphics/inc/DropShadowRecipe.h (all
//! shadows of WinUI 3 are drop shadows, CThemeShadow::IsDropShadowMode). The templates of the theme
//! set the same Translation and WinUIThemeShadow, a ThemeShadow shared as WinUI applications share
//! one; FerroUI draws its recipe, in the theme of the element (none in high contrast).
//! The margins are the insets of the recipe:
//!   elevation = Translation.Z / 2;
//!   the insets the shadow needs around the element: the largest blur at the sides, the blur less
//!   the offset at the top, the blur and the offset at the bottom.
//! A popup of FerroUI is a window of the size of its content, which would cut the shadow off; a
//! popup whose child has a margin is placed by the bounds of the child without it
//! (PopupPositionerParameters.deflate), so the margin is room for the shadow inside the popup
//! without moving the element, as a windowed popup of WinUI grows by the insets of its shadow
//! (CThemeShadow::GetInsetsForWindowedPopup) (docs/porting/winui-theme/DEVIATIONS.md, G-24).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::media::ThemeShadow;
use ferroui_base::Thickness;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // Translation.Z 32: elevation 16, blur 16, offset 8; insets 16 at the sides, 8 above, 24 below.
    resources.add("ThemeShadowFlyoutMargin", Thickness::new(16.0, 8.0, 16.0, 24.0));
    // Translation.Z 40 (a submenu): elevation 20, ambient blur 6.67 and offset 2, directional blur
    // 20 and offset 10; insets 20 at the sides, 10 above, 30 below.
    resources.add("ThemeShadowSubMenuMargin", Thickness::new(20.0, 10.0, 20.0, 30.0));
    // Translation.Z 16 (a tool tip): elevation 8, blur 8, offset 4; insets 8 at the sides, 4 above,
    // 12 below.
    resources.add("ThemeShadowToolTipMargin", Thickness::new(8.0, 4.0, 8.0, 12.0));
    // The shadow the elevated elements of the templates cast (UIElement.Shadow).
    resources.deferred("WinUIThemeShadow", |scope| {
        Ok(value(scope.make(ThemeShadow::new()).end()))
    });
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
