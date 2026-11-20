//! `Controls/TextSelectionHandle.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a style: the selection handles of the text controls of WinUI 3 (the grippers)
//! are drawn in code by the framework, without a template, in microsoft-ui-xaml
//! (https://github.com/microsoft/microsoft-ui-xaml, commit
//! 7e4b0c2096be402f51cc8f7d203ebf67cff183d0): CTextSelectionGripper of
//! dxaml/xcp/core/text/common/Gripper.cpp, with the sizes of
//! dxaml/xcp/core/text/common/TextSelectionSettings.cpp in millimetres at 96 dots per inch. A
//! gripper is an outer circle of 4.8 mm (18.14 px) in the gripper fill, white, or the window
//! colour of a high contrast theme (CCoreServices::GetTextSelectionGripperBrush), with an inner
//! circle of 3.7 mm (13.98 px) in the same fill, outlined 0.4 mm (1.51 px) in the selection
//! colour of the text control made opaque, and a pole of 0.6 mm (2.27 px) in the selection colour
//! from the top of the text line down into the circles. The hit area is 8 mm by 9.3 mm (30.24 px
//! by 35.15 px), centred under the selection point, from the top of the circles. The start, end
//! and caret grippers are the same.
//!
//! This is that gripper as the template FerroUI's TextSelectionHandle requires: the circles
//! centred under PART_Indicator, the point of the selection (FerroUI places the top of the handle
//! at the bottom of the text line, at the x of the centre of PART_Indicator), in the hit area.
//! The fill is SystemColorWindowColor (white in the light and dark dictionaries, the window colour
//! in the high contrast one) and the outline TextControlSelectionHighlightColor, the selection
//! colour of FerroUI's text box (its alpha is not removed).
//!
//! The pole is left out: WinUI draws it through the height of the text line above the gripper,
//! which FerroUI's handle does not know. The start and end handles are the circle of the caret,
//! as in WinUI (FerroUI's :start and :end pseudo-classes change nothing).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::Brush;
use ferroui_base::Thickness;
use ferroui_controls::primitives::TextSelectionHandle;
use ferroui_controls::shapes::Ellipse;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Control;
use ferroui_controls::Grid;
use ferroui_controls::Panel;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<TextSelectionHandle>(), text_selection_handle_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `TextSelectionHandle` of `text_selection_handle_theme`.
fn text_selection_handle_theme_template(t: &Template<TextSelectionHandle>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("HitTestBox")?
        .set(Layoutable::width_property(), 30.24)
        .set(Layoutable::height_property(), 35.15)
        .set(
            Panel::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        )
        .children([
            t.make(Border::new())
                .name("PART_Indicator")?
                .set(Layoutable::width_property(), 1.0)
                .set(Layoutable::height_property(), 1.0)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                .control(),
            t.make(Ellipse::new())
                .name("EllipseExternal")?
                .set(Layoutable::width_property(), 18.14)
                .set(Layoutable::height_property(), 18.14)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                .dynamic(Shape::fill_property(), "SystemColorWindowColor")
                .control(),
            t.make(Ellipse::new())
                .name("EllipseInternal")?
                .set(Layoutable::width_property(), 13.98)
                .set(Layoutable::height_property(), 13.98)
                .set(Layoutable::margin_property(), Thickness::new(0.0, 2.08, 0.0, 0.0))
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                .dynamic(Shape::fill_property(), "SystemColorWindowColor")
                .dynamic(Shape::stroke_property(), "TextControlSelectionHighlightColor")
                .set(Shape::stroke_thickness_property(), 1.51)
                .control(),
        ])
        .control())
}

/// The default control theme of `TextSelectionHandle`.
fn text_selection_handle_theme(theme: &StyleBuilder) -> Result {
    theme.target::<TextSelectionHandle>();
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center);
    theme.template(text_selection_handle_theme_template);
    Ok(())
}
