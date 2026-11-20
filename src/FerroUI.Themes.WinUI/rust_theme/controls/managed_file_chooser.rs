//! `Controls/ManagedFileChooser.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/ContentDialog_themeresources.xaml (DefaultContentDialogStyle and the
//!   implicit ContentDialog style, for ManagedFileChooserOverwritePrompt; its resources are
//!   Resources/ContentDialog_themeresources.xaml)
//!   controls/dev/CommonStyles/ListViewItem_themeresources.xaml (DefaultListViewItemStyle and the
//!   chrome of ListViewItemPresenter, dxaml/xcp/core/core/elements/ListViewBaseItemChrome.cpp, for
//!   the lists of ManagedFileChooser; the resources are Resources/ListViewItem_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! ManagedFileChooser, the file dialog FerroUI draws itself, has no WinUI counterpart (a WinUI
//! application shows the file picker of the system). Its template is the one FerroUI's control
//! requires (the parts PART_QuickLinks and PART_Files, the bindings of its view model and the layout
//! of src/FerroUI.Themes.Fluent/Controls/ManagedFileChooser.xaml) composed of the WinUI controls of
//! the theme: the quick links and the entries of the folder are lists in the look of WinUI's
//! ListView (the keyed themes ManagedFileChooserListViewStyle and ManagedFileChooserListViewItemStyle,
//! a port of DefaultListViewItemStyle with the chrome ListViewItemPresenter draws: the rounded
//! backplate inset by 4,2, the selection indicator of 3 x 16 at the leading edge), the location and
//! the file name are TextBox, the actions Button, CheckBox and ComboBox; the background is the one of
//! a window (SolidBackgroundFillColorBaseBrush). The icons of the entries are the glyphs of the symbol
//! font E8B7 Folder, E7C3 Page and EDA2 HardDrive, the icons of the up and refresh buttons E74A Up and
//! E72C Refresh (Resources/Glyphs.xaml).
//!
//! ManagedFileChooserOverwritePrompt is the question the file chooser asks before it replaces a file:
//! DefaultContentDialogStyle with the content of the question and two buttons, the confirmation as
//! the primary button that is the default button (PrimaryAsDefaultButton, AccentButtonStyle) and the
//! cancellation as the close button (PrimaryAndCloseVisible). Translation notes (docs/porting/
//! winui-theme.md, section 4):
//! - The prompt is shown in a flyout of the file chooser, not as a modal dialog: LayoutRoot, the smoke
//!   layer (ContentDialogSmokeFill), the title, the scroll viewer of the content and the secondary
//!   button have no counterpart; BackgroundElement is the root of the template, DialogSpace and
//!   CommandSpace (Grid elements with a padding or a corner radius) are Border elements.
//! - The DialogShowing transition (the scale from 1.05 to 1 over ControlNormalAnimationDuration with
//!   ControlFastOutSlowInKeySpline, the opacity from 0 to 1 over ControlFasterAnimationDuration) is an
//!   animation of BackgroundElement when the template is applied; the DialogHidden transition has no
//!   counterpart (the flyout closes).
//! - FullDialogSizing, AccentColorBorder, IsPrimaryButtonEnabled and the button styles of the
//!   dialog have no counterpart in the prompt.
//! The texts of the two controls are the strings of FerroUI (StringManagedFileChooser*), which an
//! application may replace by its own resources.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::animation::easings::Easing;
use ferroui_base::animation::Animation;
use ferroui_base::animation::Cue;
use ferroui_base::animation::FillMode;
use ferroui_base::animation::KeyFrame;
use ferroui_base::animation::SignedDuration;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::converters::ObjectConverters;
use ferroui_base::data::MultiBinding;
use ferroui_base::input::InputElement;
use ferroui_base::input::KeyBinding;
use ferroui_base::input::KeyGesture;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::Orientation;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::media::ScaleTransform;
use ferroui_base::media::Stretch;
use ferroui_base::media::TextTrimming;
use ferroui_base::media::TextWrapping;
use ferroui_base::RelativePoint;
use ferroui_base::StyledElement;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::converters::StringFormatConverter;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::CheckBox;
use ferroui_controls::ColumnDefinition;
use ferroui_controls::ColumnDefinitions;
use ferroui_controls::ComboBox;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::DefinitionBase;
use ferroui_controls::Dock;
use ferroui_controls::DockPanel;
use ferroui_controls::Grid;
use ferroui_controls::GridLength;
use ferroui_controls::GridSplitter;
use ferroui_controls::ItemsControl;
use ferroui_controls::ListBox;
use ferroui_controls::ListBoxItem;
use ferroui_controls::Panel;
use ferroui_controls::RowDefinitions;
use ferroui_controls::ScrollViewer;
use ferroui_controls::StackPanel;
use ferroui_controls::TextBlock;
use ferroui_controls::TextBox;
use ferroui_dialogs::internal::ManagedFileChooserItemViewModel;
use ferroui_dialogs::internal::ManagedFileChooserViewModel;
use ferroui_dialogs::ManagedFileChooser;
use ferroui_dialogs::ManagedFileChooserOverwritePrompt;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::data_template_of;
use ferroui_markup_xaml::rust_theme::dynamic_resource_binding;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::setter;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // The texts of the file chooser (as the invariant strings of the Fluent theme of FerroUI).
    resources.text("StringManagedFileChooserFileNamePlaceholder", "File name");
    resources.text("StringManagedFileChooserShowHiddenFilesText", "Show hidden files");
    resources.text("StringManagedFileChooserOkText", "OK");
    resources.text("StringManagedFileChooserCancelText", "Cancel");
    resources.text("StringManagedFileChooserNameColumn", "Name");
    resources.text("StringManagedFileChooserDateModifiedColumn", "Date Modified");
    resources.text("StringManagedFileChooserTypeColumn", "Type");
    resources.text("StringManagedFileChooserSizeColumn", "Size");
    resources.text(
        "StringManagedFileChooserOverwritePromptFileAlreadyExistsText",
        "{0} already exists. Do you want to replace it?",
    );
    resources.text("StringManagedFileChooserOverwritePromptConfirmText", "Yes");
    resources.text("StringManagedFileChooserOverwritePromptCancelText", "No");
    // A list of the file chooser in the look of WinUI's ListView: no background, no border, the
    // items scroll vertically.
    resources.control_theme("ManagedFileChooserListViewStyle", managed_file_chooser_list_view_style);
    // An item of a list of the file chooser: DefaultListViewItemStyle, with the chrome of
    // ListViewItemPresenter (CListViewBaseItemChrome: the backplate inset by 4,2,4,2 with
    // ListViewItemCornerRadius, the selection indicator of 3 x 16 with a left margin of 4, 6 px
    // shorter while pressed, the content at ListViewItemDisabledThemeOpacity when disabled).
    resources.control_theme(
        "ManagedFileChooserListViewItemStyle",
        managed_file_chooser_list_view_item_style,
    );
    resources.control_theme(type_key::<ManagedFileChooser>(), managed_file_chooser_theme);
    // DefaultContentDialogStyle (ContentDialog_themeresources.xaml), as the overwrite prompt of the
    // file chooser.
    resources.control_theme("DefaultContentDialogStyle", default_content_dialog_style);
    resources.control_theme(
        type_key::<ManagedFileChooserOverwritePrompt>(),
        managed_file_chooser_overwrite_prompt_theme,
    );
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

