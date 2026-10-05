//! The server-side counterparts of the composition brushes.

use crate::media::{
    Color, GradientSpreadMethod, IBrush, IConicGradientBrush, IGradientBrush, IGradientStop, ILinearGradientBrush,
    IRadialGradientBrush, ISolidColorBrush, ITransform,
};
use crate::rendering::composition::generated::{
    ServerCompositionBrushHooks, ServerCompositionBrushProps,
    ServerCompositionConicGradientBrushHooks, ServerCompositionConicGradientBrushProps,
    ServerCompositionLinearGradientBrushHooks, ServerCompositionLinearGradientBrushProps,
    ServerCompositionRadialGradientBrushHooks, ServerCompositionRadialGradientBrushProps,
    ServerCompositionSolidColorBrushHooks, ServerCompositionSolidColorBrushProps,
};
use crate::rendering::composition::server::{
    impl_server_render_resource, props_of, resolve_resource, CompositionProperty, CompositionPropertyOf,
    IAnimatedServerObject, IServerObject, IServerRenderResource, IServerRenderResourceHost, ServerCompositor,
    ServerRenderResource, ServerRenderResourceCore, ServerResourceRef,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::{RelativePoint, RelativeScalar};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::OnceLock;
use std::time::Duration;

/// The part of the server-side gradient brushes that upstream's
/// hand-written `ServerCompositionGradientBrush` adds to the generated
/// `ServerCompositionBrush`: the brush properties, the gradient stops and
/// the spread method.
pub struct ServerCompositionGradientBrush {
    brush: ServerCompositionBrushProps,
    gradient_stops: RefCell<Vec<ServerResourceRef<dyn IGradientStop>>>,
    spread_method: Cell<GradientSpreadMethod>,
}

impl Default for ServerCompositionGradientBrush {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerCompositionGradientBrush {
    pub fn new() -> Self {
        Self {
            brush: ServerCompositionBrushProps::new(),
            gradient_stops: RefCell::new(Vec::new()),
            spread_method: Cell::new(GradientSpreadMethod::default()),
        }
    }

    /// The properties of the generated base class.
    pub fn brush(&self) -> &ServerCompositionBrushProps {
        &self.brush
    }

    /// This part or the block of its base class, by type.
    pub fn find_props(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            return Some(self);
        }
        self.brush.find_props(type_id)
    }

    /// `s_IdOfGradientStopsProperty`.
    pub fn id_of_gradient_stops_property() -> &'static CompositionPropertyOf<Vec<Rc<dyn IGradientStop>>> {
        static PROPERTY: OnceLock<CompositionPropertyOf<Vec<Rc<dyn IGradientStop>>>> = OnceLock::new();
        PROPERTY.get_or_init(|| {
            CompositionProperty::register::<ServerCompositionGradientBrush, Vec<Rc<dyn IGradientStop>>>(
                "GradientStops",
                "ServerCompositionGradientBrush",
                |obj| props_of::<ServerCompositionGradientBrush>(obj).gradient_stops(),
                |obj, v| {
                    *props_of::<ServerCompositionGradientBrush>(obj).gradient_stops.borrow_mut() =
                        v.into_iter().map(ServerResourceRef::from_value).collect()
                },
                None,
                None,
            )
        })
    }

    pub fn gradient_stops(&self) -> Vec<Rc<dyn IGradientStop>> {
        self.gradient_stops.borrow().iter().map(|stop| stop.value.clone()).collect()
    }

    /// Sets the gradient stops: the render resources among them are
    /// observed by the brush (`SetValue` of the render resource).
    pub fn set_gradient_stops(
        &self,
        host: &dyn IServerRenderResourceHost,
        core: &ServerRenderResourceCore,
        value: Vec<ServerResourceRef<dyn IGradientStop>>,
    ) {
        // Lists compare by reference, and every list read from a batch is
        // a new one.
        let old_children = self.gradient_stops.borrow().iter().filter_map(|stop| stop.resource.clone()).collect();
        let new_children = value.iter().filter_map(|stop| stop.resource.clone()).collect();
        let mut value = Some(value);
        core.set_list_value(host, old_children, new_children, &mut || {
            if let Some(value) = value.take() {
                *self.gradient_stops.borrow_mut() = value;
            }
        });
    }

    pub fn spread_method(&self) -> GradientSpreadMethod {
        self.spread_method.get()
    }

    /// `DeserializeChangesCore`: the generated base class, then the spread
    /// method and the stops. `host` and `resource` are the brush.
    pub fn deserialize_changes_core(
        &self,
        host: &dyn ServerCompositionBrushHooks,
        resource: &dyn IServerRenderResourceHost,
        core: &ServerRenderResourceCore,
        reader: &mut BatchStreamReader<'_>,
        committed_at: Duration,
    ) {
        self.brush.deserialize_changes_core(host, reader, committed_at);
        self.spread_method.set(reader.read::<GradientSpreadMethod>());
        let compositor = host.server_compositor();
        let count = reader.read::<i32>();
        let mut stops = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count {
            let read = resolve_resource(reader.read_resource::<dyn IGradientStop>(), compositor.as_ref(), |o| {
                o.as_gradient_stop()
            });
            match read {
                Some(read) => stops.push(read),
                None => panic!("a batch lists a null gradient stop"),
            }
        }
        self.set_gradient_stops(resource, core, stops);
    }
}

