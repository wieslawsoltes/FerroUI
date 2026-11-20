//! `Controls/PathIcon.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a style: WinUI's PathIcon has no template in microsoft-ui-xaml
//! (https://github.com/microsoft/microsoft-ui-xaml, commit
//! 7e4b0c2096be402f51cc8f7d203ebf67cff183d0). The control builds its visual in code
//! (CPathIcon::ApplyTemplate, dxaml/xcp/core/core/elements/icon.cpp): one Path, stretched over
//! the icon by its alignments, whose Fill is the Foreground of the icon and whose Data is the Data
//! of the icon. WinUI's Path keeps its own Stretch (None), so the geometry is drawn in its own
//! coordinates; IconElement has no default size. The port sizes the icon 20 by 20 and scales the
//! geometry uniformly, as the framework's other themes do and its applications expect (PI-2), inside
//! a border of the Background of the control (null by default, so nothing more is drawn).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::Layoutable;
use ferroui_base::media::Stretch;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::PathIcon;
use ferroui_controls::Viewbox;
use ferroui_markup_xaml::rust_theme::control_template;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // The path icon of WinUI (CPathIcon::ApplyTemplate: a Path with Stretch None, its geometry in
    // its own coordinates, the size of the icon the extent of the geometry from its origin), for
    // the templates of WinUI that size the icon in a Viewbox (AppBarButton and
    // AppBarToggleButton: ContentViewbox of AppBarButtonContentHeight; MenuFlyoutItem: IconRoot
    // of 16; NavigationViewItem: IconBox of NavigationViewItemOnLeftIconBoxHeight; TabViewItem:
    // IconBox of TabViewItemHeaderIconSize): there the Viewbox scales the geometry as WinUI's does
    // (PI-3), where the default theme below scales the bounds of the geometry to 20 by 20 (PI-2).
    // Set with Width and Height NaN by a style of the icon presenter of those templates, and for
    // the icon of a menu item, a logical child of the item that no style of its template reaches,
    // by the style MenuItem > PathIcon of Controls/WinUIControls.xaml (a style of a control theme
    // cannot name a parent: the nesting selector ^ stands for the whole selector of the theme,
    // and what is written before it is dropped).
    resources.deferred("PathIconUnscaledTemplate", |scope| {
        Ok(value(control_template(scope, path_icon_unscaled_template_template)))
    });
    resources.control_theme(type_key::<PathIcon>(), path_icon_theme);
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

/// The template of `PathIcon` of `PathIconUnscaledTemplate`.
fn path_icon_unscaled_template_template(t: &Template<PathIcon>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .child(
            Decorator::child_property(),
            t.make(Path::new())
                .name("Path")?
                .template_bind(Path::data_property(), PathIcon::data_property())
                .template_bind(Shape::fill_property(), TemplatedControl::foreground_property()),
        )
        .control())
}

/// The template of `PathIcon` of `path_icon_theme`.
fn path_icon_theme_template(t: &Template<PathIcon>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .child(
            Decorator::child_property(),
            t.make(Viewbox::new())
                .template_bind(Layoutable::height_property(), Layoutable::height_property())
                .template_bind(Layoutable::width_property(), Layoutable::width_property())
                .child(
                    Viewbox::child_property(),
                    t.make(Path::new())
                        .name("Path")?
                        .template_bind(Path::data_property(), PathIcon::data_property())
                        .template_bind(Shape::fill_property(), TemplatedControl::foreground_property())
                        .set(Shape::stretch_property(), Stretch::Uniform),
                ),
        )
        .control())
}

/// The default control theme of `PathIcon`.
fn path_icon_theme(theme: &StyleBuilder) -> Result {
    theme.target::<PathIcon>();
    // The size and the uniform scaling of the framework's path icons (PI-2): 20 by 20, the
    // IconElementThemeWidth and IconElementThemeHeight of its other themes.
    theme.set(Layoutable::width_property(), 20.0);
    theme.set(Layoutable::height_property(), 20.0);
    theme.template(path_icon_theme_template);
    Ok(())
}