/// The template of `ListBox` of `managed_file_chooser_list_view_style`.
fn managed_file_chooser_list_view_style_template(t: &Template<ListBox>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("border")?
        .template_bind(Visual::clip_to_bounds_property(), Visual::clip_to_bounds_property())
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
            t.make(ScrollViewer::new())
                .name("PART_ScrollViewer")?
                .template_bind(
                    ScrollViewer::vertical_snap_points_type_property(),
                    ScrollViewer::vertical_snap_points_type_property(),
                )
                .template_bind(
                    ScrollViewer::horizontal_snap_points_type_property(),
                    ScrollViewer::horizontal_snap_points_type_property(),
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
                    ScrollViewer::is_scroll_chaining_enabled_property(),
                    ScrollViewer::is_scroll_chaining_enabled_property(),
                )
                .template_bind(
                    ScrollViewer::is_scroll_inertia_enabled_property(),
                    ScrollViewer::is_scroll_inertia_enabled_property(),
                )
                .template_bind(
                    ScrollViewer::is_deferred_scrolling_enabled_property(),
                    ScrollViewer::is_deferred_scrolling_enabled_property(),
                )
                .template_bind(
                    ScrollViewer::allow_auto_hide_property(),
                    ScrollViewer::allow_auto_hide_property(),
                )
                .template_bind(
                    ScrollViewer::bring_into_view_on_focus_change_property(),
                    ScrollViewer::bring_into_view_on_focus_change_property(),
                )
                .content(
                    ContentControl::content_property(),
                    t.make(ItemsPresenter::new())
                        .name("PART_ItemsPresenter")?
                        .template_bind(
                            ItemsPresenter::items_panel_property(),
                            ItemsControl::items_panel_property(),
                        )
                        .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property()),
                )?,
        )
        .control())
}

/// `ManagedFileChooserListViewStyle`: a control theme of `ListBox`.
fn managed_file_chooser_list_view_style(theme: &StyleBuilder) -> Result {
    theme.target::<ListBox>();
    theme.set(
        TemplatedControl::background_property(),
        Some(literal(Brush::parse("Transparent"))?),
    );
    theme.set(TemplatedControl::border_thickness_property(), Thickness::uniform(0.0));
    theme.set(
        ScrollViewer::horizontal_scroll_bar_visibility_property(),
        ScrollBarVisibility::Disabled,
    );
    theme.set(
        ScrollViewer::vertical_scroll_bar_visibility_property(),
        ScrollBarVisibility::Auto,
    );
    theme.set(ScrollViewer::is_scroll_chaining_enabled_property(), true);
    theme.set(ScrollViewer::is_scroll_inertia_enabled_property(), true);
    theme.template(managed_file_chooser_list_view_style_template);
    Ok(())
}

/// The focus adorner of `managed_file_chooser_list_view_item_style`.
fn managed_file_chooser_list_view_item_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::margin_property(), Thickness::uniform(1.0))
        .set(Border::border_thickness_property(), Thickness::uniform(2.0))
        .dynamic(Border::border_brush_property(), "ListViewItemFocusVisualPrimaryBrush")
        .dynamic(Border::corner_radius_property(), "ControlCornerRadius")
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .set(Border::border_thickness_property(), Thickness::uniform(1.0))
                .dynamic(Border::border_brush_property(), "ListViewItemFocusVisualSecondaryBrush")
                .dynamic(Border::corner_radius_property(), "ControlCornerRadius"),
        )
        .control())
}

/// The template of `ListBoxItem` of `managed_file_chooser_list_view_item_style`.
fn managed_file_chooser_list_view_item_style_template(t: &Template<ListBoxItem>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .children([
            t.make(Border::new())
                .name("Backplate")?
                .set(Layoutable::margin_property(), Thickness::new(4.0, 2.0, 4.0, 2.0))
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
                .name("PART_ContentPresenter")?
                .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                .template_bind(
                    ContentPresenter::content_template_property(),
                    ContentControl::content_template_property(),
                )
                .template_bind(
                    ContentPresenter::foreground_property(),
                    TemplatedControl::foreground_property(),
                )
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
                .control(),
            t.make(Border::new())
                .name("SelectionIndicator")?
                .set(Layoutable::width_property(), 3.0)
                .set(Layoutable::height_property(), 16.0)
                .set(Layoutable::margin_property(), Thickness::new(4.0, 0.0, 0.0, 0.0))
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .dynamic(
                    Border::corner_radius_property(),
                    "ListViewItemSelectionIndicatorCornerRadius",
                )
                .dynamic(Border::background_property(), "ListViewItemSelectionIndicatorBrush")
                .set(Visual::is_visible_property(), false)
                .control(),
        ])
        .control())
}

