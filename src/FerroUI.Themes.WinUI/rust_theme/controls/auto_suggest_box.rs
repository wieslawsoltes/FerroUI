//! `Controls/AutoSuggestBox.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/AutoSuggestBox/AutoSuggestBox_themeresources.xaml (the implicit style of
//!   AutoSuggestBox and DefaultAutoSuggestBoxStyle, the QueryButton and QueryButtonStyle of
//!   AutoSuggestBoxTextBoxStyle; DefaultAutoSuggestBoxStyle and AutoSuggestBoxTextBoxStyle are
//!   ported in AutoCompleteBox.xaml, and the resources of the document are
//!   Resources/AutoSuggestBox_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - FerroUI's AutoSuggestBox is WinUI's AutoSuggestBox (a WinUI control in ferroui-controls,
//!   docs/porting/winui-controls.md); AutoCompleteBox.xaml draws the upstream AutoCompleteBox with
//!   the same style, keyed DefaultAutoSuggestBoxStyle. That key belongs to AutoCompleteBox in this
//!   theme (its TargetType), so the implicit theme of AutoSuggestBox is not based on it: it has the
//!   setters and the template of DefaultAutoSuggestBoxStyle itself. The text box has the keyed theme
//!   AutoSuggestBoxTextBoxStyle (the TextBoxStyle of the source), as in AutoCompleteBox.xaml.
//! - The parts keep WinUI's names, which the control looks up: TextBox, SuggestionsPopup,
//!   SuggestionsContainer, SuggestionsList (a ListBox with the item theme DefaultListViewItemStyle,
//!   as the ListView of the source) and QueryButton.
//! - QueryButton is a part of AutoSuggestBoxTextBoxStyle in WinUI (AutoSuggestBox finds it in the
//!   template of its text box). FerroUI's text box template of AutoCompleteBox.xaml has no query
//!   button, so the button is the InnerRightContent of the text box, which that template shows at the
//!   right of the delete button, after the column of AutoSuggestBoxRightButtonMargin: the margin
//!   -2,0,4,0 puts it 2 px from the delete button and 4 px from the edge, as the columns of the
//!   source do. Its template-local QueryButtonStyle is the keyed theme AutoSuggestBoxQueryButtonStyle.
//!   The control shows the button and gives it the QueryIcon (SetTextBoxQueryButtonIcon).
//! - The glyph of a SymbolIcon query icon is drawn at AutoSuggestBoxIconFontSize (WinUI sets the
//!   FontSize of the icon to zero so that its glyph inherits that size from the button): a style of
//!   the text part of the icon.
//! - The header and the description are presenters of the text box template in WinUI (the Header
//!   and Description of TextBox); FerroUI's TextBox has neither, so they are rows of this template:
//!   the header presenter PART_HeaderPresenter (the part of the headered items control,
//!   HeaderContentPresenter in the source) with AutoSuggestBoxTopHeaderMargin, shown when there is a
//!   header, and DescriptionPresenter.
//! - The suggestion list: SuggestionsContainer has the MaxHeight and the Width the control sets
//!   (UpdateSuggestionListSize), the list the MaxHeight AutoSuggestListMaxHeight. The shadow of the
//!   list (ApplyElevationEffect on the popup: a ThemeShadow at a Translation.Z of 32) is the
//!   same on SuggestionsContainer (the shared WinUIThemeShadow), with the margin ThemeShadowFlyoutMargin as the
//!   room for it in the popup (Resources/ThemeShadows.xaml; DEVIATIONS.md, G-24). The popup is placed
//!   under the text box; the placement above it when there is no room below is not ported.
//! - Left out, as in AutoCompleteBox.xaml: Orientation (Landscape, Portrait), UseSystemFocusVisuals
//!   of IsApplicationFocusVisualKindReveal (no focus adorner), AutoSuggestBoxHelper.KeepInteriorCornersSquare,
//!   DesiredCandidateWindowAlignment, Canvas.ZIndex, DisplayMemberPath, ItemTemplateSelector,
//!   ItemContainerStyle and IsItemClickEnabled of the list (the control submits a suggestion on a
//!   click), the item container transitions, ShouldConstrainToRootBounds and
//!   AutomationProperties.AccessibilityView.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::converters::ObjectConverters;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::Brush;
use ferroui_base::media::FontStyle;
use ferroui_base::media::FontWeight;
use ferroui_base::media::TextAlignment;
use ferroui_base::media::TextWrapping;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Vector3D;
use ferroui_base::Visual;
use ferroui_controls::automation::AccessibilityView;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::HeaderedItemsControl;
use ferroui_controls::primitives::Popup;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::AutoSuggestBox;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::ItemsControl;
use ferroui_controls::ListBox;
use ferroui_controls::PlacementMode;
use ferroui_controls::RowDefinitions;
use ferroui_controls::ScrollViewer;
use ferroui_controls::SymbolIcon;
use ferroui_controls::TextBlock;
use ferroui_controls::TextBox;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::control_template;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // QueryButtonStyle of the template of AutoSuggestBoxTextBoxStyle.
    resources.control_theme("AutoSuggestBoxQueryButtonStyle", auto_suggest_box_query_button_style);
    // The template of SymbolIcon (Controls/SymbolIcon.xaml) with the glyph at
    //       AutoSuggestBoxIconFontSize, for the query icon (ASB-3).
    resources.deferred("AutoSuggestBoxQuerySymbolIconTemplate", |scope| {
        Ok(value(control_template(
            scope,
            auto_suggest_box_query_symbol_icon_template_template,
        )))
    });
    // The implicit style of AutoSuggestBox, with the setters and the template of
    //       DefaultAutoSuggestBoxStyle.
    resources.control_theme(type_key::<AutoSuggestBox>(), auto_suggest_box_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `Button` of `auto_suggest_box_query_button_style`.
fn auto_suggest_box_query_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(ContentPresenter::new())
        .name("PART_ContentPresenter")?
        .dynamic(ContentPresenter::background_property(), "TextControlButtonBackground")
        .template_bind(
            ContentPresenter::background_sizing_property(),
            TemplatedControl::background_sizing_property(),
        )
        .dynamic(
            ContentPresenter::border_brush_property(),
            "TextControlButtonBorderBrush",
        )
        .template_bind(
            ContentPresenter::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
        .template_bind(
            ContentPresenter::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .template_bind(
            ContentPresenter::content_template_property(),
            ContentControl::content_template_property(),
        )
        .dynamic(ContentPresenter::foreground_property(), "TextControlButtonForeground")
        .dynamic(ContentPresenter::font_size_property(), "AutoSuggestBoxIconFontSize")
        .dynamic(Layoutable::margin_property(), "AutoSuggestBoxInnerButtonMargin")
        .template_bind(
            ContentPresenter::padding_property(),
            TemplatedControl::padding_property(),
        )
        .template_bind(
            ContentPresenter::horizontal_content_alignment_property(),
            ContentControl::horizontal_content_alignment_property(),
        )
        .template_bind(
            ContentPresenter::vertical_content_alignment_property(),
            ContentControl::vertical_content_alignment_property(),
        )
        .control())
}

/// `AutoSuggestBoxQueryButtonStyle`: a control theme of `Button`.
fn auto_suggest_box_query_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.template(auto_suggest_box_query_button_style_template);
    // PointerOver
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "TextControlButtonBackgroundPointerOver",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "TextControlButtonBorderBrushPointerOver",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "TextControlButtonForegroundPointerOver",
            );
            Ok(())
        },
    )?;
    // Pressed
    theme.style(
        nesting()
            .class(":pressed")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.dynamic(
                ContentPresenter::background_property(),
                "TextControlButtonBackgroundPressed",
            );
            style.dynamic(
                ContentPresenter::border_brush_property(),
                "TextControlButtonBorderBrushPressed",
            );
            style.dynamic(
                ContentPresenter::foreground_property(),
                "TextControlButtonForegroundPressed",
            );
            Ok(())
        },
    )?;
    // Disabled
    theme.style(
        nesting()
            .class(":disabled")
            .template()
            .of::<ContentPresenter>()
            .name("PART_ContentPresenter"),
        |style| {
            style.set(Visual::opacity_property(), 0.0);
            Ok(())
        },
    )?;
    Ok(())
}

