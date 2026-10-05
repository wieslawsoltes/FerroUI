use crate::media::effects::{Effect, IEffect};
use crate::media::{DrawingContext, IImage, Transform};
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::HandlerList;
use crate::{
    ferro_class, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect,
    Ref,
};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

type ValueSubscription = (&'static FerroProperty, Rc<dyn IDisposable>);

/// Abstract base class of the objects describing drawn content.
#[repr(C)]
pub struct Drawing {
    base: FerroObject,
    invalidated: HandlerList<dyn Fn()>,
    value_subscriptions: RefCell<Vec<ValueSubscription>>,
}

ferro_class! {
    Drawing: FerroObject, virtuals DrawingImpl: FerroObjectImpl {
        /// Draws the content to a drawing context; see [`Drawing::draw`].
        fn draw_core(this, context: &mut DrawingContext);
        /// The bounds of the drawn content.
        fn get_bounds(this) -> Rect;
    }
}

impl FerroObjectImpl for Drawing {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        // Every property of a drawing is baked into the recorded drawing
        // commands or selects which resource is used, so any change requires
        // a re-record. Keep subscriptions to nested values whose own changes
        // are also baked in sync.
        this.update_value_subscription(change.property(), None);
        this.update_value_subscription(change.property(), Some(change.new_value()));
        this.raise_invalidated();
    }
}

impl DrawingImpl for Drawing {
    fn draw_core(_this: &Self, _context: &mut DrawingContext) {
        panic!("Drawing is abstract: 'draw_core' must be implemented by the deriving class")
    }

    fn get_bounds(_this: &Self) -> Rect {
        panic!("Drawing is abstract: 'get_bounds' must be implemented by the deriving class")
    }
}

impl Drawing {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            invalidated: HandlerList::new(),
            value_subscriptions: RefCell::new(Vec::new()),
        }
    }

    /// Draws this drawing to the given drawing context.
    pub fn draw(&self, context: &mut DrawingContext<'_>) {
        self.draw_core(context)
    }

    /// Raised when the drawing changed in a way that requires its content to
    /// be recorded again.
    pub(crate) fn invalidated(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.invalidated.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.invalidated.remove(token);
            }
        })
    }

    pub(crate) fn raise_invalidated(&self) {
        if self.invalidated.is_empty() {
            return;
        }
        for (_, handler) in self.invalidated.snapshot().iter() {
            handler();
        }
    }

    /// Subscribes to (`Some`) or unsubscribes from (`None`) the changes of
    /// the value of `property`.
    fn update_value_subscription(&self, property: &'static FerroProperty, value: Option<&dyn Any>) {
        let Some(value) = value else {
            let position = self.value_subscriptions.borrow().iter().position(|(p, _)| *p == property);
            if let Some(position) = position {
                let (_, subscription) = self.value_subscriptions.borrow_mut().swap_remove(position);
                subscription.dispose();
            }
            return;
        };

        let weak = self.to_ref().downgrade();
        let value_invalidated = move || {
            if let Some(this) = weak.upgrade() {
                this.raise_invalidated();
            }
        };

        let subscription = if let Some(value) = value.downcast_ref::<Option<Ref<Transform>>>() {
            // A transform's matrix is baked into the recorded commands, so
            // its value changes (not just its replacement) require a
            // re-record.
            value.as_ref().map(|transform| transform.changed(value_invalidated))
        } else if let Some(value) = value.downcast_ref::<Option<Rc<dyn IEffect>>>() {
            // Effects are baked as immutable, and nested image sources (e.g.
            // a drawing image) are inlined, so their visual changes require a
            // re-record too.
            value
                .as_ref()
                .and_then(|effect| effect.as_object())
                .and_then(|object| object.downcast_ref::<Effect>())
                .map(|effect| effect.invalidated(value_invalidated))
        } else if let Some(value) = value.downcast_ref::<Option<Rc<dyn IImage>>>() {
            value
                .as_ref()
                .and_then(|image| image.as_affects_render())
                .map(|affects_render| affects_render.invalidated(Rc::new(value_invalidated)))
        } else {
            None
        };

        if let Some(subscription) = subscription {
            self.value_subscriptions.borrow_mut().push((property, subscription));
        }
    }
}
