//! `Controls/AutoCompleteBox.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/AutoSuggestBox/AutoSuggestBox_themeresources.xaml (the styles
//!   DefaultAutoSuggestBoxStyle and AutoSuggestBoxTextBoxStyle; the resources of the document are
//!   Resources/AutoSuggestBox_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - FerroUI's AutoCompleteBox is WinUI's AutoSuggestBox. Template parts: the TextBox of the source
//!   is PART_TextBox, with the keyed theme AutoSuggestBoxTextBoxStyle (the TextBoxStyle of the
//!   source); SuggestionsPopup is PART_Popup; the ListView SuggestionsList is the list box
//!   PART_SelectingItemsControl, whose items have the keyed theme DefaultListViewItemStyle (the
//!   style of the items of a ListView, Controls/ListBoxItem.xaml) and which draws no background
//!   and no border of its own, as a ListView. PART_SelectionAdapter, an optional part of FerroUI's
//!   control, is not in the template (as in FerroUI's other themes).
//! - The text box takes the properties FerroUI's auto-complete box passes to it in its other
//!   themes: the placeholder text and foreground, the padding (TextControlThemePadding, the padding
//!   of the text box of the source), the caret index, the maximum length, the validation errors and
//!   the inner contents. The maximum height of the suggestion list, AutoSuggestListMaxHeight, is the
//!   MaxDropDownHeight of the control, the maximum height of SuggestionsContainer (whose MinWidth
//!   is the width of the control).
//! - The shadow of the suggestion list (ApplyElevationEffect on the popup, AutoSuggestBox_Partial.cpp:
//!   a ThemeShadow at a Translation.Z of 32) is a ThemeShadow (the shared WinUIThemeShadow, Translation.Z as in WinUI) on
//!   SuggestionsContainer, with the margin ThemeShadowFlyoutMargin as the room for it in the popup
//!   (Resources/ThemeShadows.xaml; DEVIATIONS.md, G-24).
//! - AutoSuggestBoxTextBoxStyle: the template of the source with FerroUI's parts (as
//!   Controls/TextBox.xaml, with its wrapper DataValidationErrors, its inner contents and its
//!   placeholder foreground); the template-local DeleteButtonStyle is the keyed theme
//!   AutoSuggestBoxDeleteButtonStyle; the delete button is shown by the selector of
//!   DefaultTextBoxStyle, the theme this one is based on. The QueryButton (the QueryIcon of
//!   AutoSuggestBox, with the template-local QueryButtonStyle) has no counterpart: FerroUI's
//!   auto-complete box has no query icon. The column of AutoSuggestBoxRightButtonMargin (4 px) at
//!   the right of the buttons is kept.
//! - Orientation (Landscape, Portrait), UseSystemFocusVisuals of IsApplicationFocusVisualKindReveal
//!   (no focus adorner), AutoSuggestBoxHelper.KeepInteriorCornersSquare, the header, description
//!   and DesiredCandidateWindowAlignment of the text box and the item container transitions of the
//!   list have no counterpart.

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
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::presenters::TextPresenter;
use ferroui_controls::primitives::Popup;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::AutoCompleteBox;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::DataValidationErrors;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::ItemsControl;
use ferroui_controls::ListBox;
use ferroui_controls::ScrollViewer;
use ferroui_controls::TextBlock;
use ferroui_controls::TextBox;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // DeleteButtonStyle of the template of AutoSuggestBoxTextBoxStyle.
    resources.control_theme("AutoSuggestBoxDeleteButtonStyle", auto_suggest_box_delete_button_style);
    // AutoSuggestBoxTextBoxStyle: the TextBoxStyle of an auto-complete box.
    resources.control_theme("AutoSuggestBoxTextBoxStyle", auto_suggest_box_text_box_style);
    resources.control_theme("DefaultAutoSuggestBoxStyle", default_auto_suggest_box_style);
    resources.control_theme(type_key::<AutoCompleteBox>(), auto_complete_box_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `Button` of `auto_suggest_box_delete_button_style`.
fn auto_suggest_box_delete_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("ButtonLayoutGrid")?
        .dynamic(Border::border_brush_property(), "TextControlButtonBorderBrush")
        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
        .template_bind(
            Border::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .dynamic(Border::background_property(), "TextControlButtonBackground")
        .static_resource(Layoutable::margin_property(), "AutoSuggestBoxDeleteButtonMargin")?
        .dynamic(Border::corner_radius_property(), "ControlCornerRadius")
        // E894 at AutoSuggestBoxIconFontSize (12 px): the glyph is drawn at the size of its
        //               ink, as the delete button of TextBox.
        .child(
            Decorator::child_property(),
            t.make(Path::new())
                .name("GlyphElement")?
                .dynamic(Shape::fill_property(), "TextControlButtonForeground")
                .static_resource(Path::data_property(), "GlyphGeometryE894")?
                .set(Shape::stretch_property(), Stretch::Uniform)
                .set(Layoutable::width_property(), 10.0)
                .set(Layoutable::height_property(), 10.0)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center),
        )
        .control())
}

