//! `ElementFactoryBenchmarks.cs`: taking an element out of the recycle pool of an element factory
//! for a parent and returning it, with 10, 100 and 1 000 pooled elements, each recycled from a
//! parent of its own.
//!
//! The reference's factory needs no application; the port starts the unit test application of
//! the framework, which the controls (the stack panels and borders) are created in.

use ferroui_base::Ref;
use ferroui_benchmarks::harness::Registry;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, Control, StackPanel};
use ferroui_controls_treedatagrid::primitives::{RecycleKey, TreeDataGridElementFactory, TreeDataGridElementFactoryImpl};
use std::any::Any;
use std::rc::Rc;

const OPERATION_COUNT: u32 = 500_000;

/// `BenchmarkElementFactory`: creates borders, all with the recycle key `"element"`.
struct BenchmarkElementFactory;

impl TreeDataGridElementFactoryImpl for BenchmarkElementFactory {
    fn create_element(&self, _factory: &TreeDataGridElementFactory, _data: &dyn Any) -> Ref<Control> {
        Border::new().upcast()
    }

    fn get_data_recycle_key(&self, _factory: &TreeDataGridElementFactory, _data: &dyn Any) -> RecycleKey {
        "element".into()
    }

    fn get_element_recycle_key(&self, _factory: &TreeDataGridElementFactory, _element: &Control) -> RecycleKey {
        "element".into()
    }
}

pub struct ElementFactoryBenchmarks {
    pool_size: usize,
    /// `new object()`: the data every element is requested for.
    data: Rc<()>,
    /// Set by the iteration setup.
    factory: Option<Rc<TreeDataGridElementFactory>>,
    parents: Vec<Ref<StackPanel>>,
    /// Disposed when the benchmark is dropped, after the controls (the fields drop in the order
    /// of their declaration).
    _app: UnitTestApplicationScope,
}

impl ElementFactoryBenchmarks {
    pub fn new(pool_size: usize) -> Self {
        let app = UnitTestApplication::start(TestServices::styled_window());
        Self { pool_size, data: Rc::new(()), factory: None, parents: Vec::new(), _app: app }
    }

    /// `[IterationSetup] Setup`: every element of the pool is created for a parent of its own,
    /// added to it and recycled.
    pub fn setup(&mut self) {
        let factory = TreeDataGridElementFactory::with_impl(BenchmarkElementFactory);
        let mut parents = Vec::with_capacity(self.pool_size);
        let mut elements = Vec::with_capacity(self.pool_size);

        for _ in 0..self.pool_size {
            let parent = StackPanel::new();
            let element = factory.get_or_create_element(&self.data, &parent);
            parent.children().add(&element);
            parents.push(parent);
            elements.push(element);
        }

        for element in &elements {
            factory.recycle_element(element);
        }

        self.factory = Some(factory);
        self.parents = parents;
    }

    /// `[Benchmark(OperationsPerInvoke = OperationCount)] SameParentReuse`.
    pub fn same_parent_reuse(&self) {
        let factory = self.factory.as_ref().expect("the iteration setup ran");
        let parents = &self.parents;

        for i in 0..OPERATION_COUNT as usize {
            // The multiplier walks every pool position without a sequential access bias.
            let parent = &parents[(i * 397) % parents.len()];
            let element = factory.get_or_create_element(&self.data, parent);
            factory.recycle_element(&element);
        }
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("treedatagrid", "ElementFactoryBenchmarks");
    for pool_size in [10, 100, 1_000] {
        class
            .benchmark_with_iteration_setup(
                "same_parent_reuse",
                format!("PoolSize={pool_size}"),
                move || ElementFactoryBenchmarks::new(pool_size),
                |b| b.setup(),
                |b| b.same_parent_reuse(),
            )
            .operations_per_invoke(OPERATION_COUNT);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn element_factory_benchmarks() {
        ferroui_benchmarks::harness::smoke_class(super::register, "ElementFactoryBenchmarks");
    }
}
