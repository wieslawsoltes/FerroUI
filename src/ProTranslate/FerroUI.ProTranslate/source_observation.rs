use crate::{Translation, TranslationBindingSource};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{FerroObject, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// One native consumer's current source. Source changes reconnect without owning
/// a target; updates are serialized so a callback can change its culture or scope.
pub(crate) struct SourceObservation {
    target: WeakRef<FerroObject>,
    explicit_source: Option<TranslationBindingSource>,
    update: Rc<dyn Fn()>,
    source_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    target_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    default_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    disposed: Cell<bool>,
    updating: Cell<bool>,
    pending: Cell<bool>,
}

impl SourceObservation {
    pub(crate) fn start(
        target: WeakRef<FerroObject>,
        explicit_source: Option<TranslationBindingSource>,
        update: Rc<dyn Fn()>,
    ) -> Rc<Self> {
        let this = Rc::new(Self {
            target,
            explicit_source,
            update,
            source_subscription: RefCell::new(None),
            target_subscription: RefCell::new(None),
            default_subscription: RefCell::new(None),
            disposed: Cell::new(false),
            updating: Cell::new(false),
            pending: Cell::new(false),
        });
        this.reconnect();
        if this.explicit_source.is_none() {
            if let Some(target) = this.target.upgrade() {
                let weak = Rc::downgrade(&this);
                let subscription = target.property_changed(move |change| {
                    if change.property() == Translation::source_property().as_property() {
                        if let Some(this) = weak.upgrade() {
                            this.reconnect();
                            this.refresh();
                        }
                    }
                });
                this.target_subscription.replace(Some(subscription));
            }
            let weak = Rc::downgrade(&this);
            this.default_subscription.replace(Some(
                TranslationBindingSource::subscribe_default_changed(move || {
                    if let Some(this) = weak.upgrade() {
                        if this.target.upgrade().is_some_and(|target| {
                            target.get_value(Translation::source_property()).is_none()
                        }) {
                            this.reconnect();
                            this.refresh();
                        }
                    }
                }),
            ));
        }
        this
    }

    fn source(&self) -> TranslationBindingSource {
        self.explicit_source
            .clone()
            .or_else(|| {
                self.target
                    .upgrade()
                    .map(|target| Translation::get_source(&target))
            })
            .unwrap_or_else(TranslationBindingSource::default_source)
    }

    fn reconnect(self: &Rc<Self>) {
        if self.disposed.get() {
            return;
        }
        let old = self.source_subscription.borrow_mut().take();
        if let Some(old) = old {
            old.dispose();
        }
        let weak = Rc::downgrade(self);
        let subscription = self.source().subscribe(move || {
            if let Some(this) = weak.upgrade() {
                this.refresh();
            }
        });
        self.source_subscription.replace(Some(subscription));
    }

    pub(crate) fn refresh(&self) {
        if self.disposed.get() {
            return;
        }
        self.pending.set(true);
        if self.updating.replace(true) {
            return;
        }
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _reset = Reset(&self.updating);
        while self.pending.replace(false) && !self.disposed.get() {
            (self.update)();
        }
    }
}

impl IDisposable for SourceObservation {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        // Release each RefCell borrow before disposal can run application code.
        let source = self.source_subscription.borrow_mut().take();
        let target = self.target_subscription.borrow_mut().take();
        let default = self.default_subscription.borrow_mut().take();
        for subscription in [source, target, default].into_iter().flatten() {
            subscription.dispose();
        }
    }
}

impl Drop for SourceObservation {
    fn drop(&mut self) {
        self.dispose();
    }
}
