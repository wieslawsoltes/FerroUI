//! `Controls/TextBox.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/TextBox_themeresources.xaml (the styles; its resources are
//!   Resources/TextBox_themeresources.xaml)
//!   controls/dev/CommonStyles/PasswordBox_themeresources.xaml (the style DefaultPasswordBoxStyle;
//!   its resources are Resources/PasswordBox_themeresources.xaml)
//!   controls/dev/CommandBarFlyout/TextCommandBarFlyout.cpp (the commands of the context flyout)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - Template parts: ContentElement, the ScrollViewer of the text, is FerroUI's PART_ScrollViewer and
//!   holds PART_TextPresenter, the text view that WinUI's ScrollViewer gets from the control.
//! - FerroUI's TextBox has no Header and no Description: HeaderContentPresenter,
//!   DescriptionPresenter and their rows are left out.
//! - The placeholder: WinUI binds its foreground to PlaceholderForeground with the theme resource of
//!   the state as TargetNullValue, and the control hides it while there is text. Here the state
//!   styles set PlaceholderForeground of the control (a value the application sets wins, as with
//!   TargetNullValue) and the placeholder is visible while the text and the text of the input
//!   method are empty.
//! - The delete button: the template-local style DeleteButtonStyle is the keyed theme
//!   TextBoxDeleteButtonStyle; the ButtonStates group (ButtonVisible, which the control selects in
//!   CTextBox::CanInvokeDeleteButton) is the selector of its condition, without the width check of
//!   the source; the button clears the text through the Clear method of the control. Its glyph
//!   E894 is the geometry GlyphGeometryE894.
//! - UseSystemFocusVisuals="{ThemeResource IsApplicationFocusVisualKindReveal}" is False: no focus
//!   adorner. CaretBrush is set because FerroUI's caret does not follow the foreground as WinUI's
//!   does. SelectionHighlightColor is SelectionBrush.
//! - The template is wrapped in DataValidationErrors, as FerroUI's text box requires for its
//!   validation errors (WinUI 3 has none): the errors are shown under the box by the theme of
//!   DataValidationErrors, and under :error the border is SystemFillColorCriticalBrush.
//! - The context flyout: WinUI's TextCommandBarFlyout (TextControlCommandBarContextFlyout, waived:
//!   FerroUI has no command bar flyout) is the menu flyout TextBoxContextFlyout of the commands it
//!   shows for a text box opened with mouse or keyboard, in its order (TextCommandBarFlyout.cpp):
//!   Cut, Copy, Paste, Undo, Redo, Select All, each shown when the text box can do it, with the
//!   icons of the standard commands (glyphs E8C6, E8C8, E77F, E7A7, E7A6; Select All has none) and
//!   the keyboard accelerators of Cut, Copy and Paste of the platform. The labels are the string
//!   resources StringTextFlyout*Text of Strings/InvariantResources.xaml. The selection flyout
//!   (TextControlCommandBarSelectionFlyout) and the horizontal command bar of touch input have no
//!   counterpart.
//! - FerroUI's text box presents InnerLeftContent and InnerRightContent, which its other themes
//!   show at the left and the right of the text: the template has a column for each, inside the
//!   border, beside the delete and reveal buttons.
//! - The style classes of FerroUI's text box (its Fluent theme): clearButton shows the delete button
//!   while the box is focused and not empty (also when the text wraps), revealPasswordButton shows
//!   the reveal button of DefaultPasswordBoxStyle while the box is not empty.
//! - DefaultPasswordBoxStyle (controls/dev/CommonStyles/PasswordBox_themeresources.xaml) is a keyed
//!   theme of TextBox, FerroUI's password box being a text box with a PasswordChar: it sets the
//!   password character of WinUI's PasswordBox, has the reveal button RevealButton instead of the
//!   delete button, and the context flyout PasswordBoxContextFlyout of the commands WinUI shows for
//!   a password box (Paste, Select All). The reveal button (the template-local RevealButtonStyle,
//!   the keyed theme PasswordBoxRevealButtonStyle) toggles RevealPassword; WinUI reveals the
//!   password while the button is pressed (PasswordRevealMode Peek).

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
use ferroui_base::media::Stretch;
use ferroui_base::media::TextWrapping;
use ferroui_base::StyledElement;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::presenters::TextPresenter;
use ferroui_controls::primitives::HeaderedSelectingItemsControl;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::DataValidationErrors;
use ferroui_controls::Decorator;
use ferroui_controls::Grid;
use ferroui_controls::MenuFlyout;
use ferroui_controls::MenuItem;
use ferroui_controls::PathIcon;
use ferroui_controls::ScrollViewer;
use ferroui_controls::TextBlock;
use ferroui_controls::TextBox;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // DeleteButtonStyle of the template of DefaultTextBoxStyle.
    resources.control_theme("TextBoxDeleteButtonStyle", text_box_delete_button_style);
    // RevealButtonStyle of the template of DefaultPasswordBoxStyle.
    resources.control_theme("PasswordBoxRevealButtonStyle", password_box_reveal_button_style);
    // The context flyout of a text box: the commands TextCommandBarFlyout shows for a TextBox
    //       (TextCommandBarFlyout::GetTextBoxButtonsToAdd), each while the text box can do it.
    resources.deferred("TextBoxContextFlyout", |scope| {
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
                                    .ancestor_of::<TextBox>(0)
                                    .method::<Ref<TextBox>>("Cut", |this| {
                                        this.cut();
                                    })
                                    .binding(),
                            )
                            .bind(
                                InputElement::is_enabled_property(),
                                path()
                                    .ancestor_of::<TextBox>(0)
                                    .prop(TextBox::can_cut_property())
                                    .binding(),
                            )
                            .bind(
                                Visual::is_visible_property(),
                                path()
                                    .ancestor_of::<TextBox>(0)
                                    .prop(TextBox::can_cut_property())
                                    .binding(),
                            )
                            .set(MenuItem::input_gesture_property(), TextBox::cut_gesture())
                            .content(
                                MenuItem::icon_property(),
                                scope
                                    .make(PathIcon::new())
                                    .static_resource(PathIcon::data_property(), "GlyphGeometryE8C6")?,
                            )?
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
                                    .ancestor_of::<TextBox>(0)
                                    .method::<Ref<TextBox>>("Copy", |this| {
                                        this.copy();
                                    })
                                    .binding(),
                            )
                            .bind(
                                InputElement::is_enabled_property(),
                                path()
                                    .ancestor_of::<TextBox>(0)
                                    .prop(TextBox::can_copy_property())
                                    .binding(),
                            )
                            .bind(
                                Visual::is_visible_property(),
                                path()
                                    .ancestor_of::<TextBox>(0)
                                    .prop(TextBox::can_copy_property())
                                    .binding(),
                            )
                            .set(MenuItem::input_gesture_property(), TextBox::copy_gesture())
                            .content(
                                MenuItem::icon_property(),
                                scope
                                    .make(PathIcon::new())
                                    .static_resource(PathIcon::data_property(), "GlyphGeometryE8C8")?,
                            )?
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
                                    .ancestor_of::<TextBox>(0)
                                    .method::<Ref<TextBox>>("Paste", |this| {
                                        this.paste();
                                    })
                                    .binding(),
                            )
                            .bind(
                                InputElement::is_enabled_property(),
                                path()
                                    .ancestor_of::<TextBox>(0)
                                    .prop(TextBox::can_paste_property())
                                    .binding(),
                            )
                            .bind(
                                Visual::is_visible_property(),
                                path()
                                    .ancestor_of::<TextBox>(0)
                                    .prop(TextBox::can_paste_property())
                                    .binding(),
                            )
                            .set(MenuItem::input_gesture_property(), TextBox::paste_gesture())
                            .content(
                                MenuItem::icon_property(),
                                scope
                                    .make(PathIcon::new())
                                    .static_resource(PathIcon::data_property(), "GlyphGeometryE77F")?,
                            )?
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
                                    .ancestor_of::<TextBox>(0)
                                    .method::<Ref<TextBox>>("Undo", |this| {
                                        this.undo();
                                    })
                                    .binding(),
                            )
                            .bind(
                                InputElement::is_enabled_property(),
                                path()
                                    .ancestor_of::<TextBox>(0)
                                    .prop(TextBox::can_undo_property())
                                    .binding(),
                            )
                            .bind(
                                Visual::is_visible_property(),
                                path()
                                    .ancestor_of::<TextBox>(0)
                                    .prop(TextBox::can_undo_property())
                                    .binding(),
                            )
                            .content(
                                MenuItem::icon_property(),
                                scope
                                    .make(PathIcon::new())
                                    .static_resource(PathIcon::data_property(), "GlyphGeometryE7A7")?,
                            )?
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
                                    .ancestor_of::<TextBox>(0)
                                    .method::<Ref<TextBox>>("Redo", |this| {
                                        this.redo();
                                    })
                                    .binding(),
                            )
                            .bind(
                                InputElement::is_enabled_property(),
                                path()
                                    .ancestor_of::<TextBox>(0)
                                    .prop(TextBox::can_redo_property())
                                    .binding(),
                            )
                            .bind(
                                Visual::is_visible_property(),
                                path()
                                    .ancestor_of::<TextBox>(0)
                                    .prop(TextBox::can_redo_property())
                                    .binding(),
                            )
                            .content(
                                MenuItem::icon_property(),
                                scope
                                    .make(PathIcon::new())
                                    .static_resource(PathIcon::data_property(), "GlyphGeometryE7A6")?,
                            )?
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
                                    .ancestor_of::<TextBox>(0)
                                    .method::<Ref<TextBox>>("SelectAll", |this| {
                                        this.select_all();
                                    })
                                    .binding(),
                            )
                            .bind(
                                Visual::is_visible_property(),
                                path()
                                    .ancestor_of::<TextBox>(0)
                                    .prop(TextBox::text_property())
                                    .binding()
                                    .with_converter_value(Some(StringConverters::is_not_null_or_empty())),
                            )
                            .end(),
                    ));
                    Ok(())
                })?
                .end(),
        ))
    });
    // The context flyout of a password box (TextCommandBarFlyout::GetPasswordBoxButtonsToAdd).
    resources.deferred("PasswordBoxContextFlyout", |scope| {
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
                                "StringTextFlyoutPasteText",
                            )
                            .bind(
                                MenuItem::command_property(),
                                path()
                                    .ancestor_of::<TextBox>(0)
                                    .method::<Ref<TextBox>>("Paste", |this| {
                                        this.paste();
                                    })
                                    .binding(),
                            )
                            .bind(
                                InputElement::is_enabled_property(),
                                path()
                                    .ancestor_of::<TextBox>(0)
                                    .prop(TextBox::can_paste_property())
                                    .binding(),
                            )
                            .bind(
                                Visual::is_visible_property(),
                                path()
                                    .ancestor_of::<TextBox>(0)
                                    .prop(TextBox::can_paste_property())
                                    .binding(),
                            )
                            .set(MenuItem::input_gesture_property(), TextBox::paste_gesture())
                            .content(
                                MenuItem::icon_property(),
                                scope
                                    .make(PathIcon::new())
                                    .static_resource(PathIcon::data_property(), "GlyphGeometryE77F")?,
                            )?
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
                                    .ancestor_of::<TextBox>(0)
                                    .method::<Ref<TextBox>>("SelectAll", |this| {
                                        this.select_all();
                                    })
                                    .binding(),
                            )
                            .bind(
                                Visual::is_visible_property(),
                                path()
                                    .ancestor_of::<TextBox>(0)
                                    .prop(TextBox::text_property())
                                    .binding()
                                    .with_converter_value(Some(StringConverters::is_not_null_or_empty())),
                            )
                            .end(),
                    ));
                    Ok(())
                })?
                .end(),
        ))
    });
    resources.control_theme("DefaultTextBoxStyle", default_text_box_style);
    // DefaultPasswordBoxStyle (PasswordBox_themeresources.xaml): a text box with the password
    //       character of WinUI's PasswordBox, the reveal button instead of the delete button and the
    //       context flyout of a password box. Its other setters and its states are those of
    //       DefaultTextBoxStyle, as the source repeats them.
    resources.control_theme("DefaultPasswordBoxStyle", default_password_box_style);
    resources.control_theme(type_key::<TextBox>(), text_box_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `Button` of `text_box_delete_button_style`.
fn text_box_delete_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("ButtonLayoutGrid")?
        .dynamic(Layoutable::margin_property(), "TextBoxInnerButtonMargin")
        .dynamic(Border::border_brush_property(), "TextControlButtonBorderBrush")
        .template_bind(
            Border::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .dynamic(Border::background_property(), "TextControlButtonBackground")
        .template_bind(
            Border::background_sizing_property(),
            TemplatedControl::background_sizing_property(),
        )
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        // E894 at TextBoxIconFontSize: the glyph is drawn at the size of its ink.
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

/// `TextBoxDeleteButtonStyle`: a control theme of `Button`.
fn text_box_delete_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    theme.template(text_box_delete_button_style_template);
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

/// The template of `ToggleButton` of `password_box_reveal_button_style`.
fn password_box_reveal_button_style_template(t: &Template<ToggleButton>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("ButtonLayoutGrid")?
        .dynamic(Layoutable::margin_property(), "TextBoxInnerButtonMargin")
        .dynamic(Border::border_brush_property(), "TextControlButtonBorderBrush")
        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
        .template_bind(
            Border::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .dynamic(Border::background_property(), "TextControlButtonBackground")
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        // F78D at PasswordBoxIconFontSize (12 px): the glyph is drawn at the size of its ink.
        .child(
            Decorator::child_property(),
            t.make(Path::new())
                .name("GlyphElement")?
                .dynamic(Shape::fill_property(), "TextControlButtonForeground")
                .static_resource(Path::data_property(), "GlyphGeometryF78D")?
                .set(Shape::stretch_property(), Stretch::Uniform)
                .set(Layoutable::width_property(), 11.0)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center),
        )
        .control())
}

/// `PasswordBoxRevealButtonStyle`: a control theme of `ToggleButton`.
fn password_box_reveal_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<ToggleButton>();
    theme.template(password_box_reveal_button_style_template);
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
    // Pressed, CheckedPressed, IndeterminatePressed
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

/// The template of `TextBox` of `default_text_box_style`.
fn default_text_box_style_template(t: &Template<TextBox>) -> Result<Ref<Control>> {
    Ok(t.make(DataValidationErrors::new())
        .content(
            ContentControl::content_property(),
            t.make(Grid::new())
                .with(|e| {
                    Grid::set_column_definitions(e, literal(ColumnDefinitions::parse("Auto,*,Auto,Auto,Auto"))?);
                    Ok(())
                })?
                // InnerLeftContent of FerroUI's text box.
                // RevealButton of DefaultPasswordBoxStyle, also shown by the style class
                //                 revealPasswordButton.
                // InnerRightContent of FerroUI's text box.
                .children([
                    t.make(Border::new())
                        .name("BorderElement")?
                        .set(Grid::column_property(), 0)
                        .set(Grid::column_span_property(), 5)
                        .template_bind(Border::background_property(), TemplatedControl::background_property())
                        .template_bind(
                            Border::background_sizing_property(),
                            TemplatedControl::background_sizing_property(),
                        )
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
                        .template_bind(Layoutable::min_width_property(), Layoutable::min_width_property())
                        .template_bind(Layoutable::min_height_property(), Layoutable::min_height_property())
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
                        .set(Grid::column_property(), 1)
                        .set(Grid::column_span_property(), 2)
                        .template_bind(
                            Layoutable::vertical_alignment_property(),
                            TextBox::vertical_content_alignment_property(),
                        )
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
                    t.make(Button::new())
                        .name("DeleteButton")?
                        .set(Grid::column_property(), 2)
                        .static_resource(StyledElement::theme_property(), "TextBoxDeleteButtonStyle")?
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
                            TemplatedControl::border_thickness_property(),
                            TemplatedControl::border_thickness_property(),
                        )
                        .template_bind(
                            TemplatedControl::corner_radius_property(),
                            TemplatedControl::corner_radius_property(),
                        )
                        .dynamic(TemplatedControl::padding_property(), "HelperButtonThemePadding")
                        .set(InputElement::focusable_property(), false)
                        .set(Visual::is_visible_property(), false)
                        .template_bind(
                            TemplatedControl::font_size_property(),
                            TemplatedControl::font_size_property(),
                        )
                        .set(Layoutable::width_property(), 30.0)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                        .control(),
                    t.make(ToggleButton::new())
                        .name("RevealButton")?
                        .set(Grid::column_property(), 3)
                        .static_resource(StyledElement::theme_property(), "PasswordBoxRevealButtonStyle")?
                        .template_bind_with(
                            ToggleButton::is_checked_property(),
                            Some(TextBox::reveal_password_property().property()),
                            None,
                            Some(BindingMode::TwoWay),
                            None,
                        )
                        .template_bind(
                            TemplatedControl::border_thickness_property(),
                            TemplatedControl::border_thickness_property(),
                        )
                        .template_bind(
                            TemplatedControl::corner_radius_property(),
                            TemplatedControl::corner_radius_property(),
                        )
                        .dynamic(TemplatedControl::padding_property(), "HelperButtonThemePadding")
                        .set(InputElement::focusable_property(), false)
                        .set(Visual::is_visible_property(), false)
                        .template_bind(
                            TemplatedControl::font_size_property(),
                            TemplatedControl::font_size_property(),
                        )
                        .set(Layoutable::width_property(), 30.0)
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

/// `DefaultTextBoxStyle`: a control theme of `TextBox`.
fn default_text_box_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBox>();
    // The line of a single-line box centred in its height. WinUI places it by the padding and
    //         the line height of Segoe UI, which fill MinHeight; with another font (macOS, Linux) the
    //         line was higher than the middle. A box of several lines keeps its text at the top (below).
    //         Deviation (winui-theme/DEVIATIONS.md, G-28).
    theme.set(
        TextBox::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(TemplatedControl::foreground_property(), "TextControlForeground");
    theme.dynamic(TemplatedControl::background_property(), "TextControlBackground");
    theme.dynamic(TemplatedControl::border_brush_property(), "TextControlBorderBrush");
    theme.dynamic(
        TextBox::selection_brush_property(),
        "TextControlSelectionHighlightColor",
    );
    theme.dynamic(TextBox::caret_brush_property(), "TextControlForegroundFocused");
    theme.dynamic(
        TextBox::placeholder_foreground_property(),
        "TextControlPlaceholderForeground",
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
    theme.dynamic(Layoutable::min_height_property(), "TextControlThemeMinHeight");
    theme.dynamic(Layoutable::min_width_property(), "TextControlThemeMinWidth");
    theme.dynamic(TemplatedControl::padding_property(), "TextControlThemePadding");
    theme.set_null(Control::focus_adorner_property());
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.set(
        TemplatedControl::background_sizing_property(),
        BackgroundSizing::InnerBorderEdge,
    );
    theme.static_resource(Control::context_flyout_property(), "TextBoxContextFlyout")?;
    theme.template(default_text_box_style_template);
    // A box of several lines: its text at the top, as in WinUI (G-28).
    theme.style(
        nesting().property_is(TextBox::accepts_return_property(), true),
        |style| {
            style.set(TextBox::vertical_content_alignment_property(), VerticalAlignment::Top);
            Ok(())
        },
    )?;
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
    // A validation error (FerroUI's DataValidationErrors, which WinUI 3 does not have): the
    //         border in the critical fill, as a WinUI application shows an error of a field.
    theme.style(
        nesting()
            .class(":error")
            .template()
            .of::<Border>()
            .name("BorderElement"),
        |style| {
            style.dynamic(Border::border_brush_property(), "SystemFillColorCriticalBrush");
            Ok(())
        },
    )?;
    // ButtonVisible of the group ButtonStates.
    theme.style(
        nesting()
            .property_is(TextBox::accepts_return_property(), false)
            .property_is(TextBox::is_read_only_property(), false)
            .property_is(TextBox::text_wrapping_property(), TextWrapping::NoWrap)
            .class(":focus")
            .not(of::<TextBox>().class(":empty"))
            .template()
            .of::<Button>()
            .name("DeleteButton"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    // The style class clearButton of FerroUI's text box: the delete button while the box is
    //         focused and not empty, whether or not its text wraps.
    theme.style(
        nesting()
            .class("clearButton")
            .property_is(TextBox::accepts_return_property(), false)
            .property_is(TextBox::is_read_only_property(), false)
            .class(":focus")
            .not(of::<TextBox>().class(":empty"))
            .template()
            .of::<Button>()
            .name("DeleteButton"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    // The style class revealPasswordButton of FerroUI's text box: the reveal button while the
    //         box is not empty.
    theme.style(
        nesting()
            .class("revealPasswordButton")
            .property_is(TextBox::accepts_return_property(), false)
            .property_is(TextBox::is_read_only_property(), false)
            .not(of::<TextBox>().class(":empty"))
            .template()
            .of::<ToggleButton>()
            .name("RevealButton"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    Ok(())
}

/// `DefaultPasswordBoxStyle`: a control theme of `TextBox`.
fn default_password_box_style(theme: &StyleBuilder) -> Result {
    theme.target::<TextBox>();
    theme.based_on("DefaultTextBoxStyle")?;
    theme.set(TextBox::password_char_property(), '●');
    theme.static_resource(Control::context_flyout_property(), "PasswordBoxContextFlyout")?;
    // ButtonCollapsed: a password box has no delete button.
    theme.style(
        nesting()
            .property_is(TextBox::accepts_return_property(), false)
            .property_is(TextBox::is_read_only_property(), false)
            .property_is(TextBox::text_wrapping_property(), TextWrapping::NoWrap)
            .class(":focus")
            .not(of::<TextBox>().class(":empty"))
            .template()
            .of::<Button>()
            .name("DeleteButton"),
        |style| {
            style.set(Visual::is_visible_property(), false);
            Ok(())
        },
    )?;
    // ButtonVisible (PasswordBox::CanInvokeRevealButton: focused, not empty).
    theme.style(
        nesting()
            .property_is(TextBox::accepts_return_property(), false)
            .property_is(TextBox::is_read_only_property(), false)
            .class(":focus")
            .not(of::<TextBox>().class(":empty"))
            .template()
            .of::<ToggleButton>()
            .name("RevealButton"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    Ok(())
}

/// The default control theme of `TextBox`.
fn text_box_theme(theme: &StyleBuilder) -> Result {
    theme.target::<TextBox>();
    theme.based_on("DefaultTextBoxStyle")?;
    Ok(())
}