/// The template of `SymbolIcon` of `AutoSuggestBoxQuerySymbolIconTemplate`.
fn auto_suggest_box_query_symbol_icon_template_template(t: &Template<SymbolIcon>) -> Result<Ref<Control>> {
    Ok(t.make(TextBlock::new())
        .name("PART_Text")?
        .set(
            Layoutable::horizontal_alignment_property(),
            HorizontalAlignment::Stretch,
        )
        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
        .set(TextBlock::text_alignment_property(), TextAlignment::Center)
        .dynamic(TextBlock::font_family_property(), "SymbolThemeFontFamily")
        .dynamic(TextBlock::font_size_property(), "AutoSuggestBoxIconFontSize")
        .set(TextBlock::font_style_property(), FontStyle::Normal)
        .set(TextBlock::font_weight_property(), FontWeight::Normal)
        .set(
            AutomationProperties::accessibility_view_property(),
            AccessibilityView::Raw,
        )
        .control())
}

/// The template of `AutoSuggestBox` of `auto_suggest_box_theme`.
fn auto_suggest_box_theme_template(t: &Template<AutoSuggestBox>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .name("LayoutRoot")?
        .with(|e| {
            Grid::set_row_definitions(e, literal(RowDefinitions::parse("Auto,*,Auto"))?);
            Ok(())
        })?
        .children([
            t.make(ContentPresenter::new())
                .name("PART_HeaderPresenter")?
                .set(Grid::row_property(), 0)
                .template_bind(
                    ContentPresenter::content_property(),
                    HeaderedItemsControl::header_property(),
                )
                .template_bind(
                    ContentPresenter::content_template_property(),
                    HeaderedItemsControl::header_template_property(),
                )
                .template_bind_with(
                    Visual::is_visible_property(),
                    Some(HeaderedItemsControl::header_property().property()),
                    Some(ObjectConverters::is_not_null()),
                    None,
                    None,
                )
                .dynamic(ContentPresenter::foreground_property(), "TextControlHeaderForeground")
                .dynamic(Layoutable::margin_property(), "AutoSuggestBoxTopHeaderMargin")
                .set(ContentPresenter::font_weight_property(), FontWeight::Normal)
                .set(ContentPresenter::text_wrapping_property(), TextWrapping::Wrap)
                .control(),
            t.make(TextBox::new())
                .name("TextBox")?
                .set(Grid::row_property(), 1)
                .static_resource(StyledElement::theme_property(), "AutoSuggestBoxTextBoxStyle")?
                .template_bind(
                    TextBox::placeholder_text_property(),
                    AutoSuggestBox::placeholder_text_property(),
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
                .set(ScrollViewer::bring_into_view_on_focus_change_property(), false)
                .set(Layoutable::margin_property(), Thickness::uniform(0.0))
                .template_bind(
                    TemplatedControl::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .content(
                    TextBox::inner_right_content_property(),
                    t.make(Button::new())
                        .name("QueryButton")?
                        .static_resource(StyledElement::theme_property(), "AutoSuggestBoxQueryButtonStyle")?
                        .template_bind(
                            TemplatedControl::corner_radius_property(),
                            TemplatedControl::corner_radius_property(),
                        )
                        .template_bind(
                            TemplatedControl::border_thickness_property(),
                            TemplatedControl::border_thickness_property(),
                        )
                        .set(Layoutable::margin_property(), Thickness::new(-2.0, 0.0, 4.0, 0.0))
                        .dynamic(TemplatedControl::padding_property(), "HelperButtonThemePadding")
                        .set(InputElement::focusable_property(), false)
                        .set(InputElement::is_tab_stop_property(), false)
                        .set(Visual::is_visible_property(), false)
                        .template_bind(
                            TemplatedControl::font_size_property(),
                            TemplatedControl::font_size_property(),
                        )
                        .set(Layoutable::width_property(), 32.0)
                        .set(Layoutable::height_property(), 28.0)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch),
                )?
                .control(),
            t.make(Popup::new())
                .name("SuggestionsPopup")?
                .set(Popup::window_manager_add_shadow_hint_property(), false)
                .set(Popup::is_light_dismiss_enabled_property(), true)
                .bind(
                    Popup::placement_target_property(),
                    path().named(t.names(), "TextBox").binding(),
                )
                .set(Popup::placement_property(), PlacementMode::Bottom)
                .child(
                    Popup::child_property(),
                    t.make(Border::new())
                        .name("SuggestionsContainer")?
                        .set(Visual::translation_property(), literal(Vector3D::parse("0,0,32"))?)
                        .static_resource(Visual::shadow_property(), "WinUIThemeShadow")?
                        .static_resource(Layoutable::margin_property(), "ThemeShadowFlyoutMargin")?
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
                                .name("SuggestionsList")?
                                .template_bind(
                                    ItemsControl::item_template_property(),
                                    ItemsControl::item_template_property(),
                                )
                                .static_resource(
                                    ItemsControl::item_container_theme_property(),
                                    "DefaultListViewItemStyle",
                                )?
                                .dynamic(Layoutable::max_height_property(), "AutoSuggestListMaxHeight")
                                .dynamic(Layoutable::margin_property(), "AutoSuggestListPadding")
                                .set(InputElement::focusable_property(), false)
                                .set(
                                    TemplatedControl::background_property(),
                                    Some(literal(Brush::parse("Transparent"))?),
                                )
                                .set(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0))
                                .set(ScrollViewer::is_scroll_chaining_enabled_property(), false),
                        ),
                )
                .control(),
            t.make(ContentPresenter::new())
                .name("DescriptionPresenter")?
                .set(Grid::row_property(), 2)
                .template_bind(
                    ContentPresenter::content_property(),
                    AutoSuggestBox::description_property(),
                )
                .template_bind_with(
                    Visual::is_visible_property(),
                    Some(AutoSuggestBox::description_property().property()),
                    Some(ObjectConverters::is_not_null()),
                    None,
                    None,
                )
                .dynamic(
                    ContentPresenter::foreground_property(),
                    "SystemControlDescriptionTextForegroundBrush",
                )
                .control(),
        ])
        .control())
}

/// The default control theme of `AutoSuggestBox`.
fn auto_suggest_box_theme(theme: &StyleBuilder) -> Result {
    theme.target::<AutoSuggestBox>();
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
    // VerticalContentAlignment and HorizontalContentAlignment (Stretch) of the source are left out:
    //         FerroUI's AutoSuggestBox is a headered items control, which has neither (ASB-8); the text box
    //         of the template stretches by its own defaults.
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
    theme.set_null(Control::focus_adorner_property());
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(auto_suggest_box_theme_template);
    // The glyph of a SymbolIcon query icon at AutoSuggestBoxIconFontSize (ASB-3): the icon gets the
    //         template of SymbolIcon with that size, since a style cannot reach into the template of a
    //         part of the template (one /template/ per selector).
    theme.style(
        nesting()
            .template()
            .of::<Button>()
            .name("QueryButton")
            .descendant()
            .of::<SymbolIcon>(),
        |style| {
            style.static_resource(
                TemplatedControl::template_property(),
                "AutoSuggestBoxQuerySymbolIconTemplate",
            )?;
            Ok(())
        },
    )?;
    Ok(())
}
