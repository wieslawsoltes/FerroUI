use super::animations::ICompositionAnimationBase;
use super::{
    CompositionBatchTypes, CompositionVisual, Compositor, ICompositionObjectAnimations, ICompositionSurface,
    LoadedImageSurface,
};
use crate::media::imaging::RenderTargetBitmap;
use crate::platform::IPlatformRenderInterface;
use crate::reactive::IDisposable;
use crate::{FerroLocator, LocatorExtensions, PixelSize, Vector, Vector3D, Visual, VisualTreeAttachmentEventArgs, WeakRef};
use std::cell::RefCell;
use std::rc::Rc;

crate::ferro_thread_local! {
    /// The implicit show and hide animations of the elements that have one, and the elements
    /// that enabled translation (`SetImplicitShowAnimation`, `SetImplicitHideAnimation`,
    /// `SetIsTranslationEnabled` of WinUI's `ElementCompositionPreview`).
    static IMPLICIT: RefCell<Vec<Rc<ImplicitEntry>>> = const { RefCell::new(Vec::new()) };
}

/// Access to the composition visuals of the visuals of the visual tree.
pub struct ElementComposition;

impl ElementComposition {
    /// Gets the composition visual of a visual; `None` while the visual is
    /// not attached to a compositor.
    pub fn get_element_visual(visual: &Visual) -> Option<Rc<CompositionVisual>> {
        visual.composition_visual().map(|v| (*v).clone())
    }

    /// Sets a composition visual that is shown as the last child of the
    /// composition visual of `visual`.
    pub fn set_element_child_visual(visual: &Visual, composition_visual: Option<Rc<CompositionVisual>>) {
        if let (Some(child), Some(own)) = (&composition_visual, visual.composition_visual()) {
            if !Rc::ptr_eq(child.compositor(), own.compositor()) {
                panic!("Composition visuals belong to different compositor instances");
            }
        }

        // Un-parent the previous child here rather than waiting for the renderer to
        // reconcile the children, because that sync may never run again for this element:
        // a child cleared from on_detached_from_visual_tree is cleared before
        // detach_from_compositor discards the element's composition visual, and a child
        // left parented there cannot be attached anywhere else.
        if let Some(old_child) = visual.child_composition_visual() {
            if !composition_visual.as_ref().is_some_and(|child| Rc::ptr_eq(child, &old_child)) {
                if let Some(own) = visual.composition_visual() {
                    own.children().remove(&old_child);
                }
            }
        }

        visual.set_child_composition_visual(composition_visual);
        if let Some(source) = visual.presentation_source() {
            source.renderer().recalculate_children(visual);
        }
    }

    /// Gets the composition visual set with
    /// [`set_element_child_visual`](Self::set_element_child_visual).
    pub fn get_element_child_visual(visual: &Visual) -> Option<Rc<CompositionVisual>> {
        visual.child_composition_visual()
    }

    /// `ElementCompositionPreview.SetImplicitShowAnimation`: the animation, or the group of
    /// animations, started on the composition visual of `visual` when the visual is added to a
    /// visual tree or its `IsVisible` becomes `true`. Every animation needs its `Target`.
    pub fn set_implicit_show_animation(visual: &Visual, animation: Option<Rc<dyn ICompositionAnimationBase>>) {
        ImplicitEntry::update(visual, |entry| *entry.show.borrow_mut() = animation);
    }

    /// The animation of [`set_implicit_show_animation`](Self::set_implicit_show_animation).
    pub fn get_implicit_show_animation(visual: &Visual) -> Option<Rc<dyn ICompositionAnimationBase>> {
        ImplicitEntry::find(visual).and_then(|entry| entry.show.borrow().clone())
    }

    /// `ElementCompositionPreview.SetImplicitHideAnimation`: the animation, or the group of
    /// animations, started on the composition visual of `visual` when its `IsVisible` becomes
    /// `false` or the visual is removed from its visual tree. The visual stays visible until the
    /// animations complete. Every animation needs its `Target`.
    ///
    /// A visual removed from its tree loses its composition visual at once, so the hide
    /// animation of a removal runs on an image of the visual taken when it is removed (WinUI
    /// keeps the element itself in its unloading storage until the animation completes).
    pub fn set_implicit_hide_animation(visual: &Visual, animation: Option<Rc<dyn ICompositionAnimationBase>>) {
        ImplicitEntry::update(visual, |entry| *entry.hide.borrow_mut() = animation);
    }

