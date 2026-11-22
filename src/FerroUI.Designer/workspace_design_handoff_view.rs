use super::DesignerWorkspace;
use crate::{design_handoff::Review, OutlinePlacement};
use ferroui_base::{
    platform::storage::{FilePickerFileType, FilePickerOpenOptions, IStorageItem},
    threading::Dispatcher,
    Ref,
};
use ferroui_controls::{
    Button, ComboBox, Control, StackPanel, TextBlock, TextBox, TopLevel, Window,
};
use ferroui_designer_project::{design_handoff::Access, file_path};
use ferroui_markup_xaml::FromXamlObject;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    rc::{Rc, Weak},
};

#[path = "workspace_design_handoff_rows.rs"]
mod rows;
use rows::{RowAction, OPEN, SOURCE};

struct HandoffReview {
    owner: Weak<DesignerWorkspace>,
    root: Ref<Control>,
    path: RefCell<Option<(PathBuf, Access)>>,
    plan: RefCell<Option<Review>>,
    open: Ref<Button>,
    refresh: Ref<Button>,
    edit: Ref<Button>,
    file: Ref<TextBlock>,
    filter: Ref<TextBox>,
    placement: Ref<ComboBox>,
    count: Ref<TextBlock>,
    status: Ref<TextBlock>,
    mappings: Ref<StackPanel>,
    closed: Cell<bool>,
}

pub(super) fn connect(owner: &Rc<DesignerWorkspace>) {
    let button = owner.root.get_control::<Button>("DesignHandoff");
    button.set_is_visible(!cfg!(target_arch = "wasm32"));
    rows::set_icon(&button, rows::HANDOFF);
    let owner = Rc::downgrade(owner);
    button.click(move |_, _| {
        if let Some(owner) = owner.upgrade() {
            owner.report(owner.open_handoff_review(None));
        }
    });
}

pub(super) fn open(
    owner: &Rc<DesignerWorkspace>,
    path: Option<(&Path, Access)>,
) -> Result<(), String> {
    if owner.solution().is_none() {
        return Err("Open a local solution to review design handoff".into());
    }
    let loaded = FerroRuntimeXamlLoader::load(
        include_str!("DesignHandoffReview.xaml"),
        None,
        None,
        None,
        false,
    )
    .map_err(|error| error.to_string())?;
    let root =
        Ref::<Control>::from_xaml_object(&loaded).ok_or("Could not create the handoff review")?;
    root.styles().add(crate::designer_chrome_style()?);
    let review = Rc::new(HandoffReview {
        owner: Rc::downgrade(owner),
        root: root.clone(),
        path: RefCell::new(path.map(|(path, access)| (path.into(), access))),
        plan: RefCell::new(None),
        open: root.get_control("OpenHandoff"),
        refresh: root.get_control("RefreshHandoff"),
        edit: root.get_control("EditHandoff"),
        file: root.get_control("HandoffFile"),
        filter: root.get_control("HandoffFilter"),
        placement: root.get_control("HandoffPlacement"),
        count: root.get_control("HandoffCount"),
        status: root.get_control("HandoffStatus"),
        mappings: root.get_control("HandoffRows"),
        closed: Cell::new(false),
    });
    review.connect();
    let window = Window::new();
    window.set_title(Some("FerroUI Design Handoff".into()));
    window.set_width(940.0);
    window.set_height(455.0);
    window.set_min_width(580.0);
    window.set_min_height(335.0);
    window.set_content(Some(Control::boxed(root)));
    let lifetime = review.clone();
    window.closed(move || lifetime.closed.set(true)).detach();
    window.show();
    if path.is_some() {
        review.refresh();
    }
    Ok(())
}

impl HandoffReview {
    fn connect(self: &Rc<Self>) {
        self.placement.set_selected_index(0);
        self.status.set_text(Some(
            "Open a handoff JSON file to resolve its mappings in the current design.",
        ));
        for (button, icon, action) in [
            (&self.open, OPEN, Self::choose as fn(&Rc<Self>)),
            (&self.refresh, rows::REFRESH, Self::refresh),
            (&self.edit, SOURCE, Self::edit),
        ] {
            rows::set_icon(button, icon);
            let review = Rc::downgrade(self);
            button.click(move |_, _| {
                if let Some(review) = review.upgrade() {
                    action(&review);
                }
            });
        }
        let review = Rc::downgrade(self);
        self.filter.text_changed(move |_, _| {
            if let Some(review) = review.upgrade() {
                review.render();
            }
        });
    }

    fn choose(self: &Rc<Self>) {
        let Some(top) = TopLevel::get_top_level(Some(&self.root)) else {
            return;
        };
        let provider = top.storage_provider();
        let mut options = FilePickerOpenOptions::new();
        options.set_title(Some("Open FerroUI Design Handoff".into()));
        options.set_allow_multiple(false);
        options.set_file_type_filter(Some(vec![FilePickerFileType::new(Some(
            "Design handoff JSON",
        ))
        .with_patterns(&["*.json"])]));
        self.open.set_is_enabled(false);
        let review = self.clone();
        Dispatcher::ui_thread().spawn_local(async move {
            let files = provider.open_file_picker_async(options).await;
            if review.closed.get() {
                return;
            }
            review.open.set_is_enabled(true);
            match files {
                Ok(files) => {
                    let Some(file) = files.into_iter().next() else {
                        return;
                    };
                    let item: Rc<dyn IStorageItem> = file;
                    if let Some(path) = item.try_get_local_path() {
                        *review.path.borrow_mut() = Some((path.into(), Access::SelectedFile));
                        review.refresh();
                    } else {
                        review
                            .status
                            .set_text(Some("Choose a handoff file on the workspace host."));
                    }
                }
                Err(error) => review.status.set_text(Some(&error.to_string())),
            }
        });
    }

