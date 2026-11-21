// Copyright (c) 2026 Wiesław Šoltés. MIT. Presentation behavior from SettingsPage.xaml.cs.
use ferroui_reactiveui::{
    ICommand, IReactiveObjectExt, ReactiveCommand, ReactiveObject, reactive_object,
};
use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsSelection {
    Theme(i32),
    Palette(i32),
    Chrome(i32),
    Surface(i32),
    Layout {
        density_index: i32,
        layout_index: i32,
    },
    Language(Option<String>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsWordAction {
    ShowState,
    ResetCustomization,
    Customize,
    CommandPalette,
}
/// Native/theme/persistence adapters live at the composition boundary, not in the view model.
pub trait SettingsServices {
    fn select(&self, selection: SettingsSelection) -> Result<(), String>;
    fn word_action(&self, action: SettingsWordAction) -> Result<Option<String>, String>;
    fn log(&self) -> Vec<String>;
}
pub struct SettingsViewModel {
    reactive: ReactiveObject,
    service: Rc<dyn SettingsServices>,
    state_json: RefCell<String>,
    log_text: RefCell<Option<String>>,
    selection: OnceCell<Rc<dyn ICommand>>,
    refresh: OnceCell<Rc<dyn ICommand>>,
    words: OnceCell<[Rc<dyn ICommand>; 4]>,
}
reactive_object!(SettingsViewModel);
impl PartialEq for SettingsViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl SettingsViewModel {
    pub fn new(service: Rc<dyn SettingsServices>) -> Rc<Self> {
        let vm = Rc::new(Self {
            reactive: ReactiveObject::new(),
            service,
            state_json: RefCell::new(String::new()),
            log_text: RefCell::new(None),
            selection: OnceCell::new(),
            refresh: OnceCell::new(),
            words: OnceCell::new(),
        });
        let weak = Rc::downgrade(&vm);
        assert!(
            vm.selection
                .set(Rc::new(ReactiveCommand::create_param(
                    move |selection: Option<SettingsSelection>| {
                        let selection = selection.expect("Settings selection requires a parameter");
                        if let Some(vm) = weak.upgrade() {
                            vm.apply_selection(selection)
                                .expect("Settings selection failed");
                        }
                    }
                )))
                .is_ok()
        );
        let weak = Rc::downgrade(&vm);
        assert!(
            vm.refresh
                .set(Rc::new(ReactiveCommand::create(move || {
                    if let Some(vm) = weak.upgrade() {
                        vm.refresh_log();
                    }
                })))
                .is_ok()
        );
        let actions = [
            SettingsWordAction::ShowState,
            SettingsWordAction::ResetCustomization,
            SettingsWordAction::Customize,
            SettingsWordAction::CommandPalette,
        ];
        let commands = actions.map(|action| {
            let weak = Rc::downgrade(&vm);
            Rc::new(ReactiveCommand::create(move || {
                if let Some(vm) = weak.upgrade() {
                    vm.apply_word_action(action)
                        .expect("Settings Word action failed");
                }
            })) as Rc<dyn ICommand>
        });
        assert!(vm.words.set(commands).is_ok());
        vm
    }
    pub fn state_json(&self) -> String {
        self.state_json.borrow().clone()
    }
    pub fn set_state_json(&self, value: String) {
        self.raise_and_set_if_changed(&self.state_json, value, "StateJson");
    }
    pub fn log_text(&self) -> Option<String> {
        self.log_text.borrow().clone()
    }
    pub fn apply_selection(&self, selection: SettingsSelection) -> Result<(), String> {
        self.service.select(selection)
    }
    pub fn apply_word_action(&self, action: SettingsWordAction) -> Result<(), String> {
        if let Some(text) = self.service.word_action(action)? {
            self.set_state_json(text);
        }
        Ok(())
    }
    pub fn refresh_log(&self) {
        let separator = if cfg!(windows) { "\r\n" } else { "\n" };
        let text = self
            .service
            .log()
            .into_iter()
            .take(40)
            .collect::<Vec<_>>()
            .join(separator);
        self.raise_and_set_if_changed(&self.log_text, Some(text), "LogText");
    }
    pub fn selection_action(&self) -> Rc<dyn ICommand> {
        self.selection.get().unwrap().clone()
    }
    pub fn refresh_action(&self) -> Rc<dyn ICommand> {
        self.refresh.get().unwrap().clone()
    }
    pub fn word_action(&self, action: SettingsWordAction) -> Rc<dyn ICommand> {
        let index = match action {
            SettingsWordAction::ShowState => 0,
            SettingsWordAction::ResetCustomization => 1,
            SettingsWordAction::Customize => 2,
            SettingsWordAction::CommandPalette => 3,
        };
        self.words.get().unwrap()[index].clone()
    }
}
ferroui_base::ferro_markup_type!(class SettingsViewModel{
namespace:"FerroRibbon.Sample",this:Rc<SettingsViewModel>,handles:[Rc<SettingsViewModel>,Option<Rc<SettingsViewModel>>],notify_property_changed:SettingsViewModel,
properties:[
StateJson:String{get:|vm:&Rc<SettingsViewModel>|vm.state_json(),set:|vm:&Rc<SettingsViewModel>,text|vm.set_state_json(text)},
LogText:Option<String>{get:|vm:&Rc<SettingsViewModel>|vm.log_text()},
ShowStateAction:Rc<dyn ferroui_base::input::ICommand>{get:|vm:&Rc<SettingsViewModel>|crate::command_adapter::adapt(vm.word_action(SettingsWordAction::ShowState))},
ResetCustomizationAction:Rc<dyn ferroui_base::input::ICommand>{get:|vm:&Rc<SettingsViewModel>|crate::command_adapter::adapt(vm.word_action(SettingsWordAction::ResetCustomization))},
CustomizeAction:Rc<dyn ferroui_base::input::ICommand>{get:|vm:&Rc<SettingsViewModel>|crate::command_adapter::adapt(vm.word_action(SettingsWordAction::Customize))},
Palette2Action:Rc<dyn ferroui_base::input::ICommand>{get:|vm:&Rc<SettingsViewModel>|crate::command_adapter::adapt(vm.word_action(SettingsWordAction::CommandPalette))}
]});
