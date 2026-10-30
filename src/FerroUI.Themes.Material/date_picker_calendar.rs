// Copyright 2022-2026 The Android Open Source Project (reference geometry).
// Copyright 2026 FerroUI contributors (Rust adaptation).
// SPDX-License-Identifier: Apache-2.0
//! Material calendar day surfaces and a bounded, virtualized range-month list.
//! Reference: DatePicker.kt and DateRangePicker.kt at AndroidX
//! e4d933c98df7baa7ea2b83eea6e086915961e0ff.

use crate::date_picker::MaterialDatePicker;
use crate::date_picker_state::{MaterialDateSelectionMode, month_leading_days};
use ferroui_base::input::{InputElementImpl, InputElementImplExt, KeyEventArgs};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::{BoxShadows, DrawingContext, FontWeight};
use ferroui_base::utilities::DateTime;
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    Rect, Ref, Size, StyledElementImpl, StyledProperty, Thickness, VisualImpl, VisualImplExt,
    WeakRef, ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_controls::automation::peers::{
    AutomationPeer, AutomationPeerImpl, ButtonAutomationPeer, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use ferroui_controls::automation::provider::{ISelectionItemProvider, ISelectionProvider};
use ferroui_controls::automation::{AutomationProperties, ElementNotEnabledException};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Button, ButtonImpl, ButtonImplExt, ContentControlImpl, Control, ControlImpl, Panel, PanelImpl,
    TextBlock,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A native Button with a selection-item automation contract. The 48dp hit cell is
/// deliberately larger than its 40dp day surface (72x36dp for a year).
#[repr(C)]
pub struct MaterialDateCell {
    base: Button,
    owner: RefCell<Option<WeakRef<MaterialDatePicker>>>,
    date: Cell<Option<DateTime>>,
    year: Cell<Option<i32>>,
}
ferro_class!(MaterialDateCell: Button);
ferro_class_info!(MaterialDateCell { new: MaterialDateCell::new,
    markup: { attributes: [PseudoClasses(":selected", ":today", ":in-range", ":year")] },
});
ferro_impl_classes!(MaterialDateCell: StyledElementImpl, VisualImpl, InteractiveImpl,
    LayoutableImpl, ContentControlImpl, TemplatedControlImpl);
ferro_properties! { impl MaterialDateCell {
    pub fn is_selected_property() -> StyledProperty<bool> { FerroProperty::register::<Self, _>("IsSelected", false) }
    pub fn is_today_property() -> StyledProperty<bool> { FerroProperty::register::<Self, _>("IsToday", false) }
    pub fn is_in_range_property() -> StyledProperty<bool> { FerroProperty::register::<Self, _>("IsInRange", false) }
    pub fn is_year_property() -> StyledProperty<bool> { FerroProperty::register::<Self, _>("IsYear", false) }
} }
impl FerroObjectImpl for MaterialDateCell {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        match change.property().name() {
            "IsSelected" => this.pseudo_classes().set(":selected", this.is_selected()),
            "IsToday" => this.pseudo_classes().set(":today", this.is_today()),
            "IsInRange" => this.pseudo_classes().set(":in-range", this.is_in_range()),
            "IsYear" => this.pseudo_classes().set(":year", this.is_year()),
            _ => false,
        };
    }
}
impl InputElementImpl for MaterialDateCell {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        if this.owner().is_some_and(|owner| owner.cell_key(this, e)) {
            e.set_handled(true);
        }
        Self::parent_on_key_down(this, e);
    }
}
impl ButtonImpl for MaterialDateCell {
    fn on_click(this: &Self) {
        if this.is_effectively_enabled() {
            this.select();
        }
        Self::parent_on_click(this);
    }
}
impl ControlImpl for MaterialDateCell {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        MaterialDateCellAutomationPeer::new(this).upcast()
    }
}
impl MaterialDateCell {
    pub fn construct() -> Self {
        Self {
            base: Button::construct(),
            owner: RefCell::new(None),
            date: Cell::new(None),
            year: Cell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn is_selected(&self) -> bool {
        self.get_value(Self::is_selected_property())
    }
    pub fn set_is_selected(&self, v: bool) {
        self.set_value(Self::is_selected_property(), v);
    }
    pub fn is_today(&self) -> bool {
        self.get_value(Self::is_today_property())
    }
    pub fn set_is_today(&self, v: bool) {
        self.set_value(Self::is_today_property(), v);
    }
    pub fn is_in_range(&self) -> bool {
        self.get_value(Self::is_in_range_property())
    }
    pub fn set_is_in_range(&self, v: bool) {
        self.set_value(Self::is_in_range_property(), v);
    }
    pub fn is_year(&self) -> bool {
        self.get_value(Self::is_year_property())
    }
    pub fn set_is_year(&self, v: bool) {
        self.set_value(Self::is_year_property(), v);
    }
    pub fn date(&self) -> Option<DateTime> {
        self.date.get()
    }
    pub fn year(&self) -> Option<i32> {
        self.year.get()
    }
    pub(crate) fn owner(&self) -> Option<Ref<MaterialDatePicker>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    pub(crate) fn for_date(owner: &MaterialDatePicker, date: DateTime) -> Ref<Self> {
        let cell = Self::new();
        *cell.owner.borrow_mut() = Some(owner.to_ref().downgrade());
        cell.date.set(Some(date));
        cell.set_name(Some(format!(
            "Date_{:04}_{:02}_{:02}",
            date.year(),
            date.month(),
            date.day()
        )));
        cell.set_content(Some(Rc::new(date.day().to_string()) as BoxedValue));
        cell.refresh();
        cell
    }
    pub(crate) fn for_year(owner: &MaterialDatePicker, year: i32) -> Ref<Self> {
        let cell = Self::new();
        *cell.owner.borrow_mut() = Some(owner.to_ref().downgrade());
        cell.year.set(Some(year));
        cell.set_is_year(true);
        cell.set_name(Some(format!("Year_{year}")));
        cell.set_content(Some(Rc::new(year.to_string()) as BoxedValue));
        cell.refresh();
        cell
    }
    pub(crate) fn refresh(&self) {
        let Some(owner) = self.owner() else { return };
        let state = owner.state();
        if let Some(date) = self.date() {
            self.set_is_selected(
                state.selected_start == Some(date) || state.selected_end == Some(date),
            );
            self.set_is_today(date == owner.today());
            self.set_is_in_range(state.is_in_range(date));
            self.set_is_enabled(owner.is_date_selectable(date));
            let mut description = Vec::new();
            if state.selection_mode == MaterialDateSelectionMode::Range
                && state.selected_end.is_some()
            {
                if state.selected_start == Some(date) {
                    description.push("Start date".to_owned());
                } else if state.selected_end == Some(date) {
                    description.push("End date".to_owned());
                } else if state.is_in_range(date) {
                    description.push("In range".to_owned());
                }
            }
            if self.is_today() {
                description.push("Today".to_owned());
            }
            description.push(owner.format_date_description(date));
            AutomationProperties::set_name(self, Some(&description.join(", ")));
        } else if let Some(year) = self.year() {
            self.set_is_selected(state.displayed_month.year() == year);
            self.set_is_today(owner.today().year() == year);
            self.set_is_enabled(owner.is_year_selectable(year));
            AutomationProperties::set_name(self, Some(&format!("Navigate to year {year}")));
        }
    }
    pub(crate) fn select(&self) {
        if let Some(owner) = self.owner() {
            if let Some(date) = self.date() {
                owner.select_date(date);
            } else if let Some(year) = self.year() {
                owner.select_year(year);
            }
        }
    }
}

#[repr(C)]
pub struct MaterialDateCellAutomationPeer {
    base: ButtonAutomationPeer,
}
ferro_class!(MaterialDateCellAutomationPeer: ButtonAutomationPeer);
ferro_class_info!(MaterialDateCellAutomationPeer {
    interfaces: [Rc<dyn ISelectionItemProvider> => MaterialDateCellAutomationPeer::provider]
});
ferro_impl_classes!(MaterialDateCellAutomationPeer: FerroObjectImpl, ControlAutomationPeerImpl);
impl AutomationPeerImpl for MaterialDateCellAutomationPeer {
    fn get_class_name_core(_: &Self) -> String {
        "MaterialDateCell".into()
    }
}
impl MaterialDateCellAutomationPeer {
    pub fn new(owner: &MaterialDateCell) -> Ref<Self> {
        instantiate(Self {
            base: ButtonAutomationPeer::construct(owner),
        })
    }
    fn provider(peer: Ref<Self>) -> Rc<dyn ISelectionItemProvider> {
        Rc::new(DateCellSelectionProvider(peer))
    }
    fn cell(&self) -> Ref<MaterialDateCell> {
        self.owner().cast().expect("date-cell peer owner")
    }
}
struct DateCellSelectionProvider(Ref<MaterialDateCellAutomationPeer>);
impl ISelectionItemProvider for DateCellSelectionProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_selected(&self) -> bool {
        self.0.cell().is_selected()
    }
    fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        self.0.cell().owner().and_then(|o| {
            ControlAutomationPeer::create_peer_for_element(&o)
                .get_provider::<dyn ISelectionProvider>()
        })
    }
    fn select(&self) -> Result<(), ElementNotEnabledException> {
        self.0.ensure_enabled()?;
        self.0.cell().select();
        Ok(())
    }
    fn add_to_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.select()
    }
    fn remove_from_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.0.ensure_enabled()?;
        let cell = self.0.cell();
        if cell.is_selected() && !cell.is_year() {
            if let Some(owner) = cell.owner() {
                owner.set_selection(None, None);
            }
        }
        Ok(())
    }
}

