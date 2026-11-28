// Exercise the two data-context access paths with the same notifying i32 model.
use ferroui_base::data::core::{
    plugins::PropertyInfoAccessorFactory, ClrPropertyInfo, ValueType, ValueTypes,
};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::data::{BindingMode, CompiledBinding, CompiledBindingPathBuilder};
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_markup_type, BoxedValue, FerroProperty, StyledElement};
use std::{cell::Cell, hint::black_box, rc::Rc, time::Instant};

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
struct CountAlloc;
#[global_allocator]
static ALLOCATOR: CountAlloc = CountAlloc;

unsafe impl GlobalAlloc for CountAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

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
    let before_attach = ALLOCATIONS.load(Ordering::Relaxed);
    let expression = target.bind_binding(property, &binding);
    let attach_alloc = ALLOCATIONS.load(Ordering::Relaxed) - before_attach;
    assert!(typed);
    model.write(11);
    assert_eq!(target.get_value(property), 11);
    target.set_value(property, 13);
    assert_eq!(model.number(), 13);
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    let start = Instant::now();
    for i in 0..50_000 {
        model.write(black_box(i));
    }
    let push_alloc = ALLOCATIONS.load(Ordering::Relaxed) - before;
    let push = start.elapsed().as_nanos() / 50_000;
    assert_eq!(target.get_value(property), 49_999);
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    let start = Instant::now();
    for i in 0..50_000 {
        target.set_value(property, black_box(i));
    }
    let pull_alloc = ALLOCATIONS.load(Ordering::Relaxed) - before;
    let pull = start.elapsed().as_nanos() / 50_000;
    assert_eq!(model.number(), 49_999);
    let peer = Rc::new(Model { number: Cell::new(29), changed: Event::new() });
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    let start = Instant::now();
    for i in 0..10_000 {
        let next = if i % 2 == 0 { &peer } else { &model };
        target.set_data_context(Some(next.clone()));
    }
    let retarget_ns = start.elapsed().as_nanos() / 10_000;
    let retarget_alloc = ALLOCATIONS.load(Ordering::Relaxed) - before;
    assert_eq!(target.get_value(property), model.number());
    expression.dispose();
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    let start = Instant::now();
    for _ in 0..10_000 {
        let expression = target.bind_binding(property, &binding);
        black_box(&expression);
        expression.dispose();
    }
    let cycle_ns = start.elapsed().as_nanos() / 10_000;
    let cycle_alloc = ALLOCATIONS.load(Ordering::Relaxed) - before;
    let expression_bytes = std::mem::size_of::<ferroui_base::data::core::TypedBindingExpression<Model, i32>>();
    println!("{typed} {attach_alloc} {push_alloc} {pull_alloc} {push} {pull} {retarget_ns} {retarget_alloc} {cycle_ns} {cycle_alloc} {expression_bytes}");
}
