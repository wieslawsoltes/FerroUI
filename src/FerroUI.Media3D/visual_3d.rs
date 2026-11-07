//! WPF's `Visual3D` and `ModelVisual3D`: the visual tree of a 3D scene.
//!
//! A visual has a `Transform` (a change of it is a change of a record); a `ModelVisual3D` shows
//! the model of its `Content` and holds `Children` of its own (their additions, removals and
//! replacements are changes of the structure).

use crate::change::{same, Link, ListLink, Notifier, Notifying, Scene3DChange};
use crate::transform_3d::{transform_rect, value_of};
use crate::{Model3D, Rect3D, Transform3D, Visual3DCollection};
use ferroui_base::reactive::Subscription;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObject, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledProperty,
};
use std::rc::Rc;

/// WPF's `Visual3D`: the base of the 3D visuals, with a `Transform`.
#[repr(C)]
pub struct Visual3D {
    base: FerroObject,
    notifier: Rc<Notifier>,
    transform_link: Link,
}

ferro_class!(Visual3D: FerroObject);

impl FerroObjectImpl for Visual3D {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::transform_property().as_property() {
            this.transform_link.set(this.transform().as_ref(), &this.notifier, same);
            this.notifier.raise(Scene3DChange::Record);
        }
    }
}

impl Notifying for Visual3D {
    fn notifier(&self) -> &Rc<Notifier> {
        &self.notifier
    }
}

ferro_properties! { impl Visual3D {
    pub fn transform_property() -> StyledProperty<Option<Ref<Transform3D>>> {
        FerroProperty::register::<Visual3D, _>("Transform", None)
    }
} }

impl Visual3D {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), notifier: Notifier::new(), transform_link: Link::default() }
    }

    /// The transform of the visual; none is the identity.
    pub fn transform(&self) -> Option<Ref<Transform3D>> {
        self.get_value(Self::transform_property())
    }

    pub fn set_transform(&self, value: Option<Ref<Transform3D>>) {
        self.set_value(Self::transform_property(), value)
    }

    /// Calls `handler` on every change of the visual or of what it shows, until the returned
    /// guard is dropped.
    pub fn changed(&self, handler: impl Fn(Scene3DChange) + 'static) -> Subscription {
        self.notifier.subscribe(Rc::new(handler))
    }

    /// The bounds of what the visual and its descendants show, in the coordinates of its parent
    /// (its transform applied); WPF's `VisualTreeHelper.GetDescendantBounds` with the content.
    pub fn descendant_bounds(&self) -> Rect3D {
        let mut inner = Rect3D::EMPTY;
        if let Some(model_visual) = self.to_ref().cast::<ModelVisual3D>() {
            if let Some(content) = model_visual.content() {
                inner.union(content.bounds());
            }
            for child in model_visual.children().iter() {
                inner.union(child.descendant_bounds());
            }
        }
        transform_rect(inner, &value_of(self.transform().as_ref()))
    }
}

// --- ModelVisual3D -----------------------------------------------------------------------------

/// WPF's `ModelVisual3D`: shows the model of its `Content` and its `Children`.
#[repr(C)]
pub struct ModelVisual3D {
    base: Visual3D,
    content_link: Link,
    children: Visual3DCollection,
    children_link: ListLink<Visual3D>,
}

ferro_class!(ModelVisual3D: Visual3D);
ferro_class_info!(ModelVisual3D {
    new: ModelVisual3D::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        content: Children,
        properties: [
            Children: Visual3DCollection { get: ModelVisual3D::children },
        ],
    },
});

impl FerroObjectImpl for ModelVisual3D {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.children_link.set(Some(&*this.children), this.notifier(), Scene3DChange::Structure);
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::content_property().as_property() {
            this.content_link.set(this.content().as_ref(), this.notifier(), same);
            this.notifier().raise(Scene3DChange::Structure);
        }
    }
}

ferro_properties! { impl ModelVisual3D {
    pub fn content_property() -> StyledProperty<Option<Ref<Model3D>>> {
        FerroProperty::register::<ModelVisual3D, _>("Content", None)
    }
} }

impl ModelVisual3D {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self {
            base: Visual3D::construct(),
            content_link: Link::default(),
            children: Visual3DCollection::new(),
            children_link: ListLink::default(),
        }
    }

    /// A visual without content or children.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A visual showing `content`.
    pub fn with_content(content: Ref<Model3D>) -> Ref<Self> {
        let result = Self::new();
        result.set_content(Some(content));
        result
    }

    /// The model the visual shows.
    pub fn content(&self) -> Option<Ref<Model3D>> {
        self.get_value(Self::content_property())
    }

    pub fn set_content(&self, value: Option<Ref<Model3D>>) {
        self.set_value(Self::content_property(), value)
    }

    /// The child visuals (a shared handle to the collection).
    pub fn children(&self) -> Visual3DCollection {
        self.children.clone()
    }
}
