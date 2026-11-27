// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.
// Native Rust/FerroUI adaptation of Uno ImageIcon.cs/Properties.cs and ImageIcon.Uno.cs.
// Uno-specific portions: Copyright (c) Uno Platform Inc., Apache-2.0.
//! What the ribbon's menus and icon presenter show for an `ImageIcon`, an `ImageIconSource` or an
//! image URI: a native Image child, which picks the scale and theme variant of the source and follows
//! its deferred load. It is internal: the public `ImageIcon` is the framework's.
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
pub struct RibbonImageElement {
    base: IconElement,
    root: OnceCell<Ref<Grid>>,
    image: OnceCell<Ref<Image>>,
}
ferro_class!(RibbonImageElement: IconElement);
ferro_class_info!(RibbonImageElement {});
ferro_impl_classes!(RibbonImageElement: StyledElementImpl, VisualImpl, InteractiveImpl,
    InputElementImpl, TemplatedControlImpl);
ferro_properties! {impl RibbonImageElement {
    pub fn source_property() -> StyledProperty<Option<Rc<dyn IImage>>> {
        FerroProperty::register::<RibbonImageElement, _>("Source", None)
    }
}}
impl RibbonImageElement {
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
    /// What a presenter or a menu shows for an image.
    pub(crate) fn with_source(source: Option<Rc<dyn IImage>>) -> Ref<IconElement> {
        let element = Self::new();
        element.set_source(source);
        element.upcast()
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
            .expect("image initialized once");
        self.root
            .set(root.clone())
            .expect("root initialized once");
        root.set_parent(self.to_ref());
        self.visual_children().add(root.clone().upcast());
        self.logical_children().add(root.upcast());
        self.invalidate_measure();
    }
}
impl FerroObjectImpl for RibbonImageElement {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::source_property().as_property()
            && let Some(image) = this.image.get()
        {
            image.set_source(this.source());
        }
    }
}
impl ControlImpl for RibbonImageElement {
    fn on_loaded(this: &Self, event: &RoutedEventArgs) {
        // The source installs its initialization handler in the constructor.
        // Initialize before public Loaded handlers and retain the tree on reload.
        this.ensure_initialized();
        Self::parent_on_loaded(this, event);
    }
}
impl LayoutableImpl for RibbonImageElement {
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
