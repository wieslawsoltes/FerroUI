//! WPF's materials: `Material`, `DiffuseMaterial`, `SpecularMaterial`, `EmissiveMaterial` and
//! `MaterialGroup`, painting with the brushes of FerroUI.
//!
//! A change of a colour or of the specular power, or of the values of a brush, is a change of a
//! record ([`Scene3DChange::Record`]); a new brush (or none), or a change of the children of a
//! group, is a change of the structure ([`Scene3DChange::Structure`]): it changes the layers
//! the model is drawn with.
//!
//! A `VisualBrush` or a `DrawingBrush` is recorded into a [`Scene3DBrushContent`] (step D6 of
//! `docs/porting/media3d.md`, section 3.18) when a snapshot is made and its content is dirty: a
//! change of the brush (its properties) or of what it draws (the drawing of a `DrawingBrush`)
//! marks it dirty and is a change of a record; every other snapshot takes the kept recording,
//! with the same identity and generation, so that the renderers keep the texture they rendered of
//! it.

use crate::change::{ListLink, Notifier, Notifying, Scene3DChange};
use crate::MaterialCollection;
use ferroui_base::animation::Animatable;
use ferroui_base::media::{shared_brush_of, subscribe_invalidated, Color, IBrush, SharedBrush};
use ferroui_base::reactive::{IDisposable, Subscription};
use ferroui_base::rendering::scene3d::Scene3DBrushContent;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledProperty,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

/// The white of WPF's material and light defaults.
pub(crate) const WHITE: Color = Color::from_argb(255, 255, 255, 255);

/// WPF's `Material`: the base of the materials.
#[repr(C)]
pub struct Material {
    base: Animatable,
    notifier: Rc<Notifier>,
    brush_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    /// What the content of a visual or drawing brush follows (the drawing of a `DrawingBrush`).
    content_subscription: RefCell<Option<Subscription>>,
    /// Whether the recorded content of the brush is out of date.
    content_dirty: Rc<Cell<bool>>,
    /// The recorded content of a visual or drawing brush.
    content: RefCell<Option<KeptContent>>,
}

/// The recording of a visual or drawing brush a material keeps.
struct KeptContent {
    /// The address of the brush it was recorded of.
    brush: usize,
    id: u64,
    generation: u64,
    shared: Option<SharedBrush>,
}

/// The identities of the contents of visual and drawing brushes.
static NEXT_CONTENT: AtomicU64 = AtomicU64::new(1);

ferro_class!(Material: Animatable);
ferro_impl_classes!(Material: FerroObjectImpl);

impl Notifying for Material {
    fn notifier(&self) -> &Rc<Notifier> {
        &self.notifier
    }
}

impl Material {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Animatable::construct(),
            notifier: Notifier::new(),
            brush_subscription: RefCell::new(None),
            content_subscription: RefCell::new(None),
            content_dirty: Rc::new(Cell::new(true)),
            content: RefCell::new(None),
        }
    }

    /// The form of `brush` (this material's brush) in a snapshot: its shared values, or for a
    /// visual or drawing brush its recorded content, recorded again only when it is dirty (a new
    /// generation of the same identity) or the brush is another one (a new identity). `None` for
    /// a brush with neither, or a scene brush that draws nothing.
    pub(crate) fn layer_brush(&self, brush: &Rc<dyn IBrush>) -> Option<SharedBrush> {
        if let Some(shared) = shared_brush_of(&**brush) {
            return Some(shared);
        }
        let scene = brush.as_scene_brush()?;
        let identity = Rc::as_ptr(brush) as *const () as usize;
        let mut kept = self.content.borrow_mut();
        let same_brush = kept.as_ref().is_some_and(|kept| kept.brush == identity);
        if !same_brush || self.content_dirty.get() {
            let (id, generation) = match kept.as_ref() {
                Some(kept) if same_brush => (kept.id, kept.generation + 1),
                _ => (NEXT_CONTENT.fetch_add(1, Ordering::Relaxed), 1),
            };
            let shared = Scene3DBrushContent::record(scene, id, generation)
                .and_then(|content| SharedBrush::from_scene_content(&**brush, content));
            *kept = Some(KeptContent { brush: identity, id, generation, shared });
            self.content_dirty.set(false);
        }
        kept.as_ref().and_then(|kept| kept.shared.clone())
    }

    /// Calls `handler` on every change of the material, until the returned guard is dropped.
    pub fn changed(&self, handler: impl Fn(Scene3DChange) + 'static) -> Subscription {
        self.notifier.subscribe(Rc::new(handler))
    }

    /// Follows the brush that is the new value of the `Brush` property of `change` (a change of
    /// its values is a change of a record) and reports the new brush as a change of the
    /// structure.
    fn brush_changed(&self, change: &FerroPropertyChangedEventArgs<'_>) {
        if let Some(previous) = self.brush_subscription.borrow_mut().take() {
            previous.dispose();
        }
        if let Some(previous) = self.content_subscription.borrow_mut().take() {
            previous.dispose();
        }
        let notifier = Rc::downgrade(&self.notifier);
        let dirty = self.content_dirty.clone();
        let changed: Rc<dyn Fn()> = Rc::new(move || {
            // A change of the brush or of what it draws: a visual or drawing brush is recorded
            // again for the next snapshot.
            dirty.set(true);
            if let Some(notifier) = notifier.upgrade() {
                notifier.raise(Scene3DChange::Record);
            }
        });
        let subscription = subscribe_invalidated(change.new_value(), changed.clone());
        *self.brush_subscription.borrow_mut() = subscription;
        let brush = change.get_new_value::<Option<Rc<dyn IBrush>>>();
        *self.content_subscription.borrow_mut() =
            brush.as_ref().and_then(|brush| brush.as_scene_brush()?.subscribe_content_changed(changed));
        self.content_dirty.set(true);
        self.notifier.raise(Scene3DChange::Structure);
    }
}

