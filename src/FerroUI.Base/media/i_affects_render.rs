use crate::media::effects::{Effect, IEffect};
use crate::media::imaging::CroppedBitmap;
use crate::media::{Brush, DrawingImage, Geometry, IBrush, IImage, IPen, ITransform, Pen, Transform};
use crate::reactive::IDisposable;
use crate::Ref;
use std::any::{Any, TypeId};
use std::rc::Rc;

/// Signals that a change of the object requires everything drawn with it to
/// be redrawn.
///
/// Implemented by the mutable media classes: brushes, pens, geometries,
/// transforms, effects and the images that can change.
pub trait IAffectsRender {
    /// Subscribes to invalidation of the object. Disposing the returned
    /// handle unsubscribes.
    fn invalidated(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;
}

impl IAffectsRender for Brush {
    fn invalidated(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Brush::invalidated(self, move || handler())
    }
}

impl IAffectsRender for Pen {
    fn invalidated(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Pen::invalidated(self, move || handler())
    }
}

impl IAffectsRender for Geometry {
    fn invalidated(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.changed(move || handler())
    }
}

impl IAffectsRender for Transform {
    fn invalidated(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.changed(move || handler())
    }
}

impl IAffectsRender for Effect {
    fn invalidated(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Effect::invalidated(self, move || handler())
    }
}

impl IAffectsRender for CroppedBitmap {
    fn invalidated(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        CroppedBitmap::invalidated(self, move || handler())
    }
}

impl IAffectsRender for DrawingImage {
    fn invalidated(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        DrawingImage::invalidated(self, move || handler())
    }
}

/// Subscribes `handler` to the invalidation of a property value, when the
/// value is a media object that can change.
///
/// `value` is the untyped value of a property as carried by a property change
/// notification; the property types that can hold an [`IAffectsRender`] are
/// recognised: optional brushes, pens, transforms, effects and images (as
/// interface handles) and optional geometries and transforms (as class
/// handles). Returns `None` when the value is of another type, is absent or
/// is immutable.
///
/// This is the counterpart of testing a property value for the interface;
/// it lets a visual subscribe uniformly to whatever its render-affecting
/// properties hold.
pub fn subscribe_invalidated(value: &dyn Any, handler: Rc<dyn Fn()>) -> Option<Rc<dyn IDisposable>> {
    if let Some(value) = value.downcast_ref::<Option<Rc<dyn IBrush>>>() {
        let brush = value.as_ref()?.as_object()?.downcast_ref::<Brush>()?;
        return Some(IAffectsRender::invalidated(brush, handler));
    }
    if let Some(value) = value.downcast_ref::<Option<Rc<dyn IPen>>>() {
        let pen = value.as_ref()?.as_object()?.downcast_ref::<Pen>()?;
        return Some(IAffectsRender::invalidated(pen, handler));
    }
    if let Some(value) = value.downcast_ref::<Option<Rc<dyn ITransform>>>() {
        let transform = value.as_ref()?.as_object()?.downcast_ref::<Transform>()?;
        return Some(IAffectsRender::invalidated(transform, handler));
    }
    if let Some(value) = value.downcast_ref::<Option<Ref<Transform>>>() {
        let transform: &Transform = value.as_ref()?;
        return Some(IAffectsRender::invalidated(transform, handler));
    }
    if let Some(value) = value.downcast_ref::<Option<Ref<Geometry>>>() {
        let geometry: &Geometry = value.as_ref()?;
        return Some(IAffectsRender::invalidated(geometry, handler));
    }
    if let Some(value) = value.downcast_ref::<Option<Rc<dyn IEffect>>>() {
        let effect = value.as_ref()?.as_object()?.downcast_ref::<Effect>()?;
        return Some(IAffectsRender::invalidated(effect, handler));
    }
    if let Some(value) = value.downcast_ref::<Option<Rc<dyn IImage>>>() {
        return Some(value.as_ref()?.as_affects_render()?.invalidated(handler));
    }
    None
}

/// Whether a property with values of the given type can hold a media object
/// whose changes affect rendering, i.e. whether
/// [`subscribe_invalidated`] can ever subscribe to one of its values.
pub fn can_value_affect_render(property_type: TypeId) -> bool {
    property_type == TypeId::of::<Option<Rc<dyn IBrush>>>()
        || property_type == TypeId::of::<Option<Rc<dyn IPen>>>()
        || property_type == TypeId::of::<Option<Rc<dyn ITransform>>>()
        || property_type == TypeId::of::<Option<Ref<Transform>>>()
        || property_type == TypeId::of::<Option<Ref<Geometry>>>()
        || property_type == TypeId::of::<Option<Rc<dyn IEffect>>>()
        || property_type == TypeId::of::<Option<Rc<dyn IImage>>>()
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;
    use crate::media::effects::{BlurEffect, ImmutableBlurEffect};
    use crate::media::immutable::ImmutableSolidColorBrush;
    use crate::media::{Colors, DrawingImage, RectangleGeometry, SolidColorBrush, TranslateTransform};
    use crate::Rect;
    use std::cell::Cell;

    fn counting() -> (Rc<Cell<i32>>, Rc<dyn Fn()>) {
        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        (count, Rc::new(move || c.set(c.get() + 1)))
    }

    #[test]
    fn subscribes_to_mutable_media_values() {
        let (count, handler) = counting();

        let brush = SolidColorBrush::new();
        let value: Option<Rc<dyn IBrush>> = Some(brush.clone().into());
        let subscription = subscribe_invalidated(&value, handler.clone()).unwrap();
        brush.set_color(Colors::RED);
        assert_eq!(1, count.get());
        subscription.dispose();
        brush.set_color(Colors::BLUE);
        assert_eq!(1, count.get());

        let pen = Pen::new();
        let value: Option<Rc<dyn IPen>> = Some(pen.clone().into());
        subscribe_invalidated(&value, handler.clone()).unwrap();
        pen.set_thickness(3.0);
        assert_eq!(2, count.get());

        let transform = TranslateTransform::new();
        let value: Option<Rc<dyn ITransform>> = Some(transform.clone().into());
        subscribe_invalidated(&value, handler.clone()).unwrap();
        let value: Option<Ref<Transform>> = Some(transform.clone().upcast());
        subscribe_invalidated(&value, handler.clone()).unwrap();
        transform.set_x(1.0);
        assert_eq!(4, count.get());

        let geometry = RectangleGeometry::new();
        let value: Option<Ref<Geometry>> = Some(geometry.clone().upcast());
        subscribe_invalidated(&value, handler.clone()).unwrap();
        geometry.set_rect(Rect::new(0.0, 0.0, 1.0, 1.0));
        assert_eq!(5, count.get());

        let effect = BlurEffect::new();
        let value: Option<Rc<dyn IEffect>> = Some(effect.clone().into());
        subscribe_invalidated(&value, handler.clone()).unwrap();
        effect.set_radius(1.0);
        assert_eq!(6, count.get());

        let image = DrawingImage::new();
        let value: Option<Rc<dyn IImage>> = Some(image.clone().into());
        subscribe_invalidated(&value, handler).unwrap();
        image.set_viewbox(Some(Rect::new(0.0, 0.0, 1.0, 1.0)));
        assert_eq!(7, count.get());
    }

    #[test]
    fn ignores_immutable_absent_and_unrelated_values() {
        let (_, handler) = counting();

        let value: Option<Rc<dyn IBrush>> = Some(Rc::new(ImmutableSolidColorBrush::new(Colors::RED)));
        assert!(subscribe_invalidated(&value, handler.clone()).is_none());
        let value: Option<Rc<dyn IBrush>> = None;
        assert!(subscribe_invalidated(&value, handler.clone()).is_none());
        let value: Option<Rc<dyn IEffect>> = Some(Rc::new(ImmutableBlurEffect::new(1.0)));
        assert!(subscribe_invalidated(&value, handler.clone()).is_none());
        assert!(subscribe_invalidated(&1.0f64, handler.clone()).is_none());
        assert!(subscribe_invalidated(&Rect::default(), handler).is_none());

        assert!(can_value_affect_render(TypeId::of::<Option<Rc<dyn IBrush>>>()));
        assert!(can_value_affect_render(TypeId::of::<Option<Ref<Geometry>>>()));
        assert!(!can_value_affect_render(TypeId::of::<f64>()));
        assert!(!can_value_affect_render(TypeId::of::<Rect>()));
    }
}
