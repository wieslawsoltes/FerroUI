//! `Controls/PipsPager.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/PipsPager/PipsPager.xaml (the implicit style of PipsPager)
//!   controls/dev/PipsPager/PipsPager_themeresources.xaml (PipsPagerNavigationButtonBaseStyle,
//!   PipsPagerPreviousPageButtonStyle, PipsPagerNextPageButtonStyle, PipsPagerButtonBaseStyle,
//!   PipsPagerSelectedPipButtonStyle, PipsPagerNormalPipButtonStyle; its resources are
//!   Resources/PipsPager_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - Template parts: PreviousPageButton is PART_PreviousButton, NextPageButton PART_NextButton.
//!   The pips: WinUI's ScrollViewer PipsPagerScrollViewer holds an ItemsRepeater of Buttons, to
//!   which the control applies SelectedPipStyle or NormalPipStyle; FerroUI's PipsPager shows its
//!   pips in the ListBox PART_PipsPagerList, whose selected item is the selected page. The list has
//!   the keyed theme PipsPagerListStyle (the scroll viewer, PART_ScrollViewer, with hidden bars
//!   around the items) and its items the theme PipsPagerPipItemStyle: the template and the states
//!   of PipsPagerButtonBaseStyle, with the values of PipsPagerSelectedPipButtonStyle in :selected
//!   and of PipsPagerNormalPipButtonStyle otherwise. The pip button styles themselves are ported as
//!   keyed Button themes for applications.
//! - PreviousButtonStyle and NextButtonStyle are FerroUI's PreviousButtonTheme and
//!   NextButtonTheme (the defaults are PipsPagerPreviousPageButtonStyle and
//!   PipsPagerNextPageButtonStyle); SelectedPipStyle and NormalPipStyle have no counterpart.
//! - The glyphs: the templates show their Content (a glyph of the symbol font) in a FontIcon at the
//!   FontSize of the button. They are Paths named Content, sized by FontSize, whose geometry the
//!   keyed themes set: EDDB (PipsPagerPreviousPageButtonGlyph) and EDDC
//!   (PipsPagerNextPageButtonGlyph) for the navigation buttons, EA3B (PipsPagerNormalGlyph,
//!   PipsPagerSelectedGlyph) for the pips; the Content setters of the glyph strings are left out.
//!   PipsPagerNormalGlyphFontSize (4) and PipsPagerSelectedGlyphFontSize (6) are x:String
//!   resources: the values are written.
//! - The Grids RootGrid draw a background, a border and a corner radius: Borders. The pressed
//!   navigation button scales its glyph to PipsPagerNavigationButtonScalePressed (0.875; an
//!   animation with a frame at 16 ms repeated forever): a RenderTransform of the Border named
//!   ScaleTransform after its transform.
//! - The visual states of the pager: PreviousPageButtonHidden and NextPageButtonHidden (the page at
//!   the edge, Opacity 0) are :first-page and :last-page; PreviousPageButtonCollapsed and
//!   NextPageButtonCollapsed are IsPreviousButtonVisible and IsNextButtonVisible (FerroUI has no
//!   VisibleOnPointerHover); the Enabled states are the IsEnabled the control sets on the buttons;
//!   HorizontalOrientationView is :horizontal (the navigation buttons turned by -90 degrees, the
//!   pips side by side); the OrientationStates of the pips (VerticalOrientation, 24 x 12) are the
//!   size of the items in :vertical. The tooltips of the navigation buttons (the automation name,
//!   placed by the orientation) are left out.
//! - UseSystemFocusVisuals with FocusVisualMargin="0" is the FocusAdorner.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::Orientation;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::transformation::TransformOperations;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::media::Stretch;
use ferroui_base::styling::Selectors;
use ferroui_base::RelativePoint;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::ItemsControl;
use ferroui_controls::ListBox;
use ferroui_controls::ListBoxItem;
use ferroui_controls::Panel;
use ferroui_controls::PipsPager;
use ferroui_controls::PipsPagerTemplateSettings;
use ferroui_controls::ScrollViewer;
use ferroui_controls::StackPanel;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::items_panel;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // WinUI applies the default style of Button beneath these keyed styles
    // (docs/porting/winui-theme.md, section 4.2).
    resources.control_theme(
        "PipsPagerNavigationButtonBaseStyle",
        pips_pager_navigation_button_base_style,
    );
    // Content: PipsPagerPreviousPageButtonGlyph (EDDB).
    resources.control_theme(
        "PipsPagerPreviousPageButtonStyle",
        pips_pager_previous_page_button_style,
    );
    // Content: PipsPagerNextPageButtonGlyph (EDDC).
    resources.control_theme("PipsPagerNextPageButtonStyle", pips_pager_next_page_button_style);
    resources.control_theme("PipsPagerButtonBaseStyle", pips_pager_button_base_style);
    resources.control_theme("PipsPagerSelectedPipButtonStyle", pips_pager_selected_pip_button_style);
    resources.control_theme("PipsPagerNormalPipButtonStyle", pips_pager_normal_pip_button_style);
    // The pips of the pager: PipsPagerButtonBaseStyle with PipsPagerSelectedPipButtonStyle in
    // :selected and PipsPagerNormalPipButtonStyle otherwise; the size of RootGrid
    // (PipsPagerHorizontalOrientationButtonWidth/Height) is the size of the item, which the pager
    // turns in :vertical.
    resources.control_theme("PipsPagerPipItemStyle", pips_pager_pip_item_style);
    // PipsPagerScrollViewer and its ItemsRepeater: the list of the pips.
    resources.control_theme("PipsPagerListStyle", pips_pager_list_style);
    resources.control_theme(type_key::<PipsPager>(), pips_pager_theme);
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

