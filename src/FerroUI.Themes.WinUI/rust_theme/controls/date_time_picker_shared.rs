//! `Controls/DateTimePickerShared.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/DateTimePickerFlyout_themeresources.xaml (DateTimePickerFlyoutButtonStyle,
//!   DateTimePickerFlyoutLoopingSelectorNavigationButtonStyle, DefaultLoopingSelectorStyle and the
//!   implicit styles of LoopingSelector and LoopingSelectorItem; its resources are
//!   Resources/DateTimePickerFlyout_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! The themes the flyout presenters of the date and time pickers share (Controls/DatePicker.xaml,
//! Controls/TimePicker.xaml).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - DateTimePickerFlyoutButtonStyle and DateTimePickerFlyoutLoopingSelectorNavigationButtonStyle
//!   have the target type ButtonBase: they are keyed control themes of Button, which apply to
//!   Button and RepeatButton. They are not based on DefaultButtonStyle (section 4.2): the source
//!   applies them to both kinds of buttons, and the state styles of DefaultButtonStyle (which set the
//!   background, the border and the foreground of PART_ContentPresenter when the button is pressed or
//!   disabled) would set what these templates leave alone; the setters of the default style that
//!   take effect in the source (the foreground and the font) are written here.
//! - LoopingSelector: WinUI's looping list of a column of the flyouts (a ScrollViewer and the up and
//!   down RepeatButtons, shown while the pointer is over the list, in a Grid). FerroUI's
//!   DateTimePickerPanel is a panel that the presenters put in a ScrollViewer of a host panel
//!   (PART_DayHost, PART_HourHost, ...) with the up and down buttons; the ScrollViewer has the
//!   template of the source, ScrollViewerScrollBarlessTemplate (Controls/ScrollViewer.xaml), and
//!   hidden bars, so the template of DefaultLoopingSelectorStyle is written into the hosts of the
//!   presenter templates; the up and down buttons are the keyed themes
//!   DateTimePickerLoopingSelectorUpButtonStyle and DateTimePickerLoopingSelectorDownButtonStyle
//!   (the RepeatButtons UpButton and DownButton of the template, with their glyphs EDDB and EDDC at
//!   8 px as GlyphGeometryEDDB and GlyphGeometryEDDC).
//!   WinUI: DefaultLoopingSelectorStyle, {LoopingSelector}
//! - LoopingSelectorItem: the items of FerroUI's DateTimePickerPanel are ListBoxItems with a class
//!   of their column (MonthItem, DayItem, YearItem, HourItem, MinuteItem, SecondItem,
//!   TimePeriodItem): the implicit style of LoopingSelectorItem is the keyed theme
//!   DateTimePickerLoopingSelectorItemStyle of ListBoxItem, which the presenters give their items.
//!   WinUI: {LoopingSelectorItem}
//!   The padding and the alignment of the items, which the code of the WinUI presenters sets on
//!   each LoopingSelector (DatePickerFlyoutPresenterItemPadding, DatePickerFlyoutPresenterMonthPadding
//!   with the month left aligned, TimePickerFlyoutPresenterItemPadding), are setters of the classes.
//! - The selected item: WinUI draws the highlight (HighlightRect) over the items and a
//!   MonochromaticOverlayPresenter over it, which draws the selected item again in
//!   DatePickerFlyoutPresenterHighlightForegroundColor (TextOnAccentAAFillColorPrimary). The port
//!   draws the highlight under the items and the selected item in TextOnAccentAAFillColorPrimaryBrush,
//!   without a background of its own (the Selected state of the item, LoopingSelectorItemBackgroundSelected,
//!   is under the highlight in the source).
//! - The ScaleTransform of the glyph of the navigation buttons, animated to
//!   LoopingSelectorUpDownButtonScalePressed (0.875) 16 ms into the Pressed state, is the
//!   RenderTransform scale(0.875) of the pressed state (the value is written).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::transformation::TransformOperations;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::Brush;
use ferroui_base::media::Stretch;
use ferroui_base::styling::Selectors;
use ferroui_base::RelativePoint;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::ListBoxItem;
use ferroui_controls::Panel;
use ferroui_controls::RepeatButton;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::focus_adorner;
use ferroui_markup_xaml::rust_theme::named;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // The accept and dismiss buttons of the flyout presenters.
    resources.control_theme("DateTimePickerFlyoutButtonStyle", date_time_picker_flyout_button_style);
    // The up and down buttons of a looping list.
    resources.control_theme(
        "DateTimePickerFlyoutLoopingSelectorNavigationButtonStyle",
        date_time_picker_flyout_looping_selector_navigation_button_style,
    );
    // UpButton of the template of DefaultLoopingSelectorStyle.
    resources.control_theme(
        "DateTimePickerLoopingSelectorUpButtonStyle",
        date_time_picker_looping_selector_up_button_style,
    );
    // DownButton of the template of DefaultLoopingSelectorStyle.
    resources.control_theme(
        "DateTimePickerLoopingSelectorDownButtonStyle",
        date_time_picker_looping_selector_down_button_style,
    );
    // The implicit style of LoopingSelectorItem, for the items of the date and time picker presenters.
    resources.control_theme(
        "DateTimePickerLoopingSelectorItemStyle",
        date_time_picker_looping_selector_item_style,
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

/// The focus adorner of `date_time_picker_flyout_button_style`.
fn date_time_picker_flyout_button_style_focus_adorner(t: &Template<Control>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
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

/// The template of `Button` of `date_time_picker_flyout_button_style`.
fn date_time_picker_flyout_button_style_template(t: &Template<Button>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .template_bind(Panel::background_property(), TemplatedControl::background_property())
        .children([t
            .make(ContentPresenter::new())
            .name("PART_ContentPresenter")?
            .template_bind(
                ContentPresenter::background_property(),
                TemplatedControl::background_property(),
            )
            .template_bind(
                ContentPresenter::background_sizing_property(),
                TemplatedControl::background_sizing_property(),
            )
            .dynamic(
                ContentPresenter::border_brush_property(),
                "DateTimePickerFlyoutButtonBorderBrush",
            )
            .dynamic(
                ContentPresenter::border_thickness_property(),
                "DateTimeFlyoutButtonBorderThickness",
            )
            .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
            .template_bind(
                ContentPresenter::content_template_property(),
                ContentControl::content_template_property(),
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
            .template_bind(
                ContentPresenter::corner_radius_property(),
                TemplatedControl::corner_radius_property(),
            )
            .control()])
        .control())
}

/// `DateTimePickerFlyoutButtonStyle`: a control theme of `Button`.
fn date_time_picker_flyout_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    // The defaults of a WinUI 3 control, which its style leaves unset and its template binds
    // (FerroUI's defaults are Stretch). Deviation (winui-theme/DEVIATIONS.md, G-27).
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(
        TemplatedControl::background_property(),
        "DateTimePickerFlyoutButtonBackground",
    );
    theme.dynamic(TemplatedControl::foreground_property(), "ButtonForeground");
    theme.set(
        TemplatedControl::background_sizing_property(),
        BackgroundSizing::InnerBorderEdge,
    );
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    // UseSystemFocusVisuals with FocusVisualMargin="0".
    theme.set(
        Control::focus_adorner_property(),
        Some(focus_adorner(
            theme.scope(),
            date_time_picker_flyout_button_style_focus_adorner,
        )),
    );
    theme.template(date_time_picker_flyout_button_style_template);
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "DateTimePickerFlyoutButtonForegroundPointerOver",
        );
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::background_property(),
                    "DateTimePickerFlyoutButtonBackgroundPointerOver",
                );
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "DateTimePickerFlyoutButtonBorderBrushPointerOver",
                );
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "DateTimePickerFlyoutButtonForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.dynamic(
            TemplatedControl::foreground_property(),
            "DateTimePickerFlyoutButtonForegroundPressed",
        );
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::background_property(),
                    "DateTimePickerFlyoutButtonBackgroundPressed",
                );
                style.dynamic(
                    ContentPresenter::border_brush_property(),
                    "DateTimePickerFlyoutButtonBorderBrushPressed",
                );
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "DateTimePickerFlyoutButtonForegroundPressed",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}