/// `ManagedFileChooserListViewItemStyle`: a control theme of `ListBoxItem`.
fn managed_file_chooser_list_view_item_style(theme: &StyleBuilder) -> Result {
    theme.target::<ListBoxItem>();
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.dynamic(TemplatedControl::background_property(), "ListViewItemBackground");
    theme.dynamic(TemplatedControl::foreground_property(), "ListViewItemForeground");
    theme.set(
        TemplatedControl::padding_property(),
        Thickness::new(16.0, 0.0, 12.0, 0.0),
    );
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(Layoutable::min_width_property(), "ListViewItemMinWidth");
    theme.dynamic(Layoutable::min_height_property(), "ListViewItemMinHeight");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ListViewItemCornerRadius");
    // FocusVisualMargin="1" with the focus visual brushes of the list view item.
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(
            theme.scope(),
            managed_file_chooser_list_view_item_style_focus_adorner,
        )),
    );
    theme.template(managed_file_chooser_list_view_item_style_template);
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "ListViewItemForegroundPointerOver",
        );
        Ok(())
    })?;
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<Border>()
            .name("Backplate"),
        |style| {
            style.dynamic(Border::background_property(), "ListViewItemBackgroundPointerOver");
            Ok(())
        },
    )?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.dynamic(TemplatedControl::foreground_property(), "ListViewItemForegroundPressed");
        Ok(())
    })?;
    theme.style(
        nesting().class(":pressed").template().of::<Border>().name("Backplate"),
        |style| {
            style.dynamic(Border::background_property(), "ListViewItemBackgroundPressed");
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
            style.dynamic(Visual::opacity_property(), "ListViewItemDisabledThemeOpacity");
            Ok(())
        },
    )?;
    // Selected
    theme.style(nesting().class(":selected"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "ListViewItemForegroundSelected",
        );
        Ok(())
    })?;
    theme.style(
        nesting().class(":selected").template().of::<Border>().name("Backplate"),
        |style| {
            style.dynamic(Border::background_property(), "ListViewItemBackgroundSelected");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":selected")
            .template()
            .of::<Border>()
            .name("SelectionIndicator"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    // SelectedPointerOver
    theme.style(nesting().class(":selected").class(":pointerover"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "ListViewItemForegroundSelectedPointerOver",
        );
        Ok(())
    })?;
    theme.style(
        nesting()
            .class(":selected")
            .class(":pointerover")
            .template()
            .of::<Border>()
            .name("Backplate"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "ListViewItemBackgroundSelectedPointerOver",
            );
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":selected")
            .class(":pointerover")
            .template()
            .of::<Border>()
            .name("SelectionIndicator"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "ListViewItemSelectionIndicatorPointerOverBrush",
            );
            Ok(())
        },
    )?;
    // SelectedPressed
    theme.style(nesting().class(":selected").class(":pressed"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "ListViewItemForegroundSelectedPressed",
        );
        Ok(())
    })?;
    theme.style(
        nesting()
            .class(":selected")
            .class(":pressed")
            .template()
            .of::<Border>()
            .name("Backplate"),
        |style| {
            style.dynamic(Border::background_property(), "ListViewItemBackgroundSelectedPressed");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":selected")
            .class(":pressed")
            .template()
            .of::<Border>()
            .name("SelectionIndicator"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "ListViewItemSelectionIndicatorPressedBrush",
            );
            style.set(Layoutable::height_property(), 10.0);
            Ok(())
        },
    )?;
    // SelectedDisabled
    theme.style(
        nesting()
            .class(":selected")
            .class(":disabled")
            .template()
            .of::<Border>()
            .name("Backplate"),
        |style| {
            style.dynamic(Border::background_property(), "ListViewItemBackgroundSelectedDisabled");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":selected")
            .class(":disabled")
            .template()
            .of::<Border>()
            .name("SelectionIndicator"),
        |style| {
            style.dynamic(
                Border::background_property(),
                "ListViewItemSelectionIndicatorDisabledBrush",
            );
            Ok(())
        },
    )?;
    Ok(())
}

/// A data template of `template`.
fn template_data_template(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(StackPanel::new())
        .set(StackPanel::orientation_property(), Orientation::Horizontal)
        .set(StackPanel::spacing_property(), 12.0)
        .set(
            Panel::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        )
        .children([
            t.make(Panel::new())
                .set(Layoutable::width_property(), 16.0)
                .set(Layoutable::height_property(), 16.0)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .children([
                    t.make(Path::new())
                        .static_resource(Path::data_property(), "GlyphGeometryE8B7")?
                        .bind(
                            Shape::fill_property(),
                            path()
                                .ancestor_of::<ListBoxItem>(0)
                                .prop(TemplatedControl::foreground_property())
                                .binding(),
                        )
                        .set(Shape::stretch_property(), Stretch::Uniform)
                        .set(Layoutable::width_property(), 16.0)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                        .bind(
                            Visual::is_visible_property(),
                            path()
                                .member::<ManagedFileChooserItemViewModel>("IconKey")
                                .binding()
                                .with_converter_value(Some(ObjectConverters::equal()))
                                .with_converter_parameter(value("Icon_Folder".to_string())),
                        )
                        .control(),
                    t.make(Path::new())
                        .static_resource(Path::data_property(), "GlyphGeometryE7C3")?
                        .bind(
                            Shape::fill_property(),
                            path()
                                .ancestor_of::<ListBoxItem>(0)
                                .prop(TemplatedControl::foreground_property())
                                .binding(),
                        )
                        .set(Shape::stretch_property(), Stretch::Uniform)
                        .set(Layoutable::height_property(), 16.0)
                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                        .bind(
                            Visual::is_visible_property(),
                            path()
                                .member::<ManagedFileChooserItemViewModel>("IconKey")
                                .binding()
                                .with_converter_value(Some(ObjectConverters::equal()))
                                .with_converter_parameter(value("Icon_File".to_string())),
                        )
                        .control(),
                    t.make(Path::new())
                        .static_resource(Path::data_property(), "GlyphGeometryEDA2")?
                        .bind(
                            Shape::fill_property(),
                            path()
                                .ancestor_of::<ListBoxItem>(0)
                                .prop(TemplatedControl::foreground_property())
                                .binding(),
                        )
                        .set(Shape::stretch_property(), Stretch::Uniform)
                        .set(Layoutable::width_property(), 16.0)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                        .bind(
                            Visual::is_visible_property(),
                            path()
                                .member::<ManagedFileChooserItemViewModel>("IconKey")
                                .binding()
                                .with_converter_value(Some(ObjectConverters::equal()))
                                .with_converter_parameter(value("Icon_Volume".to_string())),
                        )
                        .control(),
                ])
                .control(),
            t.make(TextBlock::new())
                .bind(
                    TextBlock::text_property(),
                    path()
                        .member::<ManagedFileChooserItemViewModel>("DisplayName")
                        .binding(),
                )
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .set(
                    TextBlock::text_trimming_property(),
                    <dyn TextTrimming>::character_ellipsis(),
                )
                .control(),
        ])
        .control())
}

