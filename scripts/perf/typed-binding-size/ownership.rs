// Observable source ownership and callback ordering across a reentrant retarget.
use ferroui_base::data::core::{TypedBindingExpression, TypedClrPropertyInfo};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::data::{BindingMode, CompiledBinding, CompiledBindingPathBuilder};
use ferroui_base::{FerroProperty, StyledElement};
use std::{cell::{Cell, RefCell}, rc::Rc};

struct Model {
    id: i32,
    value: Cell<i32>,
    changed: Event<str>,
}

impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool { std::ptr::eq(self, other) }
}

impl INotifyPropertyChanged for Model {
    fn property_changed(&self) -> &Event<str> { &self.changed }
}

fn main() {
    let make_model = |id| Rc::new(Model { id, value: Cell::new(id), changed: Event::new() });
    let first = make_model(1);
    let second = make_model(2);
    let target = StyledElement::new();
    let property = FerroProperty::register::<StyledElement, i32>("Counter", 0);
    target.set_data_context(Some(first.clone()));
    let armed = Rc::new(Cell::new(false));
    let trace = Rc::new(RefCell::new(Vec::new()));
    let getter = {
        let armed = armed.clone();
        let trace = trace.clone();
        let weak_target = target.to_weak();
        let next = Rc::downgrade(&second);
        move |owner: &Rc<Model>| {
            trace.borrow_mut().push(("get", owner.id, Rc::strong_count(owner)));
            if armed.replace(false) {
                weak_target.upgrade().unwrap().downcast_ref::<StyledElement>().unwrap().set_data_context(Some(next.upgrade().unwrap()));
                trace.borrow_mut().push(("resumed", owner.id, Rc::strong_count(owner)));
            }
            Ok(owner.value.get())
        }
    };
    let setter = {
        let trace = trace.clone();
        move |owner: &Rc<Model>, value| {
            trace.borrow_mut().push(("set", owner.id, Rc::strong_count(owner)));
            owner.value.set(value);
            owner.changed.raise("Counter");
        }
    };
    let info = TypedClrPropertyInfo::from_handle_accessors("Counter", Some(Rc::new(getter)), Some(Rc::new(setter)));
    let path = CompiledBindingPathBuilder::new().typed_property_info(info).build();
    let binding = CompiledBinding::new(path).with_mode(BindingMode::TwoWay);
    let expression = target.bind_binding(property, &binding);
    assert!(expression.as_any().is::<TypedBindingExpression<Model, i32>>());
    assert_eq!(target.get_value(property), 1);
    armed.set(true);
    first.changed.raise("Counter");
    let after_reentry = target.get_value(property);
    second.value.set(7);
    second.changed.raise("Counter");
    assert_eq!(target.get_value(property), 7);
    target.set_value(property, 11);
    assert_eq!(second.value.get(), 11);
    first.changed.raise("Counter");
    let before_dispose = trace.borrow().len();
    expression.dispose();
    first.changed.raise("Counter");
    second.changed.raise("Counter");
    assert_eq!(trace.borrow().len(), before_dispose);
    println!("{:?} {}", trace.borrow(), after_reentry);
}
