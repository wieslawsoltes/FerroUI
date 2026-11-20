//! `Controls/DataValidationErrors.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a style: WinUI 3 has no input validation. The validation of WinUI 2's preview
//! (IInputValidationControl, the templates DefaultInputValidationErrorTemplate and
//! DefaultCompactErrorIconTemplate of dxaml/xcp/dxaml/themes/generic.xaml) was removed before
//! WinUI 3 was released, and those templates are inert and waived (microsoft-ui-xaml,
//! https://github.com/microsoft/microsoft-ui-xaml, commit
//! 7e4b0c2096be402f51cc8f7d203ebf67cff183d0). A WinUI 3 application shows an error of a field
//! with the critical fill of the palette, SystemFillColorCriticalBrush
//! (controls/dev/CommonStyles/Common_themeresources_any.xaml): the border of the field in it and
//! the message under the field, 4 px below, in it.
//!
//! The default theme is that: the errors of the owner under its content, in
//! SystemFillColorCriticalBrush, wrapped; the controls that wrap their template in
//! DataValidationErrors (TextBox) draw their border in SystemFillColorCriticalBrush under
//! FerroUI's :error. The keyed theme TooltipDataValidationErrors (FerroUI) shows the errors in a
//! tool tip of an error icon at the right of the content: the error icon of WinUI's InfoBar
//! (controls/dev/InfoBar/InfoBar.xaml, InfoBar_themeresources.xaml), the glyph
//! InfoBarIconBackgroundGlyph (F136) in InfoBarErrorSeverityIconBackground under the glyph
//! InfoBarErrorIconGlyph (F13D) in InfoBarErrorSeverityIconForeground, at InfoBarIconFontSize
//! (16), drawn as the geometries GlyphGeometryF136 and GlyphGeometryF13D.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::media::Stretch;
use ferroui_base::media::TextWrapping;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::DataValidationErrors;
use ferroui_controls::Dock;
use ferroui_controls::DockPanel;
use ferroui_controls::ItemsControl;
use ferroui_controls::Panel;
use ferroui_controls::TextBlock;
use ferroui_controls::ToolTip;
use ferroui_markup_xaml::rust_theme::data_template;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<DataValidationErrors>(), data_validation_errors_theme);
    resources.control_theme("TooltipDataValidationErrors", tooltip_data_validation_errors);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `DataValidationErrors` of `data_validation_errors_theme`.
