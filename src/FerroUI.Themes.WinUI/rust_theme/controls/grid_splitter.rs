//! `Controls/GridSplitter.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a style: the merged theme of WinUI 3 in microsoft-ui-xaml
//! (https://github.com/microsoft/microsoft-ui-xaml, commit
//! 7e4b0c2096be402f51cc8f7d203ebf67cff183d0) has no grid splitter. The theme is built from the
//! control fills of the WinUI palette (controls/dev/CommonStyles/Common_themeresources_any.xaml):
//! the bar is ControlStrongFillColorDefaultBrush, the fill of the strong parts of the controls
//! (the thumb of a scroll bar, the knob of a toggle switch), 4 px thick, and
//! ControlStrongFillColorDisabledBrush when the splitter is disabled, as those controls are; the
//! preview of a drag (ShowsPreview) is a bar of AccentFillColorDefaultBrush. The splitter is
//! focusable and shows the default focus visual of the theme; FerroUI's GridSplitter sets its
//! resize cursor itself.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::Layoutable;
use ferroui_base::media::BackgroundSizing;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Control;
use ferroui_controls::GridSplitter;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::template_of;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<GridSplitter>(), grid_splitter_theme);
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

/// The focus adorner of `grid_splitter_theme`.
fn grid_splitter_theme_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Rectangle::new())
        .dynamic(Shape::fill_property(), "AccentFillColorDefaultBrush")
        .control())
}

/// The template of `GridSplitter` of `grid_splitter_theme`.
fn grid_splitter_theme_template(t: &Template<GridSplitter>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("SplitterBorder")?
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
        .control())
}

/// The default control theme of `GridSplitter`.
fn grid_splitter_theme(theme: &StyleBuilder) -> Result {
    theme.target::<GridSplitter>();
    theme.set(InputElement::focusable_property(), true);
    theme.set(Layoutable::min_width_property(), 4.0);
    theme.set(Layoutable::min_height_property(), 4.0);
    theme.dynamic(
        TemplatedControl::background_property(),
        "ControlStrongFillColorDefaultBrush",
    );
    theme.set(
        GridSplitter::preview_content_property(),
        Some(template_of(theme.scope(), grid_splitter_theme_focus_adorner)),
    );
    theme.template(grid_splitter_theme_template);
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.dynamic(
            TemplatedControl::background_property(),
            "ControlStrongFillColorDisabledBrush",
        );
        Ok(())
    })?;
    Ok(())
}