/// Declares a server-side composition brush class: the struct, its
/// constructor, the render resource contracts, `IServerObject` and `IBrush`.
macro_rules! server_composition_brush {
    (
        $(#[$meta:meta])*
        $name:ident { $($field:ident : $field_ty:ty = $field_init:expr),* $(,)? }
        brush: |$b:ident| $brush:expr;
        deserialize: |$d:ident, $reader:ident, $committed_at:ident| $deserialize:expr;
        find: |$f:ident, $type_id:ident| $find:expr;
        composition_property: $get:expr;
        as: { $($as_fn:ident -> $as_trait:path),* $(,)? }
    ) => {
        $(#[$meta])*
        pub struct $name {
            base: ServerRenderResource,
            $($field: $field_ty,)*
        }

        impl $name {
            /// Creates the brush, active: the UI-thread brush activates its
            /// server object when it is created (`InitializeDefaultsExtra`),
            /// which is here done when the server object is created for it.
            pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<$name> {
                let brush = Rc::new_cyclic(|this: &Weak<$name>| $name {
                    base: ServerRenderResource::new(
                        compositor,
                        this.clone() as Weak<dyn IAnimatedServerObject>,
                        this.clone() as Weak<dyn IServerRenderResource>,
                    ),
                    $($field: $field_init,)*
                });
                brush.base.object().activate();
                brush
            }

            pub fn is_disposed(&self) -> bool {
                self.base.is_disposed()
            }

            /// The properties of the generated `ServerCompositionBrush`.
            pub fn brush_props(&self) -> &ServerCompositionBrushProps {
                let $b = self;
                $brush
            }
        }

        impl_server_render_resource!($name, base, composition_property: $get);

        impl ServerCompositionBrushHooks for $name {}

        impl IServerObject for $name {
            fn deserialize_changes_core(&self, $reader: &mut BatchStreamReader<'_>, $committed_at: Duration) {
                let $d = self;
                $deserialize
            }

            fn values_invalidated(&self) {
                self.base.core().invalidated(self);
            }

            fn dispose(&self) {
                self.base.core().dispose();
            }

            fn as_render_resource(self: Rc<Self>) -> Option<Rc<dyn IServerRenderResource>> {
                Some(self)
            }

            fn as_brush(self: Rc<Self>) -> Option<Rc<dyn IBrush>> {
                Some(self)
            }

            fn as_animated(self: Rc<Self>) -> Option<Rc<dyn IAnimatedServerObject>> {
                Some(self)
            }

            fn get_props(&self, $type_id: TypeId) -> Option<&dyn Any> {
                let $f = self;
                $find
            }

            fn as_any(&self) -> &dyn Any {
                self
            }

            fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
                self
            }
        }

        impl IBrush for $name {
            fn opacity(&self) -> f64 {
                self.brush_props().opacity()
            }

            fn transform(&self) -> Option<Rc<dyn ITransform>> {
                self.brush_props().transform().map(|transform| transform.value)
            }

            fn transform_origin(&self) -> RelativePoint {
                self.brush_props().transform_origin()
            }

            fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
                self.brush_props().relative_transform().map(|transform| transform.value)
            }

            fn as_any(&self) -> &dyn Any {
                self
            }

            $(
                fn $as_fn(&self) -> Option<&dyn $as_trait> {
                    Some(self)
                }
            )*
        }
    };
}

server_composition_brush! {
    /// The server-side counterpart of a
    /// [`CompositionSolidColorBrush`](super::CompositionSolidColorBrush).
    ServerCompositionSolidColorBrush {
        props: ServerCompositionSolidColorBrushProps = ServerCompositionSolidColorBrushProps::new(),
    }
    brush: |b| b.props.base();
    deserialize: |d, reader, committed_at| d.props.deserialize_changes_core(d, reader, committed_at);
    find: |f, type_id| f.props.find_props(type_id);
    composition_property: ServerCompositionSolidColorBrushProps::get_composition_property;
    as: { as_solid_color_brush -> ISolidColorBrush }
}

