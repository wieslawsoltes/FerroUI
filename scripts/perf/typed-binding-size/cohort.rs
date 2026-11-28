// Sixteen distinct model types each expose the same four payload types.
use ferroui_base::data::core::{TypedBindingExpression, TypedClrPropertyInfo};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::data::{BindingExpressionBase, BindingMode, BindingPriority};
use ferroui_base::PropertyValue;
use std::{cell::{Cell, RefCell}, hint::black_box, rc::Rc};

struct Model<const N: usize> {
    count: Cell<i32>,
    enabled: Cell<bool>,
    text: RefCell<String>,
    note: RefCell<Option<String>>,
    changed: Event<str>,
}

impl<const N: usize> PartialEq for Model<N> {
    fn eq(&self, other: &Self) -> bool { std::ptr::eq(self, other) }
}

impl<const N: usize> INotifyPropertyChanged for Model<N> {
    fn property_changed(&self) -> &Event<str> { &self.changed }
}

fn keep<const N: usize, T: PropertyValue>(
    name: &'static str, getter: fn(&Model<N>) -> T, setter: fn(&Model<N>, T),
) {
    let info = TypedClrPropertyInfo::new(name, Some(Rc::new(getter)), Some(Rc::new(setter))).notifying();
    let expression: Rc<dyn BindingExpressionBase> =
        TypedBindingExpression::new(Rc::new(info), BindingMode::TwoWay, BindingPriority::LocalValue);
    black_box(expression);
}

fn family<const N: usize>() {
    keep::<N, i32>("Count", |s| s.count.get(), |s, v| { s.count.set(v); s.changed.raise("Count"); });
    keep::<N, bool>("Enabled", |s| s.enabled.get(), |s, v| { s.enabled.set(v); s.changed.raise("Enabled"); });
    keep::<N, String>("Text", |s| s.text.borrow().clone(), |s, v| { s.text.replace(v); s.changed.raise("Text"); });
    keep::<N, Option<String>>("Note", |s| s.note.borrow().clone(), |s, v| { s.note.replace(v); s.changed.raise("Note"); });
}

fn main() {
    macro_rules! families { ($($n:literal),*) => { $(family::<$n>();)* }; }
    families!(0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15);
}