/// The template of `Button` of `date_time_picker_flyout_looping_selector_navigation_button_style`.
fn date_time_picker_flyout_looping_selector_navigation_button_style_template(
    t: &Template<Button>,
) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .template_bind(Panel::background_property(), TemplatedControl::background_property())
        .children([t
            .make(Border::new())
            .name("RootBorder")?
            .template_bind(
                Border::background_sizing_property(),
                TemplatedControl::background_sizing_property(),
            )
            .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
            .child(
                Decorator::child_property(),
                t.make(ContentPresenter::new())
                    .name("PART_ContentPresenter")?
                    .template_bind(
                        ContentPresenter::foreground_property(),
                        TemplatedControl::foreground_property(),
                    )
                    .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                    .template_bind(
                        ContentPresenter::content_template_property(),
                        ContentControl::content_template_property(),
                    )
                    .template_bind(
                        Layoutable::horizontal_alignment_property(),
                        ContentControl::horizontal_content_alignment_property(),
                    )
                    .template_bind(
                        Layoutable::vertical_alignment_property(),
                        ContentControl::vertical_content_alignment_property(),
                    )
                    .set(
                        Visual::render_transform_origin_property(),
                        literal(RelativePoint::parse("50%,50%"))?,
                    ),
            )
            .control()])
        .control())
}

