// SPDX-License-Identifier: Apache-2.0
// Copyright (c) Uno Platform Inc.
// Changed: native Rust/FerroUI adaptation of the reference FontIcon child architecture.
//! Native glyph IconElement for source FontIcon inputs. The reference's centered TextBlock child
//! architecture is retained; this is not a TextBlock substituted for the public icon object.
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::{
    FlowDirection, FontFamily, FontStyle, FontWeight, ScaleTransform, TextAlignment,
};
use ferroui_base::{
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref,
    RelativePoint, Size, StyledElementImpl, StyledProperty, Visual, VisualImpl, ferro_class,
    ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_controls::automation::{AccessibilityView, AutomationProperties};
use ferroui_controls::primitives::{TemplatedControl, TemplatedControlImpl};
use ferroui_controls::{ControlImpl, IconElement, TextBlock};
use std::cell::{OnceCell, RefCell};

#[repr(C)]
pub struct FontIcon {
    base: IconElement,
    text: OnceCell<Ref<TextBlock>>,
    mirror_scale: RefCell<Option<Ref<ScaleTransform>>>,
}
ferro_class!(FontIcon:IconElement);
// A snapshot icon value must be populated before the parent receives it.
// Override StyledElement's inherited early-attachment attribute using the
// native compiler/runtime contract; programmatic cloning stays immediate.
ferro_class_info!(FontIcon {new:FontIcon::new,markup:{namespace:"FerroRibbon.Compatibility",attributes:[UsableDuringInitialization(false)]}});
ferro_impl_classes!(FontIcon:StyledElementImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
ferro_properties! {impl FontIcon {
    pub fn glyph_property()->StyledProperty<Option<String>> {FerroProperty::register::<FontIcon,_>("Glyph",Some(String::new()))}
    // Reference icon font properties are independent, non-inherited DPs. The
    // native TemplatedControl's ambient text properties are not equivalents.
    pub fn font_family_property()->StyledProperty<FontFamily> {FerroProperty::register::<FontIcon,_>("FontFamily",FontFamily::new("Segoe Fluent Icons,Segoe MDL2 Assets"))}
    pub fn font_size_property()->StyledProperty<f64> {FerroProperty::register::<FontIcon,_>("FontSize",20.)}
    pub fn font_style_property()->StyledProperty<FontStyle> {FerroProperty::register::<FontIcon,_>("FontStyle",FontStyle::Normal)}
    pub fn font_weight_property()->StyledProperty<FontWeight> {FerroProperty::register::<FontIcon,_>("FontWeight",FontWeight::Normal)}
    pub fn mirrored_when_right_to_left_property()->StyledProperty<bool> {FerroProperty::register::<FontIcon,_>("MirroredWhenRightToLeft",false)}
}}
impl FontIcon {
    fn static_constructor() {
        Visual::render_transform_origin_property()
            .override_default_value::<Self>(RelativePoint::TOP_LEFT);
    }
    pub fn construct() -> Self {
        Self {
            base: IconElement::construct(),
            text: OnceCell::new(),
            mirror_scale: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn glyph(&self) -> Option<String> {
        self.get_value(Self::glyph_property())
    }
    pub fn set_glyph(&self, value: Option<String>) {
        self.set_value(Self::glyph_property(), value)
    }
    pub fn font_family(&self) -> FontFamily {
        self.get_value(Self::font_family_property())
    }
    pub fn set_font_family(&self, value: FontFamily) {
        self.set_value(Self::font_family_property(), value)
    }
    pub fn font_size(&self) -> f64 {
        self.get_value(Self::font_size_property())
    }
    pub fn set_font_size(&self, value: f64) {
        self.set_value(Self::font_size_property(), value)
    }
    pub fn font_style(&self) -> FontStyle {
        self.get_value(Self::font_style_property())
    }
    pub fn set_font_style(&self, value: FontStyle) {
        self.set_value(Self::font_style_property(), value)
    }
    pub fn font_weight(&self) -> FontWeight {
        self.get_value(Self::font_weight_property())
    }
    pub fn set_font_weight(&self, value: FontWeight) {
        self.set_value(Self::font_weight_property(), value)
    }
    pub fn mirrored_when_right_to_left(&self) -> bool {
        self.get_value(Self::mirrored_when_right_to_left_property())
    }
    pub fn set_mirrored_when_right_to_left(&self, value: bool) {
        self.set_value(Self::mirrored_when_right_to_left_property(), value)
    }
    fn update_mirroring(&self) {
        let cached = self.mirror_scale.borrow().clone();
        if cached.is_none()
            && self.flow_direction() == FlowDirection::RightToLeft
            && self.mirrored_when_right_to_left()
        {
            let scale = ScaleTransform::new();
            // Publish the cache before native property callbacks can reenter.
            self.mirror_scale.replace(Some(scale.clone()));
            self.set_render_transform(Some(scale.into()));
            self.set_render_transform_origin(RelativePoint::CENTER);
        }
        // Re-read state after the property callbacks. A downstream replacement
        // stays installed; subsequent flag changes update only the cached scale.
        let cached = self.mirror_scale.borrow().clone();
        if let Some(scale) = cached {
            scale.set_scale_x(
                if self.flow_direction() == FlowDirection::RightToLeft
                    && self.mirrored_when_right_to_left()
                {
                    -1.
                } else {
                    1.
                },
            );
        }
    }
    fn sync(&self) {
        let Some(text) = self.text.get() else { return };
        text.set_text(self.glyph().as_deref());
        text.set_font_family(self.font_family());
        text.set_font_size(self.font_size());
        text.set_font_style(self.font_style());
        // The inspected reference constructor does not synchronize FontWeight here;
        // a later FontWeight property change does so. Preserve that distinction.
        text.set_foreground(self.foreground());
    }
}
impl FerroObjectImpl for FontIcon {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let text = TextBlock::new();
        text.set_horizontal_alignment(HorizontalAlignment::Center);
        text.set_vertical_alignment(VerticalAlignment::Center);
        text.set_text_alignment(TextAlignment::Center);
        text.set_value(
            AutomationProperties::accessibility_view_property(),
            AccessibilityView::Raw,
        );
        text.set_parent(this.to_ref());
        this.visual_children().add(text.clone().upcast());
        this.logical_children().add(text.clone().upcast());
        this.text.set(text).expect("FontIcon initialized once");
        this.sync();
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::mirrored_when_right_to_left_property().as_property() {
            this.update_mirroring();
            return;
        }
        let Some(text) = this.text.get() else { return };
        if property == Self::glyph_property().as_property() {
            text.set_text(this.glyph().as_deref());
        } else if property == Self::font_family_property().as_property() {
            text.set_font_family(this.font_family());
        } else if property == Self::font_size_property().as_property() {
            text.set_font_size(this.font_size());
        } else if property == Self::font_style_property().as_property() {
            text.set_font_style(this.font_style());
        } else if property == Self::font_weight_property().as_property() {
            text.set_font_weight(this.font_weight());
        } else if property == TemplatedControl::foreground_property().as_property() {
            text.set_foreground(this.foreground());
        } else {
            return;
        }
        this.invalidate_measure();
    }
}
impl VisualImpl for FontIcon {
    // Native flow compensation keeps glyphs upright within an RTL layout.
    // The reference opt-in mirror is the explicit cached RenderTransform above.
    fn bypass_flow_direction_policies(_this: &Self) -> bool {
        true
    }
}

#[cfg(test)]
#[path = "../font_icon_mirroring_tests.rs"]
mod mirroring_tests;
impl LayoutableImpl for FontIcon {
    fn apply_template(this: &Self) {
        assert!(
            this.template().is_none(),
            "FerroRibbon FontIcon adapter does not support a TemplatedControl template; source FontIcon has no Template property"
        );
    }
    fn measure_override(this: &Self, available: Size) -> Size {
        let text = this.text.get().expect("FontIcon initialized");
        text.measure(available);
        text.desired_size()
    }
    fn arrange_override(this: &Self, size: Size) -> Size {
        this.text
            .get()
            .expect("FontIcon initialized")
            .arrange(Rect::from_size(size));
        size
    }
}