/// The template of `GridSplitter` of `element_style`.
fn element_style_template(t: &Template<GridSplitter>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
        .set(Border::border_thickness_property(), Thickness::uniform(0.0))
        .set(
            Border::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        )
        .child(
            Decorator::child_property(),
            t.make(Rectangle::new())
                .set(Layoutable::width_property(), 1.0)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch)
                .template_bind(Shape::fill_property(), TemplatedControl::background_property()),
        )
        .control())
}

/// A data template of `template`.
fn template_data_template_2(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Grid::new())
        .set(
            Panel::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        )
        .with(|e| {
            let items = Grid::column_definitions(e);
            items.add(
                t.make(ColumnDefinition::new())
                    .set(DefinitionBase::shared_size_group_property(), Some(String::from("Icon")))
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .set(DefinitionBase::shared_size_group_property(), Some(String::from("Name")))
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .set(
                        DefinitionBase::shared_size_group_property(),
                        Some(String::from("Splitter")),
                    )
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .set(
                        DefinitionBase::shared_size_group_property(),
                        Some(String::from("Modified")),
                    )
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .set(
                        DefinitionBase::shared_size_group_property(),
                        Some(String::from("Splitter")),
                    )
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .set(DefinitionBase::shared_size_group_property(), Some(String::from("Type")))
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .set(
                        DefinitionBase::shared_size_group_property(),
                        Some(String::from("Splitter")),
                    )
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .set(DefinitionBase::shared_size_group_property(), Some(String::from("Size")))
                    .end()
                    .into(),
            );
            items.add(
                t.make(ColumnDefinition::new())
                    .set(
                        DefinitionBase::shared_size_group_property(),
                        Some(String::from("Splitter")),
                    )
                    .end()
                    .into(),
            );
            Ok(())
        })?
        .children([
            t.make(Panel::new())
                .set(Layoutable::width_property(), 16.0)
                .set(Layoutable::height_property(), 16.0)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .children([
                    t.make(Path::new())
                        .static_resource(Path::data_property(), "GlyphGeometryE8B7")?
                        .bind(
                            Shape::fill_property(),
                            path()
                                .ancestor_of::<ListBoxItem>(0)
                                .prop(TemplatedControl::foreground_property())
                                .binding(),
                        )
                        .set(Shape::stretch_property(), Stretch::Uniform)
                        .set(Layoutable::width_property(), 16.0)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                        .bind(
                            Visual::is_visible_property(),
                            path()
                                .member::<ManagedFileChooserItemViewModel>("IconKey")
                                .binding()
                                .with_converter_value(Some(ObjectConverters::equal()))
                                .with_converter_parameter(value("Icon_Folder".to_string())),
                        )
                        .control(),
                    t.make(Path::new())
                        .static_resource(Path::data_property(), "GlyphGeometryE7C3")?
                        .bind(
                            Shape::fill_property(),
                            path()
                                .ancestor_of::<ListBoxItem>(0)
                                .prop(TemplatedControl::foreground_property())
                                .binding(),
                        )
                        .set(Shape::stretch_property(), Stretch::Uniform)
                        .set(Layoutable::height_property(), 16.0)
                        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                        .bind(
                            Visual::is_visible_property(),
                            path()
                                .member::<ManagedFileChooserItemViewModel>("IconKey")
                                .binding()
                                .with_converter_value(Some(ObjectConverters::equal()))
                                .with_converter_parameter(value("Icon_File".to_string())),
                        )
                        .control(),
                    t.make(Path::new())
                        .static_resource(Path::data_property(), "GlyphGeometryEDA2")?
                        .bind(
                            Shape::fill_property(),
                            path()
                                .ancestor_of::<ListBoxItem>(0)
                                .prop(TemplatedControl::foreground_property())
                                .binding(),
                        )
                        .set(Shape::stretch_property(), Stretch::Uniform)
                        .set(Layoutable::width_property(), 16.0)
                        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                        .bind(
                            Visual::is_visible_property(),
                            path()
                                .member::<ManagedFileChooserItemViewModel>("IconKey")
                                .binding()
                                .with_converter_value(Some(ObjectConverters::equal()))
                                .with_converter_parameter(value("Icon_Volume".to_string())),
                        )
                        .control(),
                ])
                .control(),
            t.make(TextBlock::new())
                .set(Grid::column_property(), 1)
                .bind(
                    TextBlock::text_property(),
                    path()
                        .member::<ManagedFileChooserItemViewModel>("DisplayName")
                        .binding(),
                )
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .set(
                    TextBlock::text_trimming_property(),
                    <dyn TextTrimming>::character_ellipsis(),
                )
                .control(),
            t.make(TextBlock::new())
                .set(Grid::column_property(), 3)
                .bind(
                    TextBlock::text_property(),
                    path().member::<ManagedFileChooserItemViewModel>("Modified").binding(),
                )
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .control(),
            t.make(TextBlock::new())
                .set(Grid::column_property(), 5)
                .bind(
                    TextBlock::text_property(),
                    path().member::<ManagedFileChooserItemViewModel>("Type").binding(),
                )
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .control(),
            t.make(TextBlock::new())
                .set(Grid::column_property(), 7)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .bind(
                    TextBlock::text_property(),
                    path().member::<ManagedFileChooserItemViewModel>("Size").binding(),
                )
                .control(),
        ])
        .control())
}