/// The focus adorner of `pips_pager_navigation_button_base_style`.
fn pips_pager_navigation_button_base_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::uniform(0.0))
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

/// The template of `Button` of `pips_pager_navigation_button_base_style`.
fn pips_pager_navigation_button_base_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("RootGrid")?
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
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .name("ScaleTransform")?
                .set(
                    Visual::render_transform_origin_property(),
                    literal(RelativePoint::parse("50%,50%"))?,
                )
                // The FontIcon of the glyph in Content, at FontSize.
                .child(
                    Decorator::child_property(),
                    t.make(Path::new())
                        .name("Content")?
                        .template_bind(Shape::fill_property(), TemplatedControl::foreground_property())
                        .set(Shape::stretch_property(), Stretch::Uniform)
                        .template_bind(Layoutable::width_property(), TemplatedControl::font_size_property())
                        .template_bind(
                            Layoutable::horizontal_alignment_property(),
                            ContentControl::horizontal_content_alignment_property(),
                        )
                        .template_bind(
                            Layoutable::vertical_alignment_property(),
                            ContentControl::vertical_content_alignment_property(),
                        ),
                ),
        )
        .control())
}

/// `PipsPagerNavigationButtonBaseStyle`: a control theme of `Button`.
fn pips_pager_navigation_button_base_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("DefaultButtonStyle")?;
    theme.dynamic(
        TemplatedControl::background_property(),
        "PipsPagerNavigationButtonBackground",
    );
    theme.dynamic(
        TemplatedControl::foreground_property(),
        "PipsPagerNavigationButtonForeground",
    );
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "PipsPagerNavigationButtonBorderBrush",
    );
    theme.static_resource(
        TemplatedControl::border_thickness_property(),
        "PipsPagerNavigationButtonBorderThickness",
    )?;
    theme.static_resource(
        TemplatedControl::font_size_property(),
        "PipsPagerNavigationButtonFontSize",
    )?;
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(
            theme.scope(),
            pips_pager_navigation_button_base_style_focus_adorner,
        )),
    );
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center);
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.static_resource(Layoutable::width_property(), "PipsPagerNavigationButtonWidth")?;
    theme.static_resource(Layoutable::height_property(), "PipsPagerNavigationButtonHeight")?;
    theme.static_resource(TemplatedControl::corner_radius_property(), "ControlCornerRadius")?;
    theme.template(pips_pager_navigation_button_base_style_template);
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(
                Border::background_property(),
                "PipsPagerNavigationButtonBackgroundPointerOver",
            );
            style.dynamic(
                Border::border_brush_property(),
                "PipsPagerNavigationButtonBorderBrushPointerOver",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("Content"), |style| {
            style.dynamic(Shape::fill_property(), "PipsPagerNavigationButtonForegroundPointerOver");
            Ok(())
        })?;
        Ok(())
    })?;
    // Pressed (PipsPagerNavigationButtonScalePressed, 0.875)
    theme.style(nesting().class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(
                Border::background_property(),
                "PipsPagerNavigationButtonBackgroundPressed",
            );
            style.dynamic(
                Border::border_brush_property(),
                "PipsPagerNavigationButtonBorderBrushPressed",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("Content"), |style| {
            style.dynamic(Shape::fill_property(), "PipsPagerNavigationButtonForegroundPressed");
            Ok(())
        })?;
        style.style(nesting().template().of::<Border>().name("ScaleTransform"), |style| {
            style.set(
                Visual::render_transform_property(),
                Some(literal(TransformOperations::parse("scale(0.875)"))?),
            );
            Ok(())
        })?;
        Ok(())
    })?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(
                Border::background_property(),
                "PipsPagerNavigationButtonBackgroundDisabled",
            );
            style.dynamic(
                Border::border_brush_property(),
                "PipsPagerNavigationButtonBorderBrushDisabled",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("Content"), |style| {
            style.dynamic(Shape::fill_property(), "PipsPagerNavigationButtonForegroundDisabled");
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}

/// `PipsPagerPreviousPageButtonStyle`: a control theme of `Button`.
fn pips_pager_previous_page_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("PipsPagerNavigationButtonBaseStyle")?;
    theme.style(nesting().template().of::<Path>().name("Content"), |style| {
        style.static_resource(Path::data_property(), "GlyphGeometryEDDB")?;
        Ok(())
    })?;
    Ok(())
}

