//! `Controls/Thumb.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   dxaml/xcp/dxaml/themes/generic.xaml (the style of Thumb; its resources are
//!   Resources/Generic.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! The visual states PointerOver and Pressed show one of three stacked borders through a zero
//! length animation of their opacity, which is a value here (docs/porting/winui-theme.md,
//! section 4.4). The thumbs of ScrollBar and Slider have keyed themes of their own.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::primitives::Thumb;
use ferroui_controls::Border;
use ferroui_controls::Control;
use ferroui_controls::Grid;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<Thumb>(), thumb_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `Thumb` of `thumb_theme`.
fn thumb_theme_template(t: &Template<Thumb>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .children([
            t.make(Border::new())
                .name("Background")?
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
                .control(),
            t.make(Border::new())
                .name("BackgroundPointerOver")?
                .dynamic(Border::background_property(), "ThumbBackgroundPointerOver")
                .dynamic(Border::border_brush_property(), "ThumbBorderBrushPointerOver")
                .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                .template_bind(
                    Border::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(
                    Border::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .set(Visual::opacity_property(), 0.0)
                .control(),
            t.make(Border::new())
                .name("BackgroundPressed")?
                .dynamic(Border::background_property(), "ThumbBackgroundPressed")
                .dynamic(Border::border_brush_property(), "ThumbBorderBrushPressed")
                .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                .template_bind(
                    Border::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(
                    Border::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .set(Visual::opacity_property(), 0.0)
                .control(),
        ])
        .control())
}

/// The default control theme of `Thumb`.
fn thumb_theme(theme: &StyleBuilder) -> Result {
    theme.target::<Thumb>();
    theme.dynamic(TemplatedControl::background_property(), "ThumbBackground");
    theme.set(TemplatedControl::border_thickness_property(), Thickness::uniform(1.0));
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.dynamic(TemplatedControl::border_brush_property(), "ThumbBorderBrush");
    theme.template(thumb_theme_template);
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(
            nesting().template().of::<Border>().name("BackgroundPointerOver"),
            |style| {
                style.set(Visual::opacity_property(), 1.0);
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("Background"), |style| {
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        })?;
        Ok(())
    })?;
    // Pressed (the state replaces PointerOver)
    theme.style(nesting().class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("BackgroundPressed"), |style| {
            style.set(Visual::opacity_property(), 1.0);
            Ok(())
        })?;
        style.style(
            nesting().template().of::<Border>().name("BackgroundPointerOver"),
            |style| {
                style.set(Visual::opacity_property(), 0.0);
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Border>().name("Background"), |style| {
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}
