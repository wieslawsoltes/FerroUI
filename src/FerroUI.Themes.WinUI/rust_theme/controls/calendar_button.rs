//! `Controls/CalendarButton.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/CalendarView_themeresources.xaml (the brushes and the month and year
//!   item properties DefaultCalendarViewStyle gives the items of the year and decade views; its
//!   resources are Resources/CalendarView_themeresources.xaml)
//!   dxaml/xcp/core/core/elements/CalendarViewBaseItemChrome.cpp (the chrome WinUI draws for an
//!   item: no document, the rules of its states)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - FerroUI's CalendarButton is a month of the year view or a year of the decade view: WinUI's
//!   CalendarViewItem, which has no style; CalendarViewBaseItemChrome draws it in native code as it
//!   draws a day (Controls/CalendarDayButton.xaml): with the rounded chrome a circle of half the size
//!   of the item, here the corner radius 36.75 of the 73.5 by 74 px of an item of the year view of
//!   the calendar (Controls/CalendarItem.xaml).
//! - The states of the chrome map to the pseudo-classes of the button: hovered :pointerover, pressed
//!   :pressed, out of scope :inactive (the years outside the decade), disabled :disabled. A month or
//!   a year has no selected state in WinUI; the item of the current month or year is "today" (the
//!   accent background). FerroUI's button has no today state: its :selected marks the month or the
//!   year of the date the calendar displays, which is the current one unless the application moves
//!   it, and takes the today look.
//! - The text: MonthYearItemFontSize (CalendarViewMonthYearItemFontSize), MonthYearItemFontFamily
//!   (XamlAutoFontFamily), the margin MonthYearItemMargin (CalendarViewMonthYearItemMargin), centred;
//!   the labels of the first month of a year and the first year of a decade (IsGroupLabelVisible,
//!   False by default) are not ported.
//! - The focus visual of the item is the Border FocusVisual, shown in the :btnfocused state, as the
//!   days' (CalendarDayButton).
//! - ClickMode="Release" is the one FerroUI's calendar needs.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElement;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::media::FontFamily;
use ferroui_base::media::FontWeight;
use ferroui_base::CornerRadius;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::CalendarButton;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ClickMode;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::Panel;
use ferroui_markup_xaml::rust_theme::class;
use ferroui_markup_xaml::rust_theme::named;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<CalendarButton>(), calendar_button_theme);
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

/// The template of `CalendarButton` of `calendar_button_theme`.
fn calendar_button_theme_template(t: &Template<CalendarButton>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .children([
            t.make(Border::new())
                .name("OuterBorder")?
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
                .set(InputElement::is_hit_test_visible_property(), false)
                .control(),
            t.make(Panel::new())
                .template_bind(
                    Layoutable::margin_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .children([t
                    .make(ContentPresenter::new())
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
                    .template_bind(Layoutable::margin_property(), TemplatedControl::padding_property())
                    .dynamic(ContentPresenter::padding_property(), "CalendarViewMonthYearItemMargin")
                    .template_bind(
                        ContentPresenter::horizontal_content_alignment_property(),
                        ContentControl::horizontal_content_alignment_property(),
                    )
                    .template_bind(
                        ContentPresenter::vertical_content_alignment_property(),
                        ContentControl::vertical_content_alignment_property(),
                    )
                    .control()])
                .control(),
            t.make(Border::new())
                .name("FocusVisual")?
                .set(Layoutable::margin_property(), Thickness::uniform(-3.0))
                .set(Border::border_thickness_property(), Thickness::uniform(2.0))
                .dynamic(Border::border_brush_property(), "CalendarViewFocusVisualPrimaryBrush")
                .set(Border::corner_radius_property(), CornerRadius::uniform(39.75))
                .set(InputElement::is_hit_test_visible_property(), false)
                .set(Visual::is_visible_property(), false)
                .child(
                    Decorator::child_property(),
                    t.make(Border::new())
                        .set(Border::border_thickness_property(), Thickness::uniform(1.0))
                        .dynamic(Border::border_brush_property(), "CalendarViewFocusVisualSecondaryBrush")
                        .set(Border::corner_radius_property(), CornerRadius::uniform(37.75)),
                )
                .control(),
        ])
        .control())
}

