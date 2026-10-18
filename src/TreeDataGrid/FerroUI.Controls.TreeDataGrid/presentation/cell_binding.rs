//! `Presentation/CellBinding.cs`: the binding of a cell of a neutral core column to the value of
//! its model.
//!
//! The reference builds a typed binding from the column's getter expression (its compiled links,
//! or the model alone for a delegate column) and its setter. The port's presentation holds models
//! as untyped values, so the binding reads and writes through the core's erased access to the
//! column ([`IUntypedValueColumn`]) and follows the model's `PropertyChanged` for the column's
//! member name, or the objects of the value's path (`IUntypedValueColumn::observe`).

use crate::models::{ICellBinding, IRetargetableCellBinding};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::utilities::HandlerList;
use ferroui_base::BoxedValue;
use ferroui_treedatagrid_core::accessor::as_notifier;
use ferroui_treedatagrid_core::models::{IColumn as CoreColumn, IUntypedValueColumn, UntypedSubscription};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

enum Listening {
    // The model's `PropertyChanged`.
    Model { model: BoxedValue, token: u64 },
    // The objects of the value's path.
    Path(#[allow(dead_code)] UntypedSubscription),
}

/// The conversions of a binding between the untyped value of the column and the value of the cell.
pub struct CellValueConversion<V> {
    /// The cell value of the column value (`Read`).
    pub to_cell: fn(Option<BoxedValue>) -> V,
    /// The column value of a cell value (`Write`).
    pub to_column: fn(&V) -> Option<BoxedValue>,
}

impl<V> Clone for CellValueConversion<V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<V> Copy for CellValueConversion<V> {}

/// The conversions of a cell that shows the untyped value as it is (a text cell).
pub fn untyped_conversion() -> CellValueConversion<Option<BoxedValue>> {
    CellValueConversion { to_cell: |value| value, to_column: |value| value.clone() }
}

/// The conversions of a check box cell (`bool?`).
pub fn check_box_conversion() -> CellValueConversion<Option<bool>> {
    CellValueConversion {
        to_cell: |value| {
            let value = value?;
            if let Some(value) = value.downcast_ref::<bool>() {
                return Some(*value);
            }
            if let Some(value) = value.downcast_ref::<Option<bool>>() {
                return *value;
            }
            ValueTypes::try_convert(Some(&value), ValueType::of::<bool>())
                .flatten()
                .and_then(|b| b.downcast_ref::<bool>().copied())
        },
        // The core's check box columns are value columns of `bool?`: the value is written as an
        // `Option<bool>`, so that a three-state cell can write null.
        to_column: |value| Some(Rc::new(*value) as BoxedValue),
    }
}

/// The binding of a cell to the value a neutral column reads from a model (`CellBinding.Create`
/// instantiated for a cell).
pub struct CoreCellBinding<V: Clone + 'static> {
    weak_self: Weak<Self>,
    column: Rc<dyn CoreColumn>,
    conversion: CellValueConversion<V>,
    root: RefCell<Option<BoxedValue>>,
    observers: HandlerList<dyn Fn(&V)>,
    listening: RefCell<Option<Listening>>,
    publish_count: Cell<u32>,
}

