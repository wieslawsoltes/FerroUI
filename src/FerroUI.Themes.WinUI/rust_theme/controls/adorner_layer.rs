//! `Controls/AdornerLayer.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a style: the focus visual of WinUI 3 is drawn by the framework for an element
//! with UseSystemFocusVisuals="True" (microsoft-ui-xaml, https://github.com/microsoft/microsoft-ui-xaml,
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0). It is two rectangles: the outer one of
//! FocusVisualPrimaryThickness (2) in FocusVisualPrimaryBrush, the inner one of
//! FocusVisualSecondaryThickness (1) in FocusVisualSecondaryBrush, whose defaults are the resources
//! SystemControlFocusVisualPrimaryBrush and SystemControlFocusVisualSecondaryBrush
//! (controls/dev/CommonStyles/Common_themeresources.xaml), at FocusVisualMargin (0) around the
//! element and with its corner radius. FerroUI draws focus visuals in the adorner layer: this is
//! the focus adorner of the controls without one of their own; the controls with a
//! FocusVisualMargin set their FocusAdorner (docs/porting/winui-theme.md, section 4.6).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::Thickness;
use ferroui_controls::primitives::AdornerLayer;
use ferroui_controls::Border;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<AdornerLayer>(), adorner_layer_theme);
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

/// The focus adorner of `adorner_layer_theme`.
fn adorner_layer_theme_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Border::border_thickness_property(), Thickness::uniform(2.0))
        .dynamic(Border::border_brush_property(), "SystemControlFocusVisualPrimaryBrush")
        .dynamic(Border::corner_radius_property(), "ControlCornerRadius")
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .set(Border::border_thickness_property(), Thickness::uniform(1.0))
                .dynamic(
                    Border::border_brush_property(),
                    "SystemControlFocusVisualSecondaryBrush",
                )
                .dynamic(Border::corner_radius_property(), "ControlCornerRadius"),
        )
        .control())
}

/// The default control theme of `AdornerLayer`.
fn adorner_layer_theme(theme: &StyleBuilder) -> Result {
    theme.target::<AdornerLayer>();
    theme.set(
        AdornerLayer::default_focus_adorner_property(),
        Some(focus_adorner(theme.scope(), adorner_layer_theme_focus_adorner)),
    );
    Ok(())
}
