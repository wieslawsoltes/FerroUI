//! A Material outlined text-field border with a real label cutout. The standard
//! FerroUI Border renderer still owns background sizing, rounded corners, layout
//! rounding and shadows; only the outline is clipped around the floating label.
//! Cutout padding follows AndroidX OutlinedTextField.kt, pinned to
//! e4d933c98df7baa7ea2b83eea6e086915961e0ff (Apache-2.0).

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{
    BoxShadows, CombinedGeometry, DrawingContext, GeometryCombineMode, RectangleGeometry,
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref,
    StaticType, StyledElementImpl, StyledProperty, TypeInfo, VisualImpl, VisualImplExt,
};
use ferroui_controls::utils::BorderRenderHelper;
use ferroui_controls::{Border, ControlImpl};
use std::cell::RefCell;

#[repr(C)]
pub struct MaterialTextFieldBorder {
    base: Border,
    background_renderer: RefCell<BorderRenderHelper>,
    outline_renderer: RefCell<BorderRenderHelper>,
}
ferro_class!(MaterialTextFieldBorder:Border);
ferro_class_info!(MaterialTextFieldBorder {
    new: MaterialTextFieldBorder::new
});
ferro_impl_classes!(MaterialTextFieldBorder:InteractiveImpl,InputElementImpl,LayoutableImpl,ControlImpl);
impl StyledElementImpl for MaterialTextFieldBorder {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        // Retain Border#BorderElement selectors in TextBox-derived editor themes
        // and application styles. The cutout properties use explicit owners.
        <Border as StaticType>::TYPE
    }
}
impl FerroObjectImpl for MaterialTextFieldBorder {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        this.invalidate_visual();
    }
}
impl VisualImpl for MaterialTextFieldBorder {
    fn render(this: &Self, dc: &mut DrawingContext) {
        let size = this.bounds().size();
        let Some((start, end)) = notch_range(
            this.notch_start(),
            this.notch_width(),
            this.notch_open(),
            size.width,
        ) else {
            Self::parent_render(this, dc);
            return;
        };
        let thickness = this.layout_thickness();
        let corners = this.corner_radius();
        let sizing = this.background_sizing();
        // Paint the background and any shadow before clipping; an outlined field
        // remains correct on any parent surface, including translucent brushes.
        this.background_renderer.borrow_mut().render(
            dc,
            size,
            thickness,
            corners,
            sizing,
            this.background().as_ref(),
            None,
            &this.box_shadow(),
        );
        let full = RectangleGeometry::with_rect(Rect::from_size(size));
        let cut = RectangleGeometry::with_rect(Rect::new(
            start,
            -1.0,
            end - start,
            thickness.top.max(0.0) + 2.0,
        ));
        let clip = CombinedGeometry::with_mode(
            GeometryCombineMode::Exclude,
            Some(full.upcast()),
            Some(cut.upcast()),
        );
        let state = dc.push_geometry_clip(&clip.upcast());
        this.outline_renderer.borrow_mut().render(
            dc,
            size,
            thickness,
            corners,
            sizing,
            None,
            this.border_brush().as_ref(),
            &BoxShadows::default(),
        );
        dc.pop(state);
    }
}
ferro_properties! {impl MaterialTextFieldBorder {
    pub fn notch_start_property()->StyledProperty<f64>{FerroProperty::register::<MaterialTextFieldBorder,_>("NotchStart",0.0)}
    pub fn notch_width_property()->StyledProperty<f64>{FerroProperty::register::<MaterialTextFieldBorder,_>("NotchWidth",0.0)}
    pub fn notch_open_property()->StyledProperty<f64>{FerroProperty::register::<MaterialTextFieldBorder,_>("NotchOpen",0.0)}
}}
impl MaterialTextFieldBorder {
    pub fn construct() -> Self {
        Self {
            base: Border::construct(),
            background_renderer: RefCell::new(BorderRenderHelper::new()),
            outline_renderer: RefCell::new(BorderRenderHelper::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn notch_start(&self) -> f64 {
        self.get_value(Self::notch_start_property())
    }
    pub fn set_notch_start(&self, value: f64) {
        self.set_value(Self::notch_start_property(), value);
    }
    pub fn notch_width(&self) -> f64 {
        self.get_value(Self::notch_width_property())
    }
    pub fn set_notch_width(&self, value: f64) {
        self.set_value(Self::notch_width_property(), value);
    }
    pub fn notch_open(&self) -> f64 {
        self.get_value(Self::notch_open_property())
    }
    pub fn set_notch_open(&self, value: f64) {
        self.set_value(Self::notch_open_property(), value);
    }
}
fn notch_range(start: f64, width: f64, open: f64, total: f64) -> Option<(f64, f64)> {
    if ![start, width, open, total]
        .iter()
        .all(|value| value.is_finite())
        || width <= 0.0
        || total <= 0.0
        || open <= 0.0
    {
        return None;
    }
    let half = (width + 8.0) * open.clamp(0.0, 1.0) / 2.0;
    let center = start + width / 2.0;
    let left = (center - half).clamp(0.0, total);
    let right = (center + half).clamp(0.0, total);
    if right > left {
        Some((left, right))
    } else {
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notch_matches_label_width_with_native_4dp_horizontal_padding() {
        assert_eq!(Some((12.0, 68.0)), notch_range(16.0, 48.0, 1.0, 280.0));
        assert_eq!(Some((26.0, 54.0)), notch_range(16.0, 48.0, 0.5, 280.0));
        assert_eq!(None, notch_range(16.0, 48.0, 0.0, 280.0));
    }
    #[test]
    fn notch_cannot_escape_field_or_poison_geometry() {
        assert_eq!(Some((0.0, 40.0)), notch_range(-20.0, 100.0, 1.0, 40.0));
        assert_eq!(None, notch_range(16.0, f64::NAN, 1.0, 280.0));
        assert_eq!(None, notch_range(16.0, 0.0, 1.0, 280.0));
    }
}
