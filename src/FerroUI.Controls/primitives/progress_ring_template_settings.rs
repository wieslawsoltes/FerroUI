//! WinUI's `ProgressRingTemplateSettings` (`controls/dev/ProgressRing/ProgressRingTemplateSettings.cpp`,
//! `.h`, `ProgressRing.idl` and `ProgressRing::ApplyTemplateSettings` of `ProgressRing.cpp`,
//! microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_property, instantiate, DirectProperty,
    FerroObject, FerroObjectImpl, FerroProperty, Ref, Thickness,
};
use std::cell::Cell;

/// The values of the template of a [`ProgressRing`](crate::ProgressRing) that WinUI keeps for the
/// templates of the earlier ProgressRing of Windows (read-only): the diameter and the offset of
/// its ellipses and the side of the ring, from the actual width of the ring.
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
    ferro_property!(
        /// Defines the `EllipseDiameter` property (read-only).
        pub fn ellipse_diameter_property() -> DirectProperty<ProgressRingTemplateSettings, f64> {
            FerroProperty::register_direct::<ProgressRingTemplateSettings, _>(
                "EllipseDiameter",
                |settings| settings.ellipse_diameter(),
                None,
                0.0,
            )
        }
    );

    ferro_property!(
        /// Defines the `EllipseOffset` property (read-only).
        pub fn ellipse_offset_property() -> DirectProperty<ProgressRingTemplateSettings, Thickness> {
            FerroProperty::register_direct::<ProgressRingTemplateSettings, _>(
                "EllipseOffset",
                |settings| settings.ellipse_offset(),
                None,
                Thickness::default(),
            )
        }
    );

    ferro_property!(
        /// Defines the `MaxSideLength` property (read-only).
        pub fn max_side_length_property() -> DirectProperty<ProgressRingTemplateSettings, f64> {
            FerroProperty::register_direct::<ProgressRingTemplateSettings, _>(
                "MaxSideLength",
                |settings| settings.max_side_length(),
                None,
                0.0,
            )
        }
    );
} }

impl ProgressRingTemplateSettings {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            ellipse_diameter: Cell::new(0.0),
            ellipse_offset: Cell::new(Thickness::default()),
            max_side_length: Cell::new(0.0),
        }
    }

    /// Creates the settings of a ring.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the diameter of the ellipses: a tenth of the width, one more up to a width of 40.
    pub fn ellipse_diameter(&self) -> f64 {
        self.ellipse_diameter.get()
    }

    /// Gets the offset of the ellipses: half the width less their diameter, at the top.
    pub fn ellipse_offset(&self) -> Thickness {
        self.ellipse_offset.get()
    }

    /// Gets the side of the ring: its width.
    pub fn max_side_length(&self) -> f64 {
        self.max_side_length.get()
    }

    /// ProgressRing::ApplyTemplateSettings: the values for the actual width of the ring (0 when
    /// it has none).
    pub(crate) fn update(&self, width: f64) {
        let width = if width.is_finite() && width > 0.0 { width } else { 0.0 };
        let diameter = if width == 0.0 { 0.0 } else { width * 0.1 + if width <= 40.0 { 1.0 } else { 0.0 } };
        let anchor = if width == 0.0 { 0.0 } else { width * 0.5 - diameter };
        self.set_and_raise_cell(Self::ellipse_diameter_property(), &self.ellipse_diameter, diameter);
        self.set_and_raise_cell(Self::ellipse_offset_property(), &self.ellipse_offset, Thickness::new(0.0, anchor, 0.0, 0.0));
        self.set_and_raise_cell(Self::max_side_length_property(), &self.max_side_length, width);
    }
}
