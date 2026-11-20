//! `Controls/RichEditBox.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/RichEditBox_themeresources.xaml (DefaultRichEditBoxStyle; its
//!   resources are Resources/RichEditBox_themeresources.xaml)
//!   controls/dev/CommandBarFlyout/TextCommandBarFlyout.cpp (the commands of the context menu)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! The template of WinUI's RichEditBox, which FerroUI ports as a control (ferroui-controls,
//! docs/porting/winui-rich-text.md); the Fluent and Simple themes have no document for it.
//! Translation notes (docs/porting/winui-theme.md, section 4; docs/porting/winui-theme/DEVIATIONS.md):
//! - ContentElement hosts the persistent rich presenter of the control; the template creates no
//!   document or editor. Header, placeholder and description are content controls.
//! - The CommonStates are pseudo-class selectors; the owned command flyout keeps the focused look
//!   through :flyout-focus.
//! - The context menu is a MenuFlyout with WinUI's text commands and the format toggles of
//!   TextCommandBarFlyout instead of the command bar flyout; there is no selection flyout for touch,
//!   and x:Load (lazy parts) is not supported.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::converters::StringConverters;
use ferroui_base::data::BindingMode;
use ferroui_base::input::Cursor;
use ferroui_base::input::InputElement;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::FontWeight;
use ferroui_base::media::TextWrapping;
use ferroui_base::styling::Selectors;
use ferroui_base::Visual;
use ferroui_controls::automation::AccessibilityView;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::primitives::HeaderedSelectingItemsControl;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::Control;
use ferroui_controls::Grid;
use ferroui_controls::MenuFlyout;
use ferroui_controls::MenuItem;
use ferroui_controls::MenuItemToggleType;
use ferroui_controls::RichEditBox;
use ferroui_controls::RowDefinitions;
use ferroui_controls::ScrollViewer;
use ferroui_controls::TextBlock;
use ferroui_controls::TextBox;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.text("StringRichEditBoldText", "Bold");
    resources.text("StringRichEditItalicText", "Italic");
    resources.text("StringRichEditUnderlineText", "Underline");
    resources.deferred("WinUIRichEditBoxContextFlyout", |scope| {
        Ok(value(
            scope
                .make(MenuFlyout::new())
                .with(|e| {
                    let items = MenuFlyout::items(e);
                    items.add(value(
                        scope
                            .make(MenuItem::new())
                            .dynamic(
                                HeaderedSelectingItemsControl::header_property(),
                                "StringTextFlyoutCutText",
                            )
                            .bind(
                                MenuItem::command_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .method::<Ref<RichEditBox>>("Cut", |this| {
                                        let _ = this.cut();
                                    })
                                    .binding(),
                            )
                            .bind(
                                InputElement::is_enabled_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .prop(RichEditBox::can_cut_property())
                                    .binding(),
                            )
                            .bind(
                                Visual::is_visible_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .prop(RichEditBox::can_cut_property())
                                    .binding(),
                            )
                            .set(MenuItem::input_gesture_property(), TextBox::cut_gesture())
                            .end(),
                    ));
                    items.add(value(
                        scope
                            .make(MenuItem::new())
                            .dynamic(
                                HeaderedSelectingItemsControl::header_property(),
                                "StringTextFlyoutCopyText",
                            )
                            .bind(
                                MenuItem::command_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .method::<Ref<RichEditBox>>("Copy", |this| {
                                        let _ = this.copy();
                                    })
                                    .binding(),
                            )
                            .bind(
                                InputElement::is_enabled_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .prop(RichEditBox::can_copy_property())
                                    .binding(),
                            )
                            .bind(
                                Visual::is_visible_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .prop(RichEditBox::can_copy_property())
                                    .binding(),
                            )
                            .set(MenuItem::input_gesture_property(), TextBox::copy_gesture())
                            .end(),
                    ));
                    items.add(value(
                        scope
                            .make(MenuItem::new())
                            .dynamic(
                                HeaderedSelectingItemsControl::header_property(),
                                "StringTextFlyoutPasteText",
                            )
                            .bind(
                                MenuItem::command_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .method::<Ref<RichEditBox>>("Paste", |this| {
                                        let _ = this.paste();
                                    })
                                    .binding(),
                            )
                            .bind(
                                InputElement::is_enabled_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .prop(RichEditBox::can_paste_property())
                                    .binding(),
                            )
                            .bind(
                                Visual::is_visible_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .prop(RichEditBox::can_paste_property())
                                    .binding(),
                            )
                            .set(MenuItem::input_gesture_property(), TextBox::paste_gesture())
                            .end(),
                    ));
                    items.add(value(
                        scope
                            .make(MenuItem::new())
                            .dynamic(
                                HeaderedSelectingItemsControl::header_property(),
                                "StringTextFlyoutUndoText",
                            )
                            .bind(
                                MenuItem::command_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .method::<Ref<RichEditBox>>("Undo", |this| {
                                        let _ = this.undo();
                                    })
                                    .binding(),
                            )
                            .bind(
                                InputElement::is_enabled_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .prop(RichEditBox::can_undo_property())
                                    .binding(),
                            )
                            .bind(
                                Visual::is_visible_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .prop(RichEditBox::can_undo_property())
                                    .binding(),
                            )
                            .end(),
                    ));
                    items.add(value(
                        scope
                            .make(MenuItem::new())
                            .dynamic(
                                HeaderedSelectingItemsControl::header_property(),
                                "StringTextFlyoutRedoText",
                            )
                            .bind(
                                MenuItem::command_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .method::<Ref<RichEditBox>>("Redo", |this| {
                                        let _ = this.redo();
                                    })
                                    .binding(),
                            )
                            .bind(
                                InputElement::is_enabled_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .prop(RichEditBox::can_redo_property())
                                    .binding(),
                            )
                            .bind(
                                Visual::is_visible_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .prop(RichEditBox::can_redo_property())
                                    .binding(),
                            )
                            .end(),
                    ));
                    items.add(value(
                        scope
                            .make(MenuItem::new())
                            .dynamic(
                                HeaderedSelectingItemsControl::header_property(),
                                "StringTextFlyoutSelectAllText",
                            )
                            .bind(
                                MenuItem::command_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .method::<Ref<RichEditBox>>("SelectAll", |this| {
                                        let _ = this.select_all();
                                    })
                                    .binding(),
                            )
                            .bind(
                                Visual::is_visible_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .prop(RichEditBox::text_property())
                                    .binding()
                                    .with_converter_value(Some(StringConverters::is_not_null_or_empty())),
                            )
                            .end(),
                    ));
                    items.add(value(
                        scope
                            .make(MenuItem::new())
                            .dynamic(
                                HeaderedSelectingItemsControl::header_property(),
                                "StringRichEditBoldText",
                            )
                            .bind(
                                MenuItem::command_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .method::<Ref<RichEditBox>>("ToggleBold", |this| {
                                        let _ = this.toggle_bold();
                                    })
                                    .binding(),
                            )
                            .bind(
                                InputElement::is_enabled_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .prop(RichEditBox::can_format_property())
                                    .binding(),
                            )
                            .set(MenuItem::toggle_type_property(), MenuItemToggleType::CheckBox)
                            .bind(
                                MenuItem::is_checked_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .prop(RichEditBox::is_bold_property())
                                    .binding()
                                    .with_mode(BindingMode::OneWay),
                            )
                            .end(),
                    ));
                    items.add(value(
                        scope
                            .make(MenuItem::new())
                            .dynamic(
                                HeaderedSelectingItemsControl::header_property(),
                                "StringRichEditItalicText",
                            )
                            .bind(
                                MenuItem::command_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .method::<Ref<RichEditBox>>("ToggleItalic", |this| {
                                        let _ = this.toggle_italic();
                                    })
                                    .binding(),
                            )
                            .bind(
                                InputElement::is_enabled_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .prop(RichEditBox::can_format_property())
                                    .binding(),
                            )
                            .set(MenuItem::toggle_type_property(), MenuItemToggleType::CheckBox)
                            .bind(
                                MenuItem::is_checked_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .prop(RichEditBox::is_italic_property())
                                    .binding()
                                    .with_mode(BindingMode::OneWay),
                            )
                            .end(),
                    ));
                    items.add(value(
                        scope
                            .make(MenuItem::new())
                            .dynamic(
                                HeaderedSelectingItemsControl::header_property(),
                                "StringRichEditUnderlineText",
                            )
                            .bind(
                                MenuItem::command_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .method::<Ref<RichEditBox>>("ToggleUnderline", |this| {
                                        let _ = this.toggle_underline();
                                    })
                                    .binding(),
                            )
                            .bind(
                                InputElement::is_enabled_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .prop(RichEditBox::can_format_property())
                                    .binding(),
                            )
                            .set(MenuItem::toggle_type_property(), MenuItemToggleType::CheckBox)
                            .bind(
                                MenuItem::is_checked_property(),
                                path()
                                    .ancestor_of::<RichEditBox>(0)
                                    .prop(RichEditBox::is_underlined_property())
                                    .binding()
                                    .with_mode(BindingMode::OneWay),
                            )
                            .end(),
                    ));
                    Ok(())
                })?
                .end(),
        ))
    });
    resources.control_theme("DefaultRichEditBoxStyle", default_rich_edit_box_style);
    resources.control_theme(type_key::<RichEditBox>(), rich_edit_box_theme);
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

