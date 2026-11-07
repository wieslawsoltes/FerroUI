//! WPF's models: `Model3D`, `GeometryModel3D` and `Model3DGroup` (the lights are models too, in
//! `light.rs`).
//!
//! A model has a `Transform` (a change of it is a change of a record) and `Bounds`, the bounds
//! of what it draws after its transform.

use crate::change::{same, Link, ListLink, Notifier, Notifying, Scene3DChange};
use crate::transform_3d::{transform_rect, value_of};
use crate::{Geometry3D, Material, Model3DCollection, Rect3D, Transform3D};
use ferroui_base::animation::Animatable;
use ferroui_base::reactive::Subscription;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledProperty,
};
use std::rc::Rc;

/// WPF's `Model3D`: the base of the models and the lights.
#[repr(C)]
pub struct Model3D {
    base: Animatable,
    notifier: Rc<Notifier>,
    transform_link: Link,
}

ferro_class! {
    Model3D: Animatable, virtuals Model3DImpl: FerroObjectImpl {
        /// The bounds of the model before its own transform (WPF's `CalculateSubgraphBoundsInnerSpace`).
        fn bounds_core(this) -> Rect3D;
    }
}

impl FerroObjectImpl for Model3D {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::transform_property().as_property() {
            this.transform_link.set(this.transform().as_ref(), &this.notifier, same);
            this.notifier.raise(Scene3DChange::Record);
        }
    }
}

impl Model3DImpl for Model3D {
    fn bounds_core(_this: &Self) -> Rect3D {
        Rect3D::EMPTY
    }
}

impl Notifying for Model3D {
    fn notifier(&self) -> &Rc<Notifier> {
        &self.notifier
    }
}

ferro_properties! { impl Model3D {
    pub fn transform_property() -> StyledProperty<Option<Ref<Transform3D>>> {
        FerroProperty::register::<Model3D, _>("Transform", None)
    }
} }

impl Model3D {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Animatable::construct(), notifier: Notifier::new(), transform_link: Link::default() }
    }

    /// The transform of the model; none is the identity.
    pub fn transform(&self) -> Option<Ref<Transform3D>> {
        self.get_value(Self::transform_property())
    }

    pub fn set_transform(&self, value: Option<Ref<Transform3D>>) {
        self.set_value(Self::transform_property(), value)
    }

    /// The axis-aligned bounds of the model after its transform (WPF's `Bounds`):
    /// [`Rect3D::EMPTY`] for a light, the bounds of the geometry of a `GeometryModel3D`, the
    /// union of the children of a `Model3DGroup`.
    pub fn bounds(&self) -> Rect3D {
        transform_rect(self.bounds_core(), &value_of(self.transform().as_ref()))
    }

    /// Calls `handler` on every change of the model, until the returned guard is dropped.
    pub fn changed(&self, handler: impl Fn(Scene3DChange) + 'static) -> Subscription {
        self.notifier.subscribe(Rc::new(handler))
    }
}

// --- GeometryModel3D ---------------------------------------------------------------------------

/// WPF's `GeometryModel3D`: a `Geometry` drawn with `Material` on its front faces
/// (counterclockwise to the camera) and `BackMaterial` on its back faces.
#[repr(C)]
pub struct GeometryModel3D {
    base: Model3D,
    geometry_link: Link,
    material_link: Link,
    back_material_link: Link,
}

ferro_class!(GeometryModel3D: Model3D);
ferro_class_info!(GeometryModel3D {
    new: GeometryModel3D::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        constructors: [
            (Ref<Geometry3D>, Ref<Material>) => GeometryModel3D::with_geometry_and_material,
        ],
    },
});

impl FerroObjectImpl for GeometryModel3D {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::geometry_property().as_property() {
            this.geometry_link.set(this.geometry().as_ref(), this.notifier(), same);
        } else if property == Self::material_property().as_property() {
            this.material_link.set(this.material().as_ref(), this.notifier(), same);
        } else if property == Self::back_material_property().as_property() {
            this.back_material_link.set(this.back_material().as_ref(), this.notifier(), same);
        } else {
            return;
        }
        this.notifier().raise(Scene3DChange::Structure);
    }
}

