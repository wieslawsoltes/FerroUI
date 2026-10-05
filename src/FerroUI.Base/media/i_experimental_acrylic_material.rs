use crate::media::ref_adapter::RefAdapter;
use crate::media::{
    AcrylicBackgroundSource, Color, ExperimentalAcrylicMaterial, IMutableExperimentalAcrylicMaterial,
};
use crate::{FerroObject, ObjectType, Ref, Upcast};
use std::any::Any;
use std::rc::Rc;

/// Experimental interface for producing acrylic-like materials.
///
/// Implemented by [`ImmutableExperimentalAcrylicMaterial`](crate::media::ImmutableExperimentalAcrylicMaterial)
/// and, through an adapter, by handles of [`ExperimentalAcrylicMaterial`].
pub trait IExperimentalAcrylicMaterial: 'static {
    /// The background source mode.
    fn background_source(&self) -> AcrylicBackgroundSource;

    /// The tint color of the material.
    fn tint_color(&self) -> Color;

    /// The tint opacity of the material.
    fn tint_opacity(&self) -> f64;

    /// The effective material color.
    fn material_color(&self) -> Color;

    /// The fallback color if acrylic is unavailable.
    fn fallback_color(&self) -> Color;

    /// The implementing value, for downcasts to the immutable material.
    fn as_any(&self) -> &dyn Any;

    /// The object behind the material when it is mutable.
    fn as_object(&self) -> Option<&FerroObject> {
        None
    }

    /// The material viewed as [`IMutableExperimentalAcrylicMaterial`], when
    /// it is one.
    fn as_mutable_material(&self) -> Option<&dyn IMutableExperimentalAcrylicMaterial> {
        None
    }

    /// The identity of the material, used for reference equality.
    #[doc(hidden)]
    fn reference_id(&self) -> *const () {
        self as *const Self as *const ()
    }
}

/// Materials compare by reference.
impl PartialEq for dyn IExperimentalAcrylicMaterial {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}

impl std::fmt::Debug for dyn IExperimentalAcrylicMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IExperimentalAcrylicMaterial({})", self.tint_color())
    }
}

impl<T: ObjectType + Upcast<ExperimentalAcrylicMaterial>> RefAdapter<T> {
    #[inline]
    fn material(&self) -> &ExperimentalAcrylicMaterial {
        (*self.0).upcast()
    }
}

impl<T: ObjectType + Upcast<ExperimentalAcrylicMaterial>> IExperimentalAcrylicMaterial for RefAdapter<T> {
    #[inline]
    fn background_source(&self) -> AcrylicBackgroundSource {
        self.material().background_source()
    }

    #[inline]
    fn tint_color(&self) -> Color {
        self.material().effective_tint_color()
    }

    #[inline]
    fn tint_opacity(&self) -> f64 {
        self.material().tint_opacity()
    }

    #[inline]
    fn material_color(&self) -> Color {
        self.material().effective_luminosity_color()
    }

    #[inline]
    fn fallback_color(&self) -> Color {
        self.material().fallback_color()
    }

    fn as_any(&self) -> &dyn Any {
        self.material()
    }

    fn as_object(&self) -> Option<&FerroObject> {
        Some(self.object())
    }

    fn as_mutable_material(&self) -> Option<&dyn IMutableExperimentalAcrylicMaterial> {
        Some(self)
    }

    fn reference_id(&self) -> *const () {
        RefAdapter::reference_id(self)
    }
}

impl<T: ObjectType + Upcast<ExperimentalAcrylicMaterial>> IMutableExperimentalAcrylicMaterial for RefAdapter<T> {
    fn to_immutable(&self) -> Rc<dyn IExperimentalAcrylicMaterial> {
        Rc::new(crate::media::ImmutableExperimentalAcrylicMaterial::new(self))
    }
}

impl<T: ObjectType + Upcast<ExperimentalAcrylicMaterial>> From<Ref<T>> for Rc<dyn IExperimentalAcrylicMaterial> {
    #[inline]
    fn from(value: Ref<T>) -> Self {
        Rc::new(RefAdapter(value))
    }
}

impl<T: ObjectType + Upcast<ExperimentalAcrylicMaterial>> From<&Ref<T>> for Rc<dyn IExperimentalAcrylicMaterial> {
    #[inline]
    fn from(value: &Ref<T>) -> Self {
        Rc::new(RefAdapter(value.clone()))
    }
}
