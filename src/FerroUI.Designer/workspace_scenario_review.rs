//! A compact native comparison window using the Studio scenario process.
use super::{scenarios::ScenarioProgress, DesignerWorkspace};
use crate::{icons, workspace::inspector_actions::SOURCE};
use ferroui_base::{
    media::imaging::Bitmap,
    threading::{DispatcherPriority, DispatcherTimer},
    Ref,
};
use ferroui_controls::{Button, ComboBox, Control, Image, TextBlock, TextBox, Window};
use ferroui_designer_project::scenario_process::Options;
use ferroui_designer_tools::scenario::{ComparisonStatus, ScenarioReport, VisualReview};
use ferroui_markup_xaml::FromXamlObject;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::{
    cell::{Cell, RefCell},
    path::Path,
    rc::{Rc, Weak},
    time::Duration,
};

const PLAY: &str = "F1 M4 1L15 8L4 15Z";

pub(crate) struct OpenRun {
    pub(crate) id: u64,
    pub(crate) path: String,
    pub(crate) reference: Option<String>,
}

struct Review {
    workspace: Weak<DesignerWorkspace>,
    root: Ref<Control>,
    files: Vec<String>,
    file: Ref<ComboBox>,
    reference: Ref<TextBox>,
    captures: Ref<ComboBox>,
    status: Ref<TextBlock>,
    start: Ref<Button>,
    approve: Ref<Button>,
    difference: Ref<Button>,
    request: RefCell<Option<OpenRun>>,
    report: RefCell<Option<Rc<ScenarioReport>>>,
    busy: Cell<bool>,
    diff: Cell<bool>,
}

impl DesignerWorkspace {
    pub(super) fn connect_scenario_review(self: &Rc<Self>) {
        let button = self.root.get_control::<Button>("ReviewScenario");
        button.set_is_visible(!cfg!(target_arch = "wasm32"));
        button.set_content(Some(Control::boxed(icons::vector_icon(SOURCE, 14.0))));
        let weak = Rc::downgrade(self);
        button.click(move |_, _| {
            if let Some(workspace) = weak.upgrade() {
                workspace.report(workspace.open_scenario_review(None));
            }
        });
    }

    pub(crate) fn open_scenario_review(
        self: &Rc<Self>,
        initial: Option<OpenRun>,
    ) -> Result<(), String> {
        let files = self.list_scenarios()?;
        if files.is_empty() {
            return Err("The project folder has no saved FerroUI scenarios".into());
        }
        let loaded = FerroRuntimeXamlLoader::load(
            include_str!("ScenarioReview.xaml"),
            None,
            None,
            None,
            false,
        )
        .map_err(|error| error.to_string())?;
        let root = Ref::<Control>::from_xaml_object(&loaded)
            .ok_or("Could not create the baseline review")?;
        root.styles().add(crate::designer_chrome_style()?);
        let dialog = Rc::new(Review {
            workspace: Rc::downgrade(self),
            files,
            file: root.get_control("ScenarioFile"),
            reference: root.get_control("ScenarioReference"),
            captures: root.get_control("ScenarioCapture"),
            status: root.get_control("ScenarioStatus"),
            start: root.get_control("RunScenario"),
            approve: root.get_control("ApproveScenario"),
            difference: root.get_control("ScenarioDifference"),
            root: root.clone(),
            request: RefCell::new(None),
            report: RefCell::new(None),
            busy: Cell::new(false),
            diff: Cell::new(false),
        });
        dialog.connect();
        let window = Window::new();
        window.set_title(Some("FerroUI Baseline Review".into()));
        window.set_width(1000.0);
        window.set_height(620.0);
        window.set_content(Some(Control::boxed(root)));
        if let Some(initial) = initial {
            if let Some(index) = dialog.files.iter().position(|path| path == &initial.path) {
                dialog.file.set_selected_index(index as i32);
            }
            dialog.reference.set_text(initial.reference.as_deref());
            dialog.track(initial);
        }
        window
            .closed(move || {
                if dialog.busy.get() {
                    if let (Some(workspace), Some(request)) =
                        (dialog.workspace.upgrade(), dialog.request.borrow().as_ref())
                    {
                        let _ = workspace.cancel_scenario(request.id);
                    }
                }
            })
            .detach();
        window.show();
        Ok(())
    }
}

impl Review {
    fn connect(self: &Rc<Self>) {
        for path in &self.files {
            self.file.items().add(Some(Rc::new(path.clone())));
        }
        self.file.set_selected_index(0);
        for (button, icon) in [
            (&self.start, PLAY),
            (&self.approve, icons::CHECK),
            (&self.difference, SOURCE),
        ] {
            button.set_content(Some(Control::boxed(icons::vector_icon(icon, 14.0))));
        }
        self.button(&self.start, Self::review);
        self.button(&self.approve, Self::accept);
        self.button(&self.difference, |review| {
            review.diff.set(!review.diff.get());
            review.paint();
        });
        let weak = Rc::downgrade(self);
        self.captures.selection_changed(move |_, _| {
            if let Some(review) = weak.upgrade() {
                review.paint();
            }
        });
    }

    fn button(self: &Rc<Self>, button: &Button, invoke: fn(&Rc<Self>)) {
        let weak = Rc::downgrade(self);
        button.click(move |_, _| {
            if let Some(review) = weak.upgrade() {
                invoke(&review);
            }
        });
    }