/// A six-row month. Empty leading and trailing cells are layout space, without
/// selectable dates from adjacent months. Range paint is behind the real buttons.
#[repr(C)]
pub struct MaterialDateMonthView {
    base: Panel,
    owner: RefCell<Option<WeakRef<MaterialDatePicker>>>,
    month: Cell<DateTime>,
    leading: Cell<i32>,
}
ferro_class!(MaterialDateMonthView: Panel);
ferro_class_info!(MaterialDateMonthView {
    new: MaterialDateMonthView::new
});
ferro_impl_classes!(MaterialDateMonthView: FerroObjectImpl, StyledElementImpl, InteractiveImpl,
    InputElementImpl, ControlImpl, PanelImpl);
impl MaterialDateMonthView {
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            owner: RefCell::new(None),
            month: Cell::new(DateTime::new(2000, 1, 1)),
            leading: Cell::new(0),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub(crate) fn set_month(&self, owner: &MaterialDatePicker, month: DateTime) {
        let changed = self.owner.borrow().is_none() || self.month.get() != month;
        *self.owner.borrow_mut() = Some(owner.to_ref().downgrade());
        self.month.set(month);
        self.leading
            .set(month_leading_days(month, owner.first_day_of_week()));
        if changed {
            self.children().clear();
            for day in 1..=DateTime::days_in_month(month.year(), month.month()) {
                self.children().add(
                    MaterialDateCell::for_date(
                        owner,
                        DateTime::new(month.year(), month.month(), day),
                    )
                    .upcast::<Control>(),
                );
            }
        }
        self.refresh();
        self.invalidate_measure();
    }
    pub fn month(&self) -> DateTime {
        self.month.get()
    }
    pub(crate) fn refresh(&self) {
        if let Some(owner) = self.owner.borrow().as_ref().and_then(WeakRef::upgrade) {
            self.leading
                .set(month_leading_days(self.month(), owner.first_day_of_week()));
        }
        for child in self.children().snapshot().iter() {
            if let Some(cell) = child.cast::<MaterialDateCell>() {
                cell.refresh();
            }
        }
        self.invalidate_arrange();
        self.invalidate_visual();
    }
    /// Logical, unmirrored range rectangles; Visual supplies flow-direction mirroring.
    pub fn range_rectangles(&self, width: f64) -> Vec<Rect> {
        let Some(owner) = self.owner.borrow().as_ref().and_then(WeakRef::upgrade) else {
            return Vec::new();
        };
        let state = owner.state();
        let Some((start, end)) = state.selected_start.zip(state.selected_end) else {
            return Vec::new();
        };
        let month = self.month();
        let last = DateTime::new(
            month.year(),
            month.month(),
            DateTime::days_in_month(month.year(), month.month()),
        );
        if end < month || start > last {
            return Vec::new();
        }
        let first_date = start.max(month);
        let last_date = end.min(last);
        let first = self.leading.get() + first_date.day() - 1;
        let last = self.leading.get() + last_date.day() - 1;
        let cell_width = width / 7.0;
        let extra = (cell_width - 48.0) / 2.0;
        let mut result = Vec::new();
        for row in first / 7..=last / 7 {
            let x1 = if row == first / 7 {
                (first % 7) as f64 * cell_width
                    + extra
                    + if first_date == start { 24.0 } else { 0.0 }
            } else {
                0.0
            };
            let x2 = if row == last / 7 {
                (last % 7) as f64 * cell_width + extra + if last_date == end { 24.0 } else { 48.0 }
            } else {
                width
            };
            if x2 > x1 {
                result.push(Rect::new(x1, row as f64 * 48.0 + 4.0, x2 - x1, 40.0));
            }
        }
        result
    }
}
impl LayoutableImpl for MaterialDateMonthView {
    fn measure_override(this: &Self, available: Size) -> Size {
        for child in this.children().snapshot().iter() {
            child.measure(Size::new(48.0, 48.0));
        }
        Size::new(
            if available.width.is_finite() {
                available.width.max(336.0)
            } else {
                336.0
            },
            288.0,
        )
    }
    fn arrange_override(this: &Self, size: Size) -> Size {
        let width = size.width / 7.0;
        for (index, child) in this.children().snapshot().iter().enumerate() {
            let cell = this.leading.get() as usize + index;
            child.arrange(Rect::new(
                (cell % 7) as f64 * width + (width - 48.0) / 2.0,
                (cell / 7) as f64 * 48.0,
                48.0,
                48.0,
            ));
        }
        size
    }
}
impl VisualImpl for MaterialDateMonthView {
    fn render(this: &Self, dc: &mut DrawingContext) {
        Self::parent_render(this, dc);
        let Some(owner) = this.owner.borrow().as_ref().and_then(WeakRef::upgrade) else {
            return;
        };
        let brush = owner.range_background();
        for rect in this.range_rectangles(this.bounds().width) {
            dc.draw_rectangle(brush.as_ref(), None, rect, 0.0, 0.0, &BoxShadows::default());
        }
    }
}