/// The template of `ManagedFileChooser` of `managed_file_chooser_theme`.
fn managed_file_chooser_theme_template(t: &Template<ManagedFileChooser>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
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
        .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
        .child(
            Decorator::child_property(),
            t.make(DockPanel::new())
                // The up button, the location and the refresh button.
                // The file name, the filters, hidden files and the two actions.
                // The column headers over the entries of the folder; the columns are aligned with the
                // content of the items (their padding, 16) and share their widths with them.
                .children([
                    t.make(ListBox::new())
                        .name("PART_QuickLinks")?
                        .set(DockPanel::dock_property(), Dock::Left)
                        .set(Layoutable::width_property(), 160.0)
                        .static_resource(StyledElement::theme_property(), "ManagedFileChooserListViewStyle")?
                        .static_resource(
                            ItemsControl::item_container_theme_property(),
                            "ManagedFileChooserListViewItemStyle",
                        )?
                        .bind(
                            ItemsControl::items_source_property(),
                            path().member::<ManagedFileChooserViewModel>("QuickLinks").binding(),
                        )
                        .bind(
                            SelectingItemsControl::selected_index_property(),
                            path()
                                .member::<ManagedFileChooserViewModel>("QuickLinksSelectedIndex")
                                .binding(),
                        )
                        .set(InputElement::focusable_property(), false)
                        .set(
                            ItemsControl::item_template_property(),
                            Some(data_template_of::<ManagedFileChooserItemViewModel>(
                                t.scope(),
                                template_data_template,
                            )),
                        )
                        .control(),
                    t.make(Grid::new())
                        .name("NavBar")?
                        .set(DockPanel::dock_property(), Dock::Top)
                        .set(Layoutable::margin_property(), Thickness::new(12.0, 5.0, 12.0, 5.0))
                        .with(|e| {
                            e.set_column_definitions(literal(ColumnDefinitions::parse("Auto,*,Auto"))?);
                            Ok(())
                        })?
                        .set(Grid::column_spacing_property(), 8.0)
                        .children([
                            t.make(Button::new())
                                .set(Grid::column_property(), 0)
                                .bind(
                                    Button::command_property(),
                                    path()
                                        .method::<ManagedFileChooserViewModel>("GoUp", |this| {
                                            let _ = this.go_up();
                                        })
                                        .binding(),
                                )
                                .content(
                                    ContentControl::content_property(),
                                    t.make(Path::new())
                                        .static_resource(Path::data_property(), "GlyphGeometryE74A")?
                                        .bind(
                                            Shape::fill_property(),
                                            path()
                                                .ancestor_of::<Button>(0)
                                                .prop(TemplatedControl::foreground_property())
                                                .binding(),
                                        )
                                        .set(Shape::stretch_property(), Stretch::Uniform)
                                        .set(Layoutable::width_property(), 14.0),
                                )?
                                .control(),
                            t.make(TextBox::new())
                                .name("Location")?
                                .set(Grid::column_property(), 1)
                                .bind(
                                    TextBox::text_property(),
                                    path().member::<ManagedFileChooserViewModel>("Location").binding(),
                                )
                                .with(|e| {
                                    let items = InputElement::key_bindings(e);
                                    items.add(
                                        t.make(KeyBinding::new())
                                            .bind(
                                                KeyBinding::command_property(),
                                                path()
                                                    .method::<ManagedFileChooserViewModel>("EnterPressed", |this| {
                                                        let _ = this.enter_pressed();
                                                    })
                                                    .binding(),
                                            )
                                            .set(
                                                KeyBinding::gesture_property(),
                                                Some(literal(KeyGesture::parse("Enter"))?),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    Ok(())
                                })?
                                .control(),
                            t.make(Button::new())
                                .set(Grid::column_property(), 2)
                                .bind(
                                    Button::command_property(),
                                    path()
                                        .method::<ManagedFileChooserViewModel>("Refresh", |this| {
                                            let _ = this.refresh();
                                        })
                                        .binding(),
                                )
                                .content(
                                    ContentControl::content_property(),
                                    t.make(Path::new())
                                        .static_resource(Path::data_property(), "GlyphGeometryE72C")?
                                        .bind(
                                            Shape::fill_property(),
                                            path()
                                                .ancestor_of::<Button>(0)
                                                .prop(TemplatedControl::foreground_property())
                                                .binding(),
                                        )
                                        .set(Shape::stretch_property(), Stretch::Uniform)
                                        .set(Layoutable::width_property(), 14.0),
                                )?
                                .control(),
                        ])
                        .control(),
                    t.make(Grid::new())
                        .set(DockPanel::dock_property(), Dock::Bottom)
                        .set(Layoutable::margin_property(), Thickness::new(12.0, 0.0, 12.0, 5.0))
                        .with(|e| {
                            e.set_row_definitions(literal(RowDefinitions::parse("Auto,Auto"))?);
                            Ok(())
                        })?
                        .set(Grid::row_spacing_property(), 4.0)
                        .children([
                            t.make(DockPanel::new())
                                .set(Grid::row_property(), 0)
                                .children([
                                    t.make(ComboBox::new())
                                        .set(DockPanel::dock_property(), Dock::Right)
                                        .set(Layoutable::margin_property(), Thickness::new(8.0, 0.0, 0.0, 0.0))
                                        .bind(
                                            Visual::is_visible_property(),
                                            path().member::<ManagedFileChooserViewModel>("ShowFilters").binding(),
                                        )
                                        .bind(
                                            ItemsControl::items_source_property(),
                                            path().member::<ManagedFileChooserViewModel>("Filters").binding(),
                                        )
                                        .bind(
                                            SelectingItemsControl::selected_item_property(),
                                            path().member::<ManagedFileChooserViewModel>("SelectedFilter").binding(),
                                        )
                                        .control(),
                                    t.make(TextBox::new())
                                        .bind(
                                            TextBox::text_property(),
                                            path().member::<ManagedFileChooserViewModel>("FileName").binding(),
                                        )
                                        .dynamic(
                                            TextBox::placeholder_text_property(),
                                            "StringManagedFileChooserFileNamePlaceholder",
                                        )
                                        .bind(
                                            Visual::is_visible_property(),
                                            path()
                                                .member::<ManagedFileChooserViewModel>("SelectingFolder")
                                                .not()
                                                .binding(),
                                        )
                                        .control(),
                                ])
                                .control(),
                            t.make(CheckBox::new())
                                .set(Grid::row_property(), 1)
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left)
                                .bind(
                                    ToggleButton::is_checked_property(),
                                    path()
                                        .member::<ManagedFileChooserViewModel>("ShowHiddenFiles")
                                        .binding(),
                                )
                                .dynamic(
                                    ContentControl::content_property(),
                                    "StringManagedFileChooserShowHiddenFilesText",
                                )
                                .control(),
                            t.make(StackPanel::new())
                                .name("Finalize")?
                                .set(Grid::row_property(), 1)
                                .set(StackPanel::orientation_property(), Orientation::Horizontal)
                                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Right)
                                .set(StackPanel::spacing_property(), 4.0)
                                .children([
                                    t.make(Button::new())
                                        .bind(
                                            Button::command_property(),
                                            path()
                                                .method::<ManagedFileChooserViewModel>("Ok", |this| {
                                                    let _ = this.ok();
                                                })
                                                .binding(),
                                        )
                                        .set(Layoutable::min_width_property(), 80.0)
                                        .set(
                                            ContentControl::horizontal_content_alignment_property(),
                                            HorizontalAlignment::Center,
                                        )
                                        .dynamic(ContentControl::content_property(), "StringManagedFileChooserOkText")
                                        .control(),
                                    t.make(Button::new())
                                        .bind(
                                            Button::command_property(),
                                            path()
                                                .method::<ManagedFileChooserViewModel>("Cancel", |this| {
                                                    let _ = this.cancel();
                                                })
                                                .binding(),
                                        )
                                        .set(Layoutable::min_width_property(), 80.0)
                                        .set(
                                            ContentControl::horizontal_content_alignment_property(),
                                            HorizontalAlignment::Center,
                                        )
                                        .dynamic(
                                            ContentControl::content_property(),
                                            "StringManagedFileChooserCancelText",
                                        )
                                        .control(),
                                ])
                                .control(),
                        ])
                        .control(),
                    t.make(DockPanel::new())
                        .set(Grid::is_shared_size_scope_property(), true)
                        .children([
                            t.make(Grid::new())
                                .set(DockPanel::dock_property(), Dock::Top)
                                .set(Layoutable::margin_property(), Thickness::new(16.0, 5.0, 0.0, 0.0))
                                .set(
                                    Layoutable::horizontal_alignment_property(),
                                    HorizontalAlignment::Stretch,
                                )
                                .with(|e| {
                                    let items = Grid::column_definitions(e);
                                    items.add(
                                        t.make(ColumnDefinition::new())
                                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("28"))?)
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("Icon")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(ColumnDefinition::new())
                                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("275"))?)
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("Name")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(ColumnDefinition::new())
                                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("16"))?)
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("Splitter")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(ColumnDefinition::new())
                                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("200"))?)
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("Modified")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(ColumnDefinition::new())
                                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("16"))?)
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("Splitter")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(ColumnDefinition::new())
                                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("150"))?)
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("Type")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(ColumnDefinition::new())
                                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("16"))?)
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("Splitter")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(ColumnDefinition::new())
                                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("200"))?)
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("Size")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    items.add(
                                        t.make(ColumnDefinition::new())
                                            .set(ColumnDefinition::width_property(), literal(GridLength::parse("16"))?)
                                            .set(
                                                DefinitionBase::shared_size_group_property(),
                                                Some(String::from("Splitter")),
                                            )
                                            .end()
                                            .into(),
                                    );
                                    Ok(())
                                })?
                                .style(of::<GridSplitter>(), |style| {
                                    style.dynamic(
                                        TemplatedControl::background_property(),
                                        "DividerStrokeColorDefaultBrush",
                                    );
                                    style.template(element_style_template);
                                    Ok(())
                                })?
                                .children([
                                    t.make(TextBlock::new())
                                        .set(Grid::column_property(), 1)
                                        .dynamic(TextBlock::text_property(), "StringManagedFileChooserNameColumn")
                                        .control(),
                                    t.make(GridSplitter::new()).set(Grid::column_property(), 2).control(),
                                    t.make(TextBlock::new())
                                        .set(Grid::column_property(), 3)
                                        .dynamic(
                                            TextBlock::text_property(),
                                            "StringManagedFileChooserDateModifiedColumn",
                                        )
                                        .control(),
                                    t.make(GridSplitter::new()).set(Grid::column_property(), 4).control(),
                                    t.make(TextBlock::new())
                                        .set(Grid::column_property(), 5)
                                        .dynamic(TextBlock::text_property(), "StringManagedFileChooserTypeColumn")
                                        .control(),
                                    t.make(GridSplitter::new()).set(Grid::column_property(), 6).control(),
                                    t.make(TextBlock::new())
                                        .set(Grid::column_property(), 7)
                                        .dynamic(TextBlock::text_property(), "StringManagedFileChooserSizeColumn")
                                        .control(),
                                    t.make(GridSplitter::new()).set(Grid::column_property(), 8).control(),
                                ])
                                .control(),
                            t.make(ListBox::new())
                                .name("PART_Files")?
                                .set(Layoutable::margin_property(), Thickness::symmetric(0.0, 5.0))
                                .static_resource(StyledElement::theme_property(), "ManagedFileChooserListViewStyle")?
                                .static_resource(
                                    ItemsControl::item_container_theme_property(),
                                    "ManagedFileChooserListViewItemStyle",
                                )?
                                .bind(
                                    ItemsControl::items_source_property(),
                                    path().member::<ManagedFileChooserViewModel>("Items").binding(),
                                )
                                .bind(
                                    SelectingItemsControl::selection_mode_property(),
                                    path().member::<ManagedFileChooserViewModel>("SelectionMode").binding(),
                                )
                                .bind(
                                    SelectingItemsControl::selected_items_property(),
                                    path().member::<ManagedFileChooserViewModel>("SelectedItems").binding(),
                                )
                                .set(
                                    ItemsControl::item_template_property(),
                                    Some(data_template_of::<ManagedFileChooserItemViewModel>(
                                        t.scope(),
                                        template_data_template_2,
                                    )),
                                )
                                .control(),
                        ])
                        .control(),
                ]),
        )
        .control())
}

