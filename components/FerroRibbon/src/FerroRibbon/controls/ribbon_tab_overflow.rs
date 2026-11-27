//! Original RibbonTab display ordering and linked-control simplified overflow.
use super::{IRibbonItem, RibbonGroup, RibbonItemHelper, RibbonItemLayout, RibbonTab};
use crate::{metadata::ribbon_collections::RibbonGroups, primitives::RibbonMetrics};
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::{
    RibbonItemSize, RibbonString,
    collections::{RibbonArray, RibbonReadOnlyList},
};
use ferroui_base::{
    Ref, Thickness,
    data::core::{ValueType, ValueTypes},
    media::{FontWeight, IBrush},
};
use ferroui_controls::{
    Application, Border, Control, Flyout, PlacementMode, ScrollViewer, StackPanel, TextBlock,
    primitives::ScrollBarVisibility, shapes::Rectangle,
};
use std::rc::Rc;
#[derive(Clone)]
pub struct RibbonDisplayGroups(DisplayStorage);
#[derive(Clone)]
enum DisplayStorage {
    Groups(RibbonGroups),
    Snapshot(RibbonArray<Option<Ref<RibbonGroup>>>),
}
impl PartialEq for RibbonDisplayGroups {
    fn eq(&self, o: &Self) -> bool {
        match (&self.0, &o.0) {
            (DisplayStorage::Groups(a), DisplayStorage::Groups(b)) => a == b,
            (DisplayStorage::Snapshot(a), DisplayStorage::Snapshot(b)) => std::rc::Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}
impl RibbonDisplayGroups {
    pub(super) fn from_groups(groups: RibbonGroups) -> Rc<Self> {
        Rc::new(Self(DisplayStorage::Groups(groups)))
    }
    pub fn count(&self) -> i32 {
        match &self.0 {
            DisplayStorage::Groups(g) => i32::try_from(g.count()).expect("Int32 Count"),
            DisplayStorage::Snapshot(a) => i32::try_from(a.len()).expect("Int32 Count"),
        }
    }
    pub fn get(&self, index: i32) -> Option<Ref<RibbonGroup>> {
        let index = usize::try_from(index).expect("index");
        match &self.0 {
            DisplayStorage::Groups(g) => g.get(index),
            DisplayStorage::Snapshot(a) => a.get(index),
        }
    }
    pub fn iter(&self) -> ferroribbon_core::model::RibbonIterator<Option<Ref<RibbonGroup>>> {
        match &self.0 {
            DisplayStorage::Groups(g) => Box::new(g.iter()),
            DisplayStorage::Snapshot(a) => Box::new(a.iter()),
        }
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        self == other
    }
}
impl ferroui_base::data::model::IBindableList for RibbonDisplayGroups {
    fn count(&self) -> usize {
        usize::try_from(RibbonDisplayGroups::count(self)).expect("Count")
    }
    fn item_type(&self) -> ValueType {
        ValueType::of::<Option<Ref<RibbonGroup>>>()
    }
    fn get_item(&self, index: usize) -> Option<ferroui_base::BoxedValue> {
        if index >= ferroui_base::data::model::IBindableList::count(self) {
            return None;
        }
        self.get(i32::try_from(index).expect("index"))
            .map(Control::boxed)
    }
    fn set_item(&self, _: usize, _: &ferroui_base::BoxedValue) -> bool {
        false
    }
    fn as_notify_collection_changed(
        &self,
    ) -> Option<&dyn ferroui_system::collections::specialized::INotifyCollectionChanged> {
        match &self.0 {
            DisplayStorage::Groups(g) => Some(g),
            DisplayStorage::Snapshot(_) => None,
        }
    }
}
pub(crate) fn register_display_groups() {
    ferroui_base::data::model::ModelTypes::register::<RibbonDisplayGroups>(|b| b.list());
}
impl RibbonTab {
    #[doc(hidden)]
    pub fn get_key_tip_targets(&self) -> ferroribbon_core::model::RibbonItemsSource<Ref<Control>> {
        let tab = self.to_ref();
        ferroribbon_core::model::RibbonItemsSource::deferred(move || {
            let tab = tab.clone();
            // DisplayGroups is evaluated when enumeration begins, as in the source.
            let groups = tab.display_groups();
            let mut groups = groups.iter();
            let mut nested: Option<ferroribbon_core::model::RibbonIterator<Ref<Control>>> = None;
            let mut done = false;
            Box::new(std::iter::from_fn(move || {
                loop {
                    if let Some(iter) = nested.as_mut() {
                        if let Some(item) = iter.next() {
                            return Some(item);
                        }
                        nested = None;
                    }
                    if done {
                        return None;
                    }
                    if let Some(group) = groups.next() {
                        let group = group.expect("NullReference: group.IsShown");
                        if group.is_visible() {
                            nested = Some(group.get_key_tip_targets().iter());
                        }
                    } else {
                        done = true;
                        if tab.groups_panel().is_simplified() && tab.groups_panel().has_overflow() {
                            return Some(tab.overflow_button().upcast());
                        }
                    }
                }
            }))
        })
    }
    /// Native read-only adapter: original Groups identity/live enumeration without an order,
    /// fresh source ToArray snapshots when a nonempty custom order applies.
    pub fn display_groups(&self) -> Rc<RibbonDisplayGroups> {
        let order = self.group_order.borrow().clone();
        let Some(order) = order.filter(|o| o.count() > 0) else {
            return self.display_group_view.clone();
        };
        let mut listed = Vec::<Ref<RibbonGroup>>::new();
        for id in order.iter() {
            if let Some(group) = self
                .groups()
                .iter()
                .map(|g| g.expect("NullReference: group"))
                .find(|g| !g.is_custom() && g.effective_id().map(RibbonString::from) == id)
                && !listed.contains(&group)
            {
                listed.push(group);
            }
        }
        let mut result: Vec<_> = listed.iter().cloned().map(Some).collect();
        for group in self.groups().iter() {
            let group = group.expect("NullReference: group");
            if !listed.contains(&group) && !group.is_custom() {
                result.push(Some(group));
            }
        }
        for group in self.groups().iter() {
            let group = group.expect("NullReference: group");
            if group.is_custom() {
                result.push(Some(group));
            }
        }
        Rc::new(RibbonDisplayGroups(DisplayStorage::Snapshot(
            RibbonFixedItems::new(result),
        )))
    }
    #[doc(hidden)]
    pub fn group_order(&self) -> Option<RibbonReadOnlyList<Option<RibbonString>>> {
        self.group_order.borrow().clone()
    }
    #[doc(hidden)]
    pub fn set_group_order(&self, value: Option<RibbonReadOnlyList<Option<RibbonString>>>) {
        let previous = self.group_order();
        if previous
            .as_ref()
            .zip(value.as_ref())
            .is_some_and(|(a, b)| std::rc::Rc::ptr_eq(a, b) || a.iter().eq(b.iter()))
            || previous.is_none() && value.is_none()
        {
            return;
        }
        self.group_order.replace(value);
        self.sync_groups();
    }
    pub fn show_overflow(&self) {
        let panel = StackPanel::new();
        panel.set_spacing(1.);
        // WinUI StackPanel Padding maps to a native decorator around the same panel.
        let padding = Border::new();
        padding.set_min_width(220.);
        padding.set_padding(Thickness::uniform(2.));
        padding.set_child(panel.clone());
        for group in self.display_groups().iter() {
            let group = group.expect("NullReference: group");
            if !group.is_visible() {
                continue;
            }
            let items: Vec<_> = group.get_overflow_items().iter().collect();
            if items.is_empty() {
                continue;
            }
            if panel.children().count() > 0 {
                let separator = Rectangle::new();
                separator.set_height(1.);
                separator.set_margin(Thickness::uniform(4.));
                separator.set_fill(Some(resource_brush("RibbonSeparatorBrush")));
                panel.children().add(separator);
            }
            let header = TextBlock::new();
            header.set_text(group.header().as_deref());
            header.set_font_size(11.);
            header.set_font_weight(FontWeight::SemiBold);
            header.set_margin(Thickness::new(6., 2., 6., 2.));
            header.set_foreground(Some(resource_brush("RibbonSecondaryForegroundBrush")));
            panel.children().add(header);
            for item in items {
                if let Some(item) = as_item(&item)
                    && let Some(copy) = item.create_linked_copy()
                {
                    RibbonItemHelper::set_owner(&copy, self.ribbon().map(|r| r.item_owner()))
                        .expect("overflow copy owner");
                    if let Some(item) = as_item(&copy) {
                        item.apply_layout(RibbonItemLayout {
                            size: RibbonItemSize::Medium,
                            metrics: Some(
                                self.ribbon()
                                    .map(|r| r.metrics())
                                    .unwrap_or_else(RibbonMetrics::comfortable),
                            ),
                            is_simplified: false,
                            show_label: Some(true),
                            ..Default::default()
                        });
                    }
                    copy.set_horizontal_alignment(
                        ferroui_base::layout::HorizontalAlignment::Stretch,
                    );
                    panel.children().add(copy);
                }
            }
        }
        let scroll = ScrollViewer::new();
        scroll.set_content(Some(Control::boxed(padding)));
        scroll.set_max_height(520.);
        scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        let flyout = Flyout::new();
        flyout.set_content(Some(Control::boxed(scroll)));
        flyout.set_placement(PlacementMode::BottomEdgeAlignedRight);
        super::RibbonMenu::apply_theme(Some(&flyout));
        RibbonItemHelper::set_owner(&panel, self.ribbon().map(|r| r.item_owner()))
            .expect("overflow panel owner");
        let weak = self.to_ref().downgrade();
        let closed = flyout.downgrade();
        flyout
            .closed(move || {
                RibbonItemHelper::unlink_tree(Some(panel.clone().upcast()));
                if let Some(tab) = weak.upgrade() {
                    tab.open_overflows
                        .borrow_mut()
                        .retain(|f| !closed.points_to(f));
                }
            })
            .detach();
        // The native popup callbacks are weak. Keep each ephemeral flyout alive
        // until Closed, matching the source platform's open-popup lifetime.
        flyout.show_at(&self.overflow_button());
        if flyout.is_open() {
            self.open_overflows.borrow_mut().push(flyout);
        }
    }
    #[cfg(test)]
    pub(super) fn active_overflow_flyouts(&self) -> Vec<Ref<Flyout>> {
        self.open_overflows.borrow().clone()
    }
}
fn as_item(control: &Control) -> Option<Rc<dyn IRibbonItem>> {
    ValueTypes::try_cast(
        &Control::boxed(control.to_ref()),
        ValueType::of::<Rc<dyn IRibbonItem>>(),
    )
    .and_then(|v| v.downcast_ref::<Rc<dyn IRibbonItem>>().cloned())
}
fn resource_brush(key: &str) -> Rc<dyn IBrush> {
    let app = Application::current().expect("Application.Current");
    let value = app
        .resources()
        .try_get_resource(&key.into(), app.actual_theme_variant().as_ref())
        .flatten()
        .expect("missing overflow brush resource");
    ValueTypes::try_convert(Some(&value), ValueType::of::<Rc<dyn IBrush>>())
        .flatten()
        .and_then(|v| v.downcast_ref::<Rc<dyn IBrush>>().cloned())
        .expect("overflow resource is not a Brush")
}
pub struct RibbonDisplayGroupsMetadata;
ferroui_base::ferro_markup_type!(class RibbonDisplayGroupsMetadata as "IReadOnlyList`1"{namespace:"System.Collections.Generic",handles:[RibbonDisplayGroups,Rc<RibbonDisplayGroups>,Option<Rc<RibbonDisplayGroups>>],this:Rc<RibbonDisplayGroups>,generic:"IReadOnlyList`1"[Option<Ref<RibbonGroup>>],properties:[Count:i32{get:|v:&Rc<RibbonDisplayGroups>|v.count()}],indexers:[(i32)->Option<Ref<RibbonGroup>>{get:|v:&Rc<RibbonDisplayGroups>,i|v.get(i)}]});