    /// The animation of [`set_implicit_hide_animation`](Self::set_implicit_hide_animation).
    pub fn get_implicit_hide_animation(visual: &Visual) -> Option<Rc<dyn ICompositionAnimationBase>> {
        ImplicitEntry::find(visual).and_then(|entry| entry.hide.borrow().clone())
    }

    /// `ElementCompositionPreview.SetIsTranslationEnabled`. Every composition visual has a
    /// `Translation` property applied after its other transforms, so translation is always
    /// enabled; the request is recorded and changes nothing else.
    pub fn set_is_translation_enabled(visual: &Visual, value: bool) {
        ImplicitEntry::update(visual, |entry| entry.translation_enabled.set(value));
    }

    /// Whether [`set_is_translation_enabled`](Self::set_is_translation_enabled) was called with
    /// `true` for `visual`.
    pub fn is_translation_enabled(visual: &Visual) -> bool {
        ImplicitEntry::find(visual).is_some_and(|entry| entry.translation_enabled.get())
    }
}

/// The implicit animations of one visual and the subscriptions that run them.
struct ImplicitEntry {
    visual: WeakRef<Visual>,
    show: RefCell<Option<Rc<dyn ICompositionAnimationBase>>>,
    hide: RefCell<Option<Rc<dyn ICompositionAnimationBase>>>,
    translation_enabled: std::cell::Cell<bool>,
    /// While a hide animation runs: the subscription to the completion of its batch. The visual
    /// holds a show request meanwhile.
    hiding: RefCell<Option<Rc<dyn IDisposable>>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

impl ImplicitEntry {
    fn find(visual: &Visual) -> Option<Rc<ImplicitEntry>> {
        IMPLICIT.with(|entries| {
            entries
                .borrow()
                .iter()
                .find(|entry| entry.visual.upgrade().is_some_and(|v| std::ptr::eq(&*v, visual)))
                .cloned()
        })
    }

    /// The entry of `visual`, created with its subscriptions, changed by `f`. The entries of
    /// visuals that are gone are dropped on the way.
    fn update(visual: &Visual, f: impl FnOnce(&ImplicitEntry)) {
        IMPLICIT.with(|entries| {
            entries.borrow_mut().retain(|entry| {
                let alive = entry.visual.upgrade().is_some();
                if !alive {
                    for subscription in entry.subscriptions.borrow_mut().drain(..) {
                        subscription.dispose();
                    }
                }
                alive
            })
        });
        let entry = match Self::find(visual) {
            Some(entry) => entry,
            None => {
                let entry = Rc::new(ImplicitEntry {
                    visual: visual.to_ref().downgrade(),
                    show: RefCell::new(None),
                    hide: RefCell::new(None),
                    translation_enabled: std::cell::Cell::new(false),
                    hiding: RefCell::new(None),
                    subscriptions: RefCell::new(Vec::new()),
                });
                entry.subscribe(visual);
                IMPLICIT.with(|entries| entries.borrow_mut().push(entry.clone()));
                entry
            }
        };
        f(&entry);
    }

    fn subscribe(self: &Rc<Self>, visual: &Visual) {
        let weak = Rc::downgrade(self);
        let changed = visual.property_changed(move |e| {
            if e.property() != Visual::is_visible_property().as_property() {
                return;
            }
            let Some(entry) = weak.upgrade() else { return };
            if e.get_new_value::<bool>() {
                entry.on_shown();
            } else {
                entry.on_hidden();
            }
        });
        let weak = Rc::downgrade(self);
        let attached = visual.attached_to_visual_tree(move |_| {
            if let Some(entry) = weak.upgrade() {
                entry.on_shown();
            }
        });
        let weak = Rc::downgrade(self);
        let detached = visual.detached_from_visual_tree(move |e| {
            if let Some(entry) = weak.upgrade() {
                entry.on_removed(e);
            }
        });
        self.subscriptions.borrow_mut().extend([changed, attached, detached]);
    }

    /// The compositor of the visual, while it is in a tree rendered by one.
    fn compositor(visual: &Visual) -> Option<Rc<Compositor>> {
        visual.presentation_source().and_then(|source| source.renderer().compositor())
    }

    /// Ends a running hide animation: the visual follows `IsVisible` again.
    fn end_hiding(&self, visual: &Visual) {
        if let Some(subscription) = self.hiding.borrow_mut().take() {
            subscription.dispose();
            visual.remove_composition_show_request();
        }
    }

