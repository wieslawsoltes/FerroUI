use ferroui_base::reactive::{Disposable, IDisposable, IObservable, IObserver, LightweightSubject, Observable, ObservableExt};
use ferroui_base::{ObjectType, Ref, StyledElement, TypeInfo, WeakRef};
use std::cell::RefCell;
use std::rc::Rc;

/// Follows the parent chain of an element and publishes the ancestor of the
/// requested class at its end (or `None` when the chain ends without one)
/// whenever it changes.
struct FinderNode {
    control: WeakRef<StyledElement>,
    ancestor_type: &'static TypeInfo,
    subject: LightweightSubject<Option<Ref<StyledElement>>>,
    child: RefCell<Option<Rc<FinderNode>>>,
    disposable: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl FinderNode {
    fn new(control: &Ref<StyledElement>, ancestor_type: &'static TypeInfo) -> Rc<Self> {
        Rc::new(Self {
            control: control.downgrade(),
            ancestor_type,
            subject: LightweightSubject::new(),
            child: RefCell::new(None),
            disposable: RefCell::new(None),
        })
    }

    fn init(self: &Rc<Self>) {
        let Some(control) = self.control.upgrade() else { return };
        let property = StyledElement::parent_property().as_property();
        // The element owns the node through its change handler; the node
        // refers back to the element weakly.
        let this = self.clone();
        let subscription = control.property_changed(move |e| {
            if e.property() == property {
                this.on_value_changed(e.get_new_value::<Option<Ref<StyledElement>>>());
            }
        });
        *self.disposable.borrow_mut() = Some(subscription);
        self.on_value_changed(control.parent());
    }

    fn on_value_changed(&self, next: Option<Ref<StyledElement>>) {
        match next {
            Some(next) if !self.ancestor_type.is_assignable_from(next.get_type()) => {
                let old_child = self.child.borrow_mut().take();
                if let Some(old_child) = old_child {
                    old_child.dispose();
                }

                let child = FinderNode::new(&next, self.ancestor_type);
                *self.child.borrow_mut() = Some(child.clone());
                // The subscription lives as long as the child's subject.
                let _ = child.subject.subscribe(Rc::new(self.subject.clone()));
                child.init();
            }
            next => self.subject.on_next(next),
        }
    }

    fn dispose(&self) {
        let child = self.child.borrow().clone();
        if let Some(child) = child {
            child.dispose();
        }
        self.subject.on_completed();
        let disposable = self.disposable.borrow_mut().take();
        if let Some(disposable) = disposable {
            disposable.dispose();
        }
    }
}

/// Observes the nearest ancestor of a class along the parent chain of an
/// element.
pub struct AncestorFinder;

impl AncestorFinder {
    /// An observable of the nearest ancestor of class `T` of `control`:
    /// `None` while there is none.
    pub fn create<T: ObjectType>(control: &Ref<StyledElement>) -> Rc<dyn IObservable<Option<Ref<T>>>> {
        Self::create_for_type(control, T::TYPE).select(|x| x.and_then(|x| x.cast::<T>()))
    }

    /// An observable of the nearest ancestor of class `ancestor_type` of
    /// `control`: `None` while there is none.
    pub fn create_for_type(
        control: &Ref<StyledElement>,
        ancestor_type: &'static TypeInfo,
    ) -> Rc<dyn IObservable<Option<Ref<StyledElement>>>> {
        let control = control.clone();
        Observable::create(move |observer: Rc<dyn IObserver<Option<Ref<StyledElement>>>>| {
            let finder = FinderNode::new(&control, ancestor_type);
            let subscription = finder.subject.subscribe(observer);
            finder.init();

            Disposable::create(move || {
                subscription.dispose();
                finder.dispose();
            })
        })
    }
}
