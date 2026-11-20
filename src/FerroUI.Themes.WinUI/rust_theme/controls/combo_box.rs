//! `Controls/ComboBox.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/ComboBox/ComboBox_themeresources.xaml (the styles DefaultComboBoxStyle and
//!   ComboBoxTextBoxStyle; the resources of the document are Resources/ComboBox_themeresources.xaml)
//!   dxaml/xcp/dxaml/lib/ComboBox_Partial.cpp (the choice of the visual states)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - Template parts: Popup is FerroUI's PART_Popup, the ItemsPresenter PART_ItemsPresenter, the
//!   TextBox EditableText PART_EditableTextBox (with the keyed theme ComboBoxTextBoxStyle, the
//!   TextBoxStyle of the source). The selection box item is shown by a ContentControl named
//!   ContentPresenter (FerroUI's SelectionBoxItem and SelectionBoxItemTemplate); the placeholder,
//!   which WinUI puts into that presenter while nothing is selected, is a TextBlock beside it, shown
//!   while there is no selection box item and the box is not editable.
//! - FerroUI's ComboBox has no Header and no Description: HeaderContentPresenter,
//!   DescriptionPresenter and their rows are left out. The template is wrapped in
//!   DataValidationErrors, which FerroUI's combo box requires for its validation errors (WinUI 3
//!   has none), and under :error the border is SystemFillColorCriticalBrush, as for TextBox.
//! - The placeholder foreground: WinUI binds it to PlaceholderForeground with the theme resource of
//!   the state as TargetNullValue; here the states set PlaceholderForeground of the control (a value
//!   the application sets wins), as for TextBox. The editable text box takes PlaceholderForeground
//!   as its foreground, as the source binds it.
//! - Focused and FocusedPressed are the keyboard focus (:focus-visible) while the drop-down is
//!   closed (ComboBox::UpdateVisualState); PointerFocused and FocusedDropDown have no values.
//!   UseSystemFocusVisuals="{ThemeResource IsApplicationFocusVisualKindReveal}" is False: the
//!   focus visual is HighlightBackground and the pill, no focus adorner.
//! - EditableModeStates: the overlay of the drop-down button of an editable box shows on
//!   [IsEditable=true]; TextBoxFocused is the keyboard focus within the box (:focus-within), the
//!   overlay states the pointer over the overlay and the pressed box.
//! - The drop-down glyph is an AnimatedIcon (AnimatedChevronDownSmallVisualSource, 12 x 12) with the
//!   fallback glyph E70D at 12 px: the path of GlyphGeometryE96E 8 px wide in a box of 12 x 12, the
//!   chevron the animated visual draws (measured against native WinUI 3; G-10); its MinHeight
//!   ComboBoxMinHeight, which makes the box at least 32 px high, is the MinHeight of the Border
//!   Background. The pill is the keyed theme ComboBoxItemPill
//!   of a Border (Controls/ComboBoxItem.xaml).
//! - The drop-down opens below the box at its width (FerroUI's placement of PART_Popup); WinUI
//!   places it so that the selected item covers the box, animates it open (SplitOpenThemeAnimation
//!   of DropDownStates, OverlayOpeningAnimation) and squares the corners the box and the drop-down
//!   share (ComboBoxHelper.KeepInteriorCornersSquare). The ItemsPanel CarouselPanel (a looping
//!   panel of touch) is the default panel of FerroUI's combo box. The MinWidth of the scroll viewer
//!   (TemplateSettings.DropDownContentMinWidth) is the MinWidth of PopupBorder, the width of the
//!   box.
//! - The shadow of the drop-down (ApplyElevationEffect on the child of the popup,
//!   ComboBox_Partial.cpp: a ThemeShadow at a Translation.Z of 32) is a ThemeShadow (the shared
//!   WinUIThemeShadow, Translation.Z as in WinUI) on ThemeShadowWrapper, a border of its own around
//!   PopupBorder; the wrapper's margin is the room for the shadow in the popup
//!   (ThemeShadowFlyoutMargin, 16,8,16,24) added to the margin of the source (0,-0.5,0,-1), and the
//!   maximum height of the drop-down is the MaxHeight of PopupBorder, so that neither takes the
//!   room of the shadow (Resources/ThemeShadows.xaml; DEVIATIONS.md, G-24).
//! - ComboBoxTextBoxStyle: the template of the source with FerroUI's parts (as Controls/TextBox.xaml);
//!   the RequestedTheme="Light" of its Focused state, which draws the text of the focused box in the
//!   light theme, has no counterpart in a template.
//! - VerticalSnapPointsType="OptionalSingle" of the scroll viewer of the drop-down (snapping to an
//!   item only when the scrolling stops near one) has no counterpart: FerroUI's snap points are
//!   mandatory or none; the drop-down does not snap.
//! - The scroll, rail and zoom modes and TabNavigation are left out or are
//!   KeyboardNavigation.TabNavigation.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::converters::BoolConverters;
use ferroui_base::data::converters::ObjectConverters;
use ferroui_base::data::converters::StringConverters;
use ferroui_base::data::BindingMode;
use ferroui_base::data::MultiBinding;
use ferroui_base::input::Cursor;
use ferroui_base::input::InputElement;
use ferroui_base::input::KeyboardNavigation;
use ferroui_base::input::KeyboardNavigationMode;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::media::Stretch;
use ferroui_base::Rect;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Vector3D;
use ferroui_base::Visual;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::presenters::TextPresenter;
use ferroui_controls::primitives::Popup;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ComboBox;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::DataValidationErrors;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::ItemsControl;
use ferroui_controls::Panel;
use ferroui_controls::ScrollViewer;
use ferroui_controls::TextBlock;
use ferroui_controls::TextBox;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // ComboBoxTextBoxStyle: the TextBoxStyle of an editable combo box.
    resources.control_theme("ComboBoxTextBoxStyle", combo_box_text_box_style);
    resources.control_theme("DefaultComboBoxStyle", default_combo_box_style);
    resources.control_theme(type_key::<ComboBox>(), combo_box_theme);
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