/// `PipsPagerNextPageButtonStyle`: a control theme of `Button`.
fn pips_pager_next_page_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("PipsPagerNavigationButtonBaseStyle")?;
    theme.style(nesting().template().of::<Path>().name("Content"), |style| {
        style.static_resource(Path::data_property(), "GlyphGeometryEDDC")?;
        Ok(())
    })?;
    Ok(())
}

/// The focus adorner of `pips_pager_button_base_style`.
fn pips_pager_button_base_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::uniform(0.0))
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

/// The template of `Button` of `pips_pager_button_base_style`.
fn pips_pager_button_base_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("RootGrid")?
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
        .static_resource(
            Layoutable::width_property(),
            "PipsPagerHorizontalOrientationButtonWidth",
        )?
        .static_resource(
            Layoutable::height_property(),
            "PipsPagerHorizontalOrientationButtonHeight",
        )?
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        // The FontIcon of the glyph EA3B at FontSize.
        .child(
            Decorator::child_property(),
            t.make(Path::new())
                .name("Content")?
                .static_resource(Path::data_property(), "GlyphGeometryEA3B")?
                .template_bind(Shape::fill_property(), TemplatedControl::foreground_property())
                .set(Shape::stretch_property(), Stretch::Uniform)
                .template_bind(Layoutable::width_property(), TemplatedControl::font_size_property())
                .template_bind(Layoutable::height_property(), TemplatedControl::font_size_property())
                .template_bind(
                    Layoutable::horizontal_alignment_property(),
                    ContentControl::horizontal_content_alignment_property(),
                )
                .template_bind(
                    Layoutable::vertical_alignment_property(),
                    ContentControl::vertical_content_alignment_property(),
                ),
        )
        .control())
}