/// `AutoSuggestBoxDeleteButtonStyle`: a control theme of `Button`.
fn auto_suggest_box_delete_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.template(auto_suggest_box_delete_button_style_template);
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("ButtonLayoutGrid"), |style| {
            style.dynamic(Border::background_property(), "TextControlButtonBackgroundPointerOver");
            style.dynamic(
                Border::border_brush_property(),
                "TextControlButtonBorderBrushPointerOver",
            );
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("GlyphElement"), |style| {
            style.dynamic(Shape::fill_property(), "TextControlButtonForegroundPointerOver");
            Ok(())
        })?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("ButtonLayoutGrid"), |style| {
            style.dynamic(Border::background_property(), "TextControlButtonBackgroundPressed");
            style.dynamic(Border::border_brush_property(), "TextControlButtonBorderBrushPressed");
            Ok(())
        })?;
        style.style(nesting().template().of::<Path>().name("GlyphElement"), |style| {
            style.dynamic(Shape::fill_property(), "TextControlButtonForegroundPressed");
            Ok(())
        })?;
        Ok(())
    })?;
    // Disabled
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<Border>()
            .name("ButtonLayoutGrid"),
        |style| {
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `TextBox` of `auto_suggest_box_text_box_style`.
fn auto_suggest_box_text_box_style_template(t: &Template<TextBox>) -> Result<Ref<Control>> {
    Ok(t.make(DataValidationErrors::new())
        .content(
            ContentControl::content_property(),
            t.make(Grid::new())
                .with(|e| {
                    Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("Auto,*,Auto,4,Auto"))?);
                    Ok(())
                })?
                // InnerLeftContent of FerroUI's text box.
                // InnerRightContent of FerroUI's text box.
                .children([
                    t.make(Border::new())
                        .name("BorderElement")?
                        .set(Grid::column_property(), 0)
                        .set(Grid::column_span_property(), 5)
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
                    t.make(ContentPresenter::new())
                        .name("InnerLeftContentPresenter")?
                        .template_bind_with(
                            Visual::is_visible_property(),
                            Some(TextBox::inner_left_content_property().property()),
                            Some(ObjectConverters::is_not_null()),
                            None,
                            None,
                        )
                        .set(Grid::column_property(), 0)
                        .template_bind(
                            Layoutable::margin_property(),
                            TemplatedControl::border_thickness_property(),
                        )
                        .template_bind(
                            ContentPresenter::content_property(),
                            TextBox::inner_left_content_property(),
                        )
                        .template_bind(
                            ContentPresenter::vertical_content_alignment_property(),
                            TextBox::vertical_content_alignment_property(),
                        )
                        .control(),
                    t.make(ScrollViewer::new())
                        .name("PART_ScrollViewer")?
                        .set(Grid::column_property(), 1)
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
                            Layoutable::vertical_alignment_property(),
                            TextBox::vertical_content_alignment_property(),
                        )
                        .template_bind(
                            Layoutable::horizontal_alignment_property(),
                            TextBox::horizontal_content_alignment_property(),
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
                                .template_bind(TextPresenter::caret_brush_property(), TextBox::caret_brush_property()),
                        )?
                        .control(),
                    t.make(TextBlock::new())
                        .name("PlaceholderTextContentPresenter")?
                        .set(Grid::column_property(), 1)
                        .set(Grid::column_span_property(), 2)
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
                        .template_bind(
                            Layoutable::vertical_alignment_property(),
                            TextBox::vertical_content_alignment_property(),
                        )
                        .template_bind(
                            Layoutable::horizontal_alignment_property(),
                            TextBox::horizontal_content_alignment_property(),
                        )
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
                    t.make(Button::new())
                        .name("DeleteButton")?
                        .set(Grid::column_property(), 2)
                        .static_resource(StyledElement::theme_property(), "AutoSuggestBoxDeleteButtonStyle")?
                        .bind(
                            Button::command_property(),
                            path()
                                .ancestor_of::<TextBox>(0)
                                .method::<Ref<TextBox>>("Clear", |this| {
                                    this.clear();
                                })
                                .binding(),
                        )
                        .template_bind(
                            TemplatedControl::corner_radius_property(),
                            TemplatedControl::corner_radius_property(),
                        )
                        .template_bind(
                            TemplatedControl::border_thickness_property(),
                            TemplatedControl::border_thickness_property(),
                        )
                        .dynamic(TemplatedControl::padding_property(), "HelperButtonThemePadding")
                        .set(InputElement::focusable_property(), false)
                        .set(Visual::is_visible_property(), false)
                        .template_bind(
                            TemplatedControl::font_size_property(),
                            TemplatedControl::font_size_property(),
                        )
                        .set(Layoutable::width_property(), 32.0)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                        .control(),
                    t.make(ContentPresenter::new())
                        .name("InnerRightContentPresenter")?
                        .template_bind_with(
                            Visual::is_visible_property(),
                            Some(TextBox::inner_right_content_property().property()),
                            Some(ObjectConverters::is_not_null()),
                            None,
                            None,
                        )
                        .set(Grid::column_property(), 4)
                        .template_bind(
                            Layoutable::margin_property(),
                            TemplatedControl::border_thickness_property(),
                        )
                        .template_bind(
                            ContentPresenter::content_property(),
                            TextBox::inner_right_content_property(),
                        )
                        .template_bind(
                            ContentPresenter::vertical_content_alignment_property(),
                            TextBox::vertical_content_alignment_property(),
                        )
                        .control(),
                ]),
        )?
        .control())
}

