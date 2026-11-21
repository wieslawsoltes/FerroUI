// Copyright (c) 2026 Wiesław Šoltés. MIT. Application logic from CadPage.xaml.cs.
//! Presentation logic has no native control, popup, window or rendering dependency.
use super::cad_commands::{ALIASES, COMMANDS};
#[path = "cad_page_actions.rs"]
mod page_actions;
use ferroribbon_core::text::{invariant_uppercase, ordinal_equals_ascii};
use ferroui_reactiveui::{
    ICommand, IReactiveObjectExt, ReactiveCommand, ReactiveObject, reactive_object,
};
pub use page_actions::{CadDocumentTab, CadPageAction, CadSceneAction, CadSurface};
use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::Rc,
};

pub trait CadSearchHit {
    fn score(&self) -> f64;
    fn execute(&self);
}
pub trait CadRibbonSession {
    /// True means an IRibbonItem was found, even when invocation is disabled.
    fn invoke_id(&self, id: &str) -> bool;
    fn search(&self, text: &str) -> Option<Box<dyn CadSearchHit>>;
    fn apply(&self, effect: CadEffect);
}
pub trait CadLog {
    fn write(&self, line: &str);
}
#[cfg(test)]
#[derive(Default)]
pub struct CadMemoryLog(pub RefCell<Vec<String>>);
#[cfg(test)]
impl CadLog for CadMemoryLog {
    fn write(&self, line: &str) {
        self.0.borrow_mut().push(line.to_owned());
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum CadEffect {
    TabSelection(CadDocumentTab),
    Contexts(Vec<String>),
    PlaceTextEditor,
    ZoomExtents,
    WorkspaceTabs(Vec<String>),
    SelectTab(String),
    SetLightTheme(bool),
    WorkspaceText(Option<String>),
    Scene(CadSceneAction),
}
#[derive(Clone, Debug, Default)]
pub struct CadInvocation {
    pub item_id: Option<String>,
    pub command_id: Option<String>,
    pub choice: Option<String>,
    pub split: bool,
    pub label: Option<String>,
}
#[derive(Clone, Copy, Debug)]
pub enum CadKey {
    Enter,
    Space,
    Escape,
    Other,
}
#[derive(Clone, Copy, Default)]
pub enum CadContextAction {
    #[default]
    ShowTextEditor,
    ShowHatchCreation,
    CloseTextEditor,
    CloseHatch,
    SetLightTheme(bool),
}

pub struct CadViewModel {
    reactive: ReactiveObject,
    session: Rc<dyn CadRibbonSession>,
    log: Rc<dyn CadLog>,
    history: RefCell<Vec<String>>,
    history_text: RefCell<String>,
    input: RefCell<Option<String>>,
    workspace: RefCell<Option<String>>,
    text_visible: RefCell<bool>,
    run: OnceCell<Rc<dyn ICommand>>,
    invoked: OnceCell<Rc<dyn ICommand>>,
    workspace_action: OnceCell<Rc<dyn ICommand>>,
    contextual_action: OnceCell<Rc<dyn ICommand>>,
    page_action: OnceCell<Rc<dyn ICommand>>,
    selected_file_tab: Cell<CadDocumentTab>,
    selected_layout_tab: Cell<CadDocumentTab>,
}
reactive_object!(CadViewModel);
ferroui_base::ferro_markup_type!(class CadViewModel {
 namespace:"FerroRibbon.Sample.Cad",this:Rc<CadViewModel>,handles:[Rc<CadViewModel>,Option<Rc<CadViewModel>>],notify_property_changed:CadViewModel,
 properties:[HistoryText:String{get:|this:&Rc<CadViewModel>|this.history_text()},CommandInput:Option<String>{get:|this:&Rc<CadViewModel>|this.input(),set:|this:&Rc<CadViewModel>,value|this.set_input(value)},Workspace:Option<String>{get:|this:&Rc<CadViewModel>|this.workspace()},MTextVisible:bool{get:|this:&Rc<CadViewModel>|this.text_visible()}]
});
impl PartialEq for CadViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl CadViewModel {
    pub fn new(session: Rc<dyn CadRibbonSession>, log: Rc<dyn CadLog>) -> Rc<Self> {
        let vm = Rc::new(Self {
            reactive: ReactiveObject::new(),
            selected_file_tab: Cell::new(CadDocumentTab::Drawing),
            selected_layout_tab: Cell::new(CadDocumentTab::Model),
            session,
            log,
            history: RefCell::new(vec![
                "Command: _.OPEN \"Drawing1.dwg\"".into(),
                "Regenerating model.".into(),
                "Command:".into(),
            ]),
            history_text: RefCell::new(String::new()),
            input: RefCell::new(Some(String::new())),
            workspace: RefCell::new(Some("Drafting & Annotation".into())),
            text_visible: RefCell::new(false),
            run: OnceCell::new(),
            invoked: OnceCell::new(),
            workspace_action: OnceCell::new(),
            contextual_action: OnceCell::new(),
            page_action: OnceCell::new(),
        });
        vm.update_history();
        let weak = Rc::downgrade(&vm);
        assert!(
            vm.run
                .set(Rc::new(ReactiveCommand::create_param(
                    move |text: String| {
                        if let Some(vm) = weak.upgrade() {
                            vm.run_command(&text);
                        }
                    }
                )))
                .is_ok()
        );
        let weak = Rc::downgrade(&vm);
        assert!(
            vm.invoked
                .set(Rc::new(ReactiveCommand::create_param(
                    move |item: CadInvocation| {
                        if let Some(vm) = weak.upgrade() {
                            vm.on_item_invoked(&item);
                        }
                    }
                )))
                .is_ok()
        );
        let weak = Rc::downgrade(&vm);
        assert!(
            vm.workspace_action
                .set(Rc::new(ReactiveCommand::create_param(
                    move |workspace: String| {
                        if let Some(vm) = weak.upgrade() {
                            vm.apply_workspace(Some(workspace));
                        }
                    }
                )))
                .is_ok()
        );
        let weak = Rc::downgrade(&vm);
        assert!(
            vm.contextual_action
                .set(Rc::new(ReactiveCommand::create_param(
                    move |action: CadContextAction| {
                        if let Some(vm) = weak.upgrade() {
                            match action {
                                CadContextAction::ShowTextEditor => vm.show_text_editor(),
                                CadContextAction::ShowHatchCreation => vm.show_hatch_creation(),
                                CadContextAction::CloseTextEditor => {
                                    vm.close_contextual("Text Editor")
                                }
                                CadContextAction::SetLightTheme(light) => {
                                    vm.session.apply(CadEffect::SetLightTheme(light))
                                }
                                CadContextAction::CloseHatch => {
                                    vm.close_contextual("Hatch Creation")
                                }
                            }
                        }
                    }
                )))
                .is_ok()
        );
        let weak = Rc::downgrade(&vm);
        assert!(
            vm.page_action
                .set(Rc::new(ReactiveCommand::create_param(
                    move |action: CadPageAction| {
                        if let Some(vm) = weak.upgrade() {
                            vm.on_page_action(action);
                        }
                    }
                )))
                .is_ok()
        );
        vm
    }
    pub fn run_action(&self) -> Rc<dyn ICommand> {
        self.run.get().unwrap().clone()
    }
    pub fn page_action(&self) -> Rc<dyn ICommand> {
        self.page_action.get().unwrap().clone()
    }
    pub fn invoked_action(&self) -> Rc<dyn ICommand> {
        self.invoked.get().unwrap().clone()
    }
    pub fn workspace_action(&self) -> Rc<dyn ICommand> {
        self.workspace_action.get().unwrap().clone()
    }
    pub fn contextual_action(&self) -> Rc<dyn ICommand> {
        self.contextual_action.get().unwrap().clone()
    }
    pub fn history(&self) -> Vec<String> {
        self.history.borrow().clone()
    }
    pub fn history_text(&self) -> String {
        self.history_text.borrow().clone()
    }
    pub fn input(&self) -> Option<String> {
        self.input.borrow().clone()
    }
    pub fn set_input(&self, value: Option<String>) {
        self.raise_and_set_if_changed(&self.input, value, "CommandInput");
    }
    pub fn workspace(&self) -> Option<String> {
        self.workspace.borrow().clone()
    }
    /// Synchronize the text property only. Source workspace filtering occurs on
    /// commit or ApplyWorkspace, not on arbitrary programmatic Text changes.
    pub(crate) fn set_workspace_text(&self, value: Option<String>) {
        self.raise_and_set_if_changed(&self.workspace, value, "Workspace");
    }
    pub fn text_visible(&self) -> bool {
        *self.text_visible.borrow()
    }
    fn set_text_visible(&self, value: bool) {
        self.raise_and_set_if_changed(&self.text_visible, value, "MTextVisible");
    }
    fn update_history(&self) {
        let separator = if cfg!(windows) { "\r\n" } else { "\n" };
        let text = {
            let history = self.history.borrow();
            let start = history.len().saturating_sub(3);
            history[start..].join(separator)
        };
        self.raise_and_set_if_changed(&self.history_text, text, "HistoryText");
    }
    pub fn echo(&self, line: &str, as_command: bool) {
        {
            let mut history = self.history.borrow_mut();
            if history.last().is_some_and(|line| line == "Command:") {
                history.pop();
            }
            history.push(if as_command {
                format!("Command: {line}")
            } else {
                line.to_owned()
            });
            while history.len() > 40 {
                history.remove(0);
            }
        }
        self.update_history();
        self.log.write(&format!("CAD · {line}"));
    }
    pub fn run_command(&self, text: &str) {
        let units: Vec<u16> = text.encode_utf16().collect();
        if let Some((_, id)) = ALIASES
            .iter()
            .find(|(alias, _)| ordinal_equals_ascii(&units, alias))
            && self.session.invoke_id(id)
        {
            return;
        }
        if let Some(hit) = self.session.search(text).filter(|hit| hit.score() > 0.) {
            self.echo(&format!("Command: {}", invariant_uppercase(text)), false);
            hit.execute();
            return;
        }
        self.echo(
            &format!(
                "Unknown command \"{}\".  Press F1 for help.",
                invariant_uppercase(text)
            ),
            false,
        );
    }
    pub fn handle_key(&self, key: CadKey) -> bool {
        if matches!(key, CadKey::Escape) {
            self.set_input(Some(String::new()));
            self.echo("*Cancel*", false);
            return true;
        }
        let input = self.input();
        if !matches!(key, CadKey::Enter | CadKey::Space)
            || input.as_ref().is_none_or(|s| s.trim().is_empty())
        {
            return false;
        }
        self.run_command(input.unwrap().trim());
        self.set_input(Some(String::new()));
        true
    }
    pub fn on_item_invoked(&self, item: &CadInvocation) {
        let id = item
            .item_id
            .as_deref()
            .or(item.command_id.as_deref())
            .unwrap_or("");
        let choice = item.choice.as_deref();
        match id {
            "text" | "annotateMText" if choice.is_none_or(|c| c == "Multiline Text") => {
                self.show_text_editor()
            }
            "hatch" if choice.is_none_or(|c| c == "Hatch") => self.show_hatch_creation(),
            "gradient" => self.show_hatch_creation(),
            "nav.zoomExtents" | "zoomExtents" => self.session.apply(CadEffect::ZoomExtents),
            _ => {}
        }
        if id.starts_with("status.") || id.starts_with("nav.") && id != "nav.zoomExtents" {
            return;
        }
        let units: Vec<u16> = id.encode_utf16().collect();
        if let Some((_, command, prompt)) = COMMANDS
            .iter()
            .find(|(key, _, _)| ordinal_equals_ascii(&units, key))
        {
            let name = if let Some(choice) = choice.filter(|_| item.split) {
                format!("{command} ({choice})")
            } else {
                (*command).to_owned()
            };
            self.echo(&format!("Command: _{name}"), false);
            self.echo(prompt, false);
        } else if let Some(label) = item.label.as_deref().filter(|s| !s.is_empty()) {
            self.echo(
                &format!(
                    "Command: {}",
                    invariant_uppercase(&label.replace('\n', " ")).replace(' ', "_")
                ),
                false,
            );
        }
    }
    pub fn show_text_editor(&self) {
        self.session
            .apply(CadEffect::Contexts(vec!["textEditor".into()]));
        self.session.apply(CadEffect::PlaceTextEditor);
        self.set_text_visible(true);
        self.echo("Command: _MTEXT  Current text style:  \"Standard\"  Text height:  2.5  Annotative:  No",false);
    }
    pub fn show_hatch_creation(&self) {
        self.session
            .apply(CadEffect::Contexts(vec!["hatchCreation".into()]));
        self.echo("Command: _HATCH", false);
        self.echo(
            "Pick internal point or [Select objects/Undo/seTtings]:",
            false,
        );
    }
    pub fn close_contextual(&self, name: &str) {
        self.set_text_visible(false);
        self.session.apply(CadEffect::Contexts(vec![]));
        self.session.apply(CadEffect::SelectTab("home".into()));
        self.echo(&format!("{name} closed."), false);
    }
    pub fn apply_workspace(&self, workspace: Option<String>) {
        let tabs = match workspace.as_deref() {
            Some("3D Basics") => vec![
                "home",
                "visualize",
                "insert",
                "view",
                "manage",
                "output",
                "collaborate",
            ],
            Some("3D Modeling") => vec![
                "home",
                "solid",
                "surface",
                "mesh",
                "visualize",
                "parametric",
                "insert",
                "annotate",
                "view",
                "manage",
                "output",
                "collaborate",
            ],
            _ => vec![
                "home",
                "insert",
                "annotate",
                "parametric",
                "view",
                "manage",
                "output",
                "collaborate",
            ],
        };
        self.session.apply(CadEffect::WorkspaceTabs(
            tabs.into_iter().map(str::to_owned).collect(),
        ));
        self.set_workspace_text(workspace.clone());
        self.session
            .apply(CadEffect::WorkspaceText(workspace.clone()));
        self.session.apply(CadEffect::SelectTab("home".into()));
        self.echo(
            &format!("Workspace: {}", workspace.as_deref().unwrap_or("")),
            true,
        );
    }
}
