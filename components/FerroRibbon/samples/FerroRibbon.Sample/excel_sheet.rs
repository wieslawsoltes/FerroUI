// Copyright (c) 2026 Wiesław Šoltés. MIT. ExcelPage selection/formatting actions.
//! Native views supply existing cells; this module creates no controls or brushes.
use crate::excel_view_model::ExcelViewModel;
use ferroribbon_core::{RibbonString, model::*, theming::RibbonColor};
use ferroui_reactiveui::{ICommand, ReactiveCommand};
use ferroui_system::{
    component_model::INotifyPropertyChanged,
    invariant_numbers::{NumberStyles, try_parse_double},
};
use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::{Rc, Weak},
};
pub type ExcelCellKey = (i32, i32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExcelAlignment {
    Left,
    Center,
    Right,
}
/// A transient view of the source TextBlock and its current Border parent.
/// Each call stays separate: native setters can reenter the application.
pub trait ExcelCellSurface {
    fn font_weight(&self) -> u16;
    fn italic(&self) -> bool;
    fn set_border_color(&self, color: RibbonColor);
    fn set_border_widths(&self, left: f64, top: f64, right: f64, bottom: f64);
    fn set_font_weight(&self, weight: u16);
    fn set_italic(&self, italic: bool);
    fn set_underline(&self, underline: bool);
    fn set_foreground(&self, color: RibbonColor);
    fn set_background(&self, color: RibbonColor);
    fn set_font_family(&self, family: RibbonString) -> Result<(), String>;
    fn set_font_size(&self, size: f64);
    fn set_alignment(&self, alignment: ExcelAlignment);
}
pub trait ExcelSheetSurface {
    /// None also covers a known cell whose current parent is no longer a Border.
    fn cell(&self, key: ExcelCellKey) -> Option<Rc<dyn ExcelCellSurface>>;
}
type ModelSubscription = (Weak<dyn INotifyPropertyChanged>, u64);
pub struct ExcelSheet {
    model: Rc<ExcelViewModel>,
    surface: Rc<dyn ExcelSheetSurface>,
    selected: Cell<ExcelCellKey>,
    subscriptions: RefCell<Vec<ModelSubscription>>,
    select_command: OnceCell<Rc<dyn ICommand>>,
}
impl ExcelSheet {
    pub fn new(model: Rc<ExcelViewModel>, surface: Rc<dyn ExcelSheetSurface>) -> Rc<Self> {
        let sheet = Rc::new(Self {
            model,
            surface,
            selected: Cell::new((1, 1)),
            subscriptions: RefCell::new(Vec::new()),
            select_command: OnceCell::new(),
        });
        let weak = Rc::downgrade(&sheet);
        assert!(
            sheet
                .select_command
                .set(Rc::new(ReactiveCommand::create_param(
                    move |key: ExcelCellKey| {
                        if let Some(sheet) = weak.upgrade() {
                            sheet.select(key);
                        }
                    }
                )))
                .is_ok()
        );
        // Source BuildSheet calls Select before installing model-format subscriptions.
        sheet.select((1, 1));
        let m = &sheet.model;
        let a = m.align().borrow().clone();
        let models: Vec<Rc<dyn INotifyPropertyChanged>> = vec![
            m.bold(),
            m.italic(),
            m.underline(),
            m.fill_color(),
            m.font_color(),
            m.font_family(),
            m.font_size(),
            a[0].clone(),
            a[1].clone(),
            a[2].clone(),
        ];
        for model in models {
            let weak = Rc::downgrade(&sheet);
            let token = model.property_changed().add(Rc::new(move |_| {
                if let Some(sheet) = weak.upgrade() {
                    sheet.apply_to_selection().expect("Excel cell formatting");
                }
            }));
            sheet
                .subscriptions
                .borrow_mut()
                .push((Rc::downgrade(&model), token));
        }
        sheet
    }
    pub fn selected(&self) -> ExcelCellKey {
        self.selected.get()
    }
    pub fn select_command(&self) -> Rc<dyn ICommand> {
        self.select_command.get().unwrap().clone()
    }
    pub fn select(&self, key: ExcelCellKey) {
        if let Some(previous) = self.surface.cell(self.selected.get()) {
            previous.set_border_color(RibbonColor::from_rgb(0xe1, 0xe1, 0xe1));
            previous.set_border_widths(0., 0., 1., 1.);
        }
        self.selected.set(key);
        if let Some(cell) = self.surface.cell(key) {
            cell.set_border_color(RibbonColor::from_rgb(0x10, 0x7c, 0x41));
            cell.set_border_widths(2., 2., 2., 2.);
            let column = char::from_u32(('A' as u32) + (key.1 as u32) - 1)
                .expect("original sheet column A..L");
            self.model
                .set_selected_cell(Some(format!("{column}{}", key.0)));
            self.model.bold().set_is_checked(cell.font_weight() >= 600);
            // Read the cell again AFTER Bold's synchronous formatting notifications.
            self.model.italic().set_is_checked(cell.italic());
        }
    }
    pub fn apply_to_selection(&self) -> Result<(), String> {
        let Some(cell) = self.surface.cell(self.selected.get()) else {
            return Ok(());
        };
        cell.set_font_weight(if self.model.bold().is_checked() {
            700
        } else {
            400
        });
        cell.set_italic(self.model.italic().is_checked());
        cell.set_underline(self.model.underline().is_checked());
        if let Some(fg) = self.model.font_color().selected_color() {
            cell.set_foreground(fg);
        }
        cell.set_background(
            self.model
                .fill_color()
                .selected_color()
                .unwrap_or(RibbonColor::WHITE),
        );
        if let Some(family) = self
            .model
            .font_family()
            .text()
            .filter(|s| !ferroribbon_core::text::trim(s.as_utf16()).is_empty())
        {
            cell.set_font_family(family)?;
        }
        if let Some(size) = self
            .model
            .font_size()
            .text()
            .and_then(|s| s.to_utf8().ok())
            .and_then(|s| try_parse_double(&s, NumberStyles::FLOAT))
            .filter(|n| *n > 4. && *n < 100.)
        {
            cell.set_font_size(size);
        }
        let a = self.model.align().borrow().clone();
        cell.set_alignment(if a[1].is_checked() {
            ExcelAlignment::Center
        } else if a[2].is_checked() {
            ExcelAlignment::Right
        } else {
            ExcelAlignment::Left
        });
        Ok(())
    }
}
impl Drop for ExcelSheet {
    fn drop(&mut self) {
        for (model, token) in self.subscriptions.get_mut().drain(..) {
            if let Some(model) = model.upgrade() {
                model.property_changed().remove(token);
            }
        }
    }
}