    fn review(self: &Rc<Self>) {
        let Some(path) = self
            .files
            .get(self.file.selected_index().max(0) as usize)
            .cloned()
        else {
            return;
        };
        let reference = self
            .reference
            .text()
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty());
        self.launch(path, reference, None);
    }

    fn accept(self: &Rc<Self>) {
        let request = self
            .request
            .borrow()
            .as_ref()
            .map(|request| (request.path.clone(), request.reference.clone()));
        let Some((path, reference)) = request else {
            return;
        };
        let approval = self
            .report
            .borrow()
            .as_ref()
            .and_then(|report| self.selection(report))
            .and_then(|item| item.approval.clone());
        // The current capture chooses the approval, while the original request
        // supplies the scenario and reference that were used to prepare it.
        if let Some(approval) = approval {
            self.launch(path, reference, Some(approval));
        }
    }

    fn launch(self: &Rc<Self>, path: String, reference: Option<String>, approval: Option<String>) {
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };
        if self.busy.get() {
            return;
        }
        let options = Options {
            review: true,
            reference: reference.clone(),
            approval,
        };
        match workspace.start_scenario(&path, &options) {
            Ok(id) => self.track(OpenRun {
                id,
                path,
                reference,
            }),
            Err(error) => self.status.set_text(Some(&error)),
        }
    }

    fn track(self: &Rc<Self>, request: OpenRun) {
        let id = request.id;
        self.request.replace(Some(request));
        self.busy.set(true);
        self.start.set_is_enabled(false);
        self.approve.set_is_enabled(false);
        self.status.set_text(Some("Running the native scenario…"));
        let weak = Rc::downgrade(self);
        let _timer = DispatcherTimer::with_callback(
            Duration::from_millis(40),
            DispatcherPriority::BACKGROUND,
            move |timer| {
                let Some(review) = weak.upgrade() else {
                    timer.stop();
                    return;
                };
                let Some(workspace) = review.workspace.upgrade() else {
                    timer.stop();
                    return;
                };
                let progress = workspace
                    .scenario_progress(id)
                    .unwrap_or_else(ScenarioProgress::Failed);
                if matches!(progress, ScenarioProgress::Running) {
                    return;
                }
                timer.stop();
                review.busy.set(false);
                review.start.set_is_enabled(true);
                match progress {
                    ScenarioProgress::Complete(report) => review.complete(report),
                    ScenarioProgress::Failed(error) => {
                        review.report.borrow_mut().take();
                        review.status.set_text(Some(&error));
                    }
                    ScenarioProgress::Running => unreachable!(),
                }
            },
        );
    }

    fn complete(&self, report: Rc<ScenarioReport>) {
        let selected = self.captures.selected_index().max(0);
        self.report.replace(Some(report.clone()));
        self.captures.items().clear();
        for capture in &report.visuals {
            let name = capture
                .baseline
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Screenshot");
            self.captures
                .items()
                .add(Some(Rc::new(format!("Step {} · {name}", capture.step))));
        }
        if !report.visuals.is_empty() {
            self.captures
                .set_selected_index(selected.min(report.visuals.len() as i32 - 1));
        }
        self.paint();
    }

    fn selection<'a>(&self, report: &'a ScenarioReport) -> Option<&'a VisualReview> {
        report
            .visuals
            .get(usize::try_from(self.captures.selected_index()).ok()?)
    }

    fn paint(&self) {
        let report = self.report.borrow();
        let Some(report) = report.as_ref() else {
            return;
        };
        let item = self.selection(report);
        let difference = item.and_then(|item| item.difference.as_deref());
        self.approve
            .set_is_enabled(!self.busy.get() && item.is_some_and(|item| item.approval.is_some()));
        self.difference.set_is_enabled(difference.is_some());
        let diff = self.diff.get() && difference.is_some();
        self.root
            .get_control::<Control>("ComparedImages")
            .set_is_visible(!diff);
        self.root
            .get_control::<Control>("DifferenceImage")
            .set_is_visible(diff);
        for (control, file) in [
            (
                "ScenarioExpected",
                item.and_then(|item| item.expected.as_deref()),
            ),
            ("ScenarioActual", item.map(|item| item.actual.as_path())),
            ("ScenarioDiff", difference),
        ] {
            set_image(&self.root.get_control(control), file);
        }
        let title = report
            .reference
            .as_ref()
            .map(|reference| {
                format!(
                    "Expected · {} · {}",
                    reference.name,
                    reference.commit.chars().take(8).collect::<String>()
                )
            })
            .unwrap_or_else(|| "Expected · Workspace".into());
        self.root
            .get_control::<TextBlock>("ReferenceTitle")
            .set_text(Some(&title));
        let message =
            if let Some(approved) = &report.approved {
                format!(
                    "Approved {}",
                    approved
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("baseline")
                )
            } else if let Some(step) = report.steps.iter().find(|step| {
                !step.passed && !report.visuals.iter().any(|item| item.step == step.step)
            }) {
                format!("Step {} · {}", step.step, step.message)
            } else if let Some(item) = item {
                let baseline = match item.workspace.status {
                    ComparisonStatus::Match => "Baseline verified",
                    ComparisonStatus::Missing => "Baseline needed",
                    _ => "Awaiting approval",
                };
                let change = item
                    .comparison
                    .changed_pixels
                    .map(|count| format!(" · {count} changed pixels"))
                    .unwrap_or_default();
                format!("{baseline}{change}")
            } else {
                "The scenario finished without a screenshot assertion.".into()
            };
        self.status.set_text(Some(&message));
    }
}

fn set_image(image: &Image, path: Option<&Path>) {
    let bitmap =
        path.and_then(|path| Bitmap::from_stream(&mut std::fs::File::open(path).ok()?).ok());
    image.set_source(bitmap.map(|bitmap| Rc::new(bitmap) as Rc<dyn ferroui_base::media::IImage>));
}
