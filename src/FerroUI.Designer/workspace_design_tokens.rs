//! Native review and save for the shared DTCG conversion plans.
use super::DesignerWorkspace;
use crate::{icons, workspace::inspector_actions::SOURCE};
use ferroui_base::{threading::Dispatcher, Ref};
use ferroui_controls::{Button, Control, TextBlock, TextBox, Window};
use ferroui_designer_tools::token_handoff::{self, Direction, Options, Plan};
use ferroui_markup_xaml::FromXamlObject;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::{
    cell::{Cell, RefCell},
    path::Path,
    rc::{Rc, Weak},
};

#[path = "workspace_design_token_files.rs"]
mod files;
#[path = "workspace_design_token_review_options.rs"]
mod review_options;
use files::TokenSource;
use review_options::ReviewOptions;

const OPEN: &str = "F1 M1 4H6L8 6H15V13H1Z M2.5 5.5V11.5H13.5V7.5H7.4L5.4 5.5Z";
const SAVE: &str = "F1 M2 1H12L15 4V15H1V1Z M3 3V13H13V5L11 3V7H4V3Z M5 9H11V12H5Z";

struct Reviewed {
    source: TokenSource,
    options: Options,
    plan: Plan,
}

struct TokenReview {
    owner: Weak<DesignerWorkspace>,
    root: Ref<Control>,
    current: RefCell<Option<Reviewed>>,
    options: Rc<ReviewOptions>,
    source: Ref<TextBlock>,
    report: Ref<TextBox>,
    output: Ref<TextBox>,
    status: Ref<TextBlock>,
    import: Ref<Button>,
    export: Ref<Button>,
    save: Ref<Button>,
    busy: Cell<bool>,
    closed: Cell<bool>,
    dirty: Cell<bool>,
}

impl DesignerWorkspace {
    pub(super) fn connect_design_tokens(self: &Rc<Self>) {
        let button = self.root.get_control::<Button>("DesignTokens");
        button.set_is_visible(!cfg!(target_arch = "wasm32"));
        button.set_content(Some(Control::boxed(icons::vector_icon(
            icons::EXPRESSION,
            14.0,
        ))));
        let owner = Rc::downgrade(self);
        button.click(move |_, _| {
            if let Some(owner) = owner.upgrade() {
                owner.report(open_review(&owner));
            }
        });
    }
}

fn open_review(owner: &Rc<DesignerWorkspace>) -> Result<(), String> {
    let loaded = FerroRuntimeXamlLoader::load(
        include_str!("DesignTokenReview.xaml"),
        None,
        None,
        None,
        false,
    )
    .map_err(|error| error.to_string())?;
    let root = Ref::<Control>::from_xaml_object(&loaded)
        .ok_or("Could not create the design token review")?;
    root.styles().add(crate::designer_chrome_style()?);
    let review = Rc::new(TokenReview {
        owner: Rc::downgrade(owner),
        import: root.get_control("ImportTokens"),
        export: root.get_control("ExportTokens"),
        save: root.get_control("SaveTokens"),
        source: root.get_control("TokenSource"),
        report: root.get_control("TokenReport"),
        output: root.get_control("TokenOutput"),
        status: root.get_control("TokenStatus"),
        options: ReviewOptions::new(&root),
        root: root.clone(),
        current: RefCell::new(None),
        busy: Cell::new(false),
        closed: Cell::new(false),
        dirty: Cell::new(false),
    });
    review.connect();
    let window = Window::new();
    window.set_title(Some("FerroUI Design Tokens".into()));
    window.set_width(1000.0);
    window.set_height(620.0);
    window.set_min_width(720.0);
    window.set_min_height(390.0);
    window.set_content(Some(Control::boxed(root)));
    window.closed(move || review.closed.set(true)).detach();
    window.show();
    Ok(())
}

impl TokenReview {
    fn connect(self: &Rc<Self>) {
        self.status.set_text(Some(
            "Open a design tokens file or review the active XAML resource dictionary.",
        ));
        for (button, icon, action) in [
            (&self.import, OPEN, Self::import as fn(&Rc<Self>)),
            (&self.export, SOURCE, Self::export),
            (&self.save, SAVE, Self::save),
            (&self.options.submit, icons::CHECK, Self::apply_mapping),
        ] {
            button.set_content(Some(Control::boxed(icons::vector_icon(icon, 14.0))));
            let review = Rc::downgrade(self);
            button.click(move |_, _| {
                if let Some(review) = review.upgrade().filter(|review| !review.busy.get()) {
                    action(&review);
                }
            });
        }
        let review = Rc::downgrade(self);
        self.options.connect(move || {
            if let Some(review) = review.upgrade() {
                review.dirty.set(true);
                review.update_controls();
            }
        });
    }