/// `AutoSuggestBoxTextBoxStyle`: a control theme of `TextBox`.
fn auto_suggest_box_text_box_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBox>();
    theme.based_on("DefaultTextBoxStyle")?;
    theme.dynamic(Layoutable::min_width_property(), "TextControlThemeMinWidth");
    theme.dynamic(Layoutable::min_height_property(), "TextControlThemeMinHeight");
    theme.dynamic(TemplatedControl::foreground_property(), "TextControlForeground");
    theme.dynamic(TemplatedControl::background_property(), "TextControlBackground");
    theme.dynamic(TemplatedControl::border_brush_property(), "TextControlBorderBrush");
    theme.dynamic(
        TextBox::selection_brush_property(),
        "TextControlSelectionHighlightColor",
    );
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "TextControlBorderThemeThickness",
    );
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.set(
        ScrollViewer::horizontal_scroll_bar_visibility_property(),
        ScrollBarVisibility::Hidden,
    );
    theme.set(
        ScrollViewer::vertical_scroll_bar_visibility_property(),
        ScrollBarVisibility::Hidden,
    );
    theme.set(ScrollViewer::is_deferred_scrolling_enabled_property(), false);
    theme.dynamic(TemplatedControl::padding_property(), "TextControlThemePadding");
    theme.template(auto_suggest_box_text_box_style_template);
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
            style.dynamic(Border::border_brush_property(), "TextControlBorderBrushPointerOver");
            style.dynamic(Border::background_property(), "TextControlBackgroundPointerOver");
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