    fn edit(self: &Rc<Self>) {
        if let Some((path, _)) = self.path.borrow().clone() {
            self.launch(path);
        }
    }

    fn refresh(self: &Rc<Self>) {
        let Some((path, access)) = self.path.borrow().clone() else {
            return;
        };
        let Some(owner) = self.owner.upgrade() else {
            return;
        };
        self.file
            .set_text(path.file_name().and_then(|name| name.to_str()));
        self.refresh.set_is_enabled(true);
        self.edit.set_is_enabled(true);
        match owner.review_handoff(&path, access) {
            Ok(plan) => {
                self.status.set_text(Some(&format!(
                    "Design · {}",
                    file_path(&plan.document)
                        .ok()
                        .and_then(|path| path.file_name()?.to_str().map(str::to_owned))
                        .unwrap_or(plan.document.clone())
                )));
                *self.plan.borrow_mut() = Some(plan);
            }
            Err(error) => {
                self.status.set_text(Some(&error));
                *self.plan.borrow_mut() = None;
            }
        }
        self.render();
    }

    fn render(self: &Rc<Self>) {
        self.mappings.children().clear();
        let plan = self.plan.borrow();
        let Some(plan) = plan.as_ref() else {
            self.count.set_text(None);
            return;
        };
        let query = self.filter.text().unwrap_or_default().trim().to_lowercase();
        let mut shown = 0;
        for row in &plan.rows {
            if ![&row.name, &row.id, &row.target]
                .iter()
                .any(|value| value.to_lowercase().contains(&query))
            {
                continue;
            }
            let weak = Rc::downgrade(self);
            let id = row.id.clone();
            let review = row.review.clone();
            let control = rows::mapping(row, move |action| {
                if let (Some(window), Some(review)) = (weak.upgrade(), review.as_deref()) {
                    window.act(&id, review, action);
                }
            });
            self.mappings.children().add(control);
            shown += 1;
        }
        let issues = plan.rows.iter().filter(|row| row.issue.is_some()).count();
        self.count.set_text(Some(&format!(
            "{shown} of {} mappings · {issues} to review",
            plan.rows.len()
        )));
    }

    fn act(self: &Rc<Self>, id: &str, review: &str, action: RowAction) {
        let Some((path, access)) = self.path.borrow().clone() else {
            return;
        };
        let Some(owner) = self.owner.upgrade() else {
            return;
        };
        let result: Result<String, String> = (|| {
            if matches!(action, RowAction::Copy) {
                let uri = owner.copy_handoff(&path, access, id, review)?;
                self.refresh();
                return Ok(format!(
                    "Copied {}",
                    file_path(&uri)
                        .map_err(|error| error.to_string())?
                        .display()
                ));
            }
            let resolved = owner.check_handoff(&path, access, id, review)?;
            match action {
                RowAction::Copy => unreachable!("handled checked copy"),
                RowAction::Place => {
                    let position = match self.placement.selected_index() {
                        1 => OutlinePlacement::Before,
                        2 => OutlinePlacement::After,
                        _ => OutlinePlacement::Inside,
                    };
                    let (revision, node) = {
                        let session = owner.session.borrow();
                        (
                            session.document().revision(),
                            session
                                .selection()
                                .last()
                                .copied()
                                .unwrap_or(session.document().ast().root()),
                        )
                    };
                    owner.place_handoff(resolved, revision, node, position)?;
                    self.refresh();
                    Ok("Placed the mapped item".into())
                }
                RowAction::Source | RowAction::Preview => {
                    owner.open_project_library_definition(
                        resolved.row.definition.ok_or("Choose a definition")?,
                        matches!(action, RowAction::Preview),
                    )?;
                    Ok("Opened the mapped definition".into())
                }
                RowAction::OpenAsset => {
                    let path =
                        file_path(&resolved.row.asset_uri.ok_or("Choose an available asset")?)
                            .map_err(|error| error.to_string())?;
                    self.launch(path);
                    Ok("Opening the mapped asset".into())
                }
            }
        })();
        self.status.set_text(Some(&match result {
            Ok(status) => status,
            Err(error) => error,
        }));
    }

    fn launch(self: &Rc<Self>, path: PathBuf) {
        let Some(top) = TopLevel::get_top_level(Some(&self.root)) else {
            return;
        };
        let launcher = top.launcher();
        let review = self.clone();
        Dispatcher::ui_thread().spawn_local(async move {
            if !launcher.launch_file_info_async(&path).await && !review.closed.get() {
                review
                    .status
                    .set_text(Some("The workspace host could not open the file."));
            }
        });
    }
}
