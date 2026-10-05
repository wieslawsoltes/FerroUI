//! Classes shared by the styling tests.
//!
//! The control library is a separate crate, so the tests use minimal
//! layoutable classes in place of the controls used by the tests these were
//! ported from.

use crate::layout::{Layoutable, LayoutableImpl};
use crate::logical_tree::{ChildIndexChangedEventArgs, IChildIndexProvider};
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::HandlerList;
use crate::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A logical root.
#[repr(C)]
pub struct TestRoot {
    base: Layoutable,
}

ferro_class!(TestRoot: Layoutable);
ferro_impl_classes!(TestRoot: FerroObjectImpl, VisualImpl, LayoutableImpl);

impl StyledElementImpl for TestRoot {
    fn is_logical_root(_this: &Self) -> bool {
        true
    }
}

impl TestRoot {
    pub fn new() -> Ref<Self> {
        instantiate(Self { base: Layoutable::construct() })
    }

    pub fn with_child<T: ObjectType + Upcast<Layoutable> + Upcast<StyledElement> + Upcast<Visual>>(
        child: &Ref<T>,
    ) -> Ref<Self> {
        let root = Self::new();
        set_child(&root, child);
        root
    }
}

/// Makes `child` a logical and visual child of `parent`.
pub fn set_child<P, T>(parent: &Ref<P>, child: &Ref<T>)
where
    P: ObjectType + Upcast<Visual>,
    T: ObjectType + Upcast<StyledElement> + Upcast<Visual>,
{
    let parent: Ref<Visual> = parent.clone().upcast();
    parent.logical_children().add(child.clone().upcast());
    parent.visual_children().add(child.clone().upcast());
}

/// Removes `child` from the logical and visual children of `parent`.
pub fn remove_child<P, T>(parent: &Ref<P>, child: &Ref<T>)
where
    P: ObjectType + Upcast<Visual>,
    T: ObjectType + Upcast<StyledElement> + Upcast<Visual>,
{
    let parent: Ref<Visual> = parent.clone().upcast();
    parent.logical_children().remove(&child.clone().upcast());
    parent.visual_children().remove(&child.clone().upcast());
}

/// A control with a styled string property, a styled number property and a
/// direct property.
#[repr(C)]
pub struct Class1 {
    base: Layoutable,
    direct: RefCell<String>,
}

ferro_class!(Class1: Layoutable);
ferro_impl_classes!(Class1: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl);

impl Class1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Foo", "foodefault".to_string())
    });

    ferro_property!(pub fn double_property() -> StyledProperty<f64> {
        FerroProperty::register::<Class1, _>("Double", 0.0)
    });

    ferro_property!(pub fn child_property() -> StyledProperty<Option<Ref<Class1>>> {
        FerroProperty::register::<Class1, _>("Child", None)
    });

    ferro_property!(pub fn direct_property() -> DirectProperty<Class1, String> {
        FerroProperty::register_direct::<Class1, _>(
            "Direct",
            |o| o.direct.borrow().clone(),
            Some(|o, v| { o.set_and_raise(Class1::direct_property(), &o.direct, v); }),
            String::new(),
        )
    });

    pub fn construct() -> Self {
        Self { base: Layoutable::construct(), direct: RefCell::new(String::new()) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn foo(&self) -> String {
        self.get_value(Self::foo_property())
    }

    pub fn set_foo(&self, value: &str) {
        self.set_value(Self::foo_property(), value.to_string())
    }

    pub fn double(&self) -> f64 {
        self.get_value(Self::double_property())
    }

    pub fn direct(&self) -> String {
        self.direct.borrow().clone()
    }
}

/// A control derived from [`Class1`].
#[repr(C)]
pub struct Class2 {
    base: Class1,
}

ferro_class!(Class2: Class1);
ferro_impl_classes!(Class2: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl);

impl Class2 {
    pub fn new() -> Ref<Self> {
        instantiate(Self { base: Class1::construct() })
    }
}

/// A control unrelated to [`Class1`], styled as itself.
#[repr(C)]
pub struct Class3 {
    base: Layoutable,
}

ferro_class!(Class3: Layoutable);
ferro_impl_classes!(Class3: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl);

impl Class3 {
    pub fn new() -> Ref<Self> {
        instantiate(Self { base: Layoutable::construct() })
    }
}

/// A control derived from [`Class1`] that is styled as a [`Class1`].
#[repr(C)]
pub struct StyledAsClass1 {
    base: Class1,
}

ferro_class!(StyledAsClass1: Class1);
ferro_impl_classes!(StyledAsClass1: FerroObjectImpl, VisualImpl, LayoutableImpl);

impl StyledElementImpl for StyledAsClass1 {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        Class1::TYPE
    }
}