/// The default control theme of `CalendarButton`.
fn calendar_button_theme(theme: &StyleBuilder) -> Result {
    theme.target::<CalendarButton>();
    theme.set(Button::click_mode_property(), ClickMode::Release);
    theme.set(TemplatedControl::padding_property(), Thickness::uniform(0.0));
    theme.dynamic(
        TemplatedControl::foreground_property(),
        "CalendarViewCalendarItemForeground",
    );
    theme.dynamic(
        TemplatedControl::background_property(),
        "CalendarViewCalendarItemBackground",
    );
    theme.dynamic(
        TemplatedControl::border_brush_property(),
        "CalendarViewCalendarItemBorderBrush",
    );
    // CalendarItemBorderThickness of DefaultCalendarViewStyle.
    theme.set(TemplatedControl::border_thickness_property(), Thickness::uniform(1.0));
    // MonthYearItemFontFamily (XamlAutoFontFamily) and MonthYearItemFontSize of DefaultCalendarViewStyle.
    theme.set(
        TemplatedControl::font_family_property(),
        literal(FontFamily::parse("Segoe UI Variable Text, Segoe UI, $Default"))?,
    );
    theme.dynamic(
        TemplatedControl::font_size_property(),
        "CalendarViewMonthYearItemFontSize",
    );
    theme.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
    theme.set(TemplatedControl::corner_radius_property(), CornerRadius::uniform(36.75));
    theme.set(
        Layoutable::horizontal_alignment_property(),
        HorizontalAlignment::Stretch,
    );
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Stretch);
    theme.set(
        ContentControl::horizontal_content_alignment_property(),
        HorizontalAlignment::Center,
    );
    theme.set(
        ContentControl::vertical_content_alignment_property(),
        VerticalAlignment::Center,
    );
    theme.set(Visual::clip_to_bounds_property(), false);
    theme.set_null(Control::focus_adorner_property());
    theme.template(calendar_button_theme_template);
    // Hovered
    theme.style(
        nesting()
            .class(":pointerover")
            .template()
            .of::<Border>()
            .name("OuterBorder"),
        |style| {
            style.dynamic(Border::background_property(), "CalendarViewCalendarItemHoverBackground");
            style.dynamic(Border::border_brush_property(), "CalendarViewHoverBorderBrush");
            Ok(())
        },
    )?;
    // Pressed
    theme.style(nesting().class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("OuterBorder"), |style| {
            style.dynamic(
                Border::background_property(),
                "CalendarViewCalendarItemPressedBackground",
            );
            style.dynamic(Border::border_brush_property(), "CalendarViewPressedBorderBrush");
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::foreground_property(), "CalendarViewPressedForeground");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Out of scope (the years outside the decade)
    theme.style(nesting().class(":inactive"), |style| {
        style.style(nesting().template().of::<Border>().name("OuterBorder"), |style| {
            style.dynamic(Border::background_property(), "CalendarViewOutOfScopeBackground");
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
                    "CalendarViewOutOfScopeForeground",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    theme.style(nesting().class(":inactive").class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("OuterBorder"), |style| {
            style.dynamic(Border::background_property(), "CalendarViewCalendarItemHoverBackground");
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
                    "CalendarViewOutOfScopeHoverForeground",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    theme.style(nesting().class(":inactive").class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("OuterBorder"), |style| {
            style.dynamic(
                Border::background_property(),
                "CalendarViewCalendarItemPressedBackground",
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
                    "CalendarViewOutOfScopePressedForeground",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // The month or year of the displayed date: the today state of the chrome; no border while enabled.
    theme.style(nesting().class(":selected"), |style| {
        style.style(nesting().template().of::<Border>().name("OuterBorder"), |style| {
            style.dynamic(Border::background_property(), "CalendarViewTodayBackground");
            style.set_null(Border::border_brush_property());
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::foreground_property(), "CalendarViewTodayForeground");
                style.dynamic(ContentPresenter::font_weight_property(), "CalendarViewTodayFontWeight");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    theme.style(
        nesting()
            .class(":selected")
            .class(":pointerover")
            .template()
            .of::<Border>()
            .name("OuterBorder"),
        |style| {
            style.dynamic(Border::background_property(), "CalendarViewTodayHoverBackground");
            Ok(())
        },
    )?;
    theme.style(
        nesting()
            .class(":selected")
            .class(":pressed")
            .template()
            .of::<Border>()
            .name("OuterBorder"),
        |style| {
            style.dynamic(Border::background_property(), "CalendarViewTodayPressedBackground");
            Ok(())
        },
    )?;
    // Disabled
    theme.style(nesting().class(":disabled"), |style| {
        style.style(nesting().template().of::<Border>().name("OuterBorder"), |style| {
            style.dynamic(
                Border::background_property(),
                "CalendarViewCalendarItemDisabledBackground",
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
                    "CalendarViewDisabledForeground",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    theme.style(nesting().class(":selected").class(":disabled"), |style| {
        style.style(nesting().template().of::<Border>().name("OuterBorder"), |style| {
            style.dynamic(Border::background_property(), "CalendarViewTodayDisabledBackground");
            style.dynamic(
                Border::border_brush_property(),
                "CalendarViewSelectedDisabledBorderBrush",
            );
            Ok(())
        })?;
        style.style(
            nesting()
                .template()
                .of::<ContentPresenter>()
                .name("PART_ContentPresenter"),
            |style| {
                style.dynamic(ContentPresenter::foreground_property(), "CalendarViewTodayForeground");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // The focus visual of the item that has the focus of the calendar.
    theme.style(
        nesting()
            .class(":btnfocused")
            .template()
            .of::<Border>()
            .name("FocusVisual"),
        |style| {
            style.set(Visual::is_visible_property(), true);
            Ok(())
        },
    )?;
    Ok(())
}
