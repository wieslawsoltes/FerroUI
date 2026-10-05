use crate::media::ref_adapter::RefAdapter;
use crate::media::{Drawing, DrawingContext, IAffectsRender, IImage};
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::HandlerList;
use crate::{
    ferro_class, ferro_property, instantiate, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Matrix, Nullable, Rect, Ref, Size, StyledProperty,
};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// An image that uses a [`Drawing`] for content.
#[repr(C)]
pub struct DrawingImage {
    base: FerroObject,
    invalidated: HandlerList<dyn Fn()>,
    drawing_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(DrawingImage: FerroObject);
crate::ferro_class_info!(DrawingImage { new: DrawingImage::new, interfaces: [std::rc::Rc<dyn crate::media::IImage>] });

impl FerroObjectImpl for DrawingImage {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::drawing_property().as_property() {
            let new_value = change.get_new_value::<Option<Ref<Drawing>>>();

            if let Some(subscription) = this.drawing_subscription.take() {
                subscription.dispose();
            }
            if let Some(new_value) = new_value {
                let weak = this.to_ref().downgrade();
                let subscription = new_value.invalidated(move || {
                    if let Some(this) = weak.upgrade() {
                        this.raise_invalidated();
                    }
                });
                *this.drawing_subscription.borrow_mut() = Some(subscription);
            }

            this.raise_invalidated();
        } else if change.property() == Self::viewbox_property().as_property() {
            this.raise_invalidated();
        }
    }
}

crate::ferro_properties! { impl DrawingImage {
    ferro_property!(
        /// Defines the `Drawing` property.
        pub fn drawing_property() -> StyledProperty<Option<Ref<Drawing>>> {
            FerroProperty::register::<DrawingImage, _>("Drawing", None)
        }
    );

    ferro_property!(
        /// Defines the `Viewbox` property.
        pub fn viewbox_property() -> StyledProperty<Option<Rect>> {
            FerroProperty::register::<DrawingImage, _>("Viewbox", None)
        }
    );
} }

impl DrawingImage {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            invalidated: HandlerList::new(),
            drawing_subscription: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates an image showing `drawing`.
    pub fn with_drawing(drawing: impl Into<Nullable<Drawing>>) -> Ref<Self> {
        let result = Self::new();
        result.set_drawing(drawing);
        result
    }

    /// Raised when the image changes.
    pub fn invalidated(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.invalidated.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.invalidated.remove(token);
            }
        })
    }

    /// Raises the invalidated notification.
    pub fn raise_invalidated(&self) {
        if self.invalidated.is_empty() {
            return;
        }
        for (_, handler) in self.invalidated.snapshot().iter() {
            handler();
        }
    }

    /// The drawing content.
    pub fn drawing(&self) -> Option<Ref<Drawing>> {
        self.get_value(Self::drawing_property())
    }

    pub fn set_drawing(&self, value: impl Into<Nullable<Drawing>>) {
        self.set_value(Self::drawing_property(), value.into().0)
    }

    /// A rectangular region of the drawing, in device independent pixels, to
    /// display when rendering this image.
    ///
    /// This value can be used to display only part of the drawing, or to
    /// surround it with empty space. If `None`, the bounds of the drawing
    /// are used.
    pub fn viewbox(&self) -> Option<Rect> {
        self.get_value(Self::viewbox_property())
    }

    pub fn set_viewbox(&self, value: Option<Rect>) {
        self.set_value(Self::viewbox_property(), value)
    }

    /// The size of the image, in device independent pixels.
    pub fn size(&self) -> Size {
        self.get_bounds().size()
    }

    fn get_bounds(&self) -> Rect {
        match self.viewbox() {
            Some(viewbox) => viewbox,
            None => self.drawing().map(|drawing| drawing.get_bounds()).unwrap_or_default(),
        }
    }

    /// Draws the part of the drawing inside `source_rect` into `dest_rect`.
    pub fn draw(&self, context: &mut DrawingContext<'_>, source_rect: Rect, dest_rect: Rect) {
        let Some(drawing) = self.drawing() else { return };
        if source_rect.size() == Size::default() || dest_rect.size() == Size::default() {
            return;
        }

        let bounds = self.get_bounds();

        if bounds.size() == Size::default() {
            return;
        }

        let scale = Matrix::create_scale(dest_rect.width / source_rect.width, dest_rect.height / source_rect.height);
        let translate = Matrix::create_translation(
            -source_rect.x + dest_rect.x - bounds.x,
            -source_rect.y + dest_rect.y - bounds.y,
        );

        let clip = context.push_clip(dest_rect);
        let transform = context.push_transform(translate * scale);
        drawing.draw(context);
        context.pop(transform);
        context.pop(clip);
    }
}

impl IImage for RefAdapter<DrawingImage> {
    fn size(&self) -> Size {
        self.0.size()
    }

    fn draw(&self, context: &mut DrawingContext<'_>, source_rect: Rect, dest_rect: Rect) {
        self.0.draw(context, source_rect, dest_rect)
    }

    fn as_any(&self) -> &dyn Any {
        &*self.0
    }

    fn as_object(&self) -> Option<&FerroObject> {
        Some(self.object())
    }

    fn as_affects_render(&self) -> Option<&dyn IAffectsRender> {
        Some(&*self.0)
    }

    fn reference_id(&self) -> *const () {
        RefAdapter::reference_id(self)
    }
}

impl From<Ref<DrawingImage>> for Rc<dyn IImage> {
    #[inline]
    fn from(value: Ref<DrawingImage>) -> Self {
        Rc::new(RefAdapter(value))
    }
}

impl From<&Ref<DrawingImage>> for Rc<dyn IImage> {
    #[inline]
    fn from(value: &Ref<DrawingImage>) -> Self {
        Rc::new(RefAdapter(value.clone()))
    }
}