/// Reports a change of a property of a material: `Brush` is a change of the structure, the
/// other properties (colours, power) changes of a record.
fn material_property_changed(material: &Material, brush: &'static FerroProperty, change: &FerroPropertyChangedEventArgs<'_>) {
    if change.property() == brush {
        material.brush_changed(change);
    } else {
        material.notifier.raise(Scene3DChange::Record);
    }
}

// --- DiffuseMaterial ---------------------------------------------------------------------------

/// WPF's `DiffuseMaterial`: its `Brush`, filtered by `Color`, lit by the lights (the ambient
/// lights through `AmbientColor`).
#[repr(C)]
pub struct DiffuseMaterial {
    base: Material,
}

ferro_class!(DiffuseMaterial: Material);
ferro_class_info!(DiffuseMaterial {
    new: DiffuseMaterial::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        constructors: [
            (Rc<dyn IBrush>) => |brush: Rc<dyn IBrush>| DiffuseMaterial::with_brush(Some(brush)),
        ],
    },
});

impl FerroObjectImpl for DiffuseMaterial {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        material_property_changed(this, Self::brush_property().as_property(), change);
    }
}

ferro_properties! { impl DiffuseMaterial {
    pub fn color_property() -> StyledProperty<Color> {
        FerroProperty::register::<DiffuseMaterial, _>("Color", WHITE)
    }
    pub fn ambient_color_property() -> StyledProperty<Color> {
        FerroProperty::register::<DiffuseMaterial, _>("AmbientColor", WHITE)
    }
    pub fn brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<DiffuseMaterial, _>("Brush", None)
    }
} }

impl DiffuseMaterial {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Material::construct() }
    }

    /// A material without a brush (it paints nothing until it has one).
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A material of `brush`.
    pub fn with_brush(brush: Option<Rc<dyn IBrush>>) -> Ref<Self> {
        let result = Self::new();
        result.set_brush(brush);
        result
    }

    /// The colour that filters the brush (white by default).
    pub fn color(&self) -> Color {
        self.get_value(Self::color_property())
    }

    pub fn set_color(&self, value: Color) {
        self.set_value(Self::color_property(), value)
    }

    /// The colour that filters the ambient light (white by default).
    pub fn ambient_color(&self) -> Color {
        self.get_value(Self::ambient_color_property())
    }

    pub fn set_ambient_color(&self, value: Color) {
        self.set_value(Self::ambient_color_property(), value)
    }

    /// The brush.
    pub fn brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::brush_property())
    }

    pub fn set_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::brush_property(), value)
    }
}

// --- SpecularMaterial --------------------------------------------------------------------------

/// WPF's `SpecularMaterial`: highlights of its `Brush`, filtered by `Color`, as sharp as
/// `SpecularPower` (40 by default), added to what is below.
#[repr(C)]
pub struct SpecularMaterial {
    base: Material,
}