/// The template of `TextBox` of `combo_box_text_box_style`.
fn combo_box_text_box_style_template(t: &Template<TextBox>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .children([
            t.make(Border::new())
                .name("BorderElement")?
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
                .set(TemplatedControl::is_template_focus_target_property(), true)
                .control(),
            t.make(ScrollViewer::new())
                .name("PART_ScrollViewer")?
                .template_bind(
                    ScrollViewer::horizontal_scroll_bar_visibility_property(),
                    ScrollViewer::horizontal_scroll_bar_visibility_property(),
                )
                .template_bind(
                    ScrollViewer::vertical_scroll_bar_visibility_property(),
                    ScrollViewer::vertical_scroll_bar_visibility_property(),
                )
                .template_bind(
                    ScrollViewer::is_deferred_scrolling_enabled_property(),
                    ScrollViewer::is_deferred_scrolling_enabled_property(),
                )
                .template_bind(
                    Layoutable::margin_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(
                    TemplatedControl::padding_property(),
                    TemplatedControl::padding_property(),
                )
                .template_bind(
                    TemplatedControl::foreground_property(),
                    TemplatedControl::foreground_property(),
                )
                .set(InputElement::focusable_property(), false)
                .style(of::<ScrollContentPresenter>().name("PART_ContentPresenter"), |style| {
                    style.set(InputElement::cursor_property(), Some(literal(Cursor::parse("IBeam"))?));
                    Ok(())
                })?
                .content(
                    ContentControl::content_property(),
                    t.make(TextPresenter::new())
                        .name("PART_TextPresenter")?
                        .template_bind_with(
                            TextPresenter::text_property(),
                            Some(TextBox::text_property().property()),
                            None,
                            Some(BindingMode::TwoWay),
                            None,
                        )
                        .template_bind(
                            TextPresenter::caret_blink_interval_property(),
                            TextBox::caret_blink_interval_property(),
                        )
                        .template_bind(TextPresenter::caret_index_property(), TextBox::caret_index_property())
                        .template_bind(
                            TextPresenter::selection_start_property(),
                            TextBox::selection_start_property(),
                        )
                        .template_bind(
                            TextPresenter::selection_end_property(),
                            TextBox::selection_end_property(),
                        )
                        .template_bind(
                            TextPresenter::text_alignment_property(),
                            TextBox::text_alignment_property(),
                        )
                        .template_bind(
                            TextPresenter::text_wrapping_property(),
                            TextBox::text_wrapping_property(),
                        )
                        .template_bind(TextPresenter::line_height_property(), TextBox::line_height_property())
                        .template_bind(
                            TextPresenter::letter_spacing_property(),
                            TemplatedControl::letter_spacing_property(),
                        )
                        .template_bind(
                            TextPresenter::password_char_property(),
                            TextBox::password_char_property(),
                        )
                        .template_bind(
                            TextPresenter::reveal_password_property(),
                            TextBox::reveal_password_property(),
                        )
                        .template_bind(
                            TextPresenter::selection_brush_property(),
                            TextBox::selection_brush_property(),
                        )
                        .template_bind(
                            TextPresenter::selection_foreground_brush_property(),
                            TextBox::selection_foreground_brush_property(),
                        )
                        .template_bind(TextPresenter::caret_brush_property(), TextBox::caret_brush_property())
                        .template_bind(
                            Layoutable::horizontal_alignment_property(),
                            TextBox::horizontal_content_alignment_property(),
                        )
                        .template_bind(
                            Layoutable::vertical_alignment_property(),
                            TextBox::vertical_content_alignment_property(),
                        ),
                )?
                .control(),
            t.make(TextBlock::new())
                .name("PlaceholderTextContentPresenter")?
                .template_bind(
                    TextBlock::foreground_property(),
                    TextBox::placeholder_foreground_property(),
                )
                .template_bind(
                    Layoutable::margin_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(TextBlock::padding_property(), TemplatedControl::padding_property())
                .template_bind(TextBlock::text_property(), TextBox::placeholder_text_property())
                .template_bind(TextBlock::text_alignment_property(), TextBox::text_alignment_property())
                .template_bind(TextBlock::text_wrapping_property(), TextBox::text_wrapping_property())
                .set(InputElement::is_hit_test_visible_property(), false)
                .bind(Visual::is_visible_property(), {
                    let binding = MultiBinding::new().with_converter_value(Some(BoolConverters::and()));
                    let bindings = binding.bindings();
                    bindings.add(
                        path()
                            .named(t.names(), "PART_TextPresenter")
                            .prop(TextPresenter::preedit_text_property())
                            .binding()
                            .with_converter_value(Some(StringConverters::is_null_or_empty())),
                    );
                    bindings.add(
                        path()
                            .templated_parent()
                            .prop(TextBox::text_property())
                            .binding()
                            .with_converter_value(Some(StringConverters::is_null_or_empty())),
                    );
                    binding
                })
                .control(),
        ])
        .control())
}

/// `ComboBoxTextBoxStyle`: a control theme of `TextBox`.
fn combo_box_text_box_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBox>();
    theme.based_on("DefaultTextBoxStyle")?;
    theme.template(combo_box_text_box_style_template);
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.dynamic(
            TextBox::placeholder_foreground_property(),
            "TextControlPlaceholderForegroundDisabled",
        );
        style.style(nesting().template().of::<Border>().name("BorderElement"), |style| {
            style.dynamic(Border::background_property(), "TextControlBackgroundDisabled");
            style.dynamic(Border::border_brush_property(), "TextControlBorderBrushDisabled");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<ScrollViewer>().name("PART_ScrollViewer"),
            |style| {
                style.dynamic(TemplatedControl::foreground_property(), "TextControlForegroundDisabled");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.dynamic(
            TextBox::placeholder_foreground_property(),
            "TextControlPlaceholderForegroundPointerOver",
        );
        style.style(nesting().template().of::<Border>().name("BorderElement"), |style| {
            style.dynamic(Border::background_property(), "TextControlBackgroundPointerOver");
            style.dynamic(Border::border_brush_property(), "TextControlBorderBrushPointerOver");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<ScrollViewer>().name("PART_ScrollViewer"),
            |style| {
                style.dynamic(
                    TemplatedControl::foreground_property(),
                    "TextControlForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Focused
    theme.style(nesting().class(":focus"), |style| {
        style.dynamic(
            TextBox::placeholder_foreground_property(),
            "TextControlPlaceholderForegroundFocused",
        );
        style.style(nesting().template().of::<Border>().name("BorderElement"), |style| {
            style.dynamic(Border::background_property(), "TextControlBackgroundFocused");
            style.dynamic(Border::border_brush_property(), "TextControlBorderBrushFocused");
            style.dynamic(
                Border::border_thickness_property(),
                "TextControlBorderThemeThicknessFocused",
            );
            Ok(())
        })?;
        style.style(
            nesting().template().of::<ScrollViewer>().name("PART_ScrollViewer"),
            |style| {
                style.dynamic(TemplatedControl::foreground_property(), "TextControlForegroundFocused");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}

/// The template of `ComboBox` of `default_combo_box_style`.
fn default_combo_box_style_template(t: &Template<ComboBox>) -> Result<Ref<Control>> {
    Ok(t.make(DataValidationErrors::new())
        .content(
            ContentControl::content_property(),
            t.make(Grid::new())
                .name("LayoutRoot")?
                .with(|e| {
                    Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("*,38"))?);
                    Ok(())
                })?
                // The AnimatedIcon DropDownGlyph (12 x 12, AnimatedChevronDownSmallVisualSource): the
                // chevron of its animated visual, which WinUI draws, is the ChevronDownSmall glyph
                // E96E 8 px wide (as the chevron of DropDownButton), not its fallback glyph E70D
                // at 12 px.
                .children([
                    t.make(Border::new())
                        .name("HighlightBackground")?
                        .set(Grid::column_property(), 0)
                        .set(Grid::column_span_property(), 2)
                        .set(Layoutable::margin_property(), Thickness::uniform(-4.0))
                        .dynamic(Border::background_property(), "ComboBoxBackgroundFocused")
                        .dynamic(Border::border_brush_property(), "ComboBoxBackgroundBorderBrushFocused")
                        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                        .static_resource(
                            Border::border_thickness_property(),
                            "ComboBoxBackgroundBorderThicknessFocused",
                        )?
                        .static_resource(Border::corner_radius_property(), "ComboBoxHiglightBorderCornerRadius")?
                        .set(Visual::opacity_property(), 0.0)
                        .control(),
                    t.make(Border::new())
                        .name("Background")?
                        .set(Grid::column_property(), 0)
                        .set(Grid::column_span_property(), 2)
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
                        .dynamic(Layoutable::min_width_property(), "ComboBoxThemeMinWidth")
                        .dynamic(Layoutable::min_height_property(), "ComboBoxMinHeight")
                        .control(),
                    t.make(Border::new())
                        .name("Pill")?
                        .set(Grid::column_property(), 0)
                        .static_resource(StyledElement::theme_property(), "ComboBoxItemPill")?
                        .set(Layoutable::margin_property(), Thickness::new(1.0, 0.0, 0.0, 0.0))
                        .set(Visual::opacity_property(), 0.0)
                        .control(),
                    t.make(ContentControl::new())
                        .name("ContentPresenter")?
                        .set(Grid::column_property(), 0)
                        .template_bind(
                            ContentControl::content_property(),
                            ComboBox::selection_box_item_property(),
                        )
                        .template_bind(
                            ContentControl::content_template_property(),
                            ComboBox::selection_box_item_template_property(),
                        )
                        .template_bind(
                            TemplatedControl::foreground_property(),
                            TemplatedControl::foreground_property(),
                        )
                        .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
                        .template_bind(
                            ContentControl::horizontal_content_alignment_property(),
                            ComboBox::horizontal_content_alignment_property(),
                        )
                        .template_bind(
                            ContentControl::vertical_content_alignment_property(),
                            ComboBox::vertical_content_alignment_property(),
                        )
                        .template_bind_with(
                            Visual::is_visible_property(),
                            Some(ComboBox::is_editable_property().property()),
                            Some(BoolConverters::not()),
                            None,
                            None,
                        )
                        .control(),
                    t.make(TextBlock::new())
                        .name("PlaceholderTextBlock")?
                        .set(Grid::column_property(), 0)
                        .template_bind(TextBlock::text_property(), ComboBox::placeholder_text_property())
                        .template_bind(
                            TextBlock::foreground_property(),
                            ComboBox::placeholder_foreground_property(),
                        )
                        .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
                        .template_bind(
                            Layoutable::horizontal_alignment_property(),
                            ComboBox::horizontal_content_alignment_property(),
                        )
                        .template_bind(
                            Layoutable::vertical_alignment_property(),
                            ComboBox::vertical_content_alignment_property(),
                        )
                        .bind(Visual::is_visible_property(), {
                            let binding = MultiBinding::new().with_converter_value(Some(BoolConverters::and()));
                            let bindings = binding.bindings();
                            bindings.add(
                                path()
                                    .templated_parent()
                                    .prop(ComboBox::selection_box_item_property())
                                    .binding()
                                    .with_converter_value(Some(ObjectConverters::is_null())),
                            );
                            bindings.add(
                                path()
                                    .templated_parent()
                                    .prop(ComboBox::is_editable_property())
                                    .not()
                                    .binding(),
                            );
                            binding
                        })
                        .control(),
                    t.make(TextBox::new())
                        .name("PART_EditableTextBox")?
                        .set(Grid::column_property(), 0)
                        .set(Grid::column_span_property(), 2)
                        .static_resource(StyledElement::theme_property(), "ComboBoxTextBoxStyle")?
                        .dynamic(TemplatedControl::padding_property(), "ComboBoxEditableTextPadding")
                        .set(
                            TemplatedControl::border_brush_property(),
                            Some(literal(Brush::parse("Transparent"))?),
                        )
                        .template_bind(
                            Layoutable::horizontal_alignment_property(),
                            ComboBox::horizontal_content_alignment_property(),
                        )
                        .template_bind(
                            Layoutable::vertical_alignment_property(),
                            ComboBox::vertical_content_alignment_property(),
                        )
                        .template_bind(
                            TextBox::placeholder_text_property(),
                            ComboBox::placeholder_text_property(),
                        )
                        .template_bind(
                            TemplatedControl::foreground_property(),
                            ComboBox::placeholder_foreground_property(),
                        )
                        .template_bind_with(
                            TextBox::text_property(),
                            Some(ComboBox::text_property().property()),
                            None,
                            Some(BindingMode::TwoWay),
                            None,
                        )
                        .template_bind(
                            TemplatedControl::corner_radius_property(),
                            TemplatedControl::corner_radius_property(),
                        )
                        .template_bind(Visual::is_visible_property(), ComboBox::is_editable_property())
                        .control(),
                    t.make(Border::new())
                        .name("DropDownOverlay")?
                        .set(Grid::column_property(), 1)
                        .set(
                            Border::background_property(),
                            Some(literal(Brush::parse("Transparent"))?),
                        )
                        .set(Layoutable::margin_property(), Thickness::new(4.0, 4.0, 4.0, 4.0))
                        .static_resource(
                            Border::corner_radius_property(),
                            "ComboBoxDropDownButtonBackgroundCornerRadius",
                        )?
                        .set(Layoutable::width_property(), 30.0)
                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                        .set(Visual::is_visible_property(), false)
                        .control(),
                    t.make(Panel::new())
                        .set(Grid::column_property(), 1)
                        .set(Layoutable::width_property(), 12.0)
                        .set(Layoutable::height_property(), 12.0)
                        .set(Layoutable::margin_property(), Thickness::new(0.0, 0.0, 14.0, 0.0))
                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                        .set(InputElement::is_hit_test_visible_property(), false)
                        .children([t
                            .make(Path::new())
                            .name("DropDownGlyph")?
                            .static_resource(Path::data_property(), "GlyphGeometryE96E")?
                            .dynamic(Shape::fill_property(), "ComboBoxDropDownGlyphForeground")
                            .set(Shape::stretch_property(), Stretch::Uniform)
                            .set(Layoutable::width_property(), 8.0)
                            .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                            .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                            .control()])
                        .control(),
                    t.make(Popup::new())
                        .name("PART_Popup")?
                        .set(Popup::window_manager_add_shadow_hint_property(), false)
                        .template_bind_with(
                            Popup::is_open_property(),
                            Some(ComboBox::is_drop_down_open_property().property()),
                            None,
                            Some(BindingMode::TwoWay),
                            None,
                        )
                        .set(Popup::placement_target_property(), t.element_ref("Background")?)
                        .set(Popup::is_light_dismiss_enabled_property(), true)
                        .set(Popup::inherits_transform_property(), true)
                        // The shadow on a border of its own (G-24). Margin: ThemeShadowFlyoutMargin
                        // (16,8,16,24) and the 0,-0.5,0,-1 of the source.
                        .child(
                            Popup::child_property(),
                            t.make(Border::new())
                                .name("ThemeShadowWrapper")?
                                .set(Visual::translation_property(), literal(Vector3D::parse("0,0,32"))?)
                                .static_resource(Visual::shadow_property(), "WinUIThemeShadow")?
                                .set(Layoutable::margin_property(), Thickness::new(16.0, 7.5, 16.0, 23.0))
                                .set(
                                    Layoutable::horizontal_alignment_property(),
                                    HorizontalAlignment::Stretch,
                                )
                                .dynamic(Border::corner_radius_property(), "OverlayCornerRadius")
                                .child(
                                    Decorator::child_property(),
                                    t.make(Border::new())
                                        .name("PopupBorder")?
                                        .bind(
                                            Layoutable::min_width_property(),
                                            path()
                                                .templated_parent()
                                                .prop(Visual::bounds_property())
                                                .member::<Rect>("Width")
                                                .binding(),
                                        )
                                        .template_bind(
                                            Layoutable::max_height_property(),
                                            ComboBox::max_drop_down_height_property(),
                                        )
                                        .dynamic(Border::background_property(), "ComboBoxDropDownBackground")
                                        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                                        .dynamic(Border::border_brush_property(), "ComboBoxDropDownBorderBrush")
                                        .dynamic(Border::border_thickness_property(), "ComboBoxDropdownBorderThickness")
                                        .dynamic(Decorator::padding_property(), "ComboBoxDropdownBorderPadding")
                                        .dynamic(Border::corner_radius_property(), "OverlayCornerRadius")
                                        .child(
                                            Decorator::child_property(),
                                            t.make(ScrollViewer::new())
                                                .name("ScrollViewer")?
                                                .dynamic(
                                                    TemplatedControl::foreground_property(),
                                                    "ComboBoxDropDownForeground",
                                                )
                                                .template_bind(
                                                    ScrollViewer::horizontal_scroll_bar_visibility_property(),
                                                    ScrollViewer::horizontal_scroll_bar_visibility_property(),
                                                )
                                                .template_bind(
                                                    ScrollViewer::vertical_scroll_bar_visibility_property(),
                                                    ScrollViewer::vertical_scroll_bar_visibility_property(),
                                                )
                                                .template_bind(
                                                    ScrollViewer::is_deferred_scrolling_enabled_property(),
                                                    ScrollViewer::is_deferred_scrolling_enabled_property(),
                                                )
                                                .template_bind(
                                                    ScrollViewer::bring_into_view_on_focus_change_property(),
                                                    ScrollViewer::bring_into_view_on_focus_change_property(),
                                                )
                                                .set(ScrollViewer::is_scroll_chaining_enabled_property(), false)
                                                .content(
                                                    ContentControl::content_property(),
                                                    t.make(ItemsPresenter::new())
                                                        .name("PART_ItemsPresenter")?
                                                        .dynamic(
                                                            Layoutable::margin_property(),
                                                            "ComboBoxDropdownContentMargin",
                                                        )
                                                        .template_bind(
                                                            ItemsPresenter::items_panel_property(),
                                                            ItemsControl::items_panel_property(),
                                                        ),
                                                )?,
                                        ),
                                ),
                        )
                        .control(),
                ]),
        )?
        .control())
}

/// `DefaultComboBoxStyle`: a control theme of `ComboBox`.
fn default_combo_box_style(theme: &StyleBuilder) -> Result {
    theme.target::<ComboBox>();
    // The defaults of a WinUI 3 control, which its style leaves unset and its template binds
    // (FerroUI's defaults are Stretch). Deviation (winui-theme/DEVIATIONS.md, G-27).
    theme.set(
        ComboBox::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(TemplatedControl::padding_property(), "ComboBoxPadding");
    theme.set(ComboBox::max_drop_down_height_property(), 504.0);
    theme.dynamic(TemplatedControl::foreground_property(), "ComboBoxForeground");
    theme.dynamic(TemplatedControl::background_property(), "ComboBoxBackground");
    theme.dynamic(TemplatedControl::border_brush_property(), "ComboBoxBorderBrush");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "ComboBoxBorderThemeThickness",
    );
    theme.dynamic(
        ComboBox::placeholder_foreground_property(),
        "ComboBoxPlaceHolderForeground",
    );
    theme.set(
        KeyboardNavigation::tab_navigation_property(),
        KeyboardNavigationMode::Once,
    );
    theme.set(
        ScrollViewer::horizontal_scroll_bar_visibility_property(),
        ScrollBarVisibility::Disabled,
    );
    theme.set(
        ScrollViewer::vertical_scroll_bar_visibility_property(),
        ScrollBarVisibility::Auto,
    );
    theme.set(ScrollViewer::is_deferred_scrolling_enabled_property(), false);
    theme.set(ScrollViewer::bring_into_view_on_focus_change_property(), true);
    theme.set(
        ComboBox::horizontal_content_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.set_null(Control::focus_adorner_property());
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(default_combo_box_style_template);
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.dynamic(
            ComboBox::placeholder_foreground_property(),
            "ComboBoxPlaceHolderForegroundPointerOver",
        );
        style.style(nesting().template().of::<Border>().name("Background"), |style| {
            style.dynamic(Border::background_property(), "ComboBoxBackgroundPointerOver");
            style.dynamic(Border::border_brush_property(), "ComboBoxBorderBrushPointerOver");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<ContentControl>().name("ContentPresenter"),
            |style| {
                style.dynamic(TemplatedControl::foreground_property(), "ComboBoxForegroundPointerOver");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.dynamic(
            ComboBox::placeholder_foreground_property(),
            "ComboBoxPlaceHolderForegroundPressed",
        );
        style.style(nesting().template().of::<Border>().name("Background"), |style| {
            style.dynamic(Border::background_property(), "ComboBoxBackgroundPressed");
            style.dynamic(Border::border_brush_property(), "ComboBoxBorderBrushPressed");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<ContentControl>().name("ContentPresenter"),
            |style| {
                style.dynamic(TemplatedControl::foreground_property(), "ComboBoxForegroundPressed");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.dynamic(
            ComboBox::placeholder_foreground_property(),
            "ComboBoxPlaceHolderForegroundDisabled",
        );
        style.style(nesting().template().of::<Border>().name("Background"), |style| {
            style.dynamic(Border::background_property(), "ComboBoxBackgroundDisabled");
            style.dynamic(Border::border_brush_property(), "ComboBoxBorderBrushDisabled");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<ContentControl>().name("ContentPresenter"),
            |style| {
                style.dynamic(TemplatedControl::foreground_property(), "ComboBoxForegroundDisabled");
                Ok(())
            },
        )?;
        style.style(nesting().template().of::<Path>().name("DropDownGlyph"), |style| {
            style.dynamic(Shape::fill_property(), "ComboBoxDropDownGlyphForegroundDisabled");
            Ok(())
        })?;
        Ok(())
    })?;
    // Focused (FocusStates)
    theme.style(
        nesting()
            .class(":focus-visible")
            .not(of::<ComboBox>().class(":dropdownopen")),
        |style| {
            style.dynamic(
                ComboBox::placeholder_foreground_property(),
                "ComboBoxPlaceHolderForegroundFocused",
            );
            style.style(
                nesting().template().of::<Border>().name("HighlightBackground"),
                |style| {
                    style.set(Visual::opacity_property(), 1.0);
                    Ok(())
                },
            )?;
            style.style(nesting().template().of::<Border>().name("Pill"), |style| {
                style.set(Visual::opacity_property(), 1.0);
                Ok(())
            })?;
            style.style(
                nesting().template().of::<ContentControl>().name("ContentPresenter"),
                |style| {
                    style.dynamic(TemplatedControl::foreground_property(), "ComboBoxForegroundFocused");
                    Ok(())
                },
            )?;
            style.style(nesting().template().of::<Path>().name("DropDownGlyph"), |style| {
                style.dynamic(Shape::fill_property(), "ComboBoxDropDownGlyphForegroundFocused");
                Ok(())
            })?;
            Ok(())
        },
    )?;
    // FocusedPressed
    theme.style(
        nesting()
            .class(":focus-visible")
            .class(":pressed")
            .not(of::<ComboBox>().class(":dropdownopen")),
        |style| {
            style.dynamic(
                ComboBox::placeholder_foreground_property(),
                "ComboBoxPlaceHolderForegroundFocusedPressed",
            );
            style.style(
                nesting().template().of::<ContentControl>().name("ContentPresenter"),
                |style| {
                    style.dynamic(
                        TemplatedControl::foreground_property(),
                        "ComboBoxForegroundFocusedPressed",
                    );
                    Ok(())
                },
            )?;
            style.style(nesting().template().of::<Path>().name("DropDownGlyph"), |style| {
                style.dynamic(Shape::fill_property(), "ComboBoxDropDownGlyphForegroundFocusedPressed");
                Ok(())
            })?;
            Ok(())
        },
    )?;
    // EditableModeStates
    theme.style(
        nesting()
            .property_is(ComboBox::is_editable_property(), true)
            .template()
            .of::<Border>()
            .name("DropDownOverlay"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    // TextBoxFocused
    theme.style(
        nesting()
            .property_is(ComboBox::is_editable_property(), true)
            .class(":focus-within")
            .template()
            .of::<Path>()
            .name("DropDownGlyph"),
        |style| {
            style.dynamic(Shape::fill_property(), "ComboBoxEditableDropDownGlyphForeground");
            Ok(())
        },
    )?;
    // TextBoxOverlayPointerOver, TextBoxFocusedOverlayPointerOver
    theme.style(
        nesting()
            .property_is(ComboBox::is_editable_property(), true)
            .template()
            .of::<Border>()
            .name("DropDownOverlay")
            .class(":pointerover"),
        |style| {
            style.dynamic(Border::background_property(), "ComboBoxDropDownBackgroundPointerOver");
            Ok(())
        },
    )?;
    // TextBoxOverlayPressed
    theme.style(
        nesting()
            .property_is(ComboBox::is_editable_property(), true)
            .class(":pressed")
            .template()
            .of::<Border>()
            .name("DropDownOverlay")
            .class(":pointerover"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "ComboBoxDropDownBackgroundPointerPressed",
            );
            Ok(())
        },
    )?;
    // TextBoxFocusedOverlayPressed
    theme.style(
        nesting()
            .property_is(ComboBox::is_editable_property(), true)
            .class(":focus-within")
            .class(":pressed")
            .template()
            .of::<Border>()
            .name("DropDownOverlay")
            .class(":pointerover"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "ComboBoxFocusedDropDownBackgroundPointerPressed",
            );
            Ok(())
        },
    )?;
    // A validation error (FerroUI's DataValidationErrors, which WinUI 3 does not have): the
    // border in the critical fill, as for TextBox.
    theme.style(
        nesting().class(":error").template().of::<Border>().name("Background"),
        |style| {
            style.dynamic(Border::border_brush_property(), "SystemFillColorCriticalBrush");
            Ok(())
        },
    )?;
    // An editable combo box is not a tab stop of its own: its text box is (as the other themes
    // of FerroUI).
    theme.style(nesting().property_is(ComboBox::is_editable_property(), true), |style| {
        style.set(InputElement::is_tab_stop_property(), false);
        style.set(
            KeyboardNavigation::tab_navigation_property(),
            KeyboardNavigationMode::Local,
        );
        Ok(())
    })?;
    Ok(())
}

/// The default control theme of `ComboBox`.
fn combo_box_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ComboBox>();
    theme.based_on("DefaultComboBoxStyle")?;
    Ok(())
}
