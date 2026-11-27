//! WinUI's `ImageIcon`.

use crate::placement;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::media::{DrawingContext, IImage};
use ferroui_base::reactive::Subscription;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, Size, StyledElementImpl, StyledProperty, Visual, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ControlImpl, IconElement};
use std::cell::RefCell;
use std::rc::Rc;

/// An icon that shows an image in its own colors; `Foreground` has no effect on it.
#[repr(C)]
pub struct ImageIcon {
    base: IconElement,
    /// The load of the source, for a source that finishes loading after it is set.
    source_load: RefCell<Option<Subscription>>,
}

ferro_class!(ImageIcon: IconElement);
ferroui_base::ferro_class_info!(ImageIcon { new: ImageIcon::new });
ferro_impl_classes!(
    ImageIcon: StyledElementImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl FerroObjectImpl for ImageIcon {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::source_property().as_property() {
            this.watch_source_load();
        }
    }
}

impl LayoutableImpl for ImageIcon {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.source().map_or_else(Size::default, |source| placement::measure(available_size, source.size()))
    }

    fn arrange_override(_this: &Self, final_size: Size) -> Size {
        final_size
    }
}

impl VisualImpl for ImageIcon {
    fn render(this: &Self, context: &mut DrawingContext) {
        let Some(source) = this.source() else { return };
        if let Some(rect) = placement::image_rect(this.bounds().size(), source.size()) {
            context.draw_image(&*source, rect);
        }
    }
}

ferro_properties! {
    impl ImageIcon {
        /// Defines the `Source` property.
        pub fn source_property() -> StyledProperty<Option<Rc<dyn IImage>>> {
            FerroProperty::register::<ImageIcon, _>("Source", None)
        }
    }
}

impl ImageIcon {
    fn static_constructor() {
        Visual::affects_render::<ImageIcon>(&[Self::source_property().as_property()]);
        Layoutable::affects_measure::<ImageIcon>(&[Self::source_property().as_property()]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: IconElement::construct(), source_load: RefCell::new(None) }
    }

    /// Creates an icon without an image.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The image of the icon.
    pub fn source(&self) -> Option<Rc<dyn IImage>> {
        self.get_value(Self::source_property())
    }

    pub fn set_source(&self, value: Option<Rc<dyn IImage>>) {
        self.set_value(Self::source_property(), value);
    }

    /// A source that loads later changes the size and the drawing of the icon when it is ready.
    fn watch_source_load(&self) {
        drop(self.source_load.take());
        let Some(source) = self.source() else { return };
        let target = self.to_ref().downgrade();
        let subscription = source.subscribe_load(Rc::new(move |_| {
            if let Some(icon) = target.upgrade() {
                icon.invalidate_measure();
                icon.invalidate_visual();
            }
        }));
        self.source_load.replace(subscription);
    }
}