    /// The visual was added to a tree or became visible: the show animation starts on its
    /// composition visual before the next commit, once the renderer created it.
    fn on_shown(self: &Rc<Self>) {
        let Some(visual) = self.visual.upgrade() else { return };
        self.end_hiding(&visual);
        if self.show.borrow().is_none() || !visual.is_visible() {
            return;
        }
        let Some(compositor) = Self::compositor(&visual) else { return };
        let weak = Rc::downgrade(self);
        compositor.request_composition_update(move || {
            let Some(entry) = weak.upgrade() else { return };
            let Some(visual) = entry.visual.upgrade() else { return };
            let (Some(animation), Some(composition_visual)) = (entry.show.borrow().clone(), visual.composition_visual())
            else {
                return;
            };
            composition_visual.start_animation_group(&*animation);
        });
    }

    /// `IsVisible` became `false`: the hide animation runs in a scoped batch while the
    /// composition visual is kept visible; its completion lets the visual hide.
    fn on_hidden(self: &Rc<Self>) {
        let Some(visual) = self.visual.upgrade() else { return };
        let Some(animation) = self.hide.borrow().clone() else { return };
        let Some(composition_visual) = visual.composition_visual() else { return };
        self.end_hiding(&visual);
        let compositor = composition_visual.compositor().clone();
        visual.add_composition_show_request();
        let batch = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
        composition_visual.start_animation_group(&*animation);
        batch.end();
        let weak = Rc::downgrade(self);
        let subscription = batch.completed(move |_| {
            let Some(entry) = weak.upgrade() else { return };
            let Some(visual) = entry.visual.upgrade() else { return };
            entry.end_hiding(&visual);
        });
        *self.hiding.borrow_mut() = Some(subscription);
    }

    /// The visual was removed from its tree: its composition visual is gone, so the hide
    /// animation plays on an image of it, shown by the former parent where the visual was
    /// (WinUI keeps the element itself in unloading storage). Only the root of a removed subtree
    /// plays its animation.
    fn on_removed(self: &Rc<Self>, e: &VisualTreeAttachmentEventArgs) {
        let Some(visual) = self.visual.upgrade() else { return };
        self.end_hiding(&visual);
        let Some(animation) = self.hide.borrow().clone() else { return };
        let Some(parent) = e.attachment_point().cloned() else { return };
        if visual.visual_parent().is_some_and(|own| !std::ptr::eq(&*own, &*parent)) {
            return;
        }
        let (Some(parent_visual), Some(source)) = (parent.composition_visual(), parent.presentation_source()) else {
            return;
        };
        if !visual.is_visible() || FerroLocator::current().get_service::<dyn IPlatformRenderInterface>().is_none() {
            return;
        }
        let bounds = visual.bounds();
        let scaling = source.render_scaling();
        let pixel_size = PixelSize::new((bounds.width * scaling).ceil() as i32, (bounds.height * scaling).ceil() as i32);
        if pixel_size.width <= 0 || pixel_size.height <= 0 {
            return;
        }
        let image = RenderTargetBitmap::with_dpi(pixel_size, Vector::new(96.0 * scaling, 96.0 * scaling));
        image.render(&visual.to_ref());

        let compositor = parent_visual.compositor().clone();
        let surface = LoadedImageSurface::from_bitmap(&compositor, &image);
        let brush = compositor.create_surface_brush_with_surface(surface as Rc<dyn ICompositionSurface>);
        let sprite = compositor.create_sprite_visual();
        sprite.set_brush(Some((*brush).clone()));
        sprite.set_size(Vector::new(bounds.width, bounds.height));
        sprite.set_offset(Vector3D::new(bounds.x, bounds.y, 0.0));
        sprite.set_opacity(visual.opacity() as f32);
        let ghost: Rc<CompositionVisual> = (*sprite).clone();
        parent.add_composition_unloading_visual(ghost.clone());

        let batch = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
        ghost.start_animation_group(&*animation);
        batch.end();
        let weak_parent = parent.downgrade();
        let subscription: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
        let held = subscription.clone();
        let completed = batch.completed(move |_| {
            if let Some(parent) = weak_parent.upgrade() {
                parent.remove_composition_unloading_visual(&ghost);
            }
            if let Some(subscription) = held.borrow_mut().take() {
                subscription.dispose();
            }
        });
        *subscription.borrow_mut() = Some(completed);
    }
}