    fn import(self: &Rc<Self>) {
        self.set_busy(true);
        let review = self.clone();
        Dispatcher::ui_thread().spawn_local(async move {
            let chosen = files::choose_tokens(&review.root).await;
            if review.closed.get() {
                return;
            }
            match chosen {
                Ok(Some(source)) => review.set_source(source),
                Ok(None) => {}
                Err(error) => review.status.set_text(Some(&error)),
            }
            review.set_busy(false);
        });
    }

    fn export(self: &Rc<Self>) {
        let Some(owner) = self.owner.upgrade() else {
            return;
        };
        match TokenSource::current_dictionary(&owner) {
            Ok(source) => self.set_source(source),
            Err(error) => self.status.set_text(Some(&error)),
        }
    }

    fn save(self: &Rc<Self>) {
        if self.dirty.get() {
            return;
        }
        let Some((source, candidate)) = self
            .current
            .borrow()
            .as_ref()
            .filter(|review| !review.plan.mappings.is_empty())
            .map(|review| (review.source.clone(), review.plan.source.clone()))
        else {
            return;
        };
        self.set_busy(true);
        let review = self.clone();
        Dispatcher::ui_thread().spawn_local(async move {
            match review.write_review(&source, &candidate).await {
                Ok(Some(path)) => review.status.set_text(Some(&format!("Created {path}"))),
                Ok(None) => {}
                Err(error) => review.status.set_text(Some(&error)),
            }
            review.set_busy(false);
        });
    }

    async fn write_review(
        &self,
        source: &TokenSource,
        candidate: &str,
    ) -> Result<Option<String>, String> {
        {
            let owner = self.owner.upgrade().ok_or("The source workspace closed")?;
            source.verify(&owner).await?;
        }
        let Some(path) = files::choose_destination(&self.root, source.direction()).await? else {
            return Ok(None);
        };
        let owner = self.owner.upgrade().ok_or("The source workspace closed")?;
        source.verify(&owner).await?;
        if self.closed.get() {
            return Ok(None);
        }
        ferroui_designer_project::create_new_file(Path::new(&path), candidate)
            .map_err(|error| error.to_string())?;
        Ok(Some(path))
    }

    fn set_source(&self, source: TokenSource) {
        let direction = match source.direction() {
            Direction::Import => "Import",
            Direction::Export => "Export",
        };
        self.source
            .set_text(Some(&format!("{direction} · {}", source.label())));
        self.current.borrow_mut().take();
        self.report.set_text(None);
        self.output.set_text(None);
        self.options.panel.set_is_visible(false);
        self.review(source, Options::default());
    }

    fn apply_mapping(self: &Rc<Self>) {
        let edited = self.current.borrow().as_ref().map(|review| {
            (
                review.source.clone(),
                self.options
                    .edited_options(review.source.direction(), &review.options),
            )
        });
        match edited {
            Some((source, Ok(options))) => self.review(source, options),
            Some((_, Err(error))) => self.status.set_text(Some(&error)),
            None => {}
        }
    }

    fn review(&self, source: TokenSource, options: Options) {
        let direction = source.direction();
        let result = match direction {
            Direction::Import => token_handoff::plan_import(source.text(), &options),
            Direction::Export => token_handoff::plan_export(source.text(), &options),
        };
        match result {
            Ok(plan) => {
                self.status.set_text(Some(&format!(
                    "{} mapped · {} skipped",
                    plan.mappings.len(),
                    plan.issues.len()
                )));
                self.report
                    .set_text(Some(&review_options::mapping_report(&plan)));
                self.output.set_text(Some(&plan.source));
                self.options.show(direction, &options, &plan);
                self.current.replace(Some(Reviewed {
                    source,
                    options,
                    plan,
                }));
                self.dirty.set(false);
            }
            Err(error) => self.status.set_text(Some(&error.to_string())),
        }
        self.update_controls();
    }

    fn set_busy(&self, busy: bool) {
        self.busy.set(busy);
        self.update_controls();
    }

    fn update_controls(&self) {
        let idle = !self.busy.get() && !self.closed.get();
        self.import.set_is_enabled(idle);
        self.export.set_is_enabled(idle);
        self.save.set_is_enabled(
            idle && !self.dirty.get()
                && self
                    .current
                    .borrow()
                    .as_ref()
                    .is_some_and(|review| !review.plan.mappings.is_empty()),
        );
        self.options.panel.set_is_enabled(idle);
    }
}
