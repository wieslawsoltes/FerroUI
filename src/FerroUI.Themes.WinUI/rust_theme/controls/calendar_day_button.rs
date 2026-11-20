//! `Controls/CalendarDayButton.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/CalendarView_themeresources.xaml (the implicit style of
//!   CalendarViewDayItem and the brushes DefaultCalendarViewStyle gives the items; its resources are
//!   Resources/CalendarView_themeresources.xaml)
//!   dxaml/xcp/core/core/elements/CalendarViewBaseItemChrome.cpp (the chrome WinUI draws for an
//!   item: no document, the rules of its states)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - The template of CalendarViewDayItem is empty: CalendarViewBaseItemChrome draws the item in
//!   native code, with CalendarViewBaseItemRoundedChromeEnabled (True) as a circle of half the size
//!   of the item (an outer border of CalendarItemBorderThickness, 1 px, with the background and the
//!   border of the state), an inner border of 1 px inside it (TodaySelectedInnerBorderBrush when the
//!   day is today and selected), the text of the day, and a diagonal strike-through line of 1 px over
//!   1.25 times the font size for a blackout day. The template draws the same elements: Border
//!   OuterBorder, Border InnerBorder, the content presenter and Path BlackoutStrikethrough. The
//!   corner radius is 20, half of the 40 px of an item (MinWidth, MinHeight), which the chrome
//!   computes from the actual size.
//! - The brushes are properties of CalendarView in WinUI (CalendarItemBackground, TodayBackground,
//!   SelectedBorderBrush, ...) that DefaultCalendarViewStyle sets from the resources; FerroUI's
//!   Calendar has no such properties, so the states set the same resources here.
//! - The states of the chrome (GetItemBackgroundBrush, GetItemBorderBrush, GetTextBlockForeground of
//!   CalendarViewBaseItemChrome.cpp, rounded chrome) map to the pseudo-classes of the button:
//!   hovered :pointerover, pressed :pressed, out of scope :inactive, selected :selected, blackout
//!   :blackout, today :today, disabled :disabled; the nested styles are ordered so that the last
//!   matching one gives the value the chrome chooses.
//! - The text: DayItemFontSize (CalendarViewDayItemFontSize), DayItemFontFamily (XamlAutoFontFamily),
//!   TodayFontWeight (CalendarViewTodayFontWeight) for today, the text margin DayItemMargin
//!   (CalendarViewDayItemMargin) inside the item; the labels of the first day of a month
//!   (IsGroupLabelVisible, False by default) are not ported: FerroUI's Calendar has no such labels.
//! - The Padding 0,0,0,4 of the implicit style of CalendarViewDayItem is 0,0,0,2: measured at 200 %
//!   against WinUI, 0 put the text of the days 2 px below WinUI's and 4 put it 2 px above (CDB-10).
//! - The focus visual (UseSystemFocusVisuals, FocusVisualMargin -3, CalendarViewFocusVisualPrimaryBrush
//!   of 2 px and CalendarViewFocusVisualSecondaryBrush of 1 px, a circle in the rounded chrome) is the
//!   Border FocusVisual of the template, shown in the :dayfocused state: the day buttons of FerroUI's
//!   calendar are not focused themselves, the calendar marks the day that has its focus.
//! - ClickMode="Release" is the one FerroUI's calendar needs to select by dragging over the days.

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
use ferroui_base::media::Geometry;
use ferroui_base::CornerRadius;
use ferroui_base::Thickness;
use ferroui_base::Visual;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::CalendarDayButton;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::shapes::Shape;
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
    resources.control_theme(type_key::<CalendarDayButton>(), calendar_day_button_theme);
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

/// The template of `CalendarDayButton` of `calendar_day_button_theme`.
fn calendar_day_button_theme_template(t: &Template<CalendarDayButton>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        // 1.25 x 14 px wide and high, centred (CalendarViewBaseItemChrome::SetBlackoutStrikethroughSize).
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
            t.make(Border::new())
                .name("InnerBorder")?
                .template_bind(
                    Layoutable::margin_property(),
                    TemplatedControl::border_thickness_property(),
                )
                .set(Border::border_thickness_property(), Thickness::uniform(1.0))
                .set(Border::corner_radius_property(), CornerRadius::uniform(19.0))
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
                    .dynamic(ContentPresenter::padding_property(), "CalendarViewDayItemMargin")
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
            t.make(Path::new())
                .name("BlackoutStrikethrough")?
                .set(Path::data_property(), Some(literal(Geometry::parse("M 0,0 L 18,18"))?))
                .dynamic(Shape::stroke_property(), "CalendarViewBlackoutStrikethroughBrush")
                .set(Shape::stroke_thickness_property(), 1.0)
                .set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Center)
                .set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center)
                .set(InputElement::is_hit_test_visible_property(), false)
                .set(Visual::is_visible_property(), false)
                .control(),
            t.make(Border::new())
                .name("FocusVisual")?
                .set(Layoutable::margin_property(), Thickness::uniform(-3.0))
                .set(Border::border_thickness_property(), Thickness::uniform(2.0))
                .dynamic(Border::border_brush_property(), "CalendarViewFocusVisualPrimaryBrush")
                .set(Border::corner_radius_property(), CornerRadius::uniform(23.0))
                .set(InputElement::is_hit_test_visible_property(), false)
                .set(Visual::is_visible_property(), false)
                .child(
                    Decorator::child_property(),
                    t.make(Border::new())
                        .set(Border::border_thickness_property(), Thickness::uniform(1.0))
                        .dynamic(Border::border_brush_property(), "CalendarViewFocusVisualSecondaryBrush")
                        .set(Border::corner_radius_property(), CornerRadius::uniform(21.0)),
                )
                .control(),
        ])
        .control())
}

