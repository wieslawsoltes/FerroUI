// SPDX-License-Identifier: Apache-2.0
// Native FerroUI adaptation of the pinned BitmapIcon implementation.
//! Bitmap icon with one retained native Image child and a mutable color filter.
use crate::primitives::{RibbonDeferredImage, RibbonIconPresenter, RibbonImageLoadError};
use ferroribbon_core::RibbonString;
use ferroui_base::{
    FerroObject, FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElementImpl, StyledProperty, VisualImpl,
    VisualImplExt,
    data::BindingPriority,
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    media::{
        Color, DrawingContext, SolidColorBrush, Stretch, imaging::BitmapBlendingMode,
    },
    styling::ThemeVariant,
    utilities::Uri,
};
use ferroui_controls::{
    ControlImpl, Grid, IconElement, Image,
    primitives::{TemplatedControl, TemplatedControlImpl},
};
use ferroui_system::IDisposable;
use std::{
    cell::{Cell, OnceCell},
    rc::Rc,
};

/// The source Image owns its color filter independently of Image.Source. Keep
/// that state on a private native Image subclass, including for replacement
/// sources. The native Image renders normally; SourceIn composition applies
/// the color to its isolated output without resampling source alpha.
#[repr(C)]
struct BitmapIconImage {
    base: Image,
    monochrome: Cell<Option<Color>>,
}
ferro_class!(BitmapIconImage:Image);
ferro_class_info!(BitmapIconImage {});
ferro_impl_classes!(BitmapIconImage:StyledElementImpl,InteractiveImpl,InputElementImpl,ControlImpl,LayoutableImpl);
impl BitmapIconImage {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: Image::construct(),
            monochrome: Cell::new(None),
        })
    }
    fn set_monochrome_color(&self, color: Option<Color>) {
        if self.monochrome.replace(color) != color {
            self.invalidate_visual();
        }
    }
}
impl FerroObjectImpl for BitmapIconImage {}
impl VisualImpl for BitmapIconImage {
    fn render(this: &Self, context: &mut DrawingContext<'_>) {
        let Some(color) = this.monochrome.get() else {
            Self::parent_render(this, context);
            return;
        };
        let bounds = this.bounds();
        if !(bounds.width > 0. && bounds.height > 0.) || this.source().is_none() {
            return;
        }
        let area = Rect::from_size(bounds.size());
        let Some(layer) = context.try_push_compositing_layer(BitmapBlendingMode::SourceOver, 1.0, area) else {
            // A context without compositing layers cannot isolate the tint: draw the image as it is.
            Self::parent_render(this, context);
            return;
        };
        let mut layer_scope = context.scope(layer);
        Self::parent_render(this, &mut layer_scope);
        layer_scope.draw_color(
            color,
            BitmapBlendingMode::SourceIn,
            area,
        );
    }
}

