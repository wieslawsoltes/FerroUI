//! `Controls/RatingControl.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/RatingControl/RatingControl.xaml (the implicit style of RatingControl)
//!   controls/dev/RatingControl/RatingControl_themeresources.xaml (the item templates and the default
//!   item info; its resources are Resources/RatingControl_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - FerroUI's RatingControl is WinUI's RatingControl (a WinUI control in ferroui-controls,
//!   docs/porting/winui-controls.md). The parts keep WinUI's names: RatingBackgroundStackPanel and
//!   RatingForegroundStackPanel, which the control fills with its items, and Caption.
//! - WinUI's control creates its items from the DataTemplates BackgroundGlyphDefaultTemplate,
//!   ForegroundGlyphDefaultTemplate, BackgroundImageDefaultTemplate and ForegroundImageDefaultTemplate,
//!   which it looks up in the resources of the application, and reads the resources
//!   RatingControlFontSizeForRendering and RatingControlItemSpacing in code. FerroUI's control looks
//!   up no resource: it creates the text blocks and images of the items itself, and the template
//!   gives the stack panels what the item templates and the resources give in WinUI: the font size
//!   RatingControlFontSizeForRendering and the spacing RatingControlItemSpacing of the items, and,
//!   for the background panel, the foreground RatingControlUnselectedForeground of
//!   BackgroundGlyphDefaultTemplate. The -8,-8 margins of the item templates, which compensate the
//!   scale of the items in WinUI's layout, have no counterpart in the layout of the port (each item
//!   is drawn at half its size in a cell of half its width).
//!   WinUI: BackgroundGlyphDefaultTemplate, ForegroundGlyphDefaultTemplate, BackgroundImageDefaultTemplate, ForegroundImageDefaultTemplate
//! - The default ItemInfo of the style, the resource MUX_RatingControlDefaultFontInfo (the glyphs
//!   E735 and E734), is created by each control, so that a change of the glyphs of one control
//!   changes no other; the style does not set it.
//!   WinUI: Default/MUX_RatingControlDefaultFontInfo, Light/MUX_RatingControlDefaultFontInfo, HighContrast/MUX_RatingControlDefaultFontInfo
//! - CommonStates are the pseudo-classes the control sets: Placeholder is :placeholder,
//!   PointerOverPlaceholder :pointeroverplaceholder, PointerOverUnselected :pointeroverunselected,
//!   Set :set, PointerOverSet :pointeroverset and Disabled :disabled (written last: WinUI goes to
//!   Disabled after the other states). They set the foreground of ForegroundContentPresenter, which
//!   the glyphs of the foreground items inherit.
//! - The caption: WinUI's control sets its left margin to 12 less the spacing of the items that the
//!   item templates make (c_captionSpacing); the cells of the items of the port have none, so the
//!   margin is 12,0,20,0. The TranslateTransforms of the stack panels, which the control sets to
//!   centre the items vertically in WinUI, are left out: the cells of the items are as high as the
//!   rendering font size, the MinHeight of the control.
//! - The composition animations of the scale of the items (ApplyScaleExpressionAnimation) are a
//!   scale the control sets as the pointer moves. UseSystemFocusVisuals with FocusVisualMargin
//!   -8,-7,-8,0 is the focus adorner with that margin (section 4.6). IsFocusEngagementEnabled has no
//!   counterpart. AutomationProperties.AccessibilityView and AutomationProperties.Name of the caption
//!   are left out.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::Orientation;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::Brush;
use ferroui_base::media::TextLineBounds;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_controls::documents::TextElement;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::Panel;
use ferroui_controls::RatingControl;
use ferroui_controls::StackPanel;
use ferroui_controls::TextBlock;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // The implicit style of RatingControl (it has no key in WinUI).
    resources.control_theme(type_key::<RatingControl>(), rating_control_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The focus adorner of `rating_control_theme`.
fn rating_control_theme_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::new(-8.0, -7.0, -8.0, 0.0))
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