/// The default control theme of `ManagedFileChooser`.
fn managed_file_chooser_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ManagedFileChooser>();
    theme.dynamic(
        TemplatedControl::background_property(),
        "SolidBackgroundFillColorBaseBrush",
    );
    theme.dynamic(TemplatedControl::foreground_property(), "TextFillColorPrimaryBrush");
    theme.template(managed_file_chooser_theme_template);
    Ok(())
}

/// The template of `ManagedFileChooserOverwritePrompt` of `default_content_dialog_style`.
fn default_content_dialog_style_template(t: &Template<ManagedFileChooserOverwritePrompt>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("BackgroundElement")?
        .template_bind(Border::background_property(), TemplatedControl::background_property())
        .template_bind(
            Border::border_thickness_property(),
            TemplatedControl::border_thickness_property(),
        )
        .template_bind(
            Border::border_brush_property(),
            TemplatedControl::border_brush_property(),
        )
        .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
        .dynamic(Layoutable::min_width_property(), "ContentDialogMinWidth")
        .dynamic(Layoutable::max_width_property(), "ContentDialogMaxWidth")
        .dynamic(Layoutable::min_height_property(), "ContentDialogMinHeight")
        .dynamic(Layoutable::max_height_property(), "ContentDialogMaxHeight")
        .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
        .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
        .set(
            Visual::render_transform_origin_property(),
            literal(RelativePoint::parse("50%,50%"))?,
        )
        .child(
            Decorator::child_property(),
            t.make(Border::new())
                .name("DialogSpace")?
                .set(Visual::clip_to_bounds_property(), true)
                .dynamic(Border::corner_radius_property(), "OverlayCornerRadius")
                .child(
                    Decorator::child_property(),
                    t.make(Grid::new())
                        .with(|e| {
                            e.set_row_definitions(literal(RowDefinitions::parse("*,Auto"))?);
                            Ok(())
                        })?
                        // CommandSpace in PrimaryAndCloseVisible: the primary button, the spacing
                        // ContentDialogButtonSpacing (8) and the close button.
                        .children([
                            t.make(Border::new())
                                .dynamic(Border::background_property(), "ContentDialogTopOverlay")
                                .dynamic(Decorator::padding_property(), "ContentDialogPadding")
                                .dynamic(Border::border_thickness_property(), "ContentDialogSeparatorThickness")
                                .dynamic(Border::border_brush_property(), "ContentDialogSeparatorBorderBrush")
                                .set(Border::background_sizing_property(), BackgroundSizing::InnerBorderEdge)
                                .child(
                                    Decorator::child_property(),
                                    t.make(TextBlock::new())
                                        .name("Content")?
                                        .static_resource(
                                            TextBlock::font_size_property(),
                                            "ControlContentThemeFontSize",
                                        )?
                                        .static_resource(
                                            TextBlock::font_family_property(),
                                            "ContentControlThemeFontFamily",
                                        )?
                                        .template_bind(
                                            TextBlock::foreground_property(),
                                            TemplatedControl::foreground_property(),
                                        )
                                        .set(TextBlock::text_wrapping_property(), TextWrapping::Wrap)
                                        .bind(TextBlock::text_property(), {
                                            let binding = MultiBinding::new().with_converter_value(Some(
                                                ::std::rc::Rc::new(StringFormatConverter::new()),
                                            ));
                                            let bindings = binding.bindings();
                                            bindings.add(dynamic_resource_binding(
                                                "StringManagedFileChooserOverwritePromptFileAlreadyExistsText",
                                            ));
                                            bindings.add(
                                                path()
                                                    .templated_parent()
                                                    .prop(ManagedFileChooserOverwritePrompt::file_name_property())
                                                    .binding(),
                                            );
                                            binding
                                        }),
                                )
                                .control(),
                            t.make(Border::new())
                                .name("CommandSpace")?
                                .set(Grid::row_property(), 1)
                                .set(
                                    Layoutable::horizontal_alignment_property(),
                                    HorizontalAlignment::Stretch,
                                )
                                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom)
                                .dynamic(Decorator::padding_property(), "ContentDialogPadding")
                                .template_bind(Border::background_property(), TemplatedControl::background_property())
                                .child(
                                    Decorator::child_property(),
                                    t.make(Grid::new())
                                        .with(|e| {
                                            e.set_column_definitions(literal(ColumnDefinitions::parse("*,8,*"))?);
                                            Ok(())
                                        })?
                                        .children([
                                            t.make(Button::new())
                                                .name("PrimaryButton")?
                                                .set(Grid::column_property(), 0)
                                                .static_resource(StyledElement::theme_property(), "AccentButtonStyle")?
                                                .set(
                                                    Layoutable::horizontal_alignment_property(),
                                                    HorizontalAlignment::Stretch,
                                                )
                                                .set(
                                                    ContentControl::horizontal_content_alignment_property(),
                                                    HorizontalAlignment::Center,
                                                )
                                                .set(Button::is_default_property(), true)
                                                .dynamic(
                                                    ContentControl::content_property(),
                                                    "StringManagedFileChooserOverwritePromptConfirmText",
                                                )
                                                .bind(
                                                    Button::command_property(),
                                                    path()
                                                        .templated_parent()
                                                        .method::<Ref<ManagedFileChooserOverwritePrompt>>(
                                                            "Confirm",
                                                            |this| {
                                                                let _ = this.confirm();
                                                            },
                                                        )
                                                        .binding(),
                                                )
                                                .control(),
                                            t.make(Button::new())
                                                .name("CloseButton")?
                                                .set(Grid::column_property(), 2)
                                                .set(
                                                    Layoutable::horizontal_alignment_property(),
                                                    HorizontalAlignment::Stretch,
                                                )
                                                .set(
                                                    ContentControl::horizontal_content_alignment_property(),
                                                    HorizontalAlignment::Center,
                                                )
                                                .set(Button::is_cancel_property(), true)
                                                .dynamic(
                                                    ContentControl::content_property(),
                                                    "StringManagedFileChooserOverwritePromptCancelText",
                                                )
                                                .bind(
                                                    Button::command_property(),
                                                    path()
                                                        .templated_parent()
                                                        .method::<Ref<ManagedFileChooserOverwritePrompt>>(
                                                            "Cancel",
                                                            |this| {
                                                                let _ = this.cancel();
                                                            },
                                                        )
                                                        .binding(),
                                                )
                                                .control(),
                                        ]),
                                )
                                .control(),
                        ]),
                ),
        )
        .control())
}