fn data_validation_errors_theme_template(t: &Template<DataValidationErrors>) -> Result<Ref<Control>> {
    Ok(t.make(DockPanel::new())
        .set(DockPanel::last_child_fill_property(), true)
        .children([
            t.make(ContentControl::new())
                .name("InlineDataValidationContentControl")?
                .set(Layoutable::margin_property(), Thickness::new(0.0, 4.0, 0.0, 0.0))
                .bind(
                    ContentControl::content_property(),
                    path().prop(DataValidationErrors::errors_property()).binding(),
                )
                .template_bind(
                    ContentControl::content_template_property(),
                    DataValidationErrors::error_template_property(),
                )
                .template_bind(
                    StyledElement::data_context_property(),
                    DataValidationErrors::owner_property(),
                )
                .set(DockPanel::dock_property(), Dock::Bottom)
                .bind(
                    Visual::is_visible_property(),
                    path().prop(DataValidationErrors::has_errors_property()).binding(),
                )
                .control(),
            t.make(ContentPresenter::new())
                .name("PART_ContentPresenter")?
                .template_bind(
                    ContentPresenter::padding_property(),
                    TemplatedControl::padding_property(),
                )
                .template_bind(
                    ContentPresenter::background_property(),
                    TemplatedControl::background_property(),
                )
                .template_bind(
                    ContentPresenter::border_brush_property(),
                    TemplatedControl::border_brush_property(),
                )
                .set(
                    ContentPresenter::background_sizing_property(),
                    BackgroundSizing::InnerBorderEdge,
                )
                .template_bind(
                    ContentPresenter::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                .template_bind(
                    ContentPresenter::content_template_property(),
                    ContentControl::content_template_property(),
                )
                .template_bind(
                    ContentPresenter::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .control(),
        ])
        .control())
}

/// A data template of `data_validation_errors_theme`.
fn data_validation_errors_theme_data_template(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(ItemsControl::new())
        .dynamic(TemplatedControl::foreground_property(), "SystemFillColorCriticalBrush")
        .bind(ItemsControl::items_source_property(), path().binding())
        .style(of::<TextBlock>(), |style| {
            style.set(TextBlock::text_wrapping_property(), TextWrapping::Wrap);
            Ok(())
        })?
        .control())
}

/// The default control theme of `DataValidationErrors`.
fn data_validation_errors_theme(theme: &StyleBuilder) -> Result {
    theme.target::<DataValidationErrors>();
    theme.template(data_validation_errors_theme_template);
    theme.set(
        DataValidationErrors::error_template_property(),
        Some(data_template(theme.scope(), data_validation_errors_theme_data_template)),
    );
    Ok(())
}

/// The template of `DataValidationErrors` of `tooltip_data_validation_errors`.
fn tooltip_data_validation_errors_template(t: &Template<DataValidationErrors>) -> Result<Ref<Control>> {
    Ok(t.make(DockPanel::new())
        .set(DockPanel::last_child_fill_property(), true)
        .children([
            t.make(ContentControl::new())
                .name("InlineDataValidationContentControl")?
                .bind(
                    ContentControl::content_property(),
                    path().prop(DataValidationErrors::errors_property()).binding(),
                )
                .template_bind(
                    ContentControl::content_template_property(),
                    DataValidationErrors::error_template_property(),
                )
                .template_bind(
                    StyledElement::data_context_property(),
                    DataValidationErrors::owner_property(),
                )
                .set(DockPanel::dock_property(), Dock::Right)
                .bind(
                    Visual::is_visible_property(),
                    path().prop(DataValidationErrors::has_errors_property()).binding(),
                )
                .control(),
            t.make(ContentPresenter::new())
                .name("PART_ContentPresenter")?
                .template_bind(
                    ContentPresenter::padding_property(),
                    TemplatedControl::padding_property(),
                )
                .template_bind(
                    ContentPresenter::background_property(),
                    TemplatedControl::background_property(),
                )
                .template_bind(
                    ContentPresenter::border_brush_property(),
                    TemplatedControl::border_brush_property(),
                )
                .set(
                    ContentPresenter::background_sizing_property(),
                    BackgroundSizing::InnerBorderEdge,
                )
                .template_bind(
                    ContentPresenter::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                .template_bind(
                    ContentPresenter::content_template_property(),
                    ContentControl::content_template_property(),
                )
                .template_bind(
                    ContentPresenter::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .control(),
        ])
        .control())
}

/// A data template of `tooltip_data_validation_errors`.
fn tooltip_data_validation_errors_data_template(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .name("InlineErrorIcon")?
        .set(Layoutable::width_property(), 16.0)
        .set(Layoutable::height_property(), 16.0)
        .set(Layoutable::margin_property(), Thickness::symmetric(8.0, 0.0))
        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
        .set(
            Panel::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        )
        .content(
            ToolTip::tip_property(),
            t.make(ItemsControl::new())
                .bind(ItemsControl::items_source_property(), path().binding())
                .style(of::<TextBlock>(), |style| {
                    style.set(TextBlock::text_wrapping_property(), TextWrapping::Wrap);
                    Ok(())
                })?,
        )?
        // IconBackground: InfoBarIconBackgroundGlyph (F136) at InfoBarIconFontSize.
        // StandardIcon: InfoBarErrorIconGlyph (F13D) at InfoBarIconFontSize.
        .children([
            t.make(Path::new())
                .set(Layoutable::width_property(), 16.0)
                .set(Layoutable::height_property(), 16.0)
                .set(Shape::stretch_property(), Stretch::Uniform)
                .static_resource(Path::data_property(), "GlyphGeometryF136")?
                .dynamic(Shape::fill_property(), "InfoBarErrorSeverityIconBackground")
                .control(),
            t.make(Path::new())
                .set(Layoutable::width_property(), 6.0)
                .set(Layoutable::height_property(), 6.0)
                .set(Shape::stretch_property(), Stretch::Uniform)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .static_resource(Path::data_property(), "GlyphGeometryF13D")?
                .dynamic(Shape::fill_property(), "InfoBarErrorSeverityIconForeground")
                .control(),
        ])
        .control())
}

/// `TooltipDataValidationErrors`: a control theme of `DataValidationErrors`.
fn tooltip_data_validation_errors(theme: &StyleBuilder) -> Result {
    theme.target::<DataValidationErrors>();
    theme.template(tooltip_data_validation_errors_template);
    theme.set(
        DataValidationErrors::error_template_property(),
        Some(data_template(
            theme.scope(),
            tooltip_data_validation_errors_data_template,
        )),
    );
    Ok(())
}
