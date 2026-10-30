//! WinUI's `FontIcon` (`CFontIcon` of `dxaml/xcp/core/core/elements/icon.cpp`, microsoft-ui-xaml
//! commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use crate::primitives::{TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use crate::{ControlImpl, IconElement, TextBlock};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::media::{FlowDirection, ScaleTransform};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, RelativePoint, StyledElementImpl, StyledProperty, Visual, VisualImpl,
};
use std::cell::RefCell;

/// Represents an icon that uses a glyph of a font as its content (WinUI's `FontIcon`).
///
/// The template of the theme draws [`glyph`](Self::glyph) in its text part `PART_Text` with the
/// font family, size, style, weight and foreground of the icon. The size is 20 by default, as in
/// WinUI; the font family is the theme resource `SymbolThemeFontFamily` (Segoe Fluent Icons in the
/// WinUI theme), set by the theme, where WinUI takes it from the default font of a font icon. The
/// font is not bundled: off Windows an application supplies a symbol font and sets
/// `SymbolThemeFontFamily` or the icon's `FontFamily` (docs/porting/winui-controls.md, section
/// 5.2). The WinUI theme has the template; the Fluent and Simple themes have none.
///
/// Missing (docs/porting/winui-controls.md, FontIcon): `IsTextScaleFactorEnabled` (FerroUI has no
/// text scale factor).
#[repr(C)]
pub struct FontIcon {
    base: IconElement,
    text: RefCell<Option<Ref<TextBlock>>>,
}

ferro_class!(FontIcon: IconElement);
ferroui_base::ferro_class_info!(FontIcon {
    new: FontIcon::new,
    markup: {
        namespace: "FerroUI.Controls",
        attributes: [TemplatePart("PART_Text", type(Ref<TextBlock>))],
    },
});
ferro_impl_classes!(FontIcon: StyledElementImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for FontIcon {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        // CFontIcon::OnPropertyChanged: the glyph, the text properties and the flow direction go
        // to the text block.
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
            this.update_child_text_block();
        }
    }
}

impl VisualImpl for FontIcon {
    // WinUI's icons draw their glyph as it is in a right-to-left flow; a font icon mirrors it only
    // with MirroredWhenRightToLeft (CFontIcon::ApplyScaleTransformForFlowDirection).
    fn bypass_flow_direction_policies(_this: &Self) -> bool {
        true
    }
}

impl TemplatedControlImpl for FontIcon {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, args);
        *this.text.borrow_mut() = args.name_scope().find_as::<TextBlock>("PART_Text");
        this.update_child_text_block();
    }
}

ferroui_base::ferro_properties! { impl FontIcon {
    ferro_property!(
        /// Defines the `Glyph` property: the text drawn with the icon font, one glyph or more.
        pub fn glyph_property() -> StyledProperty<String> {
            FerroProperty::register::<FontIcon, _>("Glyph", String::new())
        }
    );

    ferro_property!(
        /// Defines the `MirroredWhenRightToLeft` property: whether the glyph is mirrored when the
        /// flow direction is right to left. `false` by default.
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
        // FontIcon.FontSize defaults to 20 in WinUI (g_ClientCoreFontSize).
        TemplatedControl::font_size_property().override_default_value::<FontIcon>(20.0);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: IconElement::construct(), text: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the text drawn with the icon font.
    pub fn glyph(&self) -> String {
        self.get_value(Self::glyph_property())
    }

    /// Sets the text drawn with the icon font.
    pub fn set_glyph(&self, value: impl Into<String>) {
        self.set_value(Self::glyph_property(), value.into());
    }

    /// Gets whether the glyph is mirrored when the flow direction is right to left.
    pub fn mirrored_when_right_to_left(&self) -> bool {
        self.get_value(Self::mirrored_when_right_to_left_property())
    }

    /// Sets whether the glyph is mirrored when the flow direction is right to left.
    pub fn set_mirrored_when_right_to_left(&self, value: bool) {
        self.set_value(Self::mirrored_when_right_to_left_property(), value);
    }

    /// The text block of the template gets the glyph and the text properties of the icon, and a
    /// mirroring transform when the icon is mirrored (CFontIcon::ApplyTemplate and
    /// ApplyScaleTransformForFlowDirection).
    fn update_child_text_block(&self) {
        let text = self.text.borrow().clone();
        let Some(text) = text else { return };
        text.set_text(Some(&self.glyph()));
        text.set_font_family(self.font_family());
        text.set_font_size(self.font_size());
        text.set_font_style(self.font_style());
        text.set_font_weight(self.font_weight());
        text.set_foreground(self.foreground());
        let mirrored = self.mirrored_when_right_to_left() && self.flow_direction() == FlowDirection::RightToLeft;
        text.set_render_transform_origin(RelativePoint::CENTER);
        text.set_render_transform(if mirrored { Some(ScaleTransform::with_scale(-1.0, 1.0).into()) } else { None });
    }
}