/// `DefaultContentDialogStyle`: a control theme of `ManagedFileChooserOverwritePrompt`.
fn default_content_dialog_style(theme: &StyleBuilder) -> Result {
    theme.target::<ManagedFileChooserOverwritePrompt>();
    theme.dynamic(TemplatedControl::foreground_property(), "ContentDialogForeground");
    theme.dynamic(TemplatedControl::background_property(), "ContentDialogBackground");
    theme.dynamic(
        TemplatedControl::border_thickness_property(),
        "ContentDialogBorderWidth",
    );
    theme.dynamic(TemplatedControl::border_brush_property(), "ContentDialogBorderBrush");
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.dynamic(TemplatedControl::corner_radius_property(), "OverlayCornerRadius");
    theme.template(default_content_dialog_style_template);
    // The DialogShowing transition: ControlFasterAnimationDuration (83 ms) for the opacity,
    // ControlNormalAnimationDuration (250 ms) with ControlFastOutSlowInKeySpline (0,0,0,1) for the
    // scale.
    theme.style(nesting().template().of::<Border>().name("BackgroundElement"), |style| {
        style.animation(
            style
                .scope()
                .make(Animation::new())
                .set(Animation::duration_property(), SignedDuration::from_milliseconds(83.0))
                .set(Animation::fill_mode_property(), FillMode::Forward)
                .with(|e| {
                    let items = Animation::children(e);
                    items.add(
                        style
                            .scope()
                            .make(KeyFrame::new())
                            .with(|e| {
                                e.set_cue(literal(Cue::parse("0%"))?);
                                Ok(())
                            })?
                            .with(|e| {
                                let items = KeyFrame::setters(e);
                                items.add(setter(Visual::opacity_property(), 0.0));
                                Ok(())
                            })?
                            .end()
                            .into(),
                    );
                    items.add(
                        style
                            .scope()
                            .make(KeyFrame::new())
                            .with(|e| {
                                e.set_cue(literal(Cue::parse("100%"))?);
                                Ok(())
                            })?
                            .with(|e| {
                                let items = KeyFrame::setters(e);
                                items.add(setter(Visual::opacity_property(), 1.0));
                                Ok(())
                            })?
                            .end()
                            .into(),
                    );
                    Ok(())
                })?
                .end(),
        );
        style.animation(
            style
                .scope()
                .make(Animation::new())
                .set(Animation::duration_property(), SignedDuration::from_milliseconds(250.0))
                .set(Animation::easing_property(), literal(Easing::parse("0,0,0,1"))?)
                .set(Animation::fill_mode_property(), FillMode::Forward)
                .with(|e| {
                    let items = Animation::children(e);
                    items.add(
                        style
                            .scope()
                            .make(KeyFrame::new())
                            .with(|e| {
                                e.set_cue(literal(Cue::parse("0%"))?);
                                Ok(())
                            })?
                            .with(|e| {
                                let items = KeyFrame::setters(e);
                                items.add(setter(ScaleTransform::scale_x_property(), 1.05));
                                items.add(setter(ScaleTransform::scale_y_property(), 1.05));
                                Ok(())
                            })?
                            .end()
                            .into(),
                    );
                    items.add(
                        style
                            .scope()
                            .make(KeyFrame::new())
                            .with(|e| {
                                e.set_cue(literal(Cue::parse("100%"))?);
                                Ok(())
                            })?
                            .with(|e| {
                                let items = KeyFrame::setters(e);
                                items.add(setter(ScaleTransform::scale_x_property(), 1.0));
                                items.add(setter(ScaleTransform::scale_y_property(), 1.0));
                                Ok(())
                            })?
                            .end()
                            .into(),
                    );
                    Ok(())
                })?
                .end(),
        );
        Ok(())
    })?;
    Ok(())
}

/// The default control theme of `ManagedFileChooserOverwritePrompt`.
fn managed_file_chooser_overwrite_prompt_theme(theme: &StyleBuilder) -> Result {
    theme.target::<ManagedFileChooserOverwritePrompt>();
    theme.based_on("DefaultContentDialogStyle")?;
    Ok(())
}
