//! WPF's lights: `Light`, `AmbientLight`, `DirectionalLight`, `PointLightBase`, `PointLight` and
//! `SpotLight`, with WPF's defaults.
//!
//! The lights are models: they are placed by the transforms of the tree, and every light of a
//! viewport lights every model of it. Any change of a light is a change of a record
//! ([`Scene3DChange::Record`]).

use crate::change::{Notifying, Scene3DChange};
use crate::material::WHITE;
use crate::{Model3D, Model3DImpl, Point3D, Rect3D, Vector3D};
use ferroui_base::media::Color;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledProperty,
};

/// WPF's `Light`: the base of the lights, with a `Color` (white by default).
#[repr(C)]
pub struct Light {
    base: Model3D,
}

ferro_class!(Light: Model3D);

impl FerroObjectImpl for Light {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        // The transform is reported by the model; every other property of a light is a record.
        if change.property() != Model3D::transform_property().as_property() {
            this.notifier().raise(Scene3DChange::Record);
        }
    }
}

impl Model3DImpl for Light {
    /// A light draws nothing: its bounds are empty.
    fn bounds_core(_this: &Self) -> Rect3D {
        Rect3D::EMPTY
    }
}

ferro_properties! { impl Light {
    pub fn color_property() -> StyledProperty<Color> {
        FerroProperty::register::<Light, _>("Color", WHITE)
    }
} }

impl Light {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Model3D::construct() }
    }

    /// The colour of the light.
    pub fn color(&self) -> Color {
        self.get_value(Self::color_property())
    }

    pub fn set_color(&self, value: Color) {
        self.set_value(Self::color_property(), value)
    }
}

// --- AmbientLight ------------------------------------------------------------------------------

/// WPF's `AmbientLight`: light of the same colour everywhere, from no direction.
#[repr(C)]
pub struct AmbientLight {
    base: Light,
}

ferro_class!(AmbientLight: Light);
ferro_class_info!(AmbientLight {
    new: AmbientLight::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        constructors: [
            (Color) => AmbientLight::with_color,
        ],
    },
});
ferro_impl_classes!(AmbientLight: FerroObjectImpl, Model3DImpl);

impl AmbientLight {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Light::construct() }
    }

    /// A white ambient light.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// An ambient light of `color`.
    pub fn with_color(color: Color) -> Ref<Self> {
        let result = Self::new();
        result.set_color(color);
        result
    }
}

// --- DirectionalLight --------------------------------------------------------------------------

/// WPF's `DirectionalLight`: parallel light travelling along `Direction` ((0, 0, -1) by default).
#[repr(C)]
pub struct DirectionalLight {
    base: Light,
}

ferro_class!(DirectionalLight: Light);
ferro_class_info!(DirectionalLight {
    new: DirectionalLight::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        constructors: [
            (Color, Vector3D) => DirectionalLight::with_color_and_direction,
        ],
    },
});
ferro_impl_classes!(DirectionalLight: FerroObjectImpl, Model3DImpl);

ferro_properties! { impl DirectionalLight {
    pub fn direction_property() -> StyledProperty<Vector3D> {
        FerroProperty::register::<DirectionalLight, _>("Direction", Vector3D::new(0.0, 0.0, -1.0))
    }
} }

impl DirectionalLight {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Light::construct() }
    }

    /// A white light along the negative z axis.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A light of `color` along `direction`.
    pub fn with_color_and_direction(color: Color, direction: Vector3D) -> Ref<Self> {
        let result = Self::new();
        result.set_color(color);
        result.set_direction(direction);
        result
    }

    /// The direction the light travels in.
    pub fn direction(&self) -> Vector3D {
        self.get_value(Self::direction_property())
    }

    pub fn set_direction(&self, value: Vector3D) {
        self.set_value(Self::direction_property(), value)
    }
}

// --- PointLightBase ----------------------------------------------------------------------------

/// WPF's `PointLightBase`: light from `Position`, as far as `Range` (infinite by default),
/// attenuated by `1 / (ConstantAttenuation + LinearAttenuation d + QuadraticAttenuation d^2)`
/// (1, 0 and 0 by default).
#[repr(C)]
pub struct PointLightBase {
    base: Light,
}

ferro_class!(PointLightBase: Light);
ferro_impl_classes!(PointLightBase: FerroObjectImpl, Model3DImpl);

ferro_properties! { impl PointLightBase {
    pub fn position_property() -> StyledProperty<Point3D> {
        FerroProperty::register::<PointLightBase, _>("Position", Point3D::new(0.0, 0.0, 0.0))
    }
    pub fn range_property() -> StyledProperty<f64> {
        FerroProperty::register::<PointLightBase, _>("Range", f64::INFINITY)
    }
    pub fn constant_attenuation_property() -> StyledProperty<f64> {
        FerroProperty::register::<PointLightBase, _>("ConstantAttenuation", 1.0)
    }
    pub fn linear_attenuation_property() -> StyledProperty<f64> {
        FerroProperty::register::<PointLightBase, _>("LinearAttenuation", 0.0)
    }
    pub fn quadratic_attenuation_property() -> StyledProperty<f64> {
        FerroProperty::register::<PointLightBase, _>("QuadraticAttenuation", 0.0)
    }
} }