struct MonthItem {
    index: i32,
    title: Ref<TextBlock>,
    view: Ref<MaterialDateMonthView>,
}
/// Physical scroll extent for all months, with only visible months plus one on each
/// side realized. The native 48dp subhead + 288dp calendar yields a 336dp stride.
#[repr(C)]
pub struct MaterialDateMonthsPanel {
    base: Panel,
    owner: RefCell<Option<WeakRef<MaterialDatePicker>>>,
    items: RefCell<Vec<MonthItem>>,
    month_count: Cell<i32>,
    first_visible: Cell<i32>,
    viewport_height: Cell<f64>,
}
ferro_class!(MaterialDateMonthsPanel: Panel);
ferro_class_info!(MaterialDateMonthsPanel {
    new: MaterialDateMonthsPanel::new
});
ferro_impl_classes!(MaterialDateMonthsPanel: FerroObjectImpl, StyledElementImpl, VisualImpl,
    InteractiveImpl, InputElementImpl, ControlImpl, PanelImpl);
impl MaterialDateMonthsPanel {
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            owner: RefCell::new(None),
            items: RefCell::new(Vec::new()),
            month_count: Cell::new(0),
            first_visible: Cell::new(0),
            viewport_height: Cell::new(336.0),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub(crate) fn set_owner(&self, owner: &MaterialDatePicker) {
        *self.owner.borrow_mut() = Some(owner.to_ref().downgrade());
        self.month_count.set(owner.state().month_count());
        self.realize(
            owner.state().month_index(owner.displayed_month()) as f64 * 336.0,
            self.viewport_height.get(),
        );
    }
    pub(crate) fn realize(&self, offset: f64, viewport_height: f64) {
        let Some(owner) = self.owner.borrow().as_ref().and_then(WeakRef::upgrade) else {
            return;
        };
        let count = owner.state().month_count();
        self.month_count.set(count);
        self.viewport_height.set(viewport_height.max(336.0));
        let first = ((offset.max(0.0) / 336.0).floor() as i32).clamp(0, count - 1);
        self.first_visible.set(first);
        let start = (first - 1).max(0);
        let end = (first + (self.viewport_height.get() / 336.0).ceil() as i32 + 1).min(count - 1);
        let mut old = self.items.borrow_mut().drain(..).collect::<Vec<_>>();
        let mut retained = Vec::new();
        for item in old.drain(..) {
            if (start..=end).contains(&item.index) {
                retained.push(item);
            } else {
                self.children().remove(&item.title.upcast::<Control>());
                self.children().remove(&item.view.upcast::<Control>());
            }
        }
        for index in start..=end {
            let month = owner.state().month_at(index);
            if let Some(item) = retained.iter().find(|v| v.index == index) {
                item.title.set_text(Some(&owner.format_month(month)));
                item.title.set_foreground(owner.subhead_foreground());
                item.view.set_month(&owner, month);
                continue;
            }
            let title = TextBlock::new();
            title.set_text(Some(&owner.format_month(month)));
            title.set_font_size(14.0);
            title.set_font_weight(FontWeight::Medium);
            title.set_line_height(20.0);
            title.set_foreground(owner.subhead_foreground());
            title.set_margin(Thickness::new(24.0, 20.0, 0.0, 8.0));
            title.set_vertical_alignment(VerticalAlignment::Top);
            title.set_horizontal_alignment(HorizontalAlignment::Left);
            AutomationProperties::set_heading_level(&title, 2);
            let view = MaterialDateMonthView::new();
            view.set_month(&owner, month);
            self.children().add(title.clone().upcast::<Control>());
            self.children().add(view.clone().upcast::<Control>());
            retained.push(MonthItem { index, title, view });
        }
        retained.sort_by_key(|v| v.index);
        *self.items.borrow_mut() = retained;
        self.invalidate_measure();
    }
    pub(crate) fn refresh(&self) {
        for item in self.items.borrow().iter() {
            item.view.refresh();
        }
    }
    pub fn realized_month_count(&self) -> usize {
        self.items.borrow().len()
    }
}
impl LayoutableImpl for MaterialDateMonthsPanel {
    fn measure_override(this: &Self, available: Size) -> Size {
        let width = if available.width.is_finite() {
            available.width.max(336.0)
        } else {
            336.0
        };
        for item in this.items.borrow().iter() {
            item.title.measure(Size::new(width, 48.0));
            item.view.measure(Size::new(width, 288.0));
        }
        Size::new(width, this.month_count.get() as f64 * 336.0)
    }
    fn arrange_override(this: &Self, size: Size) -> Size {
        for item in this.items.borrow().iter() {
            let y = item.index as f64 * 336.0;
            item.title.arrange(Rect::new(0.0, y, size.width, 48.0));
            item.view
                .arrange(Rect::new(0.0, y + 48.0, size.width, 288.0));
        }
        size
    }
}