/// `DateTimePickerFlyoutLoopingSelectorNavigationButtonStyle`: a control theme of `Button`.
fn date_time_picker_flyout_looping_selector_navigation_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<Button>();
    // The defaults of a WinUI 3 control, which its style leaves unset and its template binds
    // (FerroUI's defaults are Stretch). Deviation (winui-theme/DEVIATIONS.md, G-27).
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(
        TemplatedControl::foreground_property(),
        "LoopingSelectorUpDownButtonForeground",
    );
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.dynamic(TemplatedControl::font_size_property(), "ControlContentThemeFontSize");
    theme.template(date_time_picker_flyout_looping_selector_navigation_button_style_template);
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("RootBorder"), |style| {
            style.dynamic(
                Border::background_property(),
                "LoopingSelectorUpDownButtonBackgroundPointerOver",
            );
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "LoopingSelectorUpDownButtonForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("RootBorder"), |style| {
            style.dynamic(
                Border::background_property(),
                "LoopingSelectorUpDownButtonBackgroundPressed",
            );
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "LoopingSelectorUpDownButtonForegroundPressed",
                );
                // LoopingSelectorUpDownButtonScalePressed (0.875).
                style.set(
                    Visual::render_transform_property(),
                    Some(literal(TransformOperations::parse("scale(0.875)"))?),
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}

/// The value of `Content` of `date_time_picker_looping_selector_up_button_style`, made for each control.
fn date_time_picker_looping_selector_up_button_style_content_template(
    t: &Template<Control>,
) -> Result<Option<ferroui_base::BoxedValue>> {
    // EDDB at 8 px.
    Ok(value(
        t.make(Panel::new())
            .set(Layoutable::width_property(), 8.0)
            .set(Layoutable::height_property(), 8.0)
            .children([t
                .make(Path::new())
                .static_resource(Path::data_property(), "GlyphGeometryEDDB")?
                .bind(
                    Shape::fill_property(),
                    path()
                        .ancestor_of::<RepeatButton>(0)
                        .prop(TemplatedControl::foreground_property())
                        .binding(),
                )
                .set(Shape::stretch_property(), Stretch::Uniform)
                .set(Layoutable::width_property(), 8.0)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .control()])
            .end(),
    ))
}

/// `DateTimePickerLoopingSelectorUpButtonStyle`: a control theme of `RepeatButton`.
fn date_time_picker_looping_selector_up_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<RepeatButton>();
    theme.based_on("DateTimePickerFlyoutLoopingSelectorNavigationButtonStyle")?;
    theme.dynamic(Layoutable::height_property(), "LoopingSelectorUpDownButtonHeight");
    theme.set(TemplatedControl::padding_property(), Thickness::uniform(0.0));
    theme.dynamic(Layoutable::margin_property(), "LoopingSelectorUpDownButtonMargin");
    theme.set(
        Layoutable::horizontal_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Top);
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(
        TemplatedControl::background_property(),
        "LoopingSelectorUpDownButtonBackground",
    );
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(InputElement::focusable_property(), false);
    theme.set(Visual::is_visible_property(), false);
    theme.set_template(
        ContentControl::content_property(),
        date_time_picker_looping_selector_up_button_style_content_template,
    );
    Ok(())
}

/// The value of `Content` of `date_time_picker_looping_selector_down_button_style`, made for each control.
fn date_time_picker_looping_selector_down_button_style_content_template(
    t: &Template<Control>,
) -> Result<Option<ferroui_base::BoxedValue>> {
    // EDDC at 8 px.
    Ok(value(
        t.make(Panel::new())
            .set(Layoutable::width_property(), 8.0)
            .set(Layoutable::height_property(), 8.0)
            .children([t
                .make(Path::new())
                .static_resource(Path::data_property(), "GlyphGeometryEDDC")?
                .bind(
                    Shape::fill_property(),
                    path()
                        .ancestor_of::<RepeatButton>(0)
                        .prop(TemplatedControl::foreground_property())
                        .binding(),
                )
                .set(Shape::stretch_property(), Stretch::Uniform)
                .set(Layoutable::width_property(), 8.0)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .control()])
            .end(),
    ))
}