/// The template of `RichEditBox` of `default_rich_edit_box_style`.
fn default_rich_edit_box_style_template(t: &Template<RichEditBox>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .with(|e| {
            e.set_row_definitions(literal(RowDefinitions::parse("Auto,*,Auto"))?);
            Ok(())
        })?
        .children([
            t.make(ContentPresenter::new())
                .name("HeaderContentPresenter")?
                .set(Grid::row_property(), 0)
                .dynamic(Layoutable::margin_property(), "RichEditBoxTopHeaderMargin")
                .template_bind(ContentPresenter::content_property(), RichEditBox::header_property())
                .template_bind(
                    ContentPresenter::content_template_property(),
                    RichEditBox::header_template_property(),
                )
                .dynamic(ContentPresenter::foreground_property(), "TextControlHeaderForeground")
                .set(ContentPresenter::font_weight_property(), FontWeight::Normal)
                .set(ContentPresenter::text_wrapping_property(), TextWrapping::Wrap)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top)
                .control(),
            t.make(Border::new())
                .name("BorderElement")?
                .set(Grid::row_property(), 1)
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .template_bind(
                    Border::border_brush_property(),
                    TemplatedControl::border_brush_property(),
                )
                .template_bind(
                    Border::border_thickness_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(
                    Border::corner_radius_property(),
                    TemplatedControl::corner_radius_property(),
                )
                .dynamic(Layoutable::min_width_property(), "TextControlThemeMinWidth")
                .dynamic(Layoutable::min_height_property(), "TextControlThemeMinHeight")
                .control(),
            t.make(ScrollViewer::new())
                .name("ContentElement")?
                .set(Grid::row_property(), 1)
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
                    ScrollViewer::is_scroll_chaining_enabled_property(),
                    ScrollViewer::is_scroll_chaining_enabled_property(),
                )
                .set(InputElement::focusable_property(), false)
                .set(
                    AutomationProperties::accessibility_view_property(),
                    AccessibilityView::Raw,
                )
                .style(of::<ScrollContentPresenter>().name("PART_ContentPresenter"), |style| {
                    style.set(InputElement::cursor_property(), Some(literal(Cursor::parse("IBeam"))?));
                    Ok(())
                })?
                .control(),
            t.make(TextBlock::new())
                .name("PlaceholderTextContentPresenter")?
                .set(Grid::row_property(), 1)
                .template_bind(
                    Layoutable::margin_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .template_bind(TextBlock::padding_property(), TemplatedControl::padding_property())
                .template_bind(
                    TextBlock::foreground_property(),
                    RichEditBox::placeholder_foreground_property(),
                )
                .template_bind(TextBlock::text_property(), RichEditBox::placeholder_text_property())
                .template_bind(
                    TextBlock::text_alignment_property(),
                    RichEditBox::text_alignment_property(),
                )
                .template_bind(
                    TextBlock::text_wrapping_property(),
                    RichEditBox::text_wrapping_property(),
                )
                .set(InputElement::is_hit_test_visible_property(), false)
                .control(),
            t.make(ContentPresenter::new())
                .name("DescriptionPresenter")?
                .set(Grid::row_property(), 2)
                .template_bind(
                    ContentPresenter::content_property(),
                    RichEditBox::description_property(),
                )
                .dynamic(
                    ContentPresenter::foreground_property(),
                    "SystemControlDescriptionTextForegroundBrush",
                )
                .set(ContentPresenter::text_wrapping_property(), TextWrapping::Wrap)
                .set(
                    AutomationProperties::accessibility_view_property(),
                    AccessibilityView::Raw,
                )
                .control(),
        ])
        .control())
}