ferro_class!(SpecularMaterial: Material);
ferro_class_info!(SpecularMaterial {
    new: SpecularMaterial::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        constructors: [
            (Rc<dyn IBrush>, f64) => |brush: Rc<dyn IBrush>, power: f64| SpecularMaterial::with_brush(Some(brush), power),
        ],
    },
});

impl FerroObjectImpl for SpecularMaterial {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        material_property_changed(this, Self::brush_property().as_property(), change);
    }
}

ferro_properties! { impl SpecularMaterial {
    pub fn color_property() -> StyledProperty<Color> {
        FerroProperty::register::<SpecularMaterial, _>("Color", WHITE)
    }
    pub fn specular_power_property() -> StyledProperty<f64> {
        FerroProperty::register::<SpecularMaterial, _>("SpecularPower", 40.0)
    }
    pub fn brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<SpecularMaterial, _>("Brush", None)
    }
} }

impl SpecularMaterial {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Material::construct() }
    }

    /// A material without a brush.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A material of `brush` with `specular_power`.
    pub fn with_brush(brush: Option<Rc<dyn IBrush>>, specular_power: f64) -> Ref<Self> {
        let result = Self::new();
        result.set_brush(brush);
        result.set_specular_power(specular_power);
        result
    }

    /// The colour that filters the brush (white by default).
    pub fn color(&self) -> Color {
        self.get_value(Self::color_property())
    }

    pub fn set_color(&self, value: Color) {
        self.set_value(Self::color_property(), value)
    }

    /// The sharpness of the highlights.
    pub fn specular_power(&self) -> f64 {
        self.get_value(Self::specular_power_property())
    }

    pub fn set_specular_power(&self, value: f64) {
        self.set_value(Self::specular_power_property(), value)
    }

    /// The brush.
    pub fn brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::brush_property())
    }

    pub fn set_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::brush_property(), value)
    }
}

// --- EmissiveMaterial --------------------------------------------------------------------------

/// WPF's `EmissiveMaterial`: its `Brush`, filtered by `Color`, added without light.
#[repr(C)]
pub struct EmissiveMaterial {
    base: Material,
}

ferro_class!(EmissiveMaterial: Material);
ferro_class_info!(EmissiveMaterial {
    new: EmissiveMaterial::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        constructors: [
            (Rc<dyn IBrush>) => |brush: Rc<dyn IBrush>| EmissiveMaterial::with_brush(Some(brush)),
        ],
    },
});

impl FerroObjectImpl for EmissiveMaterial {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        material_property_changed(this, Self::brush_property().as_property(), change);
    }
}

ferro_properties! { impl EmissiveMaterial {
    pub fn color_property() -> StyledProperty<Color> {
        FerroProperty::register::<EmissiveMaterial, _>("Color", WHITE)
    }
    pub fn brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<EmissiveMaterial, _>("Brush", None)
    }
} }

impl EmissiveMaterial {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Material::construct() }
    }

    /// A material without a brush.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A material of `brush`.
    pub fn with_brush(brush: Option<Rc<dyn IBrush>>) -> Ref<Self> {
        let result = Self::new();
        result.set_brush(brush);
        result
    }

    /// The colour that filters the brush (white by default).
    pub fn color(&self) -> Color {
        self.get_value(Self::color_property())
    }

    pub fn set_color(&self, value: Color) {
        self.set_value(Self::color_property(), value)
    }

    /// The brush.
    pub fn brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::brush_property())
    }

    pub fn set_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::brush_property(), value)
    }
}

// --- MaterialGroup -----------------------------------------------------------------------------

/// WPF's `MaterialGroup`: its `Children` drawn in order, each over the ones before.
#[repr(C)]
pub struct MaterialGroup {
    base: Material,
    children_link: ListLink<Material>,
}

ferro_class!(MaterialGroup: Material);
ferro_class_info!(MaterialGroup {
    new: MaterialGroup::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        content: Children,
    },
});

impl FerroObjectImpl for MaterialGroup {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_children(MaterialCollection::new());
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

ferro_properties! { impl MaterialGroup {
    pub fn children_property() -> StyledProperty<Option<MaterialCollection>> {
        FerroProperty::register::<MaterialGroup, _>("Children", None)
    }
} }

impl MaterialGroup {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Material::construct(), children_link: ListLink::default() }
    }

    /// An empty group.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The materials of the group. Panics if the property has been cleared.
    pub fn children(&self) -> MaterialCollection {
        self.get_value(Self::children_property()).expect("Children is not set")
    }

    pub fn set_children(&self, value: MaterialCollection) {
        self.set_value(Self::children_property(), Some(value))
    }
}