/// The template of `RatingControl` of `rating_control_theme`.
fn rating_control_theme_template(t: &Template<RatingControl>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("LayoutRoot")?
        .template_bind(Panel::background_property(), TemplatedControl::background_property())
        // The margin is on the stack panel, which the content presenter does not clip.
        .children([
            t.make(StackPanel::new())
                .name("CaptionStackPanel")?
                .set(StackPanel::orientation_property(), Orientation::Horizontal)
                .set(
                    Layoutable::margin_property(),
                    Thickness::new(-20.0, -20.0, -20.0, -20.0),
                )
                .children([
                    t.make(StackPanel::new())
                        .name("RatingBackgroundStackPanel")?
                        .set(StackPanel::orientation_property(), Orientation::Horizontal)
                        .set(
                            Panel::background_property(),
                            Some(literal(Brush::parse("Transparent"))?),
                        )
                        .set(Layoutable::margin_property(), Thickness::new(20.0, 20.0, 0.0, 20.0))
                        .static_resource(StackPanel::spacing_property(), "RatingControlItemSpacing")?
                        .static_resource(TextElement::font_size_property(), "RatingControlFontSizeForRendering")?
                        .dynamic(TextElement::foreground_property(), "RatingControlUnselectedForeground")
                        .control(),
                    t.make(TextBlock::new())
                        .name("Caption")?
                        .static_resource(StyledElement::theme_property(), "CaptionTextBlockStyle")?
                        .set(Layoutable::margin_property(), Thickness::new(12.0, 0.0, 20.0, 0.0))
                        .set(TextBlock::text_line_bounds_property(), TextLineBounds::Tight)
                        .set(TextBlock::font_size_property(), 12.0)
                        .template_bind(
                            TextBlock::foreground_property(),
                            TemplatedControl::foreground_property(),
                        )
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                        .set(InputElement::is_hit_test_visible_property(), false)
                        .template_bind(TextBlock::text_property(), RatingControl::caption_property())
                        .control(),
                ])
                .control(),
            t.make(ContentPresenter::new())
                .name("ForegroundContentPresenter")?
                .set(InputElement::is_hit_test_visible_property(), false)
                .content(
                    ContentPresenter::content_property(),
                    t.make(StackPanel::new())
                        .set(StackPanel::orientation_property(), Orientation::Horizontal)
                        .set(
                            Layoutable::margin_property(),
                            Thickness::new(-40.0, -40.0, -40.0, -40.0),
                        )
                        .children([t
                            .make(StackPanel::new())
                            .name("RatingForegroundStackPanel")?
                            .set(StackPanel::orientation_property(), Orientation::Horizontal)
                            .set(InputElement::is_hit_test_visible_property(), false)
                            .set(Layoutable::margin_property(), Thickness::new(40.0, 40.0, 40.0, 40.0))
                            .static_resource(StackPanel::spacing_property(), "RatingControlItemSpacing")?
                            .static_resource(TextElement::font_size_property(), "RatingControlFontSizeForRendering")?
                            .control()]),
                )?
                .control(),
        ])
        .control())
}

/// The default control theme of `RatingControl`.
fn rating_control_theme(theme: &StyleBuilder) -> Result {
    theme.target::<RatingControl>();
    theme.set(Layoutable::min_height_property(), 32.0);
    theme.dynamic(
        TemplatedControl::foreground_property(),
        "RatingControlCaptionForeground",
    );
    theme.dynamic(TemplatedControl::font_family_property(), "SymbolThemeFontFamily");
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(theme.scope(), rating_control_theme_focus_adorner)),
    );
    theme.template(rating_control_theme_template);
    // CommonStates
    theme.style(
        nesting()
            .class(":placeholder")
            .template()
            .of::<ContentPresenter>()
            .name("ForegroundContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "RatingControlPlaceholderForeground",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":pointeroverplaceholder")
            .template()
            .of::<ContentPresenter>()
            .name("ForegroundContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "RatingControlPointerOverPlaceholderForeground",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":pointeroverunselected")
            .template()
            .of::<ContentPresenter>()
            .name("ForegroundContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "RatingControlPointerOverUnselectedForeground",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":set")
            .template()
            .of::<ContentPresenter>()
            .name("ForegroundContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "RatingControlSelectedForeground",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":pointeroverset")
            .template()
            .of::<ContentPresenter>()
            .name("ForegroundContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "RatingControlSelectedForeground",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<ContentPresenter>()
            .name("ForegroundContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::foreground_property(),
                "RatingControlDisabledSelectedForeground",
            );
            Ok(())
        },
    )?;
    Ok(())
}
