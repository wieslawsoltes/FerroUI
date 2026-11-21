use crate::layout_preview::LayoutPreview;
use ferroui_reactiveui::{IReactiveObject, IReactiveObjectExt, ReactiveObject, reactive_object};
use std::{cell::RefCell, rc::Rc};

pub struct ProbeViewModel {
    reactive: ReactiveObject,
    service: Rc<dyn LayoutPreview>,
    available_width: RefCell<f64>,
    column_setting: RefCell<f64>,
    presentation_label: RefCell<String>,
    is_simplified: RefCell<bool>,
}
reactive_object!(ProbeViewModel);
impl ProbeViewModel {
    pub fn new(service: Rc<dyn LayoutPreview>) -> Rc<Self> {
        Rc::new(Self {
            reactive: ReactiveObject::new(),
            service,
            available_width: RefCell::new(400.),
            column_setting: RefCell::new(4.),
            presentation_label: RefCell::new("Format Painter".into()),
            is_simplified: RefCell::new(false),
        })
    }
    pub fn available_width(&self) -> f64 {
        *self.available_width.borrow()
    }
    pub fn set_available_width(&self, value: f64) {
        if self.available_width() == value {
            return;
        }
        self.raise_and_set_if_changed(&self.available_width, value, "AvailableWidth");
        self.raise_property_changed("LayoutSummary");
    }
    pub fn column_setting(&self) -> f64 {
        *self.column_setting.borrow()
    }
    pub fn set_column_setting(&self, value: f64) {
        if self.column_setting() == value {
            return;
        }
        self.raise_and_set_if_changed(&self.column_setting, value, "ColumnSetting");
        self.raise_property_changed("Columns");
        self.raise_property_changed("GridSummary");
    }
    pub fn columns(&self) -> i32 {
        self.column_setting().round() as i32
    }
    pub fn layout_summary(&self) -> String {
        let result = self.service.compute(self.available_width());
        let names: Vec<_> = result
            .states
            .iter()
            .map(|s| match s.raw() {
                0 => "Large",
                1 => "Medium",
                2 => "Small",
                3 => "Collapsed",
                _ => "Unknown",
            })
            .collect();
        format!(
            "{}\nAvailable: {:.1}  •  Used: {:.1}  •  Fits: {}",
            names.join(" / "),
            self.available_width(),
            result.total_width,
            result.fits
        )
    }
    pub fn grid_summary(&self) -> String {
        format!("{} columns · 12 retained controls", self.columns())
    }
    pub fn presentation_label(&self) -> String {
        self.presentation_label.borrow().clone()
    }
    pub fn set_presentation_label(&self, value: String) {
        self.raise_and_set_if_changed(&self.presentation_label, value, "PresentationLabel");
    }
    pub fn is_simplified(&self) -> bool {
        *self.is_simplified.borrow()
    }
    pub fn set_is_simplified(&self, value: bool) {
        self.raise_and_set_if_changed(&self.is_simplified, value, "IsSimplified");
    }
}
impl PartialEq for ProbeViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