impl<V: Clone + 'static> CoreCellBinding<V> {
    /// A binding of the value `column` reads from `root`.
    ///
    /// # Panics
    /// When `column` is not a value column (`untyped_value`).
    pub fn new(column: Rc<dyn CoreColumn>, conversion: CellValueConversion<V>, root: Option<BoxedValue>) -> Rc<Self> {
        assert!(column.untyped_value().is_some(), "The column is not a value column.");
        Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            column,
            conversion,
            root: RefCell::new(root),
            observers: HandlerList::new(),
            listening: RefCell::new(None),
            publish_count: Cell::new(0),
        })
    }

    fn access(&self) -> &dyn IUntypedValueColumn {
        self.column.untyped_value().expect("a value column")
    }

    /// The model the binding reads.
    pub fn root(&self) -> Option<BoxedValue> {
        self.root.borrow().clone()
    }

    /// The current value, if the binding has a root.
    pub fn value(&self) -> Option<V> {
        let root = self.root.borrow().clone()?;
        Some((self.conversion.to_cell)(self.access().get_value(&root)))
    }

    fn is_initialized(&self) -> bool {
        !self.observers.is_empty()
    }

    fn matches(&self, name: &str) -> bool {
        match self.access().property_name() {
            Some(property) => name.is_empty() || name == property,
            None => true,
        }
    }

    fn publish_value(&self) {
        let Some(value) = self.value() else {
            return;
        };
        self.publish_count.set(self.publish_count.get().wrapping_add(1));
        for (_, observer) in self.observers.snapshot().iter() {
            observer(&value);
        }
    }

    fn start_listening(&self) {
        if self.listening.borrow().is_some() {
            return;
        }
        let Some(root) = self.root.borrow().clone() else {
            return;
        };
        let weak = self.weak_self.clone();
        let changed: Rc<dyn Fn()> = Rc::new(move || {
            if let Some(this) = weak.upgrade() {
                this.publish_value();
            }
        });
        let listening = match self.access().observe(&root, changed) {
            Some(subscription) => Some(Listening::Path(subscription)),
            None => as_notifier(&*root).map(|notifier| {
                let weak = self.weak_self.clone();
                let token = notifier.property_changed().add(Rc::new(move |name: &str| {
                    if let Some(this) = weak.upgrade() {
                        if this.matches(name) {
                            this.publish_value();
                        }
                    }
                }));
                Listening::Model { model: root.clone(), token }
            }),
        };
        *self.listening.borrow_mut() = listening;
    }

    fn stop_listening(&self) {
        let listening = self.listening.borrow_mut().take();
        if let Some(Listening::Model { model, token }) = listening {
            if let Some(notifier) = as_notifier(&*model) {
                notifier.property_changed().remove(token);
            }
        }
        // A path subscription ends when it is dropped.
    }

    fn root_changed(&self, root: Option<BoxedValue>) {
        self.stop_listening();
        *self.root.borrow_mut() = root;
        if self.is_initialized() {
            self.start_listening();
        }
        self.publish_value();
    }

    // Writes `value`, converted to the type of the value the column holds when the column does not
    // take it as it is (`Convert.ChangeType` of the reference's text cells).
    fn write(&self, root: &BoxedValue, value: Option<BoxedValue>) {
        let access = self.access();
        if access.set_value(root, value.as_ref()) {
            return;
        }
        let Some(value) = value else {
            return;
        };
        let Some(current) = access.get_value(root) else {
            return;
        };
        if let Some(Some(converted)) = ValueTypes::try_convert(Some(&value), ValueType::of_value(&*current)) {
            access.set_value(root, Some(&converted));
        }
    }
}

impl<V: Clone + 'static> ICellBinding<V> for CoreCellBinding<V> {
    fn subscribe(&self, observer: Rc<dyn Fn(&V)>) -> u64 {
        let first = self.observers.is_empty();
        let token = self.observers.add(observer.clone());
        if first {
            self.start_listening();
        }
        if let Some(value) = self.value() {
            observer(&value);
        }
        token
    }

    fn unsubscribe(&self, token: u64) {
        self.observers.remove(token);
        if self.observers.is_empty() {
            self.stop_listening();
        }
    }

    fn on_next(&self, value: V) {
        let Some(root) = self.root.borrow().clone() else {
            return;
        };
        if !self.access().can_set() {
            return;
        }
        let count = self.publish_count.get();
        if self.is_initialized() {
            self.write(&root, (self.conversion.to_column)(&value));
        }
        // A model that does not notify the change still gets its value published, so that the
        // observers see what the model holds.
        if self.publish_count.get() == count {
            self.publish_value();
        }
    }

    fn can_write(&self) -> bool {
        self.access().can_set()
    }

    fn as_retargetable(&self) -> Option<&dyn IRetargetableCellBinding> {
        Some(self)
    }
}

impl<V: Clone + 'static> IRetargetableCellBinding for CoreCellBinding<V> {
    fn try_set_root(&self, root: Option<&BoxedValue>) -> bool {
        self.root_changed(root.cloned());
        true
    }

    fn try_suspend_root(&self) -> bool {
        self.stop_listening();
        *self.root.borrow_mut() = None;
        true
    }
}

impl<V: Clone + 'static> Drop for CoreCellBinding<V> {
    fn drop(&mut self) {
        self.stop_listening();
    }
}
