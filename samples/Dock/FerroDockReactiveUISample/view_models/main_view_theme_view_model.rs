//! Original MainView theme closures expressed as framework-independent commands.
use crate::services::IThemeSelection;
use ferroui_reactiveui::{reactive_object, ICommand, ReactiveCommand, ReactiveObject};
use std::{cell::Cell, rc::Rc};
/// State and commands moved from the original MainView to keep the view passive.
pub struct MainViewThemeViewModel {
    base: ReactiveObject,
    toggle_theme: Rc<dyn ICommand>,
    select_preset: Rc<dyn ICommand>,
    select_tab_theme: Rc<dyn ICommand>,
    service: Rc<dyn IThemeSelection>,
}
reactive_object!(MainViewThemeViewModel, base);
impl MainViewThemeViewModel {
    /// Captures the same original manager and initial requested-dark state.
    pub fn new(service: Rc<dyn IThemeSelection>, initial_dark: bool) -> Rc<Self> {
        let dark = Cell::new(initial_dark);
        let variant_service = service.clone();
        let preset_service = service.clone();
        let tab_service = service.clone();
        Rc::new(Self {
            base: ReactiveObject::new(),
            toggle_theme: Rc::new(ReactiveCommand::create(move || {
                dark.set(!dark.get());
                variant_service.select_variant(if dark.get() { 1 } else { 0 });
            })),
            select_preset: Rc::new(ReactiveCommand::create_param(move |index: i32| {
                if index >= 0 {
                    preset_service.select_preset(index);
                }
            })),
            select_tab_theme: Rc::new(ReactiveCommand::create_param(move |index: i32| {
                if matches!(index, 0 | 1) {
                    tab_service.select_tab_theme(index);
                }
            })),
            service,
        })
    }
    /// Gets the original dark/light toggle as a ReactiveCommand.
    pub fn toggle_theme(&self) -> Rc<dyn ICommand> {
        self.toggle_theme.clone()
    }
    /// Gets the original non-negative preset action as a ReactiveCommand.
    pub fn select_preset(&self) -> Rc<dyn ICommand> {
        self.select_preset.clone()
    }
    /// Gets the current document tab style from the application service.
    pub fn selected_tab_theme(&self) -> i32 {
        self.service.selected_tab_theme()
    }
    /// Gets the command selecting Fluent (0) or Browser (1) document tabs.
    pub fn select_tab_theme(&self) -> Rc<dyn ICommand> {
        self.select_tab_theme.clone()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    #[derive(Default)]
    struct Service {
        variants: RefCell<Vec<i32>>,
        presets: RefCell<Vec<i32>>,
        tabs: RefCell<Vec<i32>>,
    }
    impl IThemeSelection for Service {
        fn select_variant(&self, index: i32) {
            self.variants.borrow_mut().push(index)
        }
        fn select_preset(&self, index: i32) {
            self.presets.borrow_mut().push(index)
        }
        fn selected_tab_theme(&self) -> i32 {
            self.tabs.borrow().last().copied().unwrap_or(0)
        }
        fn select_tab_theme(&self, index: i32) {
            self.tabs.borrow_mut().push(index)
        }
    }
    #[test]
    fn original_captured_toggle_state_and_preset_guard_are_commands_without_ui_types() {
        for (initial, expected) in [(false, [1, 0, 1]), (true, [0, 1, 0])] {
            let service = Rc::new(Service::default());
            let model = MainViewThemeViewModel::new(service.clone(), initial);
            for _ in 0..3 {
                model.toggle_theme().execute(None);
            }
            assert_eq!(&*service.variants.borrow(), &expected);
            for index in [-1, 0, 4, 5, -2] {
                model.select_preset().execute(Some(&index));
            }
            assert_eq!(&*service.presets.borrow(), &[0, 4, 5]);
            assert_eq!(model.selected_tab_theme(), 0);
            for index in [-1, 1, 1, 2, 0] {
                model.select_tab_theme().execute(Some(&index));
            }
            assert_eq!(&*service.tabs.borrow(), &[1, 1, 0]);
            assert_eq!(model.selected_tab_theme(), 0);
            let weak = Rc::downgrade(&model);
            drop(model);
            assert!(weak.upgrade().is_none(),"the command closure holds its actual source state, without an invented owner cycle");
        }
    }
}