/// The default control theme of `CalendarDayButton`.
fn calendar_day_button_theme(theme: &StyleBuilder) -> Result {
    theme.target::<CalendarDayButton>();
    theme.set(Button::click_mode_property(), ClickMode::Release);
    theme.set(Layoutable::min_width_property(), 40.0);
    theme.set(Layoutable::min_height_property(), 40.0);
    theme.set(Layoutable::margin_property(), Thickness::uniform(1.0));
    // Not the Padding 0,0,0,4 of the implicit style of CalendarViewDayItem but 0,0,0,2 (CDB-10).
    theme.set(TemplatedControl::padding_property(), Thickness::new(0.0, 0.0, 0.0, 2.0));
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
    // DayItemFontFamily (XamlAutoFontFamily) and DayItemFontSize of DefaultCalendarViewStyle.
    theme.set(
        TemplatedControl::font_family_property(),
        literal(FontFamily::parse("Segoe UI Variable Text, Segoe UI, $Default"))?,
    );
    theme.dynamic(TemplatedControl::font_size_property(), "CalendarViewDayItemFontSize");
    theme.set(TemplatedControl::font_weight_property(), FontWeight::Normal);
    theme.set(TemplatedControl::corner_radius_property(), CornerRadius::uniform(20.0));
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
    theme.template(calendar_day_button_theme_template);
    // Hovered
    theme.style(nesting().class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("OuterBorder"), |style| {
            style.dynamic(Border::background_property(), "CalendarViewCalendarItemHoverBackground");
            style.dynamic(Border::border_brush_property(), "CalendarViewHoverBorderBrush");
            Ok(())
        })?;
        Ok(())
    })?;
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
    // Out of scope (the days of the previous and the next month)
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
    // Selected
    theme.style(nesting().class(":selected"), |style| {
        style.style(nesting().template().of::<Border>().name("OuterBorder"), |style| {
            style.dynamic(Border::border_brush_property(), "CalendarViewSelectedBorderBrush");
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
                    "CalendarViewSelectedForeground",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    theme.style(nesting().class(":selected").class(":pointerover"), |style| {
        style.style(nesting().template().of::<Border>().name("OuterBorder"), |style| {
            style.dynamic(Border::border_brush_property(), "CalendarViewSelectedHoverBorderBrush");
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
                    "CalendarViewSelectedHoverForeground",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    theme.style(nesting().class(":selected").class(":pressed"), |style| {
        style.style(nesting().template().of::<Border>().name("OuterBorder"), |style| {
            style.dynamic(
                Border::border_brush_property(),
                "CalendarViewSelectedPressedBorderBrush",
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
                    "CalendarViewSelectedPressedForeground",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Blackout: the background and the border of the item do not follow the pointer.
    theme.style(nesting().class(":blackout"), |style| {
        style.style(nesting().template().of::<Border>().name("OuterBorder"), |style| {
            style.dynamic(Border::background_property(), "CalendarViewBlackoutBackground");
            style.dynamic(Border::border_brush_property(), "CalendarViewCalendarItemBorderBrush");
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
                    "CalendarViewBlackoutForeground",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Path>().name("BlackoutStrikethrough"),
            |style| {
                style.set(Visual::is_visible_property(), true);
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // Today: no border while enabled.
    theme.style(nesting().class(":today"), |style| {
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
            .class(":today")
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
            .class(":today")
            .class(":pressed")
            .template()
            .of::<Border>()
            .name("OuterBorder"),
        |style| {
            style.dynamic(Border::background_property(), "CalendarViewTodayPressedBackground");
            Ok(())
        },
    )?;
    theme.style(nesting().class(":today").class(":blackout"), |style| {
        style.style(nesting().template().of::<Border>().name("OuterBorder"), |style| {
            style.dynamic(Border::background_property(), "CalendarViewTodayBlackoutBackground");
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
                    "CalendarViewTodayBlackoutForeground",
                );
                Ok(())
            },
        )?;
        style.style(
            nesting().template().of::<Path>().name("BlackoutStrikethrough"),
            |style| {
                style.dynamic(Shape::stroke_property(), "CalendarViewTodayForeground");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    theme.style(
        nesting()
            .class(":today")
            .class(":selected")
            .template()
            .of::<Border>()
            .name("InnerBorder"),
        |style| {
            style.dynamic(
                Border::border_brush_property(),
                "CalendarViewTodaySelectedInnerBorderBrush",
            );
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
        style.style(
            nesting().template().of::<Path>().name("BlackoutStrikethrough"),
            |style| {
                style.dynamic(Shape::stroke_property(), "CalendarViewDisabledForeground");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    theme.style(nesting().class(":selected").class(":disabled"), |style| {
        style.style(nesting().template().of::<Border>().name("OuterBorder"), |style| {
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
                style.dynamic(
                    ContentPresenter::foreground_property(),
                    "CalendarViewSelectedDisabledForeground",
                );
                Ok(())
            },
        )?;
        Ok(())
    })?;
    theme.style(
        nesting()
            .class(":selected")
            .class(":blackout")
            .class(":disabled")
            .template()
            .of::<Border>()
            .name("OuterBorder"),
        |style| {
            style.dynamic(Border::border_brush_property(), "CalendarViewCalendarItemBorderBrush");
            Ok(())
        },
    )?;
    theme.style(nesting().class(":today").class(":disabled"), |style| {
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
        style.style(
            nesting().template().of::<Path>().name("BlackoutStrikethrough"),
            |style| {
                style.dynamic(Shape::stroke_property(), "CalendarViewTodayForeground");
                Ok(())
            },
        )?;
        Ok(())
    })?;
    // The focus visual of the day that has the focus of the calendar.
    theme.style(
        nesting()
            .class(":dayfocused")
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
