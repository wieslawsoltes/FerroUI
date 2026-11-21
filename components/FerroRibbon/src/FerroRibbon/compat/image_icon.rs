// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.
// Native Rust/FerroUI adaptation of Uno ImageIcon.cs/Properties.cs and ImageIcon.Uno.cs.
// Uno-specific portions: Copyright (c) Uno Platform Inc., Apache-2.0.
//! Image-backed menu icon using native Image, IImage, layout and lifecycle services.
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{IImage, SolidColorBrush, Stretch};
use ferroui_base::{
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref,
    Size, StyledElementImpl, StyledProperty, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ControlImpl, ControlImplExt, Grid, IconElement, Image};
use std::{cell::OnceCell, rc::Rc};

#[repr(C)]
pub struct ImageIcon {
    base: IconElement,
    root: OnceCell<Ref<Grid>>,
    image: OnceCell<Ref<Image>>,
}
ferro_class!(ImageIcon: IconElement);
ferro_class_info!(ImageIcon {new: ImageIcon::new, markup: {
    namespace: "FerroRibbon.Compatibility",
    attributes: [UsableDuringInitialization(false)]
}});
ferro_impl_classes!(ImageIcon: StyledElementImpl, VisualImpl, InteractiveImpl,
    InputElementImpl, TemplatedControlImpl);
ferro_properties! {impl ImageIcon {
    pub fn source_property() -> StyledProperty<Option<Rc<dyn IImage>>> {
        FerroProperty::register::<ImageIcon, _>("Source", None)
    }
}}
impl ImageIcon {
    pub fn construct() -> Self {
        Self {
            base: IconElement::construct(),
            root: OnceCell::new(),
            image: OnceCell::new(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn source(&self) -> Option<Rc<dyn IImage>> {
        self.get_value(Self::source_property())
    }
    pub fn set_source(&self, value: Option<Rc<dyn IImage>>) {
        self.set_value(Self::source_property(), value);
    }
    fn ensure_initialized(&self) {
        if self.root.get().is_some() {
            return;
        }
        let image = Image::new();
        image.set_stretch(Stretch::Uniform);
        image.set_source(self.source());
        let root = Grid::new();
        root.set_background(Some(SolidColorBrush::from_uint32(0).into()));
        root.children().add(image.clone());
        self.image
            .set(image)
            .expect("ImageIcon image initialized once");
        self.root
            .set(root.clone())
            .expect("ImageIcon root initialized once");
        root.set_parent(self.to_ref());
        self.visual_children().add(root.clone().upcast());
        self.logical_children().add(root.upcast());
        self.invalidate_measure();
    }
}
impl FerroObjectImpl for ImageIcon {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::source_property().as_property()
            && let Some(image) = this.image.get()
        {
            image.set_source(this.source());
        }
    }
}
impl ControlImpl for ImageIcon {
    fn on_loaded(this: &Self, event: &RoutedEventArgs) {
        // The source installs its initialization handler in the constructor.
        // Initialize before public Loaded handlers and retain the tree on reload.
        this.ensure_initialized();
        Self::parent_on_loaded(this, event);
    }
}
impl LayoutableImpl for ImageIcon {
    fn measure_override(this: &Self, available: Size) -> Size {
        if let Some(root) = this.root.get() {
            root.measure(available);
            root.desired_size()
        } else {
            Size::default()
        }
    }
    fn arrange_override(this: &Self, size: Size) -> Size {
        if let Some(root) = this.root.get() {
            root.arrange(Rect::from_size(size));
        }
        size
    }
}
