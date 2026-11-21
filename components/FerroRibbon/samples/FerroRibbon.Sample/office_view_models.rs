// Copyright (c) 2026 Wiesław Šoltés. MIT. Original PowerPoint/Tools application actions.
//! Control-independent ReactiveUI presentation. Native brushes/controls stay in the views/adapters.
use ferroui_reactiveui::{
    ICommand, IReactiveObjectExt, ReactiveCommand, ReactiveObject, reactive_object,
};
use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
};
pub type DemoPageLog = Rc<dyn Fn(&str) -> Result<(), String>>;
pub type ThemeApplication = Rc<dyn Fn() -> Result<(), String>>;
pub type ItemLabel = Rc<dyn Fn() -> Option<String>>;
pub type ParameterText = Rc<dyn Fn() -> Result<String, String>>;
#[derive(Clone)]
pub enum PowerPointAction {
    Preview(Option<ThemeApplication>),
    Pick(Option<ThemeApplication>),
    Invoked {
        item_id: Option<String>,
        parameter: Option<ParameterText>,
    },
}
pub struct PowerPointViewModel {
    reactive: ReactiveObject,
    log: DemoPageLog,
    default_theme: ThemeApplication,
    theme: RefCell<Option<ThemeApplication>>,
    status: RefCell<Option<String>>,
    action: OnceCell<Rc<dyn ICommand>>,
}
reactive_object!(PowerPointViewModel);
impl PartialEq for PowerPointViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl PowerPointViewModel {
    pub fn new(log: DemoPageLog, default_theme: ThemeApplication) -> Rc<Self> {
        let vm = Rc::new(Self {
            reactive: ReactiveObject::new(),
            log,
            default_theme,
            theme: RefCell::new(None),
            status: RefCell::new(Some("Hover the Themes gallery for live preview".into())),
            action: OnceCell::new(),
        });
        let weak = Rc::downgrade(&vm);
        assert!(
            vm.action
                .set(Rc::new(ReactiveCommand::create_param(
                    move |action: Option<PowerPointAction>| {
                        let action = action.expect("PowerPoint action requires a parameter");
                        if let Some(vm) = weak.upgrade() {
                            vm.apply(action).expect("PowerPoint action failed");
                        }
                    }
                )))
                .is_ok()
        );
        vm
    }
    pub fn action(&self) -> Rc<dyn ICommand> {
        self.action.get().unwrap().clone()
    }
    pub fn status(&self) -> Option<String> {
        self.status.borrow().clone()
    }
    pub fn set_status(&self, text: Option<String>) {
        self.raise_and_set_if_changed(&self.status, text, "Status");
    }
    pub fn apply(&self, action: PowerPointAction) -> Result<(), String> {
        match action {
            PowerPointAction::Preview(theme) => {
                let theme = theme
                    .or_else(|| self.theme.borrow().clone())
                    .unwrap_or_else(|| self.default_theme.clone());
                theme()?;
            }
            PowerPointAction::Pick(theme) => {
                self.theme.replace(theme.clone());
                theme.unwrap_or_else(|| self.default_theme.clone())()?;
            }
            PowerPointAction::Invoked { item_id, parameter } => {
                let id = item_id.unwrap_or_default();
                let message = if let Some(parameter) = parameter {
                    format!("{id} ({})", parameter()?)
                } else {
                    id
                };
                self.set_status(Some(message));
                (self.log)(&format!(
                    "PowerPoint · {}",
                    self.status().unwrap_or_default()
                ))?;
            }
        }
        Ok(())
    }
}
#[derive(Clone)]
pub enum ToolsAction {
    Checked {
        context: Option<String>,
        label: ItemLabel,
    },
    Invoked {
        label: Option<String>,
    },
}
pub struct ToolsViewModel {
    reactive: ReactiveObject,
    log: DemoPageLog,
    active_context: RefCell<Option<String>>,
    tool_status: RefCell<Option<String>>,
    canvas_text: RefCell<Option<String>>,
    action: OnceCell<Rc<dyn ICommand>>,
}
reactive_object!(ToolsViewModel);
impl PartialEq for ToolsViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ToolsViewModel {
    pub fn new(log: DemoPageLog) -> Rc<Self> {
        let vm = Rc::new(Self {
            reactive: ReactiveObject::new(),
            log,
            active_context: RefCell::new(Some("brush".into())),
            tool_status: RefCell::new(Some("Brush tool".into())),
            canvas_text: RefCell::new(Some(
                "Pick a tool — the options bar follows the active tool".into(),
            )),
            action: OnceCell::new(),
        });
        let weak = Rc::downgrade(&vm);
        assert!(
            vm.action
                .set(Rc::new(ReactiveCommand::create_param(
                    move |action: Option<ToolsAction>| {
                        let action = action.expect("Tools action requires a parameter");
                        if let Some(vm) = weak.upgrade() {
                            vm.apply(action).expect("Tools action failed");
                        }
                    }
                )))
                .is_ok()
        );
        vm
    }
    pub fn action(&self) -> Rc<dyn ICommand> {
        self.action.get().unwrap().clone()
    }
    pub fn apply(&self, action: ToolsAction) -> Result<(), String> {
        match action {
            ToolsAction::Checked { context, label } => {
                self.set_active_context(context);
                self.set_tool_status(label());
                self.set_canvas_text(Some(format!("{} is active", label().unwrap_or_default())));
            }
            ToolsAction::Invoked { label } => {
                (self.log)(&format!("Tools · {}", label.unwrap_or_default()))?
            }
        }
        Ok(())
    }
    pub fn active_context(&self) -> Option<String> {
        self.active_context.borrow().clone()
    }
    pub fn set_active_context(&self, value: Option<String>) {
        self.raise_and_set_if_changed(&self.active_context, value, "ActiveContext");
    }
    pub fn tool_status(&self) -> Option<String> {
        self.tool_status.borrow().clone()
    }
    pub fn set_tool_status(&self, value: Option<String>) {
        self.raise_and_set_if_changed(&self.tool_status, value, "ToolStatus");
    }
    pub fn canvas_text(&self) -> Option<String> {
        self.canvas_text.borrow().clone()
    }
    pub fn set_canvas_text(&self, value: Option<String>) {
        self.raise_and_set_if_changed(&self.canvas_text, value, "CanvasText");
    }
}
ferroui_base::ferro_markup_type!(class PowerPointViewModel{namespace:"FerroRibbon.Sample",this:Rc<PowerPointViewModel>,handles:[Rc<PowerPointViewModel>,Option<Rc<PowerPointViewModel>>],notify_property_changed:PowerPointViewModel,properties:[Status:Option<String>{get:|vm:&Rc<PowerPointViewModel>|vm.status(),set:|vm:&Rc<PowerPointViewModel>,v|vm.set_status(v)}]});
ferroui_base::ferro_markup_type!(class ToolsViewModel{namespace:"FerroRibbon.Sample",this:Rc<ToolsViewModel>,handles:[Rc<ToolsViewModel>,Option<Rc<ToolsViewModel>>],notify_property_changed:ToolsViewModel,properties:[ActiveContext:Option<String>{get:|vm:&Rc<ToolsViewModel>|vm.active_context(),set:|vm:&Rc<ToolsViewModel>,v|vm.set_active_context(v)},ToolStatus:Option<String>{get:|vm:&Rc<ToolsViewModel>|vm.tool_status(),set:|vm:&Rc<ToolsViewModel>,v|vm.set_tool_status(v)},CanvasText:Option<String>{get:|vm:&Rc<ToolsViewModel>|vm.canvas_text(),set:|vm:&Rc<ToolsViewModel>,v|vm.set_canvas_text(v)}]});
