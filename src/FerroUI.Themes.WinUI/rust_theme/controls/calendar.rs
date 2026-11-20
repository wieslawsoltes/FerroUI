//! `Controls/Calendar.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0:
//!   controls/dev/CommonStyles/CalendarView_themeresources.xaml (DefaultCalendarViewStyle and the
//!   implicit style of CalendarView; its resources are Resources/CalendarView_themeresources.xaml)
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).
//!
//! Translation notes (docs/porting/winui-theme.md, section 4):
//! - WinUI's CalendarView is one control whose template holds the header, the navigation buttons
//!   and the views. FerroUI's Calendar hosts a CalendarItem (PART_CalendarItem) in a panel
//!   (PART_Root) and the CalendarItem holds the header and the views: this document has the setters
//!   of DefaultCalendarViewStyle and the template the Calendar requires; the template of the source
//!   is the one of Controls/CalendarItem.xaml.
//! - The brush, font and margin properties of CalendarView (TodayBackground, SelectedBorderBrush,
//!   CalendarItemForeground, DayItemFontSize, ...) feed the native chrome of the day, month and year
//!   items. FerroUI's Calendar has none of them: the themes of CalendarDayButton and CalendarButton
//!   take the same resources directly (Controls/CalendarDayButton.xaml, CalendarButton.xaml), so the
//!   setters of these properties are left out here.
//! - IsTabStop="False" is left out: FerroUI's calendar takes the keyboard focus itself and moves the
//!   focused day (the :dayfocused state of its day buttons) instead of focusing its items.
//!   UseSystemFocusVisuals is FocusAdorner="{x:Null}", since the focus is shown on the day.
//! - HorizontalContentAlignment and VerticalContentAlignment (Stretch) are left out: FerroUI's
//!   Calendar has no such properties; its item stretches in the root panel.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::layout::Layoutable;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::Thickness;
use ferroui_controls::primitives::CalendarItem;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::Calendar;
use ferroui_controls::Control;
use ferroui_controls::Panel;
use ferroui_markup_xaml::rust_theme::type_key;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme("DefaultCalendarViewStyle", default_calendar_view_style);
    resources.control_theme(type_key::<Calendar>(), calendar_theme);
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

/// The template of `Calendar` of `default_calendar_view_style`.
fn default_calendar_view_style_template(t: &Template<Calendar>) -> Result<Ref<Control>> {
    Ok(t.make(Panel::new())
        .name("PART_Root")?
        .children([t
            .make(CalendarItem::new())
            .name("PART_CalendarItem")?
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
                TemplatedControl::corner_radius_property(),
                TemplatedControl::corner_radius_property(),
            )
            .template_bind(
                CalendarItem::header_background_property(),
                Calendar::header_background_property(),
            )
            .control()])
        .control())
}

/// `DefaultCalendarViewStyle`: a control theme of `Calendar`.
fn default_calendar_view_style(theme: &StyleBuilder) -> Result {
    theme.target::<Calendar>();
    theme.dynamic(TemplatedControl::foreground_property(), "CalendarViewForeground");
    theme.dynamic(TemplatedControl::background_property(), "CalendarViewBackground");
    theme.dynamic(TemplatedControl::border_brush_property(), "CalendarViewBorderBrush");
    theme.set(TemplatedControl::border_thickness_property(), Thickness::uniform(1.0));
    theme.set(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Left);
    theme.set(Layoutable::vertical_alignment_property(), VerticalAlignment::Center);
    theme.set_null(Control::focus_adorner_property());
    theme.dynamic(TemplatedControl::corner_radius_property(), "ControlCornerRadius");
    theme.template(default_calendar_view_style_template);
    Ok(())
}

/// The default control theme of `Calendar`.
fn calendar_theme(theme: &StyleBuilder) -> Result {
    theme.target::<Calendar>();
    theme.based_on("DefaultCalendarViewStyle")?;
    Ok(())
}
