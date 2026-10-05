use crate::media::MediaContext;
use crate::reactive::{Disposable, IDisposable, IObservable, Observable};
use crate::threading::{Dispatcher, DispatcherPriority};
use crate::utilities::HandlerList;
use crate::{FerroPropertyChangedEventArgs, Matrix, Ref, Visual, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Tracks the transform of a visual to the root of its visual tree: tells
/// when it changes because the bounds of the visual or of one of its
/// ancestors changed, or because the visual was attached or detached.
///
/// The helper does not keep the tracked visual alive; the visual keeps the
/// helper alive for as long as it is tracked.
pub struct TransformTrackingHelper {
    this: Weak<TransformTrackingHelper>,
    defer_after_render_pass: bool,
    visual: RefCell<Option<WeakRef<Visual>>>,
    queued_for_update: Cell<bool>,
    attachment_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    property_changed_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    matrix: Cell<Option<Matrix>>,
    matrix_changed: HandlerList<dyn Fn()>,
}

impl TransformTrackingHelper {
    /// Creates the helper. With `defer_after_render_pass` a change of
    /// bounds is processed after the next render pass, otherwise right
    /// before it.
    pub fn new(defer_after_render_pass: bool) -> Rc<TransformTrackingHelper> {
        Rc::new_cyclic(|this| TransformTrackingHelper {
            this: this.clone(),
            defer_after_render_pass,
            visual: RefCell::new(None),
            queued_for_update: Cell::new(false),
            attachment_subscriptions: RefCell::new(Vec::new()),
            property_changed_subscriptions: RefCell::new(Vec::new()),
            matrix: Cell::new(None),
            matrix_changed: HandlerList::new(),
        })
    }

    fn strong(&self) -> Rc<TransformTrackingHelper> {
        self.this.upgrade().expect("the helper is alive while it is used")
    }

    fn visual(&self) -> Option<Ref<Visual>> {
        self.visual.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// Sets the visual to track, or stops tracking.
    pub fn set_visual(&self, visual: Option<&Ref<Visual>>) {
        self.dispose();
        *self.visual.borrow_mut() = visual.map(Ref::downgrade);

        if let Some(visual) = visual {
            let attached = {
                let this = self.strong();
                visual.attached_to_visual_tree(move |_| this.on_attached_to_visual_tree())
            };
            let detached = {
                let this = self.strong();
                visual.detached_from_visual_tree(move |_| this.on_detached_from_visual_tree())
            };
            self.attachment_subscriptions.borrow_mut().extend([attached, detached]);

            if visual.is_attached_to_visual_tree() {
                self.subscribe_to_parents();
            }
            self.update_matrix();
        }
    }

    /// The transform of the tracked visual to its visual root, if it is
    /// attached to one.
    pub fn matrix(&self) -> Option<Matrix> {
        self.matrix.get()
    }

    /// Raised when [`matrix`](Self::matrix) has changed. Disposing the
    /// returned handle unsubscribes.
    pub fn matrix_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.matrix_changed.add(Rc::new(handler));
        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.matrix_changed.remove(token);
            }
        })
    }

    /// Stops tracking the visual.
    pub fn dispose(&self) {
        if self.visual.borrow().is_none() {
            return;
        }

        self.unsubscribe_from_parents();
        let subscriptions = std::mem::take(&mut *self.attachment_subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
        *self.visual.borrow_mut() = None;
    }

    fn subscribe_to_parents(&self) {
        let mut visual = self.visual();

        while let Some(v) = visual {
            let this = self.strong();
            let subscription = v.property_changed(move |e| this.property_changed_handler(e));
            self.property_changed_subscriptions.borrow_mut().push(subscription);
            visual = v.visual_parent();
        }
    }

    fn unsubscribe_from_parents(&self) {
        let subscriptions = std::mem::take(&mut *self.property_changed_subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
    }

    fn update_matrix(&self) {
        self.queued_for_update.set(false);
        let mut matrix = None;

        if let Some(visual) = self.visual() {
            if let Some(root) = visual.visual_root() {
                matrix = visual.transform_to_visual(&root);
            }
        }

        if self.matrix.get() != matrix {
            self.matrix.set(matrix);

            if !self.matrix_changed.is_empty() {
                for (_, handler) in self.matrix_changed.snapshot().iter() {
                    handler();
                }
            }
        }
    }

    fn on_attached_to_visual_tree(&self) {
        self.subscribe_to_parents();
        self.update_matrix();
    }

    fn enqueue_for_update(&self) {
        if self.queued_for_update.get() {
            return;
        }
        self.queued_for_update.set(true);

        let this = self.strong();
        if self.defer_after_render_pass {
            Dispatcher::ui_thread().post_local(move || this.update_matrix(), DispatcherPriority::AFTER_RENDER);
        } else {
            MediaContext::instance().begin_invoke_on_render(Rc::new(move || this.update_matrix()));
        }
    }

    fn property_changed_handler(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.is_effective_value_change() && e.property() == Visual::bounds_property().as_property() {
            self.enqueue_for_update();
        }
    }

    fn on_detached_from_visual_tree(&self) {
        self.unsubscribe_from_parents();
        self.update_matrix();
    }

    /// Tracks the transform of a visual, calling `cb` with the visual and
    /// its transform whenever it changes. Disposing the returned handle
    /// stops tracking.
    pub fn track(
        visual: &Ref<Visual>,
        defer_after_render_pass: bool,
        cb: impl Fn(&Ref<Visual>, Option<Matrix>) + 'static,
    ) -> Rc<dyn IDisposable> {
        let rv = TransformTrackingHelper::new(defer_after_render_pass);
        {
            let weak = Rc::downgrade(&rv);
            let visual = visual.downgrade();
            rv.matrix_changed(move || {
                if let (Some(rv), Some(visual)) = (weak.upgrade(), visual.upgrade()) {
                    cb(&visual, rv.matrix());
                }
            });
        }
        rv.set_visual(Some(visual));
        rv
    }

    /// Returns an observable of the transform of a visual to its visual
    /// root.
    pub fn observe(visual: &Ref<Visual>, defer_after_render_pass: bool) -> Rc<dyn IObservable<Option<Matrix>>> {
        let visual = visual.downgrade();
        Observable::create(move |observer| {
            let rv = TransformTrackingHelper::new(defer_after_render_pass);
            {
                let weak = Rc::downgrade(&rv);
                rv.matrix_changed(move || {
                    if let Some(rv) = weak.upgrade() {
                        observer.on_next(rv.matrix());
                    }
                });
            }
            rv.set_visual(visual.upgrade().as_ref());
            rv as Rc<dyn IDisposable>
        })
    }
}

impl IDisposable for TransformTrackingHelper {
    fn dispose(&self) {
        TransformTrackingHelper::dispose(self)
    }
}
