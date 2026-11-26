//! Studio project setup uses the same reviewed starter plans as the editor host.
use super::DesignerWorkspace;
use crate::{
    icons,
    workspace::inspector_actions::{add_button, text, SOURCE},
};
use ferroui_base::{
    layout::Orientation,
    media::FontFamily,
    threading::{DispatcherPriority, DispatcherTimer},
    Ref,
};
use ferroui_controls::{
    automation::AutomationProperties, Button, ComboBox, Control, Flyout, PlacementMode, StackPanel,
    TabControl, TabItem, TextBox,
};
use ferroui_designer_project::{file_uri, project_templates, toolchain};
use std::{path::PathBuf, rc::Rc, time::Duration};

impl DesignerWorkspace {
    pub(super) fn connect_project_setup(self: &Rc<Self>) {
        self.project_setup_flyout("NewCargoProject", icons::ADD, pick_template);
        self.project_setup_flyout("ProjectToolchain", icons::CHECK, read_toolchain);
    }

    fn project_setup_flyout(
        self: &Rc<Self>,
        name: &str,
        icon: &str,
        open: fn(&Rc<Self>, &Ref<Flyout>),
    ) {
        let button = self.root.get_control::<Button>(name);
        button.set_is_visible(!cfg!(target_arch = "wasm32"));
        button.set_content(Some(Control::boxed(icons::vector_icon(icon, 14.0))));
        let flyout = Flyout::new();
        flyout.set_placement(PlacementMode::Bottom);
        flyout.popup().set_should_use_overlay_layer(true);
        flyout.flyout_presenter_classes().add("designer-chrome");
        let owner = Rc::downgrade(self);
        let popup = flyout.downgrade();
        flyout
            .opening(move |_| {
                if let (Some(owner), Some(popup)) = (owner.upgrade(), popup.upgrade()) {
                    open(&owner, &popup);
                }
            })
            .detach();
        button.set_flyout(Some(flyout.upcast()));
    }
}

fn pick_template(workspace: &Rc<DesignerWorkspace>, popup: &Ref<Flyout>) {
    let panel = body(workspace);
    let templates = ComboBox::new();
    AutomationProperties::set_name(&templates, Some("Project template"));
    for template in project_templates::Template::ALL {
        templates
            .items()
            .add(Some(Rc::new(template.label().to_owned())));
    }
    templates.set_selected_index(0);
    let name = field("Project name", "task-workbench");
    let parent = field("Parent folder", "");
    if let Some(path) = workspace
        .solution()
        .map(|solution| solution.root)
        .or_else(|| std::env::current_dir().ok())
    {
        parent.set_text(path.to_str());
    }
    let checkout = field("FerroUI source checkout", "");
    checkout.set_placeholder_text(Some(&format!(
        "Published FerroUI {}",
        project_templates::PUBLISHED_VERSION
    )));
    panel.children().add(templates.clone().upcast::<Control>());
    for (label, input) in [
        ("Project name", &name),
        ("Parent folder", &parent),
        ("FerroUI dependency", &checkout),
    ] {
        panel.children().add(text(label).upcast::<Control>());
        panel.children().add(input.clone().upcast::<Control>());
    }
    let actions = toolbar();
    let status = text("");
    let owner = Rc::downgrade(workspace);
    let flyout = popup.downgrade();
    let status_ref = status.downgrade();
    add_button(&actions, SOURCE, "Review project files", move || {
        let (Some(owner), Some(flyout)) = (owner.upgrade(), flyout.upgrade()) else {
            return;
        };
        let selected = templates.selected_index().max(0) as usize;
        let Some(&template) = project_templates::Template::ALL.get(selected) else {
            return;
        };
        let checkout = checkout
            .text()
            .filter(|path| !path.trim().is_empty())
            .map(|path| PathBuf::from(path.trim()));
        let request = project_templates::Request {
            template,
            name: name.text().unwrap_or_default().trim().into(),
            checkout,
        };
        let directory = PathBuf::from(parent.text().unwrap_or_default().trim());
        match project_templates::plan(&directory, &request) {
            Ok(plan) => review(&owner, &flyout, directory, request, plan),
            Err(error) => {
                if let Some(status) = status_ref.upgrade() {
                    status.set_text(Some(&error.to_string()));
                }
            }
        }
    });
    cancel_button(&actions, popup, "Cancel new project");
    panel.children().add(actions.upcast::<Control>());
    panel.children().add(status.upcast::<Control>());
    popup.set_content(Some(Control::boxed(panel)));
}