#[repr(C)]
pub struct BitmapIcon {
    base: IconElement,
    root: OnceCell<Ref<Grid>>,
    image: OnceCell<Ref<BitmapIconImage>>,
    source_binding: OnceCell<Rc<dyn IDisposable>>,
}
ferro_class!(BitmapIcon:IconElement);
ferro_class_info!(BitmapIcon {new:BitmapIcon::new,markup:{namespace:"FerroRibbon.Compatibility",attributes:[UsableDuringInitialization(false)]}});
ferro_impl_classes!(BitmapIcon:StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
ferro_properties! {impl BitmapIcon {
    pub fn uri_source_property()->StyledProperty<Option<Uri>> {FerroProperty::register::<Self,_>("UriSource",None)}
    pub fn show_as_monochrome_property()->StyledProperty<bool> {FerroProperty::register::<Self,_>("ShowAsMonochrome",true)}
}}
impl BitmapIcon {
    fn static_constructor() {
        // The source IconElement DP defaults to White before inherited/theme values.
        TemplatedControl::foreground_property()
            .override_default_value::<Self>(Some(SolidColorBrush::from_uint32(0xffffffff).into()));
    }
    pub fn construct() -> Self {
        Self {
            base: IconElement::construct(),
            root: OnceCell::new(),
            image: OnceCell::new(),
            source_binding: OnceCell::new(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn uri_source(&self) -> Option<Uri> {
        self.get_value(Self::uri_source_property())
    }
    pub fn set_uri_source(&self, value: Option<Uri>) {
        self.set_value(Self::uri_source_property(), value);
    }
    pub fn show_as_monochrome(&self) -> bool {
        self.get_value(Self::show_as_monochrome_property())
    }
    #[cfg(test)]
    pub(crate) fn cached_bitmap_tint_for_test(&self) -> Option<Color> {
        self.image.get().and_then(|image| image.monochrome.get())
    }
    pub fn set_show_as_monochrome(&self, value: bool) {
        self.set_value(Self::show_as_monochrome_property(), value);
    }
    /// Returns the loading failure of the image source currently displayed.
    pub fn load_error(&self) -> Option<Rc<RibbonImageLoadError>> {
        self.image
            .get()?
            .displayed_source()?
            .as_object()?
            .downcast_ref::<RibbonDeferredImage>()?
            .load_result()?
            .err()
    }
    fn monochrome_color(&self) -> Option<Color> {
        self.show_as_monochrome()
            .then(|| {
                self.foreground()
                    .and_then(|b| b.as_solid_color_brush().map(|b| b.color()))
            })
            .flatten()
    }
    fn update_monochrome_color(&self) {
        let color = self.monochrome_color();
        if let Some(image) = self.image.get() {
            image.set_monochrome_color(color);
        }
    }
}
impl FerroObjectImpl for BitmapIcon {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let root = Grid::new();
        let image = BitmapIconImage::new();
        image.set_stretch(Stretch::Uniform);
        root.set_background(Some(SolidColorBrush::from_uint32(0).into()));
        root.children().add(image.clone());
        root.set_parent(this.to_ref());
        this.visual_children().add(root.clone().upcast());
        this.logical_children().add(root.clone().upcast());
        this.root
            .set(root)
            .expect("BitmapIcon root initialized once");
        this.image
            .set(image)
            .expect("BitmapIcon image initialized once");
        this.update_monochrome_color();
        let source = FerroObject::get_observable_with(this, Self::uri_source_property(), |uri| {
            let uri = uri?;
            let loaded = RibbonIconPresenter::create_uri_image(
                &RibbonString::from(uri.original_string()),
                None,
            );
            Some(loaded.unwrap_or_else(RibbonDeferredImage::failed))
        });
        assert!(
            this.source_binding
                .set(this.image.get().unwrap().bind(
                    Image::source_property(),
                    source,
                    BindingPriority::LocalValue
                ))
                .is_ok(),
            "BitmapIcon source binding initialized once"
        );
    }
    fn on_property_changed(this: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, e);
        if e.property() == Self::show_as_monochrome_property().as_property()
            || e.property() == ThemeVariant::actual_theme_variant_property().as_property()
            || e.property() == TemplatedControl::foreground_property().as_property()
                && this.show_as_monochrome()
        {
            this.update_monochrome_color();
        }
    }
}
impl Drop for BitmapIcon {
    fn drop(&mut self) {
        // The selected SDK returns IDisposable. Keep that boundary local and
        // release it with the Rust owner; no port-owned disposable hierarchy.
        if let Some(binding) = self.source_binding.get() {
            binding.dispose();
        }
    }
}
impl LayoutableImpl for BitmapIcon {
    fn apply_template(this: &Self) {
        assert!(
            this.template().is_none(),
            "BitmapIcon has no source Template property"
        );
    }
    fn measure_override(this: &Self, size: Size) -> Size {
        let root = this.root.get().expect("BitmapIcon root");
        root.measure(size);
        root.desired_size()
    }
    fn arrange_override(this: &Self, size: Size) -> Size {
        this.root
            .get()
            .expect("BitmapIcon root")
            .arrange(Rect::from_size(size));
        size
    }
}
