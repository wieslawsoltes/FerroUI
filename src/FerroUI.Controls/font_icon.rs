// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See NOTICE.md.
// Adapted from the FontIcon contract and text rendering behavior of WinUI.

use crate::automation::{AccessibilityView, AutomationProperties};
use crate::primitives::{
    TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::{ControlImpl, IconElement, TextBlock};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::{FlowDirection, FontFamily, ScaleTransform, TextAlignment};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, RelativePoint, StyledElementImpl, StyledProperty,
    Visual, VisualImpl,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A font glyph rendered by the native text layout and drawing services.
///
/// Font family, size, weight, style and foreground use the existing text
/// properties. The default family is `Segoe Fluent Icons`, with a size of 20.
/// Applications must install or supply their chosen icon font; no font binary
/// is redistributed by this control. OS text scale factor integration remains
/// unsupported and is deliberately not exposed as a nonfunctional property.
#[repr(C)]
pub struct FontIcon {
    base: IconElement,
    text: RefCell<Option<Ref<TextBlock>>>,
}

ferro_class!(FontIcon: IconElement);
ferroui_base::ferro_class_info!(FontIcon { new: FontIcon::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(
    FontIcon: StyledElementImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl
);

impl FerroObjectImpl for FontIcon {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::glyph_property().as_property()
            || property == Self::mirrored_when_right_to_left_property().as_property()
            || property == Visual::flow_direction_property().as_property()
            || property == TemplatedControl::font_family_property().as_property()
            || property == TemplatedControl::font_size_property().as_property()
            || property == TemplatedControl::font_style_property().as_property()
            || property == TemplatedControl::font_weight_property().as_property()
            || property == TemplatedControl::foreground_property().as_property()
        {
            this.update_text();
        }
    }
}

impl VisualImpl for FontIcon {
    fn bypass_flow_direction_policies(_this: &Self) -> bool {
        true
    }
}

impl TemplatedControlImpl for FontIcon {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, args);
        *this.text.borrow_mut() = args.name_scope().find_as::<TextBlock>("PART_Text");
        this.update_text();
    }
}

ferroui_base::ferro_properties! { impl FontIcon {
    ferro_property!(
        /// The text drawn using the icon font. Multiple glyphs are supported.
        pub fn glyph_property() -> StyledProperty<String> {
            FerroProperty::register::<FontIcon, _>("Glyph", String::new())
        }
    );

    ferro_property!(
        /// Whether right-to-left flow reflects the glyph about its center.
        pub fn mirrored_when_right_to_left_property() -> StyledProperty<bool> {
            FerroProperty::register::<FontIcon, _>("MirroredWhenRightToLeft", false)
        }
    );
} }

impl FontIcon {
    fn static_constructor() {
        Layoutable::affects_measure::<FontIcon>(&[
            Self::glyph_property().as_property(),
            TemplatedControl::font_family_property().as_property(),
            TemplatedControl::font_size_property().as_property(),
            TemplatedControl::font_style_property().as_property(),
            TemplatedControl::font_weight_property().as_property(),
        ]);
        TemplatedControl::font_family_property()
            .override_default_value::<FontIcon>(FontFamily::new("Segoe Fluent Icons"));
        TemplatedControl::font_size_property().override_default_value::<FontIcon>(20.0);
        let template: Rc<dyn IControlTemplate> = FuncControlTemplate::for_type::<FontIcon>(|_, scope| {
            let text = TextBlock::new();
            text.set_name(Some("PART_Text".to_string()));
            text.set_text_alignment(TextAlignment::Center);
            text.set_vertical_alignment(VerticalAlignment::Center);
            AutomationProperties::set_accessibility_view(&text, AccessibilityView::Raw);
            text.register_in_name_scope(&**scope).upcast()
        });
        TemplatedControl::template_property().override_default_value::<FontIcon>(Some(template));
    }

    /// Creates the class data for subclasses.
    pub fn construct() -> Self {
        Self { base: IconElement::construct(), text: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn glyph(&self) -> String {
        self.get_value(Self::glyph_property())
    }

    pub fn set_glyph(&self, value: impl Into<String>) {
        self.set_value(Self::glyph_property(), value.into());
    }

    pub fn mirrored_when_right_to_left(&self) -> bool {
        self.get_value(Self::mirrored_when_right_to_left_property())
    }

    pub fn set_mirrored_when_right_to_left(&self, value: bool) {
        self.set_value(Self::mirrored_when_right_to_left_property(), value);
    }

    fn update_text(&self) {
        let text = self.text.borrow().clone();
        let Some(text) = text else { return };
        text.set_text(Some(&self.glyph()));
        text.set_font_family(self.font_family());
        text.set_font_size(self.font_size());
        text.set_font_style(self.font_style());
        text.set_font_weight(self.font_weight());
        text.set_foreground(self.foreground());
        text.set_render_transform_origin(RelativePoint::CENTER);
        let mirrored = self.mirrored_when_right_to_left() && self.flow_direction() == FlowDirection::RightToLeft;
        text.set_render_transform(if mirrored { Some(ScaleTransform::with_scale(-1.0, 1.0).into()) } else { None });
    }
}
