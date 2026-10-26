//! Native solution explorer, retained document tabs and supervised build processes.
use super::*;
use ferroui_controls::{Border, ComboBox};
use ferroui_designer_core::Document;
use ferroui_designer_project::{
    BuildTask, ProjectWorkspace, Solution, TaskHandle, TaskKind, TaskStatus,
};
use std::collections::BTreeMap;

#[derive(Clone)]
pub(super) struct Tab {
    pub(super) document: Document,
    pub(super) saved: String,
    pub(super) file: Option<Rc<dyn ferroui_base::platform::storage::IStorageFile>>,
}
#[derive(Default)]
pub(super) struct ProjectUi {
    pub(super) workspace: Option<ProjectWorkspace>,
    pub(super) tabs: BTreeMap<String, Tab>,
    selected: usize,
    discovery: Option<
        std::sync::mpsc::Receiver<
            Result<(Solution, ferroui_designer_language::LanguageService), String>,
        >,
    >,
    task: Option<TaskHandle>,
}
impl DesignerWorkspace {
    pub fn open_solution(self: &Rc<Self>, path: impl AsRef<std::path::Path>) -> Result<(), String> {
        let path = path.as_ref().to_owned();
        if self.project.borrow().discovery.is_some() {
            return Err("Solution discovery is already running".into());
        }
        let (send, receive) = std::sync::mpsc::channel();
        let mut language = self.source_tools.borrow().service.clone();
        std::thread::spawn(move || {
            let result = Solution::discover(path)
                .map_err(|e| e.to_string())
                .and_then(|solution| {
                    language
                        .index_solution(&solution)
                        .map_err(|e| e.to_string())?;
                    Ok((solution, language))
                });
            let _ = send.send(result);
        });
        self.project.borrow_mut().discovery = Some(receive);
        self.status
            .set_text(Some("Discovering solution and project files…"));
        Ok(())
    }
    /// Attach a host-discovered solution without starting another discovery process.
    pub fn set_solution(self: &Rc<Self>, solution: Solution) -> Result<(), String> {
        self.project.borrow_mut().discovery = None;
        self.source_tools
            .borrow_mut()
            .service
            .index_solution(&solution)
            .map_err(|e| e.to_string())?;
        self.project.borrow_mut().workspace = Some(ProjectWorkspace::new(solution));
        self.project.borrow_mut().selected = 0;
        self.rebuild_project_tree();
        Ok(())
    }
    pub fn select_project(&self, index: usize) -> Result<(), String> {
        let mut state = self.project.borrow_mut();
        if state
            .workspace
            .as_ref()
            .is_none_or(|workspace| index >= workspace.solution.projects.len())
        {
            return Err("Unknown project index".into());
        }
        state.selected = index;
        drop(state);
        self.refresh_run_targets();
        Ok(())
    }
    pub fn solution(&self) -> Option<Solution> {
        self.project
            .borrow()
            .workspace
            .as_ref()
            .map(|w| w.solution.clone())
    }
    pub fn document_uris(&self) -> Vec<String> {
        let mut result = self
            .project
            .borrow()
            .tabs
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let active = self.session.borrow().document_uri().to_owned();
        if !result.contains(&active) {
            result.push(active);
        }
        result
    }
    pub(super) fn capture_tab(&self) {
        let session = self.session.borrow();
        let uri = session.document_uri().to_owned();
        let mut state = self.project.borrow_mut();
        if let Some(workspace) = state.workspace.as_mut() {
            if let Some(buffer) = workspace.buffer(&uri) {
                let version = buffer.version();
                let _ = workspace.store_document(&uri, version, session.document().clone());
            }
        }
        state.tabs.insert(
            uri,
            Tab {
                document: session.document().clone(),
                saved: self.saved_source.borrow().clone(),
                file: self.file.borrow().clone(),
            },
        );
    }
    pub fn activate_document(self: &Rc<Self>, uri: &str) -> Result<(), String> {
        if self.session.borrow().document_uri() == uri {
            return Ok(());
        }
        if self.pending_source.get() {
            self.apply_source();
        }
        self.capture_tab();
        let (document, saved, file) = {
            let mut state = self.project.borrow_mut();
            if !state.tabs.contains_key(uri) {
                let workspace = state
                    .workspace
                    .as_mut()
                    .ok_or("Open a solution before navigating to another document")?;
                let buffer = workspace.open(uri).map_err(|e| e.to_string())?;
                let tab = Tab {
                    document: buffer.document().clone(),
                    saved: buffer.saved_source().unwrap_or("").into(),
                    file: None,
                };
                state.tabs.insert(uri.into(), tab);
            }
            let tab = &state.tabs[uri];
            (tab.document.clone(), tab.saved.clone(), tab.file.clone())
        };
        self.disconnect_sharing();
        self.session
            .borrow_mut()
            .restore_document(document, uri.to_owned())?;
        if let Some(workspace) = self.project.borrow_mut().workspace.as_mut() {
            if workspace.buffer(uri).is_some() {
                workspace.activate(uri).map_err(|e| e.to_string())?;
            }
        }
        *self.saved_source.borrow_mut() = saved;
        *self.file.borrow_mut() = file;
        self.pending_source.set(false);
        self.runtime_selection.set(None);
        self.refresh(true);
        self.rebuild_document_tabs();
        Ok(())
    }
    pub fn close_document(self: &Rc<Self>, uri: &str, discard: bool) -> Result<(), String> {
        self.capture_tab();
        let active = self.session.borrow().document_uri().to_owned();
        let next = {
            let state = self.project.borrow();
            let tab = state.tabs.get(uri).ok_or("Document is not open")?;
            if !discard && tab.document.source() != tab.saved {
                return Err(
                    "The tab has unsaved changes; save it or explicitly discard before closing"
                        .into(),
                );
            }
            state.tabs.keys().find(|key| key.as_str() != uri).cloned()
        };
        if active == uri {
            if let Some(next) = next {
                self.activate_document(&next)?;
            } else {
                return Err("Keep one document open; use Open to replace the final tab".into());
            }
        }
        self.close_language_document(uri)?;
        let mut state = self.project.borrow_mut();
        state.tabs.remove(uri);
        if let Some(workspace) = state.workspace.as_mut() {
            if workspace.buffer(uri).is_some() {
                workspace.close(uri, true).map_err(|e| e.to_string())?;
            }
        }
        drop(state);
        self.rebuild_document_tabs();
        Ok(())
    }
    pub(super) fn inactive_dirty_document(&self) -> Option<String> {
        let active = self.session.borrow().document_uri().to_owned();
        self.project
            .borrow()
            .tabs
            .iter()
            .find(|(uri, tab)| **uri != active && tab.document.source() != tab.saved)
            .map(|(uri, _)| uri.clone())
    }
    pub fn save_all_documents(self: &Rc<Self>) -> Result<(), String> {
        if self.pending_source.get() {
            self.apply_source();
        }
        self.capture_tab();
        let mut state = self.project.borrow_mut();
        // Check every destination before writing the first file.
        let mut files = Vec::new();
        for (uri, tab) in &state.tabs {
            if tab.document.source() == tab.saved {
                continue;
            }
            let path = ferroui_designer_project::file_path(uri)
                .map_err(|_| format!("Save {uri} using Save as before saving all documents"))?;
            if std::fs::read_to_string(&path).map_err(|e| e.to_string())? != tab.saved {
                return Err(format!("{} changed outside the designer", path.display()));
            }
            files.push((uri.clone(), path));
        }
        for (uri, path) in files {
            let tab = state.tabs.get(&uri).unwrap();
            let source = tab.document.source().to_owned();
            super::files::write_local(&path, Some(&tab.saved), &source)
                .map_err(|e| e.to_string())?;
            if let Some(workspace) = state.workspace.as_mut() {
                if let Some(buffer) = workspace.buffer(&uri) {
                    let version = buffer.version();
                    workspace
                        .acknowledge_saved(&uri, version, &source)
                        .map_err(|error| error.to_string())?;
                }
            }
            let tab = state.tabs.get_mut(&uri).unwrap();
            tab.saved = tab.document.source().into();
            tab.document.mark_saved();
            if uri == self.session.borrow().document_uri() {
                *self.saved_source.borrow_mut() = tab.saved.clone();
                self.session.borrow_mut().mark_saved();
            }
        }
        drop(state);
        self.refresh(true);
        self.rebuild_document_tabs();
        Ok(())
    }
    pub(super) fn acknowledge_provider_save(
        &self,
        previous_uri: &str,
        uri: &str,
        source: &str,
    ) -> Result<(), String> {
        let active_uri = self.session.borrow().document_uri().to_owned();
        let document = if self.session.borrow().document_uri() == uri {
            self.session.borrow().document().clone()
        } else {
            self.project
                .borrow()
                .tabs
                .get(uri)
                .ok_or("Saved tab is no longer available")?
                .document
                .clone()
        };
        let mut state = self.project.borrow_mut();
        let Some(workspace) = state.workspace.as_mut() else {
            return Ok(());
        };
        if previous_uri != uri && workspace.buffer(previous_uri).is_some() {
            workspace
                .close(previous_uri, true)
                .map_err(|error| error.to_string())?;
        }
        if workspace.buffer(uri).is_none() && workspace.open(uri).is_err() {
            // Save as may target a file outside the current solution; it remains a native tab.
            return Ok(());
        }
        let version = workspace
            .buffer(uri)
            .ok_or("Saved project buffer is unavailable")?
            .version();
        workspace
            .store_document(uri, version, document)
            .map_err(|error| error.to_string())?;
        let version = workspace.buffer(uri).unwrap().version();
        workspace
            .acknowledge_saved(uri, version, source)
            .map_err(|error| error.to_string())?;
        if workspace.buffer(&active_uri).is_some() {
            workspace
                .activate(&active_uri)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
    pub fn start_project_task(
        self: &Rc<Self>,
        kind: TaskKind,
        release: bool,
    ) -> Result<(), String> {
        let mut state = self.project.borrow_mut();
        if state
            .task
            .as_mut()
            .is_some_and(|task| task.poll() == TaskStatus::Running)
        {
            return Err("A project task is already running".into());
        }
        let project = state
            .workspace
            .as_ref()
            .and_then(|w| w.solution.projects.get(state.selected))
            .ok_or("Select a project in the solution tree")?;
        let task = BuildTask::cargo(project, kind, release).map_err(|e| e.to_string())?;
        drop(state);
        self.save_all_documents()?;
        self.root
            .get_control::<TextBox>("BuildOutput")
            .set_text(Some(&format!(
                "{} {:?}\n",
                task.program(),
                task.arguments()
            )));
        self.root
            .get_control::<TextBlock>("BuildStatus")
            .set_text(Some(&task.label));
        self.project.borrow_mut().task = Some(task.start().map_err(|e| e.to_string())?);
        self.root
            .get_control::<Border>("BuildDrawer")
            .set_is_visible(true);
        Ok(())
    }
    pub fn cancel_project_task(&self) -> Result<(), String> {
        if let Some(task) = self.project.borrow_mut().task.as_mut() {
            task.cancel().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn project_task_status(&self) -> Option<TaskStatus> {
        self.project
            .borrow_mut()
            .task
            .as_mut()
            .map(TaskHandle::poll)
    }
    pub fn project_task_output(&self) -> String {
        self.root
            .get_control::<TextBox>("BuildOutput")
            .text()
            .unwrap_or_default()
    }
    pub fn is_discovering_solution(&self) -> bool {
        self.project.borrow().discovery.is_some()
    }
    pub fn selected_project(&self) -> usize {
        self.project.borrow().selected
    }
    pub fn poll_project_work(self: &Rc<Self>) {
        self.poll_project();
    }
    pub(super) fn connect_project(self: &Rc<Self>) {
        for name in [
            "OpenProject",
            "SaveAll",
            "CheckProject",
            "BuildProject",
            "RunProject",
            "CancelBuild",
            "HideBuildOutput",
            "NativePreview",
        ] {
            let weak = Rc::downgrade(self);
            self.root.get_control::<Button>(name).click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let result = match name {
                        "OpenProject" => this.open_solution(
                            this.root
                                .get_control::<TextBox>("ProjectPath")
                                .text()
                                .unwrap_or_default(),
                        ),
                        "SaveAll" => this.save_all_documents(),
                        "CheckProject" => this.start_project_task(TaskKind::Check, false),
                        "BuildProject" => this.start_project_task(TaskKind::Build, false),
                        "RunProject" => {
                            let target = this
                                .root
                                .get_control::<ComboBox>("RunTarget")
                                .selected_item()
                                .and_then(|v| v.downcast_ref::<String>().cloned());
                            this.start_project_task(TaskKind::Run { target }, false)
                        }
                        "CancelBuild" => this.cancel_project_task(),
                        "NativePreview" => {
                            this.session.borrow_mut().open_native_preview().map(|_| ())
                        }
                        _ => {
                            this.root
                                .get_control::<Border>("BuildDrawer")
                                .set_is_visible(false);
                            Ok(())
                        }
                    };
                    this.report(result);
                }
            });
        }
    }
    pub(super) fn poll_project(self: &Rc<Self>) {
        let discovery =
            self.project
                .borrow()
                .discovery
                .as_ref()
                .and_then(|receiver| match receiver.try_recv() {
                    Ok(result) => Some(result),
                    Err(std::sync::mpsc::TryRecvError::Empty) => None,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => Some(Err(
                        "Solution discovery stopped without returning a result".into(),
                    )),
                });
        if let Some(result) = discovery {
            self.project.borrow_mut().discovery = None;
            match result {
                Ok((solution, mut language)) => {
                    let current = self.source_tools.borrow().service.clone();
                    *language.schema_mut() = current.schema().clone();
                    for uri in current.uris() {
                        if let (Some(document), Some(version)) =
                            (current.document(uri), current.version(uri))
                        {
                            if language.version(uri).is_none_or(|old| old < version) {
                                let _ = language.open(uri, document.source(), version);
                            }
                        }
                    }
                    self.source_tools.borrow_mut().service = language;
                    self.project.borrow_mut().workspace = Some(ProjectWorkspace::new(solution));
                    self.project.borrow_mut().selected = 0;
                    self.rebuild_project_tree();
                    self.status.set_text(Some("Solution loaded"));
                }
                Err(error) => self.report(Err(error)),
            }
        }
        let mut state = self.project.borrow_mut();
        if let Some(task) = state.task.as_mut() {
            let status = task.poll();
            let output = task.take_output();
            if !output.is_empty() {
                let control = self.root.get_control::<TextBox>("BuildOutput");
                let mut text = control.text().unwrap_or_default();
                for part in output {
                    text.push_str(&part.text);
                }
                if text.len() > 1024 * 1024 {
                    let mut at = text.len() - 1024 * 1024;
                    while !text.is_char_boundary(at) {
                        at += 1;
                    }
                    text.drain(..at);
                }
                control.set_text(Some(&text));
                control.set_caret_index(text.encode_utf16().count() as i32);
            }
            self.root
                .get_control::<TextBlock>("BuildStatus")
                .set_text(Some(&format!("{status:?}")));
        }
    }
    fn rebuild_project_tree(self: &Rc<Self>) {
        let panel = self.root.get_control::<StackPanel>("ProjectTree");
        panel.children().clear();
        let projects = self
            .project
            .borrow()
            .workspace
            .as_ref()
            .map(|w| w.solution.projects.clone())
            .unwrap_or_default();
        for (index, project) in projects.iter().enumerate() {
            let heading = button(&project.name);
            let weak = Rc::downgrade(self);
            heading.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.project.borrow_mut().selected = index;
                    this.refresh_run_targets();
                }
            });
            panel.children().add(heading.upcast::<Control>());
            for entry in &project.documents {
                if !matches!(
                    entry.kind,
                    ferroui_designer_project::DocumentKind::Xaml
                        | ferroui_designer_project::DocumentKind::Rust
                        | ferroui_designer_project::DocumentKind::Manifest
                ) {
                    continue;
                }
                let name = entry
                    .path
                    .strip_prefix(&project.root)
                    .unwrap_or(&entry.path)
                    .display()
                    .to_string();
                let row = button(&name);
                row.set_margin(Thickness::new(12.0, 0.0, 0.0, 0.0));
                row.set_horizontal_content_alignment(HorizontalAlignment::Left);
                let uri = entry.uri.clone();
                let weak = Rc::downgrade(self);
                row.click(move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        this.project.borrow_mut().selected = index;
                        this.refresh_run_targets();
                        let result = this.activate_document(&uri);
                        this.report(result);
                    }
                });
                panel.children().add(row.upcast::<Control>());
            }
        }
        self.refresh_run_targets();
    }
    fn refresh_run_targets(&self) {
        let list = self.root.get_control::<ComboBox>("RunTarget");
        list.items().clear();
        let state = self.project.borrow();
        if let Some(project) = state
            .workspace
            .as_ref()
            .and_then(|w| w.solution.projects.get(state.selected))
        {
            for target in &project.targets {
                if target.kind.iter().any(|kind| kind == "bin") {
                    list.items().add(Some(Rc::new(target.name.clone())));
                }
            }
        }
        list.set_selected_index(0);
    }
    pub(super) fn rebuild_document_tabs(self: &Rc<Self>) {
        self.capture_tab();
        let active = self.session.borrow().document_uri().to_owned();
        let rows = self
            .project
            .borrow()
            .tabs
            .iter()
            .map(|(uri, tab)| (uri.clone(), tab.document.source() != tab.saved))
            .collect::<Vec<_>>();
        let tabs = self.root.get_control::<StackPanel>("DocumentTabs");
        tabs.children().clear();
        for (uri, dirty) in rows {
            let close_uri = uri.clone();
            let group = StackPanel::new();
            group.set_orientation(Orientation::Horizontal);
            let name = uri.rsplit('/').next().unwrap_or(&uri);
            let row = button(&format!("{name}{}", if dirty { " •" } else { "" }));
            if uri == active {
                row.set_background(Some(SolidColorBrush::from_uint32(0xff334568).into()));
            }
            let weak = Rc::downgrade(self);
            row.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let result = this.activate_document(&uri);
                    this.report(result);
                }
            });
            group.children().add(row.upcast::<Control>());
            let close = button("×");
            let weak = Rc::downgrade(self);
            close.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let result = this.close_document(&close_uri, false);
                    this.report(result);
                }
            });
            group.children().add(close.upcast::<Control>());
            tabs.children().add(group.upcast::<Control>());
        }
    }
}