impl Model3DImpl for GeometryModel3D {
    fn bounds_core(this: &Self) -> Rect3D {
        this.geometry().map_or(Rect3D::EMPTY, |geometry| geometry.bounds())
    }
}

ferro_properties! { impl GeometryModel3D {
    pub fn geometry_property() -> StyledProperty<Option<Ref<Geometry3D>>> {
        FerroProperty::register::<GeometryModel3D, _>("Geometry", None)
    }
    pub fn material_property() -> StyledProperty<Option<Ref<Material>>> {
        FerroProperty::register::<GeometryModel3D, _>("Material", None)
    }
    pub fn back_material_property() -> StyledProperty<Option<Ref<Material>>> {
        FerroProperty::register::<GeometryModel3D, _>("BackMaterial", None)
    }
} }

impl GeometryModel3D {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self {
            base: Model3D::construct(),
            geometry_link: Link::default(),
            material_link: Link::default(),
            back_material_link: Link::default(),
        }
    }

    /// A model without a geometry.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A model of `geometry` drawn with `material`.
    pub fn with_geometry_and_material(geometry: Ref<Geometry3D>, material: Ref<Material>) -> Ref<Self> {
        let result = Self::new();
        result.set_geometry(Some(geometry));
        result.set_material(Some(material));
        result
    }

    /// The geometry.
    pub fn geometry(&self) -> Option<Ref<Geometry3D>> {
        self.get_value(Self::geometry_property())
    }

    pub fn set_geometry(&self, value: Option<Ref<Geometry3D>>) {
        self.set_value(Self::geometry_property(), value)
    }

    /// The material of the front faces.
    pub fn material(&self) -> Option<Ref<Material>> {
        self.get_value(Self::material_property())
    }

    pub fn set_material(&self, value: Option<Ref<Material>>) {
        self.set_value(Self::material_property(), value)
    }

    /// The material of the back faces.
    pub fn back_material(&self) -> Option<Ref<Material>> {
        self.get_value(Self::back_material_property())
    }

    pub fn set_back_material(&self, value: Option<Ref<Material>>) {
        self.set_value(Self::back_material_property(), value)
    }
}

// --- Model3DGroup ------------------------------------------------------------------------------

/// WPF's `Model3DGroup`: its `Children` under its transform.
#[repr(C)]
pub struct Model3DGroup {
    base: Model3D,
    children_link: ListLink<Model3D>,
}

ferro_class!(Model3DGroup: Model3D);
ferro_class_info!(Model3DGroup {
    new: Model3DGroup::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        content: Children,
    },
});

impl FerroObjectImpl for Model3DGroup {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_children(Model3DCollection::new());
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::children_property().as_property() {
            let children = this.get_value(Self::children_property());
            this.children_link.set(children.as_deref(), this.notifier(), Scene3DChange::Structure);
            this.notifier().raise(Scene3DChange::Structure);
        }
    }
}

impl Model3DImpl for Model3DGroup {
    fn bounds_core(this: &Self) -> Rect3D {
        let mut bounds = Rect3D::EMPTY;
        if let Some(children) = this.get_value(Self::children_property()) {
            for child in children.iter() {
                bounds.union(child.bounds());
            }
        }
        bounds
    }
}

ferro_properties! { impl Model3DGroup {
    pub fn children_property() -> StyledProperty<Option<Model3DCollection>> {
        FerroProperty::register::<Model3DGroup, _>("Children", None)
    }
} }

impl Model3DGroup {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Model3D::construct(), children_link: ListLink::default() }
    }

    /// An empty group.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The models of the group. Panics if the property has been cleared.
    pub fn children(&self) -> Model3DCollection {
        self.get_value(Self::children_property()).expect("Children is not set")
    }

    pub fn set_children(&self, value: Model3DCollection) {
        self.set_value(Self::children_property(), Some(value))
    }
}