impl ServerCompositionSolidColorBrushHooks for ServerCompositionSolidColorBrush {}

impl ServerCompositionSolidColorBrush {
    /// The properties of the brush.
    pub fn props(&self) -> &ServerCompositionSolidColorBrushProps {
        &self.props
    }
}

impl ISolidColorBrush for ServerCompositionSolidColorBrush {
    fn color(&self) -> Color {
        self.props.color()
    }
}

/// Declares a server-side gradient brush class on top of
/// [`server_composition_brush!`].
macro_rules! server_composition_gradient_brush {
    ($(#[$meta:meta])* $name:ident, $props:ident, $hooks:ident, $as_fn:ident -> $as_trait:path) => {
        server_composition_brush! {
            $(#[$meta])*
            $name {
                gradient: ServerCompositionGradientBrush = ServerCompositionGradientBrush::new(),
                props: $props = $props::new(),
            }
            brush: |b| b.gradient.brush();
            deserialize: |d, reader, committed_at| d.props.deserialize_changes_core(d, reader, committed_at);
            find: |f, type_id| f.props.find_props(type_id).or_else(|| f.gradient.find_props(type_id));
            composition_property: |name: &str| {
                $props::get_composition_property(name).or_else(|| ServerCompositionBrushProps::get_composition_property(name))
            };
            as: { as_gradient_brush -> IGradientBrush, $as_fn -> $as_trait }
        }

        impl $hooks for $name {
            fn base_deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
                self.gradient.deserialize_changes_core(self, self, self.base.core(), reader, committed_at);
            }
        }

        impl $name {
            /// The properties of the brush.
            pub fn props(&self) -> &$props {
                &self.props
            }

            /// The gradient part of the brush.
            pub fn gradient(&self) -> &ServerCompositionGradientBrush {
                &self.gradient
            }

            pub fn gradient_stops(&self) -> Vec<Rc<dyn IGradientStop>> {
                self.gradient.gradient_stops()
            }

            pub fn spread_method(&self) -> GradientSpreadMethod {
                self.gradient.spread_method()
            }
        }

        impl IGradientBrush for $name {
            fn gradient_stops(&self) -> Vec<Rc<dyn IGradientStop>> {
                self.gradient.gradient_stops()
            }

            fn spread_method(&self) -> GradientSpreadMethod {
                self.gradient.spread_method()
            }
        }
    };
}

server_composition_gradient_brush!(
    /// The server-side counterpart of a
    /// [`CompositionConicGradientBrush`](super::CompositionConicGradientBrush).
    ServerCompositionConicGradientBrush,
    ServerCompositionConicGradientBrushProps,
    ServerCompositionConicGradientBrushHooks,
    as_conic_gradient_brush -> IConicGradientBrush
);

impl IConicGradientBrush for ServerCompositionConicGradientBrush {
    fn center(&self) -> RelativePoint {
        self.props.center()
    }

    fn angle(&self) -> f64 {
        self.props.angle()
    }
}

server_composition_gradient_brush!(
    /// The server-side counterpart of a
    /// [`CompositionLinearGradientBrush`](super::CompositionLinearGradientBrush).
    ServerCompositionLinearGradientBrush,
    ServerCompositionLinearGradientBrushProps,
    ServerCompositionLinearGradientBrushHooks,
    as_linear_gradient_brush -> ILinearGradientBrush
);

impl ILinearGradientBrush for ServerCompositionLinearGradientBrush {
    fn start_point(&self) -> RelativePoint {
        self.props.start_point()
    }

    fn end_point(&self) -> RelativePoint {
        self.props.end_point()
    }
}

server_composition_gradient_brush!(
    /// The server-side counterpart of a
    /// [`CompositionRadialGradientBrush`](super::CompositionRadialGradientBrush).
    ServerCompositionRadialGradientBrush,
    ServerCompositionRadialGradientBrushProps,
    ServerCompositionRadialGradientBrushHooks,
    as_radial_gradient_brush -> IRadialGradientBrush
);

impl ServerCompositionRadialGradientBrush {
    pub fn radius(&self) -> f64 {
        self.props.radius_x().scalar
    }
}

impl IRadialGradientBrush for ServerCompositionRadialGradientBrush {
    fn center(&self) -> RelativePoint {
        self.props.center()
    }

    fn gradient_origin(&self) -> RelativePoint {
        self.props.gradient_origin()
    }

    fn radius_x(&self) -> RelativeScalar {
        self.props.radius_x()
    }

    fn radius_y(&self) -> RelativeScalar {
        self.props.radius_y()
    }
}

