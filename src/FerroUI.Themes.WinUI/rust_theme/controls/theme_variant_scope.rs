//! `Controls/ThemeVariantScope.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a style: FerroUI's ThemeVariantScope is the counterpart of RequestedTheme on an
//! element of WinUI 3, which needs no style. As in a WinUI 3 window, text in the scope takes the
//! primary text colour of its theme (TextFillColorPrimaryBrush of
//! controls/dev/CommonStyles/Common_themeresources_any.xaml in microsoft-ui-xaml,
//! https://github.com/microsoft/microsoft-ui-xaml, commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_controls::documents::TextElement;
use ferroui_controls::ThemeVariantScope;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<ThemeVariantScope>(), theme_variant_scope_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The default control theme of `ThemeVariantScope`.
fn theme_variant_scope_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ThemeVariantScope>();
    theme.dynamic(TextElement::foreground_property(), "TextFillColorPrimaryBrush");
    Ok(())
}