/// The template of `AutoCompleteBox` of `default_auto_suggest_box_style`.
fn default_auto_suggest_box_style_template(t: &Template<AutoCompleteBox>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("LayoutRoot")?
        .children([
            t.make(TextBox::new())
                .name("PART_TextBox")?
                .static_resource(StyledElement::theme_property(), "AutoSuggestBoxTextBoxStyle")?
                .template_bind(
                    TextBox::placeholder_text_property(),
                    AutoCompleteBox::placeholder_text_property(),
                )
                .template_bind(
                    TextBox::placeholder_foreground_property(),
                    AutoCompleteBox::placeholder_foreground_property(),
                )
                .template_bind(Layoutable::width_property(), Layoutable::width_property())
                .template_bind(
                    TemplatedControl::foreground_property(),
                    TemplatedControl::foreground_property(),
                )
                .template_bind(
                    TemplatedControl::background_property(),
                    TemplatedControl::background_property(),
                )
                .template_bind(
                    TemplatedControl::border_brush_property(),
                    TemplatedControl::border_brush_property(),
                )
                .template_bind(
                    TemplatedControl::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(
                    TemplatedControl::font_size_property(),
                    TemplatedControl::font_size_property(),
                )
                .template_bind(
                    TemplatedControl::font_family_property(),
                    TemplatedControl::font_family_property(),
                )
                .template_bind(
                    TemplatedControl::font_weight_property(),
                    TemplatedControl::font_weight_property(),
                )
                .template_bind(
                    TemplatedControl::font_stretch_property(),
                    TemplatedControl::font_stretch_property(),
                )
                .set(
                    TextBox::vertical_content_alignment_property(),
                    VerticalAlignment::Stretch,
                )
                .set(
                    TextBox::horizontal_content_alignment_property(),
                    HorizontalAlignment::Stretch,
                )
                .set(ScrollViewer::bring_into_view_on_focus_change_property(), false)
                .set(Layoutable::margin_property(), Thickness::uniform(0.0))
                .template_bind(
                    TemplatedControl::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .template_bind(
                    TemplatedControl::padding_property(),
                    TemplatedControl::padding_property(),
                )
                .template_bind_with(
                    TextBox::caret_index_property(),
                    Some(AutoCompleteBox::caret_index_property().property()),
                    None,
                    Some(BindingMode::TwoWay),
                    None,
                )
                .template_bind(
                    TextBox::clear_selection_on_lost_focus_property(),
                    AutoCompleteBox::clear_selection_on_lost_focus_property(),
                )
                .template_bind(TextBox::max_length_property(), AutoCompleteBox::max_length_property())
                .template_bind(
                    DataValidationErrors::errors_property(),
                    DataValidationErrors::errors_property(),
                )
                .template_bind(
                    TextBox::inner_left_content_property(),
                    AutoCompleteBox::inner_left_content_property(),
                )
                .template_bind(
                    TextBox::inner_right_content_property(),
                    AutoCompleteBox::inner_right_content_property(),
                )
                .control(),
            t.make(Popup::new())
                .name("PART_Popup")?
                .set(Popup::window_manager_add_shadow_hint_property(), false)
                .set(Popup::is_light_dismiss_enabled_property(), true)
                .template_bind_with(Popup::placement_target_property(), None, None, None, None)
                .child(
                    Popup::child_property(),
                    t.make(Border::new())
                        .name("SuggestionsContainer")?
                        .set(Visual::translation_property(), literal(Vector3D::parse("0,0,32"))?)
                        .static_resource(Visual::shadow_property(), "WinUIThemeShadow")?
                        .static_resource(Layoutable::margin_property(), "ThemeShadowFlyoutMargin")?
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
                            AutoCompleteBox::max_drop_down_height_property(),
                        )
                        .dynamic(Decorator::padding_property(), "AutoSuggestListMargin")
                        .dynamic(
                            Border::border_thickness_property(),
                            "AutoSuggestListBorderThemeThickness",
                        )
                        .dynamic(
                            Border::border_brush_property(),
                            "AutoSuggestBoxSuggestionsListBorderBrush",
                        )
                        .dynamic(Border::background_property(), "AutoSuggestBoxSuggestionsListBackground")
                        .dynamic(Border::corner_radius_property(), "OverlayCornerRadius")
                        .child(
                            Decorator::child_property(),
                            t.make(ListBox::new())
                                .name("PART_SelectingItemsControl")?
                                .template_bind(
                                    ItemsControl::item_template_property(),
                                    AutoCompleteBox::item_template_property(),
                                )
                                .static_resource(
                                    ItemsControl::item_container_theme_property(),
                                    "DefaultListViewItemStyle",
                                )?
                                .dynamic(Layoutable::margin_property(), "AutoSuggestListPadding")
                                .set(
                                    TemplatedControl::background_property(),
                                    Some(literal(Brush::parse("Transparent"))?),
                                )
                                .set(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0))
                                .set(ScrollViewer::is_scroll_chaining_enabled_property(), false),
                        ),
                )
                .control(),
        ])
        .control())
}

/// `DefaultAutoSuggestBoxStyle`: a control theme of `AutoCompleteBox`.
fn default_auto_suggest_box_style(theme: &StyleBuilder) -> Result {
    theme.target::<AutoCompleteBox>();
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.dynamic(TemplatedControl::foreground_property(), "TextControlForeground");
    theme.dynamic(TemplatedControl::background_property(), "TextControlBackground");
    theme.dynamic(TemplatedControl::border_brush_property(), "TextControlBorderBrush");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "TextControlBorderThemeThickness",
    );
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.dynamic(
        AutoCompleteBox::placeholder_foreground_property(),
        "TextControlPlaceholderForeground",
    );
    theme.dynamic(TemplatedControl::padding_property(), "TextControlThemePadding");
    theme.dynamic(
        AutoCompleteBox::max_drop_down_height_property(),
        "AutoSuggestListMaxHeight",
    );
    theme.set_null(Control::focus_adorner_property());
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(default_auto_suggest_box_style_template);
    Ok(())
}

/// The default control theme of `AutoCompleteBox`.
fn auto_complete_box_theme(theme: &StyleBuilder) -> Result {
    theme.target::<AutoCompleteBox>();
    theme.based_on("DefaultAutoSuggestBoxStyle")?;
    Ok(())
}