/// `DateTimePickerLoopingSelectorDownButtonStyle`: a control theme of `RepeatButton`.
fn date_time_picker_looping_selector_down_button_style(theme: &StyleBuilder) -> Result {
    theme.target::<RepeatButton>();
    theme.based_on("DateTimePickerFlyoutLoopingSelectorNavigationButtonStyle")?;
    theme.dynamic(Layoutable::height_property(), "LoopingSelectorUpDownButtonHeight");
    theme.set(TemplatedControl::padding_property(), Thickness::uniform(0.0));
    theme.dynamic(Layoutable::margin_property(), "LoopingSelectorUpDownButtonMargin");
    theme.set(
        Layoutable::horizontal_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Bottom);
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.dynamic(
        TemplatedControl::background_property(),
        "LoopingSelectorUpDownButtonBackground",
    );
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.set(InputElement::focusable_property(), false);
    theme.set(Visual::is_visible_property(), false);
    theme.set_template(
        ContentControl::content_property(),
        date_time_picker_looping_selector_down_button_style_content_template,
    );
    Ok(())
}

/// The template of `ListBoxItem` of `date_time_picker_looping_selector_item_style`.
fn date_time_picker_looping_selector_item_style_template(t: &Template<ListBoxItem>) -> Result<Ref<Control>> {
    Ok(t.make(Border::new())
        .name("Root")?
        .set(
            Border::background_property(),
            Some(literal(Brush::parse("Transparent"))?),
        )
        .static_resource(Layoutable::margin_property(), "LoopingSelectorItemMargin")?
        .template_bind(
            Border::corner_radius_property(),
            TemplatedControl::corner_radius_property(),
        )
        .child(
            Decorator::child_property(),
            t.make(ContentPresenter::new())
                .name("PART_ContentPresenter")?
                .template_bind(
                    ContentPresenter::foreground_property(),
                    TemplatedControl::foreground_property(),
                )
                .template_bind(ContentPresenter::content_property(), ContentControl::content_property())
                .template_bind(
                    ContentPresenter::content_template_property(),
                    ContentControl::content_template_property(),
                )
                .template_bind(
                    ContentPresenter::padding_property(),
                    TemplatedControl::padding_property(),
                )
                .set(Layoutable::margin_property(), Thickness::new(2.0, 0.0, 2.0, 0.0))
                .template_bind(
                    ContentPresenter::horizontal_content_alignment_property(),
                    ContentControl::horizontal_content_alignment_property(),
                )
                .template_bind(
                    ContentPresenter::vertical_content_alignment_property(),
                    ContentControl::vertical_content_alignment_property(),
                ),
        )
        .control())
}

/// `DateTimePickerLoopingSelectorItemStyle`: a control theme of `ListBoxItem`.
fn date_time_picker_looping_selector_item_style(theme: &StyleBuilder) -> Result {
    theme.target::<ListBoxItem>();
    theme.set(
        Layoutable::horizontal_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
    theme.dynamic(TemplatedControl::foreground_property(), "LoopingSelectorItemForeground");
    // The ItemTemplate of DefaultLoopingSelectorStyle: a TextBlock in ContentControlThemeFontFamily.
    theme.dynamic(
        TemplatedControl::font_family_property(),
        "ContentControlThemeFontFamily",
    );
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.static_resource(
        TemplatedControl::padding_property(),
        "DatePickerFlyoutPresenterItemPadding",
    )?;
    theme.set(InputElement::is_tab_stop_property(), false);
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(date_time_picker_looping_selector_item_style_template);
    // The padding and alignment the presenters give each column.
    theme.style(nesting().class("MonthItem"), |style| {
        style.static_resource(
            TemplatedControl::padding_property(),
            "DatePickerFlyoutPresenterMonthPadding",
        )?;
        style.set(
            ContentControl::horizontal_content_alignment_property(),
            HorizontalAlignment::Left,
        );
        Ok(())
    })?;
    theme.style(
        Selectors::or([
            nesting().class("HourItem"),
            nesting().class("MinuteItem"),
            nesting().class("SecondItem"),
            nesting().class("TimePeriodItem"),
        ]),
        |style| {
            style.static_resource(
                TemplatedControl::padding_property(),
                "TimePickerFlyoutPresenterItemPadding",
            )?;
            Ok(())
        },
    )?;
    // PointerOver
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("Root"), |style| {
            style.dynamic(
                Border::background_property(),
                "LoopingSelectorItemBackgroundPointerOver",
            );
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "LoopingSelectorItemForegroundPointerOver",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("Root"), |style| {
            style.dynamic(Border::background_property(), "LoopingSelectorItemBackgroundPressed");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "LoopingSelectorItemForegroundPressed",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Selected: the item under the highlight, in the colour of MonochromaticOverlayPresenter.
    theme.style(nesting().class(":selected"), |style| {
        style.style(nesting().template().of::<Border>().name("Root"), |style| {
            style.set(
                Border::background_property(),
                Some(literal(Brush::parse("Transparent"))?),
            );
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "TextOnAccentAAFillColorPrimaryBrush",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    Ok(())
}