/// `PipsPagerButtonBaseStyle`: a control theme of `Button`.
fn pips_pager_button_base_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("DefaultButtonStyle")?;
    theme.dynamic(
        TemplatedControl::background_property(),
        "PipsPagerSelectionIndicatorBackground",
    );
    theme.dynamic(
        TemplatedControl::foreground_property(),
        "PipsPagerSelectionIndicatorForeground",
    );
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "PipsPagerSelectionIndicatorBorderBrush",
    );
    theme.static_resource(
        TemplatedControl::border_thickness_property(),
        "PipsPagerButtonBorderThickness",
    )?;
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(theme.scope(), pips_pager_button_base_style_focus_adorner)),
    );
    // PipsPagerNormalGlyphFontSize
    theme.set(TemplatedControl::font_size_property(), 4.0);
    theme.static_resource(TemplatedControl::corner_radius_property(), "ControlCornerRadius")?;
    theme.template(pips_pager_button_base_style_template);
    // PointerOver (the glyph at PipsPagerSelectedGlyphFontSize)
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(
                Border::background_property(),
                "PipsPagerSelectionIndicatorBackgroundPointerOver",
            );
            style.dynamic(
                Border::border_brush_property(),
                "PipsPagerSelectionIndicatorBorderBrushPointerOver",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("Content"), |style| {
            style.dynamic(
                Shape::fill_property(),
                "PipsPagerSelectionIndicatorForegroundPointerOver",
            );
            style.set(Layoutable::width_property(), 6.0);
            style.set(Layoutable::height_property(), 6.0);
            Ok(())
        })?;
        Ok(())
    })?;
    // Pressed (the glyph at PipsPagerNormalGlyphFontSize)
    theme.style(nesting().class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(
                Border::background_property(),
                "PipsPagerSelectionIndicatorBackgroundPressed",
            );
            style.dynamic(
                Border::border_brush_property(),
                "PipsPagerSelectionIndicatorBorderBrushPressed",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("Content"), |style| {
            style.dynamic(Shape::fill_property(), "PipsPagerSelectionIndicatorForegroundPressed");
            style.set(Layoutable::width_property(), 4.0);
            style.set(Layoutable::height_property(), 4.0);
            Ok(())
        })?;
        Ok(())
    })?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(
                Border::background_property(),
                "PipsPagerSelectionIndicatorBackgroundDisabled",
            );
            style.dynamic(
                Border::border_brush_property(),
                "PipsPagerSelectionIndicatorBorderBrushDisabled",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("Content"), |style| {
            style.dynamic(Shape::fill_property(), "PipsPagerSelectionIndicatorForegroundDisabled");
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}

/// `PipsPagerSelectedPipButtonStyle`: a control theme of `Button`.
fn pips_pager_selected_pip_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("PipsPagerButtonBaseStyle")?;
    // PipsPagerSelectedGlyphFontSize
    theme.set(TemplatedControl::font_size_property(), 6.0);
    theme.dynamic(
        TemplatedControl::background_property(),
        "PipsPagerSelectionIndicatorBackgroundSelected",
    );
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "PipsPagerSelectionIndicatorBorderBrushSelected",
    );
    theme.dynamic(
        TemplatedControl::foreground_property(),
        "PipsPagerSelectionIndicatorForegroundSelected",
    );
    Ok(())
}

/// `PipsPagerNormalPipButtonStyle`: a control theme of `Button`.
fn pips_pager_normal_pip_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.based_on("PipsPagerButtonBaseStyle")?;
    // PipsPagerNormalGlyphFontSize
    theme.set(TemplatedControl::font_size_property(), 4.0);
    Ok(())
}

/// The focus adorner of `pips_pager_pip_item_style`.
fn pips_pager_pip_item_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::uniform(0.0))
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

/// The template of `ListBoxItem` of `pips_pager_pip_item_style`.
fn pips_pager_pip_item_style_template(t: &Template<ListBoxItem>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("RootGrid")?
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
        .child(
            Decorator::child_property(),
            t.make(Path::new())
                .name("Content")?
                .static_resource(Path::data_property(), "GlyphGeometryEA3B")?
                .template_bind(Shape::fill_property(), TemplatedControl::foreground_property())
                .set(Shape::stretch_property(), Stretch::Uniform)
                .template_bind(Layoutable::width_property(), TemplatedControl::font_size_property())
                .template_bind(Layoutable::height_property(), TemplatedControl::font_size_property())
                .template_bind(
                    Layoutable::horizontal_alignment_property(),
                    ContentControl::horizontal_content_alignment_property(),
                )
                .template_bind(
                    Layoutable::vertical_alignment_property(),
                    ContentControl::vertical_content_alignment_property(),
                ),
        )
        .control())
}

