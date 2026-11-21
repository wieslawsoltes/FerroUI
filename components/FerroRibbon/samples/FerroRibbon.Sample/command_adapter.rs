//! The selected SDK predates ReactiveUI.Controls. Bridge existing two ICommand
//! contracts; no dependency or vendor patch is copied from the Dock sample.
use ferroui_base::{BoxedValue, input::ICommand, reactive::IDisposable};
use ferroui_reactiveui::ICommand as ReactiveCommand;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
struct Adapter(Rc<dyn ReactiveCommand>);
impl ICommand for Adapter {
    fn can_execute(&self, parameter: Option<&BoxedValue>) -> bool {
        let p = ferroribbon::commands::command_parameter(parameter);
        self.0.can_execute(p.as_deref())
    }
    fn execute(&self, parameter: Option<&BoxedValue>) {
        let p = ferroribbon::commands::command_parameter(parameter);
        self.0.execute(p.as_deref());
    }
    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.0.can_execute_changed(handler)
    }
}
type Entry = (Weak<dyn ReactiveCommand>, Weak<dyn ICommand>);
ferroui_base::ferro_thread_local! {static ADAPTERS:RefCell<Vec<Entry>>=RefCell::default();}
pub fn adapt(source: Rc<dyn ReactiveCommand>) -> Rc<dyn ICommand> {
    ADAPTERS.with_borrow_mut(|entries| {
        entries.retain(|(s, n)| s.strong_count() > 0 && n.strong_count() > 0);
        if let Some(native) = entries.iter().find_map(|(s, n)| {
            s.upgrade()
                .filter(|s| Rc::ptr_eq(s, &source))
                .and_then(|_| n.upgrade())
        }) {
            return native;
        }
        let native: Rc<dyn ICommand> = Rc::new(Adapter(source.clone()));
        entries.push((Rc::downgrade(&source), Rc::downgrade(&native)));
        native
    })
}
// UI boundary only: the presentation command consumes the framework-independent Core color.
struct ColorAdapter(Rc<dyn ReactiveCommand>);
fn color_parameter(p: Option<&BoxedValue>) -> Option<ferroribbon_core::theming::RibbonColor> {
    let p = p?;
    if Rc::ptr_eq(
        p,
        &ferroribbon::controls::RibbonColorPicker::no_color_parameter(),
    ) {
        return None;
    }
    Some(ferroribbon::theming::to_ribbon_color(
        *p.downcast_ref::<ferroui_base::media::Color>()
            .expect("FontColor expects the source color or NoColorParameter"),
    ))
}
impl ICommand for ColorAdapter {
    fn can_execute(&self, p: Option<&BoxedValue>) -> bool {
        let c = color_parameter(p);
        self.0.can_execute(Some(&c))
    }
    fn execute(&self, p: Option<&BoxedValue>) {
        let c = color_parameter(p);
        self.0.execute(Some(&c));
    }
    fn can_execute_changed(&self, h: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.0.can_execute_changed(h)
    }
}
ferroui_base::ferro_thread_local! {static COLOR_ADAPTERS:RefCell<Vec<Entry>>=RefCell::default();}
pub fn adapt_color(source: Rc<dyn ReactiveCommand>) -> Rc<dyn ICommand> {
    COLOR_ADAPTERS.with_borrow_mut(|entries| {
        entries.retain(|(s, n)| s.strong_count() > 0 && n.strong_count() > 0);
        if let Some(n) = entries.iter().find_map(|(s, n)| {
            s.upgrade()
                .filter(|s| Rc::ptr_eq(s, &source))
                .and_then(|_| n.upgrade())
        }) {
            return n;
        }
        let n: Rc<dyn ICommand> = Rc::new(ColorAdapter(source.clone()));
        entries.push((Rc::downgrade(&source), Rc::downgrade(&n)));
        n
    })
}
