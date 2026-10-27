//! Shell navigation is independent of document edits and runtime inspection.
//! Hiding a pane keeps its control, selection, pending text and scroll state alive.
use super::{DesignerWorkspace, OutlineMode};
use ferroui_base::input::{InputElement, Key};
use ferroui_controls::{
    Border, Button, Control, Grid, GridLength, MenuItem, ScrollViewer, TextBlock,
};
use ferroui_designer_tools::{Alignment, Axis};
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WorkspaceViewMode {
    #[default]
    Design,
    Split,
    Xaml,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavigationPane {
    #[default]
    Layers,
    Assets,
    Project,
}

pub(super) struct WorkspaceShell {
    view: Cell<WorkspaceViewMode>,
    navigation: Cell<NavigationPane>,
    split_height: Cell<f64>,
    timeline_height: Cell<f64>,
}

impl Default for WorkspaceShell {
    fn default() -> Self {
        Self {
            view: Cell::new(WorkspaceViewMode::Design),
            navigation: Cell::new(NavigationPane::Layers),
            split_height: Cell::new(260.0),
            timeline_height: Cell::new(220.0),
        }
    }
}

impl DesignerWorkspace {
    pub fn view_mode(&self) -> WorkspaceViewMode {
        self.shell.view.get()
    }

    /// Switches the visible editor without reloading the document or applying pending text.
    pub fn set_view_mode(&self, mode: WorkspaceViewMode) {
        let layout = self.root.get_control::<Grid>("DocumentLayout");
        let source = self.root.get_control::<Grid>("SourcePane");
        if self.view_mode() == WorkspaceViewMode::Split {
            let height = source.bounds().height;
            if height >= 100.0 {
                self.shell.split_height.set(height);
            }
        }
        let canvas = mode != WorkspaceViewMode::Xaml;
        let code = mode != WorkspaceViewMode::Design;
        let split = mode == WorkspaceViewMode::Split;
        self.root
            .get_control::<Control>("CanvasToolbar")
            .set_is_visible(canvas);
        self.root
            .get_control::<Control>("ArtboardScroll")
            .set_is_visible(canvas);
        self.root
            .get_control::<Control>("SourceDivider")
            .set_is_visible(split);
        source.set_is_visible(code);
        let rows = layout.row_definitions();
        rows.get(1)
            .set_height(GridLength::from_pixels(if canvas { 44.0 } else { 0.0 }));
        rows.get(2).set_height(if canvas {
            GridLength::STAR
        } else {
            GridLength::from_pixels(0.0)
        });
        rows.get(3)
            .set_height(GridLength::from_pixels(if split { 5.0 } else { 0.0 }));
        rows.get(4).set_height(match mode {
            WorkspaceViewMode::Design => GridLength::from_pixels(0.0),
            WorkspaceViewMode::Split => GridLength::from_pixels(self.shell.split_height.get()),
            WorkspaceViewMode::Xaml => GridLength::STAR,
        });
        self.shell.view.set(mode);
        for (name, candidate) in [
            ("DesignView", WorkspaceViewMode::Design),
            ("SplitView", WorkspaceViewMode::Split),
            ("CodeView", WorkspaceViewMode::Xaml),
        ] {
            self.root
                .get_control::<Button>(name)
                .classes()
                .set("active", candidate == mode);
        }
    }

    pub fn navigation_pane(&self) -> NavigationPane {
        self.shell.navigation.get()
    }

    pub fn set_navigation_pane(&self, pane: NavigationPane) {
        self.shell.navigation.set(pane);
        for (tab, content, candidate) in [
            ("LayersTab", "LayersPane", NavigationPane::Layers),
            ("AssetsTab", "AssetsPane", NavigationPane::Assets),
            ("ProjectTab", "ProjectPane", NavigationPane::Project),
        ] {
            let selected = candidate == pane;
            self.root
                .get_control::<Button>(tab)
                .classes()
                .set("active", selected);
            self.root
                .get_control::<Control>(content)
                .set_is_visible(selected);
        }
    }

    pub fn timeline_visible(&self) -> bool {
        self.root
            .get_control::<Control>("TimelinePane")
            .is_visible()
    }

    /// Restores a user's resized timeline height when reopening the pane.
    pub fn set_timeline_visible(&self, visible: bool) {
        let timeline = self.root.get_control::<Control>("TimelinePane");
        if timeline.is_visible() && !visible && timeline.bounds().height >= 100.0 {
            self.shell.timeline_height.set(timeline.bounds().height);
        }
        timeline.set_is_visible(visible);
        self.root
            .get_control::<Control>("TimelineDivider")
            .set_is_visible(visible);
        let root = self.root.cast::<Grid>().expect("Workspace root is a Grid");
        root.row_definitions()
            .get(3)
            .set_height(GridLength::from_pixels(if visible { 1.0 } else { 0.0 }));
        root.row_definitions()
            .get(4)
            .set_height(GridLength::from_pixels(if visible {
                self.shell.timeline_height.get()
            } else {
                0.0
            }));
        self.root
            .get_control::<Button>("TimelineToggle")
            .classes()
            .set("active", visible);
    }

    /// Fits the whole artboard using the measured viewport and the canvas padding.
    pub fn fit_artboard(&self) -> Result<(), String> {
        let viewport = self.root.get_control::<ScrollViewer>("ArtboardScroll");
        let size = viewport.bounds().size();
        if !viewport.is_visible() || size.width <= 80.0 || size.height <= 80.0 {
            return Err("Show and arrange the design canvas before fitting the artboard".into());
        }
        let (width, height) = self.artboard_size();
        self.set_zoom(((size.width - 80.0) / width).min((size.height - 80.0) / height));
        Ok(())
    }

    pub(super) fn connect_shell(self: &Rc<Self>) {
        for (name, label) in [
            ("SelectTool", "Select and manipulate elements"),
            ("PreviewTool", "Interact with the preview"),
            ("ZoomOut", "Zoom out"),
            ("ZoomIn", "Zoom in"),
            ("ZoomReset", "Reset zoom to 100 percent"),
            ("ZoomFit", "Fit artboard"),
            ("NewAnimation", "New animation program"),
            ("DuplicateAnimation", "Duplicate animation program"),
            ("DeleteAnimation", "Delete animation program"),
        ] {
            ferroui_controls::automation::AutomationProperties::set_name(
                &self.root.get_control::<Button>(name),
                Some(label),
            );
        }
        for (name, alignment) in [
            ("AlignLeft", Alignment::Left),
            ("AlignCenter", Alignment::HorizontalCenter),
            ("AlignRight", Alignment::Right),
            ("AlignTop", Alignment::Top),
            ("AlignMiddle", Alignment::VerticalCenter),
            ("AlignBottom", Alignment::Bottom),
        ] {
            let weak = Rc::downgrade(self);
            self.root.get_control::<MenuItem>(name).click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    if this.pending_source.get() {
                        this.apply_source();
                    }
                    let result = this.session.borrow_mut().align_selection(alignment);
                    this.refresh(true);
                    this.report(result);
                }
            });
        }
        for (name, axis) in [
            ("DistributeHorizontal", Axis::Horizontal),
            ("DistributeVertical", Axis::Vertical),
        ] {
            let weak = Rc::downgrade(self);
            self.root.get_control::<MenuItem>(name).click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    if this.pending_source.get() {
                        this.apply_source();
                    }
                    let result = this.session.borrow_mut().distribute_selection(axis);
                    this.refresh(true);
                    this.report(result);
                }
            });
        }
        for (name, mode) in [
            ("DesignView", WorkspaceViewMode::Design),
            ("SplitView", WorkspaceViewMode::Split),
            ("CodeView", WorkspaceViewMode::Xaml),
        ] {
            let weak = Rc::downgrade(self);
            self.root.get_control::<Button>(name).click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.set_view_mode(mode);
                }
            });
        }
        for (name, pane) in [
            ("LayersTab", NavigationPane::Layers),
            ("AssetsTab", NavigationPane::Assets),
            ("ProjectTab", NavigationPane::Project),
        ] {
            let weak = Rc::downgrade(self);
            self.root.get_control::<Button>(name).click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.set_navigation_pane(pane);
                }
            });
        }
        let weak = Rc::downgrade(self);
        self.root
            .get_control::<Button>("TimelineToggle")
            .click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.set_timeline_visible(!this.timeline_visible());
                }
            });
        let weak = Rc::downgrade(self);
        self.root
            .get_control::<Button>("ZoomFit")
            .click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.report(this.fit_artboard());
                }
            });
        let weak = Rc::downgrade(self);
        self.root
            .get_control::<Button>("TimelineDetails")
            .click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let details = this.root.get_control::<Border>("AnimationDetails");
                    let visible = !details.is_visible();
                    details.set_is_visible(visible);
                    this.root
                        .get_control::<Button>("TimelineDetails")
                        .classes()
                        .set("active", visible);
                    let root = this.root.cast::<Grid>().expect("Workspace root is a Grid");
                    let row = root.row_definitions().get(4);
                    let current = this
                        .root
                        .get_control::<Control>("TimelinePane")
                        .bounds()
                        .height;
                    let height = if current >= 100.0 {
                        current
                    } else {
                        this.shell.timeline_height.get()
                    };
                    this.shell
                        .timeline_height
                        .set((height + if visible { 50.0 } else { -50.0 }).max(180.0));
                    row.set_height(GridLength::from_pixels(this.shell.timeline_height.get()));
                }
            });
        let weak = Rc::downgrade(self);
        self.root
            .add_handler(InputElement::key_down_event(), move |_, event| {
                if event.key == Key::F6 {
                    if let Some(this) = weak.upgrade() {
                        this.set_view_mode(match this.view_mode() {
                            WorkspaceViewMode::Design => WorkspaceViewMode::Split,
                            WorkspaceViewMode::Split => WorkspaceViewMode::Xaml,
                            WorkspaceViewMode::Xaml => WorkspaceViewMode::Design,
                        });
                        event.set_handled(true);
                    }
                }
            });
        self.refresh_shell();
    }

    pub(super) fn refresh_shell(&self) {
        for (name, mode) in [
            ("SourceMode", OutlineMode::Source),
            ("VisualMode", OutlineMode::Visual),
            ("LogicalMode", OutlineMode::Logical),
        ] {
            self.root
                .get_control::<Button>(name)
                .classes()
                .set("active", mode == self.outline_mode());
        }
        let selecting = self.surface.is_hit_test_visible();
        self.root
            .get_control::<Button>("SelectTool")
            .classes()
            .set("active", selecting);
        self.root
            .get_control::<Button>("PreviewTool")
            .classes()
            .set("active", !selecting);
        let session = self.session.borrow();
        let can_arrange = session.can_manipulate() && session.selection().len() > 1;
        self.root
            .get_control::<Button>("ArrangeSelection")
            .set_is_enabled(can_arrange);
        for name in ["DistributeHorizontal", "DistributeVertical"] {
            self.root
                .get_control::<MenuItem>(name)
                .set_is_enabled(can_arrange && session.selection().len() > 2);
        }
        drop(session);
        let (width, height) = self.artboard_size();
        self.root
            .get_control::<TextBlock>("CanvasSize")
            .set_text(Some(&format!("{width} × {height}")));
    }
}