fn review(
    workspace: &Rc<DesignerWorkspace>,
    popup: &Ref<Flyout>,
    parent: PathBuf,
    request: project_templates::Request,
    plan: project_templates::Plan,
) {
    let panel = body(workspace);
    panel
        .children()
        .add(text(&plan.root.display().to_string()).upcast::<Control>());
    let files = TabControl::new();
    for file in &plan.files {
        let tab = TabItem::new();
        tab.set_header(Some(Rc::new(file.path.to_owned())));
        let editor = field(file.path, &file.source);
        editor.set_is_read_only(true);
        editor.set_accepts_return(true);
        editor.set_height(220.0);
        editor.set_font_family(FontFamily::from("monospace"));
        editor.set_font_size(11.0);
        tab.set_content(Some(Control::boxed(editor)));
        files.items().add(Some(Control::boxed(tab)));
    }
    files.set_selected_index(
        plan.files
            .iter()
            .position(|file| file.path == plan.entry)
            .unwrap_or_default() as i32,
    );
    panel.children().add(files.upcast::<Control>());
    let actions = toolbar();
    let status = text("");
    let owner = Rc::downgrade(workspace);
    let flyout = popup.downgrade();
    let message = status.downgrade();
    add_button(&actions, icons::CHECK, "Create project", move || {
        let (Some(owner), Some(flyout)) = (owner.upgrade(), flyout.upgrade()) else {
            return;
        };
        let output = project_templates::create(&parent, &request, &plan.review);
        match output {
            Ok(project) => {
                flyout.hide();
                owner.report(open_project(&owner, &project));
            }
            Err(error) => {
                if let Some(message) = message.upgrade() {
                    message.set_text(Some(&error.to_string()));
                }
            }
        }
    });
    cancel_button(&actions, popup, "Cancel project creation");
    panel.children().add(actions.upcast::<Control>());
    panel.children().add(status.upcast::<Control>());
    popup.set_content(Some(Control::boxed(panel)));
}

fn open_project(
    workspace: &Rc<DesignerWorkspace>,
    project: &project_templates::Created,
) -> Result<(), String> {
    if workspace.pending_source.get() {
        workspace.apply_source();
    }
    let source = std::fs::read_to_string(&project.entry).map_err(|error| error.to_string())?;
    let uri = file_uri(&project.entry).map_err(|error| error.to_string())?;
    workspace.open_document(source, uri)?;
    workspace.open_solution(&project.root)
}

fn read_toolchain(workspace: &Rc<DesignerWorkspace>, popup: &Ref<Flyout>) {
    let panel = body(workspace);
    let status = text("Checking Cargo and Rust…");
    panel.children().add(status.clone().upcast::<Control>());
    popup.set_content(Some(Control::boxed(panel)));
    let directory = workspace
        .solution()
        .and_then(|solution| {
            solution
                .projects
                .get(workspace.selected_project())
                .map(|project| project.root.clone())
                .or(Some(solution.root))
        })
        .or_else(|| std::env::current_dir().ok());
    let Some(directory) = directory else {
        status.set_text(Some("Open a project folder before checking its toolchain."));
        return;
    };
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = send.send(toolchain::inspect(&toolchain::Request {
            directory,
            ..Default::default()
        }));
    });
    let status = status.downgrade();
    let _timer = DispatcherTimer::with_callback(
        Duration::from_millis(40),
        DispatcherPriority::BACKGROUND,
        move |timer| {
            let Some(status) = status.upgrade() else {
                timer.stop();
                return;
            };
            let report = match receive.try_recv() {
                Ok(report) => report,
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    status.set_text(Some("Toolchain check ended without a report."));
                    timer.stop();
                    return;
                }
            };
            match report {
                Ok(report) => status.set_text(Some(&toolchain_summary(report))),
                Err(error) => status.set_text(Some(&error.to_string())),
            }
            timer.stop();
        },
    );
}

fn toolchain_summary(report: toolchain::Report) -> String {
    let cargo = report
        .cargo
        .version
        .or(report.cargo.error)
        .unwrap_or_else(|| "Cargo unavailable".into());
    let rust = report
        .rustc
        .version
        .or(report.rustc.error)
        .unwrap_or_else(|| "Rust compiler unavailable".into());
    let target = report.target_error.unwrap_or_else(|| match report.target {
        Some(target) => format!(
            "{target} · {}",
            if report.target_installed == Some(true) {
                "Rust libraries installed"
            } else {
                "Rust libraries missing"
            }
        ),
        None => "Rust target unavailable".into(),
    });
    format!(
        "{cargo}\n{rust}\n{target}\nRequires Rust {} or later",
        report.minimum_rust
    )
}

fn field(label: &str, value: &str) -> Ref<TextBox> {
    let field = TextBox::new();
    field.set_text(Some(value));
    AutomationProperties::set_name(&field, Some(label));
    field
}

fn body(workspace: &DesignerWorkspace) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    panel.set_spacing(8.0);
    panel.set_width((workspace.root.bounds().width - 40.0).clamp(300.0, 620.0));
    panel
}

fn toolbar() -> Ref<StackPanel> {
    let row = StackPanel::new();
    row.set_orientation(Orientation::Horizontal);
    row.set_spacing(6.0);
    row
}

fn cancel_button(row: &Ref<StackPanel>, popup: &Ref<Flyout>, label: &str) {
    let popup = popup.downgrade();
    add_button(row, icons::CLOSE, label, move || {
        if let Some(popup) = popup.upgrade() {
            popup.hide();
        }
    });
}