impl StyledAsClass1 {
    pub fn new() -> Ref<Self> {
        instantiate(Self { base: Class1::construct() })
    }
}

/// A panel: provides the index of its logical children.
#[repr(C)]
pub struct TestPanel {
    base: Layoutable,
    provider: Rc<PanelIndexProvider>,
}

type ChildIndexHandlers = HandlerList<dyn Fn(&ChildIndexChangedEventArgs)>;

pub struct PanelIndexProvider {
    owner: RefCell<Option<WeakRef<TestPanel>>>,
    handlers: Rc<ChildIndexHandlers>,
}

impl IChildIndexProvider for PanelIndexProvider {
    fn get_child_index(&self, child: &StyledElement) -> i32 {
        let Some(owner) = self.owner.borrow().as_ref().and_then(WeakRef::upgrade) else { return -1 };
        let child = child.to_ref();
        match owner.logical_children().snapshot().iter().position(|c| *c == child) {
            Some(index) => index as i32,
            None => -1,
        }
    }

    fn try_get_total_count(&self) -> Option<i32> {
        let owner = self.owner.borrow().as_ref().and_then(WeakRef::upgrade)?;
        Some(owner.logical_children().count() as i32)
    }

    fn child_index_changed(&self, handler: Rc<dyn Fn(&ChildIndexChangedEventArgs)>) -> Rc<dyn IDisposable> {
        let token = self.handlers.add(handler);
        let handlers = Rc::downgrade(&self.handlers);
        Disposable::create(move || {
            if let Some(handlers) = handlers.upgrade() {
                handlers.remove(token);
            }
        })
    }
}

ferro_class!(TestPanel: Layoutable);
ferro_impl_classes!(TestPanel: VisualImpl, LayoutableImpl);

impl FerroObjectImpl for TestPanel {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        *this.provider.owner.borrow_mut() = Some(this.to_ref().downgrade());
    }
}

impl StyledElementImpl for TestPanel {
    fn child_index_provider(this: &Self) -> Option<Rc<dyn IChildIndexProvider>> {
        Some(this.provider.clone())
    }
}

impl TestPanel {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: Layoutable::construct(),
            provider: Rc::new(PanelIndexProvider { owner: RefCell::new(None), handlers: Rc::new(HandlerList::new()) }),
        })
    }

    /// Adds a child and notifies the listeners of the child index provider,
    /// as a panel does when its children change.
    pub fn add_child<T: ObjectType + Upcast<StyledElement> + Upcast<Visual>>(&self, child: &Ref<T>) {
        self.insert_child(self.logical_children().count(), child)
    }

    pub fn insert_child<T: ObjectType + Upcast<StyledElement> + Upcast<Visual>>(&self, index: usize, child: &Ref<T>) {
        self.logical_children().insert(index, child.clone().upcast());
        self.visual_children().insert(index, child.clone().upcast());
        self.raise(&ChildIndexChangedEventArgs::child_indexes_reset());
    }

    #[allow(dead_code)]
    pub fn remove_child<T: ObjectType + Upcast<StyledElement> + Upcast<Visual>>(&self, child: &Ref<T>) {
        self.logical_children().remove(&child.clone().upcast());
        self.visual_children().remove(&child.clone().upcast());
        self.raise(&ChildIndexChangedEventArgs::child_indexes_reset());
    }

    pub fn listener_count(&self) -> usize {
        self.provider.handlers.len()
    }

    fn raise(&self, e: &ChildIndexChangedEventArgs) {
        for (_, handler) in self.provider.handlers.snapshot().iter() {
            handler(e);
        }
    }
}

pub use super::testing::try_attach;

/// Records the values a styled property takes on an object, starting with
/// its current value.
pub fn record_values<T: PropertyValue>(
    target: &FerroObject,
    property: &'static StyledProperty<T>,
) -> Rc<RefCell<Vec<T>>> {
    let values = Rc::new(RefCell::new(vec![target.get_value(property)]));
    let recorded = values.clone();
    let id = property.id();
    target.property_changed(move |e| {
        if e.property().id() == id {
            recorded.borrow_mut().push(e.get_new_value::<T>());
        }
    });
    values
}

/// Boxes a value for use as a setter or resource value.
pub fn boxed<T: PartialEq + 'static>(value: T) -> BoxedValue {
    Rc::new(value)
}
