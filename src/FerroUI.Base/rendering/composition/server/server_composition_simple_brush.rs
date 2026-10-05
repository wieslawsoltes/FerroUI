//! The server-side counterparts of the mutable brushes.

use super::{
    impl_simple_server_render_resource, IServerObject, IServerRenderResource, ServerCompositor, SimpleServerRenderResource,
};
use crate::media::immutable::ImmutableGradientStop;
use crate::media::{
    AlignmentX, AlignmentY, Color, GradientSpreadMethod, IBrush, IConicGradientBrush, IGradientBrush, IGradientStop,
    ILinearGradientBrush, IRadialGradientBrush, ISolidColorBrush, ITileBrush, ITransform, Stretch, TileMode,
};
use crate::rendering::composition::generated::{
    ServerCompositionSimpleBrushHooks, ServerCompositionSimpleBrushProps, ServerCompositionSimpleConicGradientBrushHooks,
    ServerCompositionSimpleConicGradientBrushProps, ServerCompositionSimpleLinearGradientBrushHooks,
    ServerCompositionSimpleLinearGradientBrushProps, ServerCompositionSimpleRadialGradientBrushHooks,
    ServerCompositionSimpleRadialGradientBrushProps, ServerCompositionSimpleSolidColorBrushHooks,
    ServerCompositionSimpleSolidColorBrushProps, ServerCompositionSimpleTileBrushHooks,
    ServerCompositionSimpleTileBrushProps,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::{RelativePoint, RelativeRect, RelativeScalar};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The gradient part of the server-side gradient brushes: the brush
/// properties, the stops and the spread method.
///
/// Embedded by the linear, radial and conic gradient brushes, whose
/// generated properties read their changes after it.
pub struct ServerCompositionSimpleGradientBrush {
    brush: ServerCompositionSimpleBrushProps,
    gradient_stops: RefCell<Vec<Rc<dyn IGradientStop>>>,
    spread_method: Cell<GradientSpreadMethod>,
}

impl Default for ServerCompositionSimpleGradientBrush {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerCompositionSimpleGradientBrush {
    pub fn new() -> Self {
        Self {
            brush: ServerCompositionSimpleBrushProps::new(),
            gradient_stops: RefCell::new(Vec::new()),
            spread_method: Cell::new(GradientSpreadMethod::default()),
        }
    }

    /// The brush properties.
    pub fn brush(&self) -> &ServerCompositionSimpleBrushProps {
        &self.brush
    }

    pub fn gradient_stops(&self) -> Vec<Rc<dyn IGradientStop>> {
        self.gradient_stops.borrow().clone()
    }

    pub fn spread_method(&self) -> GradientSpreadMethod {
        self.spread_method.get()
    }

    pub fn deserialize_changes_core(
        &self,
        host: &dyn ServerCompositionSimpleBrushHooks,
        reader: &mut BatchStreamReader<'_>,
        committed_at: Duration,
    ) {
        self.brush.deserialize_changes_core(host, reader, committed_at);
        self.spread_method.set(reader.read::<GradientSpreadMethod>());
        let mut gradient_stops = self.gradient_stops.borrow_mut();
        gradient_stops.clear();
        let count = reader.read::<i32>();
        for _ in 0..count {
            match reader.read_value::<ImmutableGradientStop>() {
                Some(stop) => gradient_stops.push(Rc::new(stop)),
                None => panic!("a batch lists a null gradient stop"),
            }
        }
    }
}

/// Declares a server-side brush class: the struct, its constructor, the
/// render resource contracts, `IServerObject` and `IBrush`.
///
/// `brush` is the path from `self` to the brush properties, `deserialize`
/// the member that reads the changes of the whole class, `find` the lookup
/// of generated property blocks, and the trailing items are the `as_*`
/// members of `IBrush` the class answers.
macro_rules! server_simple_brush {
    (
        $(#[$meta:meta])*
        $name:ident { $($field:ident : $field_ty:ty = $field_init:expr),* $(,)? }
        brush: |$b:ident| $brush:expr;
        deserialize: |$d:ident, $reader:ident, $committed_at:ident| $deserialize:expr;
        find: |$f:ident, $type_id:ident| $find:expr;
        as: { $($as_fn:ident -> $as_trait:path),* $(,)? }
    ) => {
        $(#[$meta])*
        pub struct $name {
            base: SimpleServerRenderResource,
            $($field: $field_ty,)*
        }

        impl $name {
            pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<$name> {
                Rc::new_cyclic(|this| {
                    let this: Weak<$name> = this.clone();
                    $name { base: SimpleServerRenderResource::new(compositor, this), $($field: $field_init,)* }
                })
            }

            pub fn is_disposed(&self) -> bool {
                self.base.is_disposed()
            }

            /// The brush properties of the class.
            pub fn brush_props(&self) -> &ServerCompositionSimpleBrushProps {
                let $b = self;
                $brush
            }
        }

        impl_simple_server_render_resource!($name, base);

        impl ServerCompositionSimpleBrushHooks for $name {}

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

server_simple_brush! {
    /// The server-side counterpart of a mutable brush: the properties all
    /// brushes share.
    ServerCompositionSimpleBrush { props: ServerCompositionSimpleBrushProps = ServerCompositionSimpleBrushProps::new() }
    brush: |b| &b.props;
    deserialize: |d, reader, committed_at| d.props.deserialize_changes_core(d, reader, committed_at);
    find: |f, type_id| f.props.find_props(type_id);
    as: {}
}

server_simple_brush! {
    /// The server-side counterpart of a mutable solid color brush.
    ServerCompositionSimpleSolidColorBrush {
        props: ServerCompositionSimpleSolidColorBrushProps = ServerCompositionSimpleSolidColorBrushProps::new(),
    }
    brush: |b| b.props.base();
    deserialize: |d, reader, committed_at| d.props.deserialize_changes_core(d, reader, committed_at);
    find: |f, type_id| f.props.find_props(type_id);
    as: { as_solid_color_brush -> ISolidColorBrush }
}

impl ServerCompositionSimpleSolidColorBrushHooks for ServerCompositionSimpleSolidColorBrush {}

impl ServerCompositionSimpleSolidColorBrush {
    /// The properties of the brush.
    pub fn props(&self) -> &ServerCompositionSimpleSolidColorBrushProps {
        &self.props
    }

    pub fn set_color(&self, value: Color) {
        self.props.set_color(self, value)
    }
}

impl ISolidColorBrush for ServerCompositionSimpleSolidColorBrush {
    fn color(&self) -> Color {
        self.props.color()
    }
}

server_simple_brush! {
    /// The server-side counterpart of a mutable tile brush.
    ServerCompositionSimpleTileBrush {
        props: ServerCompositionSimpleTileBrushProps = ServerCompositionSimpleTileBrushProps::new(),
    }
    brush: |b| b.props.base();
    deserialize: |d, reader, committed_at| d.props.deserialize_changes_core(d, reader, committed_at);
    find: |f, type_id| f.props.find_props(type_id);
    as: { as_tile_brush -> ITileBrush }
}

impl ServerCompositionSimpleTileBrushHooks for ServerCompositionSimpleTileBrush {}

impl ServerCompositionSimpleTileBrush {
    /// The properties of the brush.
    pub fn props(&self) -> &ServerCompositionSimpleTileBrushProps {
        &self.props
    }
}

impl ITileBrush for ServerCompositionSimpleTileBrush {
    fn alignment_x(&self) -> AlignmentX {
        self.props.alignment_x()
    }

    fn alignment_y(&self) -> AlignmentY {
        self.props.alignment_y()
    }

    fn destination_rect(&self) -> RelativeRect {
        self.props.destination_rect()
    }

    fn source_rect(&self) -> RelativeRect {
        self.props.source_rect()
    }

    fn stretch(&self) -> Stretch {
        self.props.stretch()
    }

    fn tile_mode(&self) -> TileMode {
        self.props.tile_mode()
    }
}

/// Declares a server-side gradient brush class on top of
/// [`server_simple_brush!`].
macro_rules! server_simple_gradient_brush {
    ($(#[$meta:meta])* $name:ident, $props:ident, $hooks:ident, $as_fn:ident -> $as_trait:path) => {
        server_simple_brush! {
            $(#[$meta])*
            $name {
                gradient: ServerCompositionSimpleGradientBrush = ServerCompositionSimpleGradientBrush::new(),
                props: $props = $props::new(),
            }
            brush: |b| b.gradient.brush();
            deserialize: |d, reader, committed_at| d.props.deserialize_changes_core(d, reader, committed_at);
            find: |f, type_id| f.props.find_props(type_id).or_else(|| f.gradient.brush().find_props(type_id));
            as: { as_gradient_brush -> IGradientBrush, $as_fn -> $as_trait }
        }

        impl $hooks for $name {
            fn base_deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
                self.gradient.deserialize_changes_core(self, reader, committed_at);
            }
        }

        impl $name {
            /// The properties of the brush.
            pub fn props(&self) -> &$props {
                &self.props
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

server_simple_gradient_brush!(
    /// The server-side counterpart of a mutable conic gradient brush.
    ServerCompositionSimpleConicGradientBrush,
    ServerCompositionSimpleConicGradientBrushProps,
    ServerCompositionSimpleConicGradientBrushHooks,
    as_conic_gradient_brush -> IConicGradientBrush
);

impl IConicGradientBrush for ServerCompositionSimpleConicGradientBrush {
    fn center(&self) -> RelativePoint {
        self.props.center()
    }

    fn angle(&self) -> f64 {
        self.props.angle()
    }
}

server_simple_gradient_brush!(
    /// The server-side counterpart of a mutable linear gradient brush.
    ServerCompositionSimpleLinearGradientBrush,
    ServerCompositionSimpleLinearGradientBrushProps,
    ServerCompositionSimpleLinearGradientBrushHooks,
    as_linear_gradient_brush -> ILinearGradientBrush
);

impl ILinearGradientBrush for ServerCompositionSimpleLinearGradientBrush {
    fn start_point(&self) -> RelativePoint {
        self.props.start_point()
    }

    fn end_point(&self) -> RelativePoint {
        self.props.end_point()
    }
}

server_simple_gradient_brush!(
    /// The server-side counterpart of a mutable radial gradient brush.
    ServerCompositionSimpleRadialGradientBrush,
    ServerCompositionSimpleRadialGradientBrushProps,
    ServerCompositionSimpleRadialGradientBrushHooks,
    as_radial_gradient_brush -> IRadialGradientBrush
);

impl IRadialGradientBrush for ServerCompositionSimpleRadialGradientBrush {
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
