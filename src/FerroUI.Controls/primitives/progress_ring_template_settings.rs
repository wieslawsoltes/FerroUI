// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See ../NOTICE.md.

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    DirectProperty, FerroObject, FerroObjectImpl, FerroProperty, Ref, Thickness,
};
use std::cell::Cell;

/// Legacy WinUI template geometry, updated when the owner's actual width changes.
#[repr(C)]
pub struct ProgressRingTemplateSettings {
    base: FerroObject,
    ellipse_diameter: Cell<f64>,
    ellipse_offset: Cell<Thickness>,
    max_side_length: Cell<f64>,
}
ferro_class!(ProgressRingTemplateSettings: FerroObject);
ferro_class_info!(ProgressRingTemplateSettings {
    new: ProgressRingTemplateSettings::new,
    markup: { namespace: "FerroUI.Controls.Primitives" },
});
ferro_impl_classes!(ProgressRingTemplateSettings: FerroObjectImpl);

ferro_properties! { impl ProgressRingTemplateSettings {
    pub fn ellipse_diameter_property() -> DirectProperty<ProgressRingTemplateSettings, f64> {
        FerroProperty::register_direct::<ProgressRingTemplateSettings, _>("EllipseDiameter", |s| s.ellipse_diameter(), None, 0.0)
    }
    pub fn ellipse_offset_property() -> DirectProperty<ProgressRingTemplateSettings, Thickness> {
        FerroProperty::register_direct::<ProgressRingTemplateSettings, _>("EllipseOffset", |s| s.ellipse_offset(), None, Thickness::default())
    }
    pub fn max_side_length_property() -> DirectProperty<ProgressRingTemplateSettings, f64> {
        FerroProperty::register_direct::<ProgressRingTemplateSettings, _>("MaxSideLength", |s| s.max_side_length(), None, 0.0)
    }
} }
impl ProgressRingTemplateSettings {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: FerroObject::construct(),
            ellipse_diameter: Cell::new(0.0),
            ellipse_offset: Cell::new(Thickness::default()),
            max_side_length: Cell::new(0.0),
        })
    }
    pub fn ellipse_diameter(&self) -> f64 {
        self.ellipse_diameter.get()
    }
    pub fn ellipse_offset(&self) -> Thickness {
        self.ellipse_offset.get()
    }
    pub fn max_side_length(&self) -> f64 {
        self.max_side_length.get()
    }
    pub(crate) fn update(&self, width: f64) {
        let width = if width.is_finite() {
            width.max(0.0)
        } else {
            0.0
        };
        let diameter = if width == 0.0 {
            0.0
        } else {
            width * 0.1 + if width <= 40.0 { 1.0 } else { 0.0 }
        };
        self.set_and_raise_cell(
            Self::ellipse_diameter_property(),
            &self.ellipse_diameter,
            diameter,
        );
        self.set_and_raise_cell(
            Self::ellipse_offset_property(),
            &self.ellipse_offset,
            Thickness::new(0.0, width * 0.5 - diameter, 0.0, 0.0),
        );
        self.set_and_raise_cell(
            Self::max_side_length_property(),
            &self.max_side_length,
            width,
        );
    }
}
