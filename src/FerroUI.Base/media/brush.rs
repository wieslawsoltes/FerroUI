use crate::animation::Animatable;
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::ref_adapter::RefAdapter;
use crate::media::{Color, IBrush, ITransform, KnownColors};
use crate::reactive::{Disposable, IDisposable};
use crate::rendering::composition::drawing::{
    transform_get_server, CompositorResourceHolder, ICompositionRenderResource,
};
use crate::rendering::composition::generated::ServerCompositionSimpleBrushProps;
use crate::rendering::composition::server::{IServerObject, ServerCompositor, ServerObjectId};
use crate::rendering::composition::transport::BatchStreamWriter;
use crate::rendering::composition::{Compositor, ICompositorSerializable};
use crate::utilities::{FormatError, HandlerList};
use crate::{
    ferro_class, ferro_property, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, RelativePoint, StyledProperty, Upcast,
};
use std::rc::Rc;

/// Describes how an area is painted. Abstract base class of the mutable
/// brushes.
#[repr(C)]
pub struct Brush {
    base: Animatable,
    invalidated: HandlerList<dyn Fn()>,
    resource: CompositorResourceHolder,
}

/// Creates the server-side counterpart of a brush on the render thread.
pub type ServerBrushFactory = fn(&Rc<ServerCompositor>) -> Rc<dyn IServerObject>;

ferro_class! {
    Brush: Animatable, virtuals BrushImpl: FerroObjectImpl {
        /// The factory of the server-side counterpart of the brush. `None`
        /// for a brush class that has no server-side counterpart: such a
        /// brush is not a composition render resource.
        fn factory(this) -> Option<ServerBrushFactory>;

        /// Called when the brush got its counterpart on a compositor.
        fn on_referenced_from_compositor(this, c: &Rc<Compositor>);

        /// Called when the brush lost its counterpart on a compositor.
        fn on_unreferenced_from_compositor(this, c: &Rc<Compositor>);

        /// Writes the state of the brush for its counterpart on a
        /// compositor.
        fn serialize_changes(this, c: &Compositor, writer: &mut BatchStreamWriter<'_>);
    }
}
crate::ferro_class_info!(Brush { interfaces: [std::rc::Rc<dyn crate::media::IBrush>] });

impl FerroObjectImpl for Brush {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == Self::transform_property().as_property()
            || change.property() == Self::relative_transform_property().as_property()
        {
            let (old_value, new_value) = change.get_old_and_new_value::<Option<Rc<dyn ITransform>>>();
            this.resource.process_property_change_notification(
                old_value.as_ref().and_then(|t| t.as_composition_render_resource()),
                new_value.as_ref().and_then(|t| t.as_composition_render_resource()),
            );
        }

        this.register_for_serialization();
        this.raise_invalidated();

        Self::parent_on_property_changed(this, change);
    }
}

impl BrushImpl for Brush {
    fn factory(_this: &Self) -> Option<ServerBrushFactory> {
        None
    }

    fn on_referenced_from_compositor(this: &Self, c: &Rc<Compositor>) {
        if let Some(transform) = this.transform() {
            if let Some(transform) = transform.as_composition_render_resource() {
                transform.add_ref_on_compositor(c);
            }
        }
        if let Some(relative_transform) = this.relative_transform() {
            if let Some(relative_transform) = relative_transform.as_composition_render_resource() {
                relative_transform.add_ref_on_compositor(c);
            }
        }
    }

    fn on_unreferenced_from_compositor(this: &Self, c: &Rc<Compositor>) {
        if let Some(transform) = this.transform() {
            if let Some(transform) = transform.as_composition_render_resource() {
                transform.release_on_compositor(c);
            }
        }
        if let Some(relative_transform) = this.relative_transform() {
            if let Some(relative_transform) = relative_transform.as_composition_render_resource() {
                relative_transform.release_on_compositor(c);
            }
        }
    }

