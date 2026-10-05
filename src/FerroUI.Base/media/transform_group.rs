use crate::media::media_collection::media_collection_type;
use crate::media::{MediaCollection, ResetBehavior, Transform, TransformImpl};
use crate::reactive::IDisposable;
use crate::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Matrix,
    Ref, StyledProperty,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

media_collection_type!(
    /// A collection of [`Transform`]s.
    Transforms, Ref<Transform>
);

impl Transforms {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self(MediaCollection::new())
    }

    /// Creates a collection holding `items`.
    pub fn from_items(items: impl IntoIterator<Item = Ref<Transform>>) -> Self {
        Self(MediaCollection::from_items(items))
    }
}

type ChildSubscriptions = Rc<RefCell<Vec<(Ref<Transform>, Rc<dyn IDisposable>)>>>;

/// A transform that applies its child transforms in order.
#[repr(C)]
pub struct TransformGroup {
    base: Transform,
    children_notification_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    child_subscriptions: ChildSubscriptions,
    last_matrix: Cell<Option<Matrix>>,
}

ferro_class!(TransformGroup: Transform);
crate::ferro_class_info!(TransformGroup { new: TransformGroup::new });

impl FerroObjectImpl for TransformGroup {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_children(Transforms::new());
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::children_property().as_property() {
            if let Some(subscription) = this.children_notification_subscription.take() {
                subscription.dispose();
            }

            // Detach from the transforms of the old collection.
            let old = std::mem::take(&mut *this.child_subscriptions.borrow_mut());
            for (_, subscription) in old {
                subscription.dispose();
            }

            if let Some(new_transforms) = change.get_new_value::<Option<Transforms>>() {
                // Ensure reset behavior is Remove
                new_transforms.set_reset_behavior(ResetBehavior::Remove);

                let weak = this.to_ref().downgrade();
                let subscriptions = this.child_subscriptions.clone();
                let weak_added = weak.clone();
                let subscriptions_added = subscriptions.clone();
                let subscription = new_transforms.for_each_item(
                    move |_, tr: &Ref<Transform>| {
                        let weak_child = weak_added.clone();
                        let handle = tr.changed(move || {
                            if let Some(this) = weak_child.upgrade() {
                                this.on_transform_invalidated();
                            }
                        });
                        subscriptions_added.borrow_mut().push((tr.clone(), handle));
                        if let Some(this) = weak_added.upgrade() {
                            this.on_transform_invalidated();
                        }
                    },
                    move |_, tr: &Ref<Transform>| {
                        let position = subscriptions.borrow().iter().position(|(t, _)| t.ptr_eq(tr));
                        if let Some(position) = position {
                            let (_, handle) = subscriptions.borrow_mut().remove(position);
                            handle.dispose();
                        }
                        if let Some(this) = weak.upgrade() {
                            this.on_transform_invalidated();
                        }
                    },
                    || {},
                );
                *this.children_notification_subscription.borrow_mut() = Some(subscription);
            }

            this.on_transform_invalidated();
        }
    }
}

impl TransformImpl for TransformGroup {
    fn value(this: &Self) -> Matrix {
        if let Some(matrix) = this.last_matrix.get() {
            return matrix;
        }

        let mut matrix = Matrix::IDENTITY;
        if let Some(children) = this.get_value(Self::children_property()) {
            for t in children.iter() {
                matrix *= t.value();
            }
        }
        this.last_matrix.set(Some(matrix));
        matrix
    }
}

crate::ferro_properties! { impl TransformGroup {
    ferro_property!(pub fn children_property() -> StyledProperty<Option<Transforms>> {
        FerroProperty::register::<TransformGroup, _>("Children", None)
    });
} }

impl TransformGroup {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self {
            base: Transform::construct(),
            children_notification_subscription: RefCell::new(None),
            child_subscriptions: Rc::new(RefCell::new(Vec::new())),
            last_matrix: Cell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn on_transform_invalidated(&self) {
        self.last_matrix.set(None);
        self.raise_changed();
    }

    /// The children. Panics if the property has been cleared.
    pub fn children(&self) -> Transforms {
        self.get_value(Self::children_property()).expect("Children is not set")
    }

    pub fn set_children(&self, value: Transforms) {
        self.set_value(Self::children_property(), Some(value))
    }
}
