use ferroui_base::media::{CombinedGeometry, Geometry, GeometryCombineMode, MatrixTransform, RectangleGeometry};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{FerroPropertyChangedEventArgs, Rect, Ref, Visual, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Helpers of the adorner layer: change tracking of the ancestors of an
/// adorned element and the clip of its adorner.
pub struct AdornerHelper;

impl AdornerHelper {
    /// Calls `changed` whenever the bounds or the render transform (with
    /// `include_clip` also the clip) of the visual or of one of its
    /// ancestors changes, and when the visual is attached to or detached
    /// from a visual tree. Disposing the returned handle unsubscribes.
    pub fn subscribe_to_ancestor_property_changes(
        visual: &Ref<Visual>,
        include_clip: bool,
        changed: impl Fn() + 'static,
    ) -> Rc<dyn IDisposable> {
        AncestorPropertyChangesSubscription::new(visual, include_clip, Rc::new(changed))
    }

    /// Calculates the clip of an adorner from the clips of the adorned
    /// element and its ancestors, in the coordinates of the adorned element.
    pub fn calculate_adorner_clip(adorned_element: &Visual) -> Option<Ref<Geometry>> {
        // Walk ancestor stack and calculate clip geometry relative to the current visual.
        // If ClipToBounds = true, add extra RectangleGeometry for Bounds.Size

        let mut result: Option<Ref<Geometry>> = None;
        let mut ancestor = Some(adorned_element.to_ref());

        while let Some(visual_ancestor) = ancestor {
            let mut ancestor_clip: Option<Ref<Geometry>> = None;

            // Check if ancestor has ClipToBounds enabled
            if visual_ancestor.clip_to_bounds() {
                ancestor_clip =
                    Some(RectangleGeometry::with_rect(Rect::from_size(visual_ancestor.bounds().size())).upcast());
            }

            // Check if ancestor has explicit Clip geometry
            if let Some(clip) = visual_ancestor.clip() {
                ancestor_clip = Some(match ancestor_clip {
                    Some(bounds_clip) => {
                        CombinedGeometry::with_mode(GeometryCombineMode::Intersect, Some(bounds_clip), Some(clip))
                            .upcast()
                    }
                    None => clip,
                });
            }

            // Transform the clip geometry to adorned element's coordinate space
            if let Some(mut ancestor_clip) = ancestor_clip {
                let transform = visual_ancestor.transform_to_visual(adorned_element);
                if let Some(transform) = transform.filter(|transform| !transform.is_identity()) {
                    ancestor_clip = ancestor_clip.clone_geometry();
                    let matrix = match ancestor_clip.transform().map(|t| t.value()) {
                        Some(value) if !value.is_identity() => transform * value,
                        _ => transform,
                    };
                    ancestor_clip.set_transform(MatrixTransform::with_matrix(matrix));
                }

                // Combine with existing result
                result = Some(match result {
                    Some(result) => {
                        CombinedGeometry::with_mode(GeometryCombineMode::Intersect, Some(result), Some(ancestor_clip))
                            .upcast()
                    }
                    None => ancestor_clip,
                });
            }

            ancestor = visual_ancestor.visual_parent();
        }

        result
    }
}

/// The subscription does not keep the visual alive; the visual keeps the
/// subscription alive until it is disposed.
struct AncestorPropertyChangesSubscription {
    visual: WeakRef<Visual>,
    include_clip: bool,
    changed: Rc<dyn Fn()>,
    attachment_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    is_disposed: Cell<bool>,
}

impl AncestorPropertyChangesSubscription {
    fn new(visual: &Ref<Visual>, include_clip: bool, changed: Rc<dyn Fn()>) -> Rc<Self> {
        let this = Rc::new(Self {
            visual: visual.downgrade(),
            include_clip,
            changed,
            attachment_subscriptions: RefCell::new(Vec::new()),
            subscriptions: RefCell::new(Vec::new()),
            is_disposed: Cell::new(false),
        });

        let attached = {
            let this = this.clone();
            visual.attached_to_visual_tree(move |_| this.on_attached_to_visual_tree())
        };
        let detached = {
            let this = this.clone();
            visual.detached_from_visual_tree(move |_| this.on_detached_from_visual_tree())
        };
        this.attachment_subscriptions.borrow_mut().extend([attached, detached]);

        if visual.is_attached_to_visual_tree() {
            Self::subscribe_to_ancestors(&this);
        }

        this
    }

    fn subscribe_to_ancestors(this: &Rc<Self>) {
        this.unsubscribe_from_ancestors();

        // Subscribe to the visual's own Bounds property, then walk up the
        // ancestor chain
        let mut current = this.visual.upgrade();
        while let Some(visual) = current {
            let handler = this.clone();
            let subscription = visual.property_changed(move |e| handler.on_property_changed(e));
            this.subscriptions.borrow_mut().push(subscription);
            current = visual.visual_parent();
        }
    }

    fn unsubscribe_from_ancestors(&self) {
        let subscriptions = std::mem::take(&mut *self.subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
    }

    fn on_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if !e.is_effective_value_change() {
            return;
        }

        let property = e.property();
        let mut should_notify = false;

        if property == Visual::render_transform_property().as_property()
            || property == Visual::bounds_property().as_property()
        {
            should_notify = true;
        } else if self.include_clip
            && (property == Visual::clip_to_bounds_property().as_property()
                || property == Visual::clip_property().as_property())
        {
            should_notify = true;
        }

        if should_notify {
            (self.changed)();
        }
    }

    fn on_attached_to_visual_tree(self: &Rc<Self>) {
        Self::subscribe_to_ancestors(self);
        (self.changed)();
    }

    fn on_detached_from_visual_tree(&self) {
        self.unsubscribe_from_ancestors();
        (self.changed)();
    }
}

impl IDisposable for AncestorPropertyChangesSubscription {
    fn dispose(&self) {
        if self.is_disposed.replace(true) {
            return;
        }

        self.unsubscribe_from_ancestors();
        let subscriptions = std::mem::take(&mut *self.attachment_subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
    }
}