    fn serialize_changes(this: &Self, c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        ServerCompositionSimpleBrushProps::serialize_all_changes(
            writer,
            this.opacity(),
            this.transform_origin(),
            transform_get_server(this.transform().as_ref(), Some(c)),
            transform_get_server(this.relative_transform().as_ref(), Some(c)),
        );
    }
}

impl ICompositionRenderResource for Brush {
    fn add_ref_on_compositor(&self, c: &Rc<Compositor>) {
        let Some(factory) = self.factory() else {
            panic!("{} is not compatible with composition", self.get_type().name());
        };
        let owner = self.as_compositor_serializable();
        let (_, created) = self
            .resource
            .create_or_add_ref(c, Some(owner), |c| c.create_server_object(move |server, _| factory(server)));
        if created {
            self.on_referenced_from_compositor(c);
        }
    }

    fn release_on_compositor(&self, c: &Rc<Compositor>) {
        if self.resource.release(c) {
            self.on_unreferenced_from_compositor(c);
        }
    }

    fn get_for_compositor(&self, c: &Compositor) -> ServerObjectId {
        self.resource.get_for_compositor(c)
    }
}

impl ICompositorSerializable for RefAdapter<Brush> {
    fn try_get_server(&self, c: &Compositor) -> Option<ServerObjectId> {
        self.0.resource.try_get_for_compositor(c)
    }

    fn serialization_key(&self) -> *const () {
        RefAdapter::reference_id(self)
    }

    fn serialize_changes(&self, c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        self.0.serialize_changes(c, writer);
    }
}

crate::ferro_properties! { impl Brush {
    ferro_property!(pub fn opacity_property() -> StyledProperty<f64> {
        FerroProperty::register::<Brush, _>("Opacity", 1.0)
    });

    ferro_property!(pub fn transform_property() -> StyledProperty<Option<Rc<dyn ITransform>>> {
        FerroProperty::register::<Brush, _>("Transform", None)
    });

    ferro_property!(pub fn transform_origin_property() -> StyledProperty<RelativePoint> {
        FerroProperty::register::<Brush, _>("TransformOrigin", RelativePoint::default())
    });

    ferro_property!(pub fn relative_transform_property() -> StyledProperty<Option<Rc<dyn ITransform>>> {
        FerroProperty::register::<Brush, _>("RelativeTransform", None)
    });
} }

