//! RibbonToolBarPanel.cs, RibbonSpace 9755e281. Copyright Wiesław Šoltés, MIT.
use super::ribbon_hidden_focus;
use crate::{compat::Orientation, controls::RibbonSeparator};
use ferroribbon_core::collections::RibbonItems;
use ferroribbon_core::collections::{RibbonList, RibbonReadOnlyList};
use ferroui_base::{
    FerroObjectImpl, Rect, Ref, Size, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl, utilities::math_utilities::max,
};
use ferroui_controls::{Control, ControlImpl, Panel, PanelImpl};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
pub type RibbonToolBarOverflowItems = RibbonReadOnlyList<Option<Ref<Control>>>;
#[repr(C)]
pub struct RibbonToolBarPanel {
    base: Panel,
    overflow_button: RefCell<Option<Ref<Control>>>,
    overflow: RibbonList<Option<Ref<Control>>>,
    overflow_view: RibbonToolBarOverflowItems,
    orientation: Cell<Orientation>,
    columns: Cell<i32>,
    is_overflow_enabled: Cell<bool>,
    spacing: Cell<f64>,
}
ferro_class!(RibbonToolBarPanel:Panel);
ferro_impl_classes!(RibbonToolBarPanel:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
ferro_class_info!(RibbonToolBarPanel {new:RibbonToolBarPanel::new,markup:{namespace:"FerroRibbon.Controls.Primitives",content:Children,properties:[
Orientation:crate::compat::Orientation{get:RibbonToolBarPanel::orientation,set:RibbonToolBarPanel::set_orientation},
Columns:i32{get:RibbonToolBarPanel::columns,set:RibbonToolBarPanel::set_columns},
IsOverflowEnabled:bool{get:RibbonToolBarPanel::is_overflow_enabled,set:RibbonToolBarPanel::set_is_overflow_enabled},
Spacing:f64{get:RibbonToolBarPanel::spacing,set:RibbonToolBarPanel::set_spacing},
OverflowButton:Option<Ref<Control>>{get:RibbonToolBarPanel::overflow_button,set:RibbonToolBarPanel::set_overflow_button},
OverflowItems:RibbonToolBarOverflowItems{get:RibbonToolBarPanel::overflow_items}
]}});
impl RibbonToolBarPanel {
    pub fn construct() -> Self {
        let base = Panel::construct();
        let overflow = RibbonItems::new();
        let overflow_view = overflow.clone() as RibbonReadOnlyList<_>;
        Self {
            base,
            overflow_button: RefCell::default(),
            overflow,
            overflow_view,
            orientation: Cell::new(Orientation::Horizontal),
            columns: Cell::new(1),
            is_overflow_enabled: Cell::new(true),
            spacing: Cell::new(2.),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    // These are plain source properties. Deliberately no styled-property
    // registration or new automatic invalidation on assignment.
    pub fn orientation(&self) -> Orientation {
        self.orientation.get()
    }
    pub fn set_orientation(&self, value: Orientation) {
        self.orientation.set(value)
    }
    pub fn columns(&self) -> i32 {
        self.columns.get()
    }
    pub fn set_columns(&self, value: i32) {
        self.columns.set(value)
    }
    pub fn is_overflow_enabled(&self) -> bool {
        self.is_overflow_enabled.get()
    }
    pub fn set_is_overflow_enabled(&self, value: bool) {
        self.is_overflow_enabled.set(value)
    }
    pub fn spacing(&self) -> f64 {
        self.spacing.get()
    }
    pub fn set_spacing(&self, value: f64) {
        self.spacing.set(value)
    }
    pub fn overflow_items(&self) -> RibbonToolBarOverflowItems {
        self.overflow_view.clone()
    }
    pub fn overflow_button(&self) -> Option<Ref<Control>> {
        self.overflow_button.borrow().clone()
    }
    pub fn set_overflow_button(&self, value: Option<Ref<Control>>) {
        if let Some(old) = self.overflow_button() {
            self.children().remove(old);
        }
        self.overflow_button.replace(value.clone());
        if let Some(value) = value {
            self.children().add(value);
        }
    }
    fn items(&self) -> Vec<Ref<Control>> {
        let children = self.children();
        let snapshot = children.snapshot();
        let mut items = Vec::new();
        for child in snapshot.iter() {
            assert!(
                Rc::ptr_eq(&snapshot, &children.snapshot()),
                "InvalidOperation: toolbar children changed during enumeration"
            );
            if Some(child) != self.overflow_button().as_ref() && child.is_visible() {
                items.push(child.clone());
            }
        }
        assert!(
            Rc::ptr_eq(&snapshot, &children.snapshot()),
            "InvalidOperation: toolbar children changed during enumeration"
        );
        items
    }
    fn cell_width(&self, items: &[Ref<Control>]) -> f64 {
        if self.orientation() == Orientation::Horizontal {
            0.
        } else {
            linq_max(
                items
                    .iter()
                    .filter(|c| c.cast::<RibbonSeparator>().is_none())
                    .map(|c| c.desired_size().width),
            )
            .unwrap_or(0.)
        }
    }
    fn rows(&self, items: Vec<Ref<Control>>, cell_width: f64) -> Rows {
        Rows {
            items,
            index: 0,
            horizontal: self.orientation() == Orientation::Horizontal,
            columns: self.columns().max(1) as usize,
            cell_width,
        }
    }
}
struct Row {
    items: Vec<Ref<Control>>,
    extent: f64,
    cross: f64,
}
struct Rows {
    items: Vec<Ref<Control>>,
    index: usize,
    horizontal: bool,
    columns: usize,
    cell_width: f64,
}
impl Iterator for Rows {
    type Item = Row;
    fn next(&mut self) -> Option<Row> {
        if self.index >= self.items.len() {
            return None;
        }
        if self.horizontal {
            let item = self.items[self.index].clone();
            self.index += 1;
            let size = item.desired_size();
            return Some(Row {
                items: vec![item],
                extent: size.width,
                cross: size.height,
            });
        }
        let mut row = Vec::new();
        while self.index < self.items.len() {
            let item = &self.items[self.index];
            let full = item.cast::<RibbonSeparator>().is_some();
            if full && !row.is_empty() {
                break;
            }
            row.push(item.clone());
            self.index += 1;
            if full || row.len() >= self.columns {
                break;
            }
        }
        let extent =
            linq_max(row.iter().map(|i| i.desired_size().height)).expect("nonempty toolbar row");
        let cross = row
            .iter()
            .map(|i| {
                if i.cast::<RibbonSeparator>().is_some() {
                    i.desired_size().width
                } else {
                    max(self.cell_width, i.desired_size().width)
                }
            })
            .sum();
        Some(Row {
            items: row,
            extent,
            cross,
        })
    }
}
// Enumerable.Max(double) ignores NaN when a numeric value exists; Math.Max
// propagates it. Keep the two original operations distinct.
fn linq_max(values: impl Iterator<Item = f64>) -> Option<f64> {
    values.reduce(|a, b| {
        if !b.is_nan() && (a.is_nan() || b > a) {
            b
        } else {
            a
        }
    })
}
impl LayoutableImpl for RibbonToolBarPanel {
    fn measure_override(this: &Self, available: Size) -> Size {
        let children = this.children();
        let snapshot = children.snapshot();
        for child in snapshot.iter() {
            assert!(
                Rc::ptr_eq(&snapshot, &children.snapshot()),
                "InvalidOperation: toolbar children changed during enumeration"
            );
            child.measure(Size::INFINITY);
        }
        assert!(
            Rc::ptr_eq(&snapshot, &children.snapshot()),
            "InvalidOperation: toolbar children changed during enumeration"
        );
        this.overflow.clear();
        let horizontal = this.orientation() == Orientation::Horizontal;
        let limit = if horizontal {
            available.width
        } else {
            available.height
        };
        let overflow_size = this.overflow_button().map_or(0., |b| {
            (if horizontal {
                b.desired_size().width
            } else {
                b.desired_size().height
            }) + this.spacing()
        });
        let items = this.items();
        let cell_width = this.cell_width(&items);
        let rows = this.rows(items, cell_width).collect::<Vec<_>>();
        let total = rows.iter().map(|r| r.extent + this.spacing()).sum::<f64>()
            - if rows.is_empty() { 0. } else { this.spacing() };
        let fit_limit = if this.is_overflow_enabled() && total > limit {
            limit - overflow_size
        } else {
            f64::INFINITY
        };
        let mut used = 0.;
        let mut cross = 0.;
        for row in rows {
            if (used + row.extent > fit_limit && this.overflow.is_empty() && used > 0.)
                || !this.overflow.is_empty()
            {
                for item in row.items {
                    this.overflow.add(Some(item));
                }
                continue;
            }
            used += row.extent + this.spacing();
            cross = max(cross, row.cross);
        }
        used = max(0., used - this.spacing());
        if !this.overflow.is_empty() {
            used += overflow_size;
            let button = this
                .overflow_button()
                .expect("NullReference: toolbar overflow requires OverflowButton");
            cross = max(
                cross,
                if horizontal {
                    button.desired_size().height
                } else {
                    button.desired_size().width
                },
            );
        }
        if horizontal {
            Size::new(used, cross)
        } else {
            Size::new(cross, used)
        }
    }
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let horizontal = this.orientation() == Orientation::Horizontal;
        let mut position = 0.;
        let hidden = Rect::new(0., 0., 0., 0.);
        let items = this.items();
        let width = this.cell_width(&items);
        for row in this.rows(items, width) {
            if row
                .items
                .iter()
                .any(|i| this.overflow.iter().any(|v| v.as_ref() == Some(i)))
            {
                for item in row.items {
                    item.arrange(hidden);
                    ribbon_hidden_focus::set(&item.upcast(), true);
                }
                continue;
            }
            for item in &row.items {
                ribbon_hidden_focus::set(&item.clone().upcast(), false);
            }
            if horizontal {
                let item = &row.items[0];
                let size = item.desired_size();
                item.arrange(Rect::new(
                    position,
                    (final_size.height - size.height) / 2.,
                    size.width,
                    size.height,
                ));
                position += item.desired_size().width + this.spacing();
            } else {
                let mut x = 0.;
                for item in row.items {
                    let width = if item.cast::<RibbonSeparator>().is_some() {
                        final_size.width
                    } else {
                        max(width, item.desired_size().width)
                    };
                    item.arrange(Rect::new(x, position, width, item.desired_size().height));
                    x += width;
                }
                position += row.extent + this.spacing();
            }
        }
        if let Some(button) = this.overflow_button() {
            if !this.overflow.is_empty() {
                let size = button.desired_size();
                button.arrange(if horizontal {
                    Rect::new(
                        position,
                        (final_size.height - size.height) / 2.,
                        size.width,
                        size.height,
                    )
                } else {
                    Rect::new(0., position, size.width, size.height)
                });
                ribbon_hidden_focus::set(&button.upcast(), false);
            } else {
                button.arrange(hidden);
                ribbon_hidden_focus::set(&button.upcast(), true);
            }
        }
        final_size
    }
}
#[cfg(test)]
#[path = "../toolbar_panel_tests.rs"]
mod tests;