/// `DefaultRichEditBoxStyle`: a control theme of `RichEditBox`.
fn default_rich_edit_box_style(theme: &StyleBuilder) -> Result {
    theme.target::<RichEditBox>();
    theme.dynamic(TemplatedControl::foreground_property(), "TextControlForeground");
    theme.dynamic(TemplatedControl::background_property(), "TextControlBackground");
    theme.dynamic(TemplatedControl::border_brush_property(), "TextControlBorderBrush");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "TextControlBorderThemeThickness",
    );
    theme.dynamic(RichEditBox::caret_brush_property(), "TextControlForegroundFocused");
    theme.dynamic(
        RichEditBox::selection_highlight_color_property(),
        "TextControlSelectionHighlightColor",
    );
    theme.dynamic(
        RichEditBox::placeholder_foreground_property(),
        "TextControlPlaceholderForeground",
    );
    theme.dynamic(TemplatedControl::padding_property(), "TextControlThemePadding");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.set(RichEditBox::text_wrapping_property(), TextWrapping::Wrap);
    theme.set(
        ScrollViewer::horizontal_scroll_bar_visibility_property(),
        ScrollBarVisibility::Auto,
    );
    theme.set(
        ScrollViewer::vertical_scroll_bar_visibility_property(),
        ScrollBarVisibility::Auto,
    );
    theme.set(ScrollViewer::is_deferred_scrolling_enabled_property(), false);
    theme.set(ScrollViewer::is_scroll_chaining_enabled_property(), true);
    theme.set_null(Control::focus_adorner_property());
    theme.static_resource(Control::context_flyout_property(), "WinUIRichEditBoxContextFlyout")?;
    theme.template(default_rich_edit_box_style_template);
    theme.style(nesting().class(":pointerover"), |style| {
        style.dynamic(
            RichEditBox::placeholder_foreground_property(),
            "TextControlPlaceholderForegroundPointerOver",
        );
        style.style(nesting().template().of::<Border>().name("BorderElement"), |style| {
            style.dynamic(Border::background_property(), "TextControlBackgroundPointerOver");
            style.dynamic(Border::border_brush_property(), "TextControlBorderBrushPointerOver");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<ScrollViewer>().name("ContentElement"),
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
    theme.style(
        Selectors::or([nesting().class(":focus"), nesting().class(":flyout-focus")]),
        |style| {
            style.dynamic(
                RichEditBox::placeholder_foreground_property(),
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
                nesting().template().of::<ScrollViewer>().name("ContentElement"),
                |style| {
                    style.dynamic(TemplatedControl::foreground_property(), "TextControlForegroundFocused");
                    Ok(())
                },
            )?;
            Ok(())
        },
    )?;
    theme.style(nesting().class(":disabled"), |style| {
        style.dynamic(
            RichEditBox::placeholder_foreground_property(),
            "TextControlPlaceholderForegroundDisabled",
        );
        style.style(nesting().template().of::<Border>().name("BorderElement"), |style| {
            style.dynamic(Border::background_property(), "TextControlBackgroundDisabled");
            style.dynamic(Border::border_brush_property(), "TextControlBorderBrushDisabled");
            Ok(())
        })?;
        style.style(
            nesting().template().of::<ScrollViewer>().name("ContentElement"),
            |style| {
                style.dynamic(TemplatedControl::foreground_property(), "TextControlForegroundDisabled");
                Ok(())
            },
        )?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("HeaderContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "TextControlHeaderForegroundDisabled",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}

/// The default control theme of `RichEditBox`.
fn rich_edit_box_theme(theme: &StyleBuilder) -> Result {
    theme.target::<RichEditBox>();
    theme.based_on("DefaultRichEditBoxStyle")?;
    Ok(())
}