impl Brush {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Animatable::construct(), invalidated: HandlerList::new(), resource: CompositorResourceHolder::new() }
    }

    /// The opacity of the brush.
    pub fn opacity(&self) -> f64 {
        self.get_value(Self::opacity_property())
    }

    pub fn set_opacity(&self, value: f64) {
        self.set_value(Self::opacity_property(), value)
    }

    /// The transform of the brush.
    pub fn transform(&self) -> Option<Rc<dyn ITransform>> {
        self.get_value(Self::transform_property())
    }

    pub fn set_transform(&self, value: Option<Rc<dyn ITransform>>) {
        self.set_value(Self::transform_property(), value)
    }

    /// The origin of the brush [`transform`](Self::transform).
    pub fn transform_origin(&self) -> RelativePoint {
        self.get_value(Self::transform_origin_property())
    }

    pub fn set_transform_origin(&self, value: RelativePoint) {
        self.set_value(Self::transform_origin_property(), value)
    }

    /// The transform of the brush, relative to the bounds of the painted
    /// area.
    pub fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
        self.get_value(Self::relative_transform_property())
    }

    pub fn set_relative_transform(&self, value: Option<Rc<dyn ITransform>>) {
        self.set_value(Self::relative_transform_property(), value)
    }

    /// Parses a brush string: a known color name or any color format
    /// accepted by [`Color::parse`].
    pub fn parse(s: &str) -> Result<Rc<dyn IBrush>, FormatError> {
        if !s.is_empty() {
            // Attempt to get a cached known brush first
            // This is a performance optimization for known colors
            if let Some(brush) = KnownColors::get_known_brush(s) {
                return Ok(brush);
            }

            if let Some(color) = Color::try_parse(s) {
                return Ok(Rc::new(ImmutableSolidColorBrush::new(color)));
            }
        }

        Err(FormatError::from_string(format!("Invalid brush string: '{s}'.")))
    }

    /// The object as something a compositor serializes: the adapter keeps
    /// the object alive while it is queued.
    pub(crate) fn as_compositor_serializable(&self) -> Rc<dyn ICompositorSerializable> {
        Rc::new(RefAdapter(self.to_ref()))
    }

    /// Queues the brush for serialization on every compositor it has a
    /// server-side counterpart on.
    pub(crate) fn register_for_serialization(&self) {
        if self.resource.is_attached() {
            let serializable = self.as_compositor_serializable();
            self.resource.register_for_invalidation_on_all_compositors(&serializable);
        }
    }

    /// Subscribes to invalidation of the brush: raised whenever a change
    /// requires the brush to be redrawn. Disposing the returned handle
    /// unsubscribes.
    pub fn invalidated(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.invalidated.add(Rc::new(handler));
        let object: &FerroObject = self.upcast();
        let weak = object.to_weak();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                if let Some(this) = this.downcast_ref::<Brush>() {
                    this.invalidated.remove(token);
                }
            }
        })
    }

    /// Raises the invalidated notification.
    pub(crate) fn raise_invalidated(&self) {
        if self.invalidated.is_empty() {
            return;
        }
        for (_, handler) in self.invalidated.snapshot().iter() {
            handler();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::{BrushExtensions, Colors, ISolidColorBrush, SolidColorBrush};
    use std::cell::Cell;

    fn solid(brush: &Rc<dyn IBrush>) -> &dyn ISolidColorBrush {
        brush.as_solid_color_brush().expect("solid color brush")
    }

    #[test]
    fn parse_parses_rgb_hash_brush() {
        let result = Brush::parse("#ff8844").unwrap();
        let color = solid(&result).color();
        assert_eq!((0xff, 0x88, 0x44, 0xff), (color.r, color.g, color.b, color.a));
    }

    #[test]
    fn parse_parses_argb_hash_brush() {
        let result = Brush::parse("#40ff8844").unwrap();
        let color = solid(&result).color();
        assert_eq!((0xff, 0x88, 0x44, 0x40), (color.r, color.g, color.b, color.a));
    }

    #[test]
    fn parse_parses_named_brush_lowercase() {
        let result = Brush::parse("red").unwrap();
        let color = solid(&result).color();
        assert_eq!((0xff, 0x00, 0x00, 0xff), (color.r, color.g, color.b, color.a));
    }

    #[test]
    fn parse_parses_named_brush_uppercase() {
        let result = Brush::parse("RED").unwrap();
        let color = solid(&result).color();
        assert_eq!((0xff, 0x00, 0x00, 0xff), (color.r, color.g, color.b, color.a));
    }

    #[test]
    fn parse_to_string_named_brush_roundtrip() {
        let brush = Brush::parse("Red").unwrap();
        let brush = brush.as_any().downcast_ref::<ImmutableSolidColorBrush>().unwrap();
        assert_eq!("Red", brush.to_string());
    }

    #[test]
    fn parse_hex_value_doesnt_accept_too_few_chars() {
        assert!(Brush::parse("#ff").is_err());
    }

    #[test]
    fn parse_hex_value_doesnt_accept_too_many_chars() {
        assert!(Brush::parse("#ff5555555").is_err());
    }

    #[test]
    fn parse_hex_value_doesnt_accept_invalid_number() {
        assert!(Brush::parse("#ff808g80").is_err());
    }

    #[test]
    fn parse_parses_all_color_format_brushes() {
        for input in [
            "rgb(255, 128, 64)",
            "rgba(255, 128, 64, 0.5)",
            "hsl(120, 100%, 50%)",
            "hsla(120, 100%, 50%, 0.5)",
            "hsv(300, 100%, 25%)",
            "hsva(300, 100%, 25%, 0.75)",
            "#40ff8844",
            "Green",
        ] {
            let brush = Brush::parse(input).unwrap();
            // The color tests already validate all color formats are parsed properly.
            let expected = Color::parse(input).unwrap();
            assert_eq!(Some(expected), brush.as_solid_color_brush().map(|b| b.color()));
        }
    }

    #[test]
    fn parse_rejects_empty_string() {
        assert!(Brush::parse("").is_err());
    }

    #[test]
    fn changing_opacity_raises_invalidated() {
        let target = SolidColorBrush::new();
        let raised = Rc::new(Cell::new(false));
        let r = raised.clone();
        target.invalidated(move || r.set(true));
        target.set_opacity(0.5);
        assert!(raised.get());
    }

    #[test]
    fn brush_property_value_accepts_mutable_and_immutable_brushes() {
        let mutable = SolidColorBrush::with_color(Colors::RED);
        let a: Rc<dyn IBrush> = mutable.clone().into();
        let b: Rc<dyn IBrush> = (&mutable).into();
        // Two handles of the same mutable brush are equal; mutable brushes
        // compare by reference.
        assert!(*a == *b);
        let other: Rc<dyn IBrush> = SolidColorBrush::with_color(Colors::RED).into();
        assert!(*a != *other);

        // Immutable solid color brushes compare structurally.
        let i1: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::RED));
        let i2: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::RED));
        assert!(*i1 == *i2);
        assert!(*i1 != *a);
        assert!(*a != *i1);

        // Interface casts.
        assert_eq!(Some(Colors::RED), a.as_solid_color_brush().map(|b| b.color()));
        assert!(a.as_gradient_brush().is_none());
        assert!(a.as_mutable_brush().is_some());
        assert!(i1.as_mutable_brush().is_none());
        assert!(a.as_object().unwrap().is::<SolidColorBrush>());

        // Conversion to immutable.
        let immutable = BrushExtensions::to_immutable(&a);
        assert_eq!(Some(Colors::RED), immutable.as_solid_color_brush().map(|b| b.color()));
        let same = BrushExtensions::to_immutable(&i1);
        assert!(same.reference_id() == i1.reference_id());

        // A base-class handle keeps its interfaces.
        let base: crate::Ref<Brush> = mutable.upcast();
        let c: Rc<dyn IBrush> = base.into();
        assert!(c.as_solid_color_brush().is_some());
        assert!(*c == *a);
    }

    #[test]
    fn brush_handles_convert_to_the_brush_interface_as_untyped_values() {
        use crate::data::core::{ValueType, ValueTypes};
        use crate::{BoxedValue, Ref, TypeInfo};

        // Created through the type metadata, without naming the class.
        let object = SolidColorBrush::TYPE.create_instance().expect("default constructor");
        let brush = object.cast::<SolidColorBrush>().expect("a solid color brush");
        brush.set_color(Colors::RED);
        let boxed: BoxedValue = Rc::new(brush);

        let target = ValueType::of::<Option<Rc<dyn IBrush>>>();
        assert!(ValueTypes::is_assignable(ValueType::of::<Ref<SolidColorBrush>>(), target));
        let converted = ValueTypes::try_convert(Some(&boxed), target).flatten().expect("converted");
        let converted = converted.downcast_ref::<Option<Rc<dyn IBrush>>>().unwrap().clone().unwrap();
        assert_eq!(solid(&converted).color(), Colors::RED);

        // The abstract base class has no default constructor.
        assert!(Brush::TYPE.default_constructor().is_none());
        assert!(TypeInfo::find_by_name("SolidColorBrush").is_some());
    }
}