/// `PipsPagerPipItemStyle`: a control theme of `ListBoxItem`.
fn pips_pager_pip_item_style(theme: &StyleBuilder) -> Result {
    theme.target::<ListBoxItem>();
    theme.dynamic(
        TemplatedControl::background_property(),
        "PipsPagerSelectionIndicatorBackground",
    );
    theme.dynamic(
        TemplatedControl::foreground_property(),
        "PipsPagerSelectionIndicatorForeground",
    );
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "PipsPagerSelectionIndicatorBorderBrush",
    );
    theme.static_resource(
        TemplatedControl::border_thickness_property(),
        "PipsPagerButtonBorderThickness",
    )?;
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(theme.scope(), pips_pager_pip_item_style_focus_adorner)),
    );
    // PipsPagerNormalGlyphFontSize
    theme.set(TemplatedControl::font_size_property(), 4.0);
    theme.static_resource(TemplatedControl::corner_radius_property(), "ControlCornerRadius")?;
    theme.static_resource(
        Layoutable::width_property(),
        "PipsPagerHorizontalOrientationButtonWidth",
    )?;
    theme.static_resource(
        Layoutable::height_property(),
        "PipsPagerHorizontalOrientationButtonHeight",
    )?;
    theme.set(Layoutable::min_width_property(), 0.0);
    theme.set(Layoutable::min_height_property(), 0.0);
    theme.set(TemplatedControl::padding_property(), Thickness::uniform(0.0));
    theme.template(pips_pager_pip_item_style_template);
    // PipsPagerSelectedPipButtonStyle
    theme.style(nesting().class(":selected"), |style| {
        style.set(TemplatedControl::font_size_property(), 6.0);
        style.dynamic(
            TemplatedControl::background_property(),
            "PipsPagerSelectionIndicatorBackgroundSelected",
        );
        style.dynamic(
            TemplatedControl::border_brush_property(),
            "PipsPagerSelectionIndicatorBorderBrushSelected",
        );
        style.dynamic(
            TemplatedControl::foreground_property(),
            "PipsPagerSelectionIndicatorForegroundSelected",
        );
        Ok(())
    })?;
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(
                Border::background_property(),
                "PipsPagerSelectionIndicatorBackgroundPointerOver",
            );
            style.dynamic(
                Border::border_brush_property(),
                "PipsPagerSelectionIndicatorBorderBrushPointerOver",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("Content"), |style| {
            style.dynamic(
                Shape::fill_property(),
                "PipsPagerSelectionIndicatorForegroundPointerOver",
            );
            style.set(Layoutable::width_property(), 6.0);
            style.set(Layoutable::height_property(), 6.0);
            Ok(())
        })?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(
                Border::background_property(),
                "PipsPagerSelectionIndicatorBackgroundPressed",
            );
            style.dynamic(
                Border::border_brush_property(),
                "PipsPagerSelectionIndicatorBorderBrushPressed",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("Content"), |style| {
            style.dynamic(Shape::fill_property(), "PipsPagerSelectionIndicatorForegroundPressed");
            style.set(Layoutable::width_property(), 4.0);
            style.set(Layoutable::height_property(), 4.0);
            Ok(())
        })?;
        Ok(())
    })?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(nesting().template().of::<Border>().name("RootGrid"), |style| {
            style.dynamic(
                Border::background_property(),
                "PipsPagerSelectionIndicatorBackgroundDisabled",
            );
            style.dynamic(
                Border::border_brush_property(),
                "PipsPagerSelectionIndicatorBorderBrushDisabled",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("Content"), |style| {
            style.dynamic(Shape::fill_property(), "PipsPagerSelectionIndicatorForegroundDisabled");
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}

/// The template of `ListBox` of `pips_pager_list_style`.
fn pips_pager_list_style_template(t: &Template<ListBox>) -> Result<Ref<Control>> {
    Ok(t.make(ScrollViewer::new())
        .name("PART_ScrollViewer")?
        .set(
            ScrollViewer::horizontal_scroll_bar_visibility_property(),
            ScrollBarVisibility::Hidden,
        )
        .set(
            ScrollViewer::vertical_scroll_bar_visibility_property(),
            ScrollBarVisibility::Hidden,
        )
        .set(ScrollViewer::is_scroll_chaining_enabled_property(), false)
        .content(
            ContentControl::content_property(),
            t.make(ItemsPresenter::new())
                .name("PART_ItemsPresenter")?
                .template_bind(
                    ItemsPresenter::items_panel_property(),
                    ItemsControl::items_panel_property(),
                ),
        )?
        .control())
}

/// `PipsPagerListStyle`: a control theme of `ListBox`.
fn pips_pager_list_style(theme: &StyleBuilder) -> Result {
    theme.target::<ListBox>();
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.set(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0));
    theme.set(TemplatedControl::padding_property(), Thickness::uniform(0.0));
    theme.template(pips_pager_list_style_template);
    Ok(())
}

/// The items panel of `template`.
fn template_items_panel(t: &Template<Control>) -> Result<Ref<Panel>> {
    // The StackLayout of the ItemsRepeater, in the orientation of the pager.
    Ok(t.make(StackPanel::new())
        .bind(
            StackPanel::orientation_property(),
            path()
                .ancestor_of::<PipsPager>(0)
                .prop(PipsPager::orientation_property())
                .binding(),
        )
        .end()
        .upcast())
}

/// The template of `PipsPager` of `pips_pager_theme`.
fn pips_pager_theme_template(t: &Template<PipsPager>) -> Result<Ref<Control>> {
    Ok(t.make(StackPanel::new())
        .name("RootPanel")?
        .template_bind(StackPanel::orientation_property(), PipsPager::orientation_property())
        .template_bind(Panel::background_property(), TemplatedControl::background_property())
        .children([
            t.make(Button::new())
                .name("PART_PreviousButton")?
                .template_bind(
                    StyledElement::theme_property(),
                    PipsPager::previous_button_theme_property(),
                )
                .template_bind(
                    Visual::is_visible_property(),
                    PipsPager::is_previous_button_visible_property(),
                )
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .set(
                    Visual::render_transform_origin_property(),
                    literal(RelativePoint::parse("50%,50%"))?,
                )
                .control(),
            t.make(ListBox::new())
                .name("PART_PipsPagerList")?
                .static_resource(StyledElement::theme_property(), "PipsPagerListStyle")?
                .static_resource(ItemsControl::item_container_theme_property(), "PipsPagerPipItemStyle")?
                .bind(
                    ItemsControl::items_source_property(),
                    path()
                        .templated_parent()
                        .prop(PipsPager::template_settings_property())
                        .prop(PipsPagerTemplateSettings::pips_property())
                        .binding(),
                )
                .bind(
                    SelectingItemsControl::selected_index_property(),
                    path()
                        .templated_parent()
                        .prop(PipsPager::selected_page_index_property())
                        .binding()
                        .with_mode(BindingMode::TwoWay),
                )
                .set(SelectingItemsControl::auto_scroll_to_selected_item_property(), false)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .set(
                    ItemsControl::items_panel_property(),
                    items_panel(t.scope(), template_items_panel),
                )
                .control(),
            t.make(Button::new())
                .name("PART_NextButton")?
                .template_bind(StyledElement::theme_property(), PipsPager::next_button_theme_property())
                .template_bind(
                    Visual::is_visible_property(),
                    PipsPager::is_next_button_visible_property(),
                )
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .set(
                    Visual::render_transform_origin_property(),
                    literal(RelativePoint::parse("50%,50%"))?,
                )
                .control(),
        ])
        .control())
}

/// The default control theme of `PipsPager`.
fn pips_pager_theme(theme: &StyleBuilder) -> Result {
    theme.target::<PipsPager>();
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.static_resource(
        PipsPager::previous_button_theme_property(),
        "PipsPagerPreviousPageButtonStyle",
    )?;
    theme.static_resource(PipsPager::next_button_theme_property(), "PipsPagerNextPageButtonStyle")?;
    theme.template(pips_pager_theme_template);
    // PreviousPageButtonHidden, NextPageButtonHidden
    theme.style(
        nesting()
            .class(":first-page")
            .template()
            .of::<Button>()
            .name("PART_PreviousButton"),
        |style| {
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":last-page")
            .template()
            .of::<Button>()
            .name("PART_NextButton"),
        |style| {
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        },
    )?;
    // HorizontalOrientationView
    theme.style(nesting().class(":horizontal"), |style| {
        style.style(nesting().template().of::<StackPanel>().name("RootPanel"), |style| {
            style.set(StackPanel::orientation_property(), Orientation::Horizontal);
            Ok(())
        })?;
        style.style(
            Selectors::or([
                nesting().template().of::<Button>().name("PART_PreviousButton"),
                nesting().template().of::<Button>().name("PART_NextButton"),
            ]),
            |style| {
                style.set(
                    Visual::render_transform_property(),
                    Some(literal(TransformOperations::parse("rotate(-90deg)"))?),
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // VerticalOrientation of the pips
    theme.style(
        nesting()
            .class(":vertical")
            .template()
            .of::<ListBox>()
            .name("PART_PipsPagerList")
            .descendant()
            .of::<ListBoxItem>(),
        |style| {
            style.static_resource(Layoutable::width_property(), "PipsPagerVerticalOrientationButtonWidth")?;
            style.static_resource(
                Layoutable::height_property(),
                "PipsPagerVerticalOrientationButtonHeight",
            )?;
            Ok(())
        },
    )?;
    Ok(())
}
