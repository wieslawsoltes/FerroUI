// Exercise the two data-context access paths with the same notifying i32 model.
use ferroui_base::data::core::{
    plugins::PropertyInfoAccessorFactory, ClrPropertyInfo, ValueType, ValueTypes,
};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::data::{BindingMode, CompiledBinding, CompiledBindingPathBuilder};
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_markup_type, BoxedValue, FerroProperty, StyledElement};
use std::{cell::Cell, hint::black_box, rc::Rc, time::Instant};

struct Model {
    number: Cell<i32>,
    changed: Event<str>,
}

impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for Model {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

impl Model {
    fn number(&self) -> i32 {
        self.number.get()
    }
    fn write(&self, value: i32) {
        if self.number.replace(value) != value {
            self.changed.raise("Number");
        }
    }
}

ferro_markup_type!(class Model {
    this: Rc<Model>,
    handles: [Model, Rc<Model>, Option<Rc<Model>>],
    properties: [Number: i32 { get: Model::number, set: Model::write }],
    notify_property_changed: Model,
});

fn main() {
    let model = Rc::new(Model {
        number: Cell::new(7),
        changed: Event::new(),
    });
    let property = FerroProperty::register::<StyledElement, i32>("BenchNumber", 0);
    let target = StyledElement::new();
    ValueTypes::register_reference::<Model>();
    target.set_data_context(Some(model.clone()));
    let builder = CompiledBindingPathBuilder::new();
    let metadata = <Model as MarkupTyped>::MARKUP
        .find_property("Number")
        .unwrap();
    let path = metadata
        .typed_path_element
        .and_then(|hook| hook(&builder, false));
    let typed = path.is_some();
    let path = path.unwrap_or_else(|| {
        let info = ClrPropertyInfo::new(
            "Number",
            Some(Rc::new(|m| {
                Some(Rc::new(m.downcast_ref::<Model>().unwrap().number()) as BoxedValue)
            })),
            Some(Rc::new(|m, v| {
                m.downcast_ref::<Model>()
                    .unwrap()
                    .write(*v.unwrap().downcast_ref::<i32>().unwrap());
                Ok(())
            })),
            ValueType::of::<i32>(),
        );
        builder.property(
            Rc::new(info),
            PropertyInfoAccessorFactory::create_inpc_property_accessor::<Model>(),
        )
    });
    let binding = CompiledBinding::new(path.build()).with_mode(BindingMode::TwoWay);
    let expression = target.bind_binding(property, &binding);
    model.write(11);
    assert_eq!(target.get_value(property), 11);
    target.set_value(property, 13);
    assert_eq!(model.number(), 13);
    let start = Instant::now();
    for i in 0..50_000 {
        model.write(black_box(i));
    }
    let push = start.elapsed().as_nanos() / 50_000;
    assert_eq!(target.get_value(property), 49_999);
    let start = Instant::now();
    for i in 0..50_000 {
        target.set_value(property, black_box(i));
    }
    let pull = start.elapsed().as_nanos() / 50_000;
    assert_eq!(model.number(), 49_999);
    black_box(expression);
    println!("{typed} {push} {pull}");
}
