//! `TemplateCellRecyclingBenchmarks.cs`: detaching a template cell from its host, unrealizing it,
//! realizing it again and attaching it again, with the same template provider every time (the
//! baseline) and with two template providers in turn.
//!
//! The reference's application is the headless platform without a theme; the port's is the unit
//! test application of the framework with the services of a styled window, whose test theme gives
//! the window its template. The reference fixes 64 invocations an iteration; the harness finds the
//! invocations an iteration with its pilot.

use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, Ref};
use ferroui_benchmarks::harness::Registry;
use ferroui_controls::templates::{FuncDataTemplate, IDataTemplate};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, Control, Decorator, TextBlock, Window};
use ferroui_controls_treedatagrid::models::{CellRef, CellTemplateGetter, TemplateCell};
use ferroui_controls_treedatagrid::primitives::{TreeDataGridElementFactory, TreeDataGridTemplateCell};
use std::rc::Rc;

const OPERATION_COUNT: u32 = 1_000;

/// `new FuncDataTemplate<object>((_, _) => build())`.
fn data_template(build: impl Fn() -> Ref<Control> + 'static) -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::new(|_| true, move |_, _| Some(build()), false)
}

/// `_ => template`: the template provider of a template cell.
fn getter(template: Rc<dyn IDataTemplate>) -> CellTemplateGetter {
    Rc::new(move |_: &Control| template.clone())
}

pub struct TemplateCellRecyclingBenchmarks {
    window: Option<Ref<Window>>,
    host: Ref<Decorator>,
    cell: Ref<TreeDataGridTemplateCell>,
    factory: Rc<TreeDataGridElementFactory>,
    first_model: CellRef,
    second_model: CellRef,
    /// Disposed when the benchmark is dropped, after the controls (the fields drop in the order
    /// of their declaration).
    _app: UnitTestApplicationScope,
}

impl TemplateCellRecyclingBenchmarks {
    /// `[GlobalSetup] GlobalSetup`: a template cell realized for the first model, in a decorator
    /// that is the content of a shown window.
    pub fn new() -> Self {
        let app = UnitTestApplication::start(TestServices::styled_window());

        let first_template = data_template(|| TextBlock::new().upcast());
        let second_template = data_template(|| Border::new().upcast());

        let first_value: BoxedValue = Rc::new(());
        let second_value: BoxedValue = Rc::new(());
        let first_model: CellRef = TemplateCell::new(Some(first_value), getter(first_template), None, None);
        let second_model: CellRef = TemplateCell::new(Some(second_value), getter(second_template), None, None);
        let factory = TreeDataGridElementFactory::new();
        let cell = TreeDataGridTemplateCell::new();
        let host = Decorator::new();
        host.set_child(&cell);
        let window = Window::new();
        window.set_content(Some(Control::boxed(&host)));

        cell.realize(&factory, None, &first_model, 0, 0);
        window.show();

        Self { window: Some(window), host, cell, factory, first_model, second_model, _app: app }
    }

    /// `[Benchmark(Baseline = true, OperationsPerInvoke = OperationCount)] SameTemplateProvider`.
    pub fn same_template_provider(&self) -> Option<Rc<dyn IDataTemplate>> {
        let cell = &self.cell;
        let factory = &self.factory;
        let host = &self.host;
        let model = &self.first_model;

        for _ in 0..OPERATION_COUNT {
            host.set_child(None);
            cell.unrealize();
            cell.realize(factory, None, model, 0, 0);
            host.set_child(cell);
        }

        cell.content_template()
    }

    /// `[Benchmark(OperationsPerInvoke = OperationCount)] AlternatingTemplateProviders`.
    pub fn alternating_template_providers(&self) -> Option<Rc<dyn IDataTemplate>> {
        let cell = &self.cell;
        let factory = &self.factory;
        let host = &self.host;
        let first_model = &self.first_model;
        let second_model = &self.second_model;

        for i in 0..OPERATION_COUNT {
            let use_second_template = (i & 1) == 0;

            host.set_child(None);
            cell.unrealize();
            cell.realize(
                factory,
                None,
                if use_second_template { second_model } else { first_model },
                if use_second_template { 3 } else { 0 },
                0,
            );
            host.set_child(cell);
        }

        cell.content_template()
    }
}

/// `[GlobalCleanup] GlobalCleanup`: the window is closed.
impl Drop for TemplateCellRecyclingBenchmarks {
    fn drop(&mut self) {
        if let Some(window) = self.window.take() {
            window.close();
        }
        Dispatcher::ui_thread().run_jobs(None);
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("treedatagrid", "TemplateCellRecyclingBenchmarks");
    class
        .benchmark("same_template_provider", "", TemplateCellRecyclingBenchmarks::new, |b| b.same_template_provider())
        .baseline()
        .operations_per_invoke(OPERATION_COUNT);
    class
        .benchmark("alternating_template_providers", "", TemplateCellRecyclingBenchmarks::new, |b| {
            b.alternating_template_providers()
        })
        .operations_per_invoke(OPERATION_COUNT);
}

#[cfg(test)]
mod tests {
    #[test]
    fn template_cell_recycling_benchmarks() {
        ferroui_benchmarks::harness::smoke_class(super::register, "TemplateCellRecyclingBenchmarks");
    }
}