impl PointLightBase {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Light::construct() }
    }

    /// The position of the light.
    pub fn position(&self) -> Point3D {
        self.get_value(Self::position_property())
    }

    pub fn set_position(&self, value: Point3D) {
        self.set_value(Self::position_property(), value)
    }

    /// The distance beyond which the light has no effect.
    pub fn range(&self) -> f64 {
        self.get_value(Self::range_property())
    }

    pub fn set_range(&self, value: f64) {
        self.set_value(Self::range_property(), value)
    }

    /// The constant attenuation.
    pub fn constant_attenuation(&self) -> f64 {
        self.get_value(Self::constant_attenuation_property())
    }

    pub fn set_constant_attenuation(&self, value: f64) {
        self.set_value(Self::constant_attenuation_property(), value)
    }

    /// The attenuation proportional to the distance.
    pub fn linear_attenuation(&self) -> f64 {
        self.get_value(Self::linear_attenuation_property())
    }

    pub fn set_linear_attenuation(&self, value: f64) {
        self.set_value(Self::linear_attenuation_property(), value)
    }

    /// The attenuation proportional to the square of the distance.
    pub fn quadratic_attenuation(&self) -> f64 {
        self.get_value(Self::quadratic_attenuation_property())
    }

    pub fn set_quadratic_attenuation(&self, value: f64) {
        self.set_value(Self::quadratic_attenuation_property(), value)
    }
}

// --- PointLight --------------------------------------------------------------------------------

/// WPF's `PointLight`: light in every direction from `Position`.
#[repr(C)]
pub struct PointLight {
    base: PointLightBase,
}

ferro_class!(PointLight: PointLightBase);
ferro_class_info!(PointLight {
    new: PointLight::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        constructors: [
            (Color, Point3D) => PointLight::with_color_and_position,
        ],
    },
});
ferro_impl_classes!(PointLight: FerroObjectImpl, Model3DImpl);

impl PointLight {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: PointLightBase::construct() }
    }

    /// A white light at the origin.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A light of `color` at `position`.
    pub fn with_color_and_position(color: Color, position: Point3D) -> Ref<Self> {
        let result = Self::new();
        result.set_color(color);
        result.set_position(position);
        result
    }
}

// --- SpotLight ---------------------------------------------------------------------------------

/// WPF's `SpotLight`: light from `Position` in a cone along `Direction`, full inside
/// `InnerConeAngle` (180 degrees by default) and none beyond `OuterConeAngle` (90 degrees by
/// default), both full angles in degrees.
#[repr(C)]
pub struct SpotLight {
    base: PointLightBase,
}

ferro_class!(SpotLight: PointLightBase);
ferro_class_info!(SpotLight {
    new: SpotLight::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        constructors: [
            (Color, Point3D, Vector3D, f64, f64) => SpotLight::with_values,
        ],
    },
});
ferro_impl_classes!(SpotLight: FerroObjectImpl, Model3DImpl);

ferro_properties! { impl SpotLight {
    pub fn direction_property() -> StyledProperty<Vector3D> {
        FerroProperty::register::<SpotLight, _>("Direction", Vector3D::new(0.0, 0.0, -1.0))
    }
    pub fn inner_cone_angle_property() -> StyledProperty<f64> {
        FerroProperty::register::<SpotLight, _>("InnerConeAngle", 180.0)
    }
    pub fn outer_cone_angle_property() -> StyledProperty<f64> {
        FerroProperty::register::<SpotLight, _>("OuterConeAngle", 90.0)
    }
} }

impl SpotLight {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: PointLightBase::construct() }
    }

    /// A white light at the origin along the negative z axis.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A light of `color` at `position` along `direction` with the outer and inner cone angles,
    /// in the order of WPF's constructor.
    pub fn with_values(
        color: Color,
        position: Point3D,
        direction: Vector3D,
        outer_cone_angle: f64,
        inner_cone_angle: f64,
    ) -> Ref<Self> {
        let result = Self::new();
        result.set_color(color);
        result.set_position(position);
        result.set_direction(direction);
        result.set_outer_cone_angle(outer_cone_angle);
        result.set_inner_cone_angle(inner_cone_angle);
        result
    }

    /// The direction of the cone.
    pub fn direction(&self) -> Vector3D {
        self.get_value(Self::direction_property())
    }

    pub fn set_direction(&self, value: Vector3D) {
        self.set_value(Self::direction_property(), value)
    }

    /// The full angle of the cone of full light, in degrees.
    pub fn inner_cone_angle(&self) -> f64 {
        self.get_value(Self::inner_cone_angle_property())
    }

    pub fn set_inner_cone_angle(&self, value: f64) {
        self.set_value(Self::inner_cone_angle_property(), value)
    }

    /// The full angle of the cone beyond which there is no light, in degrees.
    pub fn outer_cone_angle(&self) -> f64 {
        self.get_value(Self::outer_cone_angle_property())
    }

    pub fn set_outer_cone_angle(&self, value: f64) {
        self.set_value(Self::outer_cone_angle_property(), value)
    }
}
