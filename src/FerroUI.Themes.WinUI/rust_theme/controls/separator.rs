//! `Controls/Separator.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/MenuFlyout_themeresources.xaml (the implicit style of
//!   MenuFlyoutSeparator; the resources of the document are
//!   Resources/MenuFlyout_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - FerroUI has one Separator, in menus and anywhere else; WinUI's separator of a menu,
//!   MenuFlyoutSeparator, is its style: a rectangle of MenuFlyoutSeparatorHeight (1) in the
//!   background at the padding MenuFlyoutSeparatorThemePadding (-4,1,-4,1), whose negative
//!   horizontal values reach over the padding of the items of a menu. A separator outside a menu
//!   keeps them. The separator of a command bar (AppBarSeparator) is CommandBarSeparator.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::Layoutable;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Control;
use ferroui_controls::Separator;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<Separator>(), separator_theme);
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

/// The template of `Separator` of `separator_theme`.
fn separator_theme_template(t: &Template<Separator>) -> Result<Ref<Control>> {
    Ok(t.make(Rectangle::new())
        .template_bind(Shape::fill_property(), TemplatedControl::background_property())
        .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
        .static_resource(Layoutable::height_property(), "MenuFlyoutSeparatorHeight")?
        .control())
}

/// The default control theme of `Separator`.
fn separator_theme(theme: &StyleBuilder) -> Result {
    theme.target::<Separator>();
    theme.dynamic(TemplatedControl::background_property(), "MenuFlyoutSeparatorBackground");
    theme.static_resource(TemplatedControl::padding_property(), "MenuFlyoutSeparatorThemePadding")?;
    theme.template(separator_theme_template);
    Ok(())
}
