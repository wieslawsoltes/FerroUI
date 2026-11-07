//! WPF's `Viewport3DVisual`: the root of a 3D scene, with its `Camera`, its `Children` and its
//! `Viewport`, which keeps the generations of the scene and its snapshot.
//!
//! In WPF the class is a 2D `Visual`; here it is the object that owns the scene of a
//! `Viewport3D` (which forwards its `Camera` and `Children` to it) and compiles it into an
//! immutable [`Scene3DSnapshot`]. A change anywhere in the scene bumps the generation of its kind
//! ([`Scene3DChange`]) and drops the snapshot; [`snapshot`](Viewport3DVisual::snapshot) makes the
//! next one when it is asked for. An unchanged scene returns the same snapshot.

use crate::change::{camera, Link, ListLink, Notifier, Scene3DChange};
use crate::scene_compiler::{compile, ModelSource, SceneInput};
use crate::{Camera, GeometryModel3D, Rect3D, Visual3D, Visual3DCollection};
use ferroui_base::reactive::Subscription;
use ferroui_base::rendering::scene3d::Scene3DSnapshot;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObject, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref, StyledProperty,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// The generations of a scene: structure, records, camera.
#[derive(Clone, Copy)]
struct Generations {
    structure: u64,
    record: u64,
    camera: u64,
}

/// WPF's `Viewport3DVisual`; see the documentation of the module.
#[repr(C)]
pub struct Viewport3DVisual {
    base: FerroObject,
    notifier: Rc<Notifier>,
    camera_link: Link,
    children: Visual3DCollection,
    children_link: ListLink<Visual3D>,
    generations: Cell<Generations>,
    snapshot: RefCell<Option<Arc<Scene3DSnapshot>>>,
    sources: RefCell<Vec<ModelSource>>,
    generation_subscription: RefCell<Option<Subscription>>,
}

ferro_class!(Viewport3DVisual: FerroObject);
ferro_class_info!(Viewport3DVisual {
    new: Viewport3DVisual::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        content: Children,
        properties: [
            Children: Visual3DCollection { get: Viewport3DVisual::children },
        ],
    },
});

impl FerroObjectImpl for Viewport3DVisual {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        // The first handler: the generations are bumped and the snapshot dropped before anyone
        // else hears of a change.
        let weak = this.to_ref().downgrade();
        let subscription = this.notifier.subscribe(Rc::new(move |change: Scene3DChange| {
            if let Some(this) = weak.upgrade() {
                this.bump(change);
            }
        }));
        *this.generation_subscription.borrow_mut() = Some(subscription);
        this.children_link.set(Some(&*this.children), &this.notifier, Scene3DChange::Structure);
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::camera_property().as_property() {
            this.camera_link.set(this.camera().as_ref(), &this.notifier, camera);
            this.notifier.raise(Scene3DChange::Camera);
        } else if property == Self::viewport_property().as_property() {
            this.notifier.raise(Scene3DChange::Camera);
        }
    }
}

ferro_properties! { impl Viewport3DVisual {
    pub fn camera_property() -> StyledProperty<Option<Ref<Camera>>> {
        FerroProperty::register::<Viewport3DVisual, _>("Camera", None)
    }
    pub fn viewport_property() -> StyledProperty<Rect> {
        FerroProperty::register::<Viewport3DVisual, _>("Viewport", Rect::default())
    }
} }

impl Viewport3DVisual {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            notifier: Notifier::new(),
            camera_link: Link::default(),
            children: Visual3DCollection::new(),
            children_link: ListLink::default(),
            generations: Cell::new(Generations { structure: 1, record: 1, camera: 1 }),
            snapshot: RefCell::new(None),
            sources: RefCell::new(Vec::new()),
            generation_subscription: RefCell::new(None),
        }
    }

    /// An empty scene without a camera.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The camera; without one nothing is drawn.
    pub fn camera(&self) -> Option<Ref<Camera>> {
        self.get_value(Self::camera_property())
    }

    pub fn set_camera(&self, value: Option<Ref<Camera>>) {
        self.set_value(Self::camera_property(), value)
    }

    /// The area the scene is drawn in (the arranged bounds of its `Viewport3D`); the camera
    /// takes its aspect ratio.
    pub fn viewport(&self) -> Rect {
        self.get_value(Self::viewport_property())
    }

    pub fn set_viewport(&self, value: Rect) {
        self.set_value(Self::viewport_property(), value)
    }

    /// The visuals of the scene (a shared handle to the collection).
    pub fn children(&self) -> Visual3DCollection {
        self.children.clone()
    }

    /// The generation of the structure of the scene.
    pub fn structure_generation(&self) -> u64 {
        self.generations.get().structure
    }

    /// The generation of the records of the scene.
    pub fn record_generation(&self) -> u64 {
        self.generations.get().record
    }

    /// The generation of the camera and the viewport.
    pub fn camera_generation(&self) -> u64 {
        self.generations.get().camera
    }

    /// Calls `handler` on every change of the scene (after its generation was bumped), until
    /// the returned guard is dropped.
    pub fn changed(&self, handler: impl Fn(Scene3DChange) + 'static) -> Subscription {
        self.notifier.subscribe(Rc::new(handler))
    }

    /// The scene as an immutable snapshot: the same one while nothing changes.
    pub fn snapshot(&self) -> Arc<Scene3DSnapshot> {
        if let Some(snapshot) = self.snapshot.borrow().as_ref() {
            return snapshot.clone();
        }
        let generations = self.generations.get();
        let (snapshot, sources) = compile(SceneInput {
            camera: self.camera(),
            children: &self.children,
            viewport: self.viewport(),
            generations: (generations.structure, generations.record, generations.camera),
        });
        let snapshot = Arc::new(snapshot);
        *self.snapshot.borrow_mut() = Some(snapshot.clone());
        *self.sources.borrow_mut() = sources;
        snapshot
    }

    /// The visual and the model the model at `index` of the current snapshot was made from (for
    /// the 3D hit testing); `None` for another index or when they no longer exist.
    pub fn model_source(&self, index: usize) -> Option<(Ref<Visual3D>, Ref<GeometryModel3D>)> {
        let sources = self.sources.borrow();
        let (visual, model) = sources.get(index)?;
        Some((visual.upgrade()?, model.upgrade()?))
    }

    /// The bounds of what the scene shows, in the world.
    pub fn descendant_bounds(&self) -> Rect3D {
        let mut bounds = Rect3D::EMPTY;
        for child in self.children.iter() {
            bounds.union(child.descendant_bounds());
        }
        bounds
    }

    /// Bumps the generation of `change` and drops the snapshot.
    fn bump(&self, change: Scene3DChange) {
        let mut generations = self.generations.get();
        match change {
            Scene3DChange::Structure => generations.structure += 1,
            Scene3DChange::Record => generations.record += 1,
            Scene3DChange::Camera => generations.camera += 1,
        }
        self.generations.set(generations);
        self.snapshot.borrow_mut().take();
    }
}
