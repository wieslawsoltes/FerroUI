//! Shell navigation is independent of document edits and runtime inspection.
//! Hiding a pane keeps its control, selection, pending text and scroll state alive.
use super::{DesignerWorkspace, OutlineCommand, OutlineMode};
use ferroui_base::input::{InputElement, Key, KeyModifiers};
use ferroui_controls::{
    Border, Button, Control, Grid, GridLength, MenuItem, ScrollViewer, TextBlock,
};
use ferroui_designer_tools::{Alignment, Axis};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

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
    canvas_focus: Cell<bool>,
    dock_widths: Cell<(f64, f64)>,
    focused_timeline_was_visible: Cell<bool>,
    arrange_ast: RefCell<Option<Arc<ferroui_designer_core::Ast>>>,
    arrange_selection: RefCell<Vec<ferroui_designer_core::NodeId>>,
    arrange_enabled: Cell<bool>,
}

impl Default for WorkspaceShell {
    fn default() -> Self {
        Self {
            view: Cell::new(WorkspaceViewMode::Design),
            navigation: Cell::new(NavigationPane::Layers),
            split_height: Cell::new(260.0),
            timeline_height: Cell::new(220.0),
            canvas_focus: Cell::new(false),
            dock_widths: Cell::new((248.0, 320.0)),
            focused_timeline_was_visible: Cell::new(true),
            arrange_ast: RefCell::new(None),
            arrange_selection: RefCell::new(Vec::new()),
            arrange_enabled: Cell::new(false),
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
            .get_control::<Control>("CanvasViewport")
            .set_is_visible(canvas);
        self.root
            .get_control::<Control>("SourceDivider")
            .set_is_visible(split);
        source.set_is_visible(code);
        let rows = layout.row_definitions();
        rows.get(1)
            .set_height(GridLength::from_pixels(if canvas { 32.0 } else { 0.0 }));
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

    /// Whether the workspace is temporarily giving its side docks and timeline to the canvas.
    pub fn canvas_focused(&self) -> bool {
        self.shell.canvas_focus.get()
    }

    /// Temporarily hide the side docks and timeline without rebuilding their controls.
    /// Returning restores their measured widths, timeline height and previous visibility.
    /// Document selection, pending source, panel modes and scroll positions stay intact.
    pub fn set_canvas_focused(&self, focused: bool) {
        if focused == self.canvas_focused() {
            return;
        }
        let layout = self.root.get_control::<Grid>("MainLayout");
        let columns = layout.column_definitions();
        if focused {
            let current = self.shell.dock_widths.get();
            let left = columns.get(0).actual_width();
            let right = columns.get(4).actual_width();
            self.shell.dock_widths.set((
                if left >= 120.0 { left } else { current.0 },
                if right >= 160.0 { right } else { current.1 },
            ));
            self.shell
                .focused_timeline_was_visible
                .set(self.timeline_visible());
            self.set_timeline_visible(false);
        } else {
            self.set_timeline_visible(self.shell.focused_timeline_was_visible.get());
        }
        let (left, right) = self.shell.dock_widths.get();
        for (index, width) in [(0, left), (1, 1.0), (3, 1.0), (4, right)] {
            columns
                .get(index)
                .set_width(GridLength::from_pixels(if focused { 0.0 } else { width }));
        }
        for name in [
            "NavigationDock",
            "NavigationDivider",
            "InspectorDock",
            "InspectorDivider",
        ] {
            self.root
                .get_control::<Control>(name)
                .set_is_visible(!focused);
        }
        self.shell.canvas_focus.set(focused);
        self.root
            .get_control::<Button>("FocusCanvas")
            .classes()
            .set("active", focused);
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
        if !viewport.is_visible() || size.width <= 80.0 || size.height <= 116.0 {
            return Err("Show and arrange the design canvas before fitting the artboard".into());
        }
        let (width, height) = self.artboard_size();
        self.set_zoom(((size.width - 80.0) / width).min((size.height - 116.0) / height));
        Ok(())
    }

    pub(super) fn connect_shell(self: &Rc<Self>) {
        for (name, command) in outline_commands() {
            let weak = Rc::downgrade(self);
            self.root.get_control::<MenuItem>(name).click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let result = this.edit_outline(command);
                    this.report(result);
                }
            });
        }
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
            ("FocusCanvas", "Focus canvas and restore panels"),
            (
                "CollapseLayers",
                "Collapse layers, keeping selected paths visible",
            ),
            ("Play", "Play or pause animation"),
            ("Stop", "Stop animation and rewind"),
            ("PlayAll", "Play all animation programs"),
            ("Record", "Toggle keyframe recording"),
            ("Keyframe", "Add keyframe at playhead"),
            ("CommitAnimation", "Save animation to XAML"),
            ("TimelineToggle", "Show or hide timeline"),
            ("TimelineDetails", "Animation name and target"),
            ("AnimationSource", "Reveal animation XAML"),
            ("AnimationSettings", "Animation settings"),
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
            .get_control::<Button>("CollapseLayers")
            .click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.collapse_outline();
                }
            });
        let weak = Rc::downgrade(self);
        self.root
            .get_control::<Button>("FocusCanvas")
            .click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.set_canvas_focused(!this.canvas_focused());
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
                if event.key == Key::F
                    && event
                        .key_modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::META)
                    && event.key_modifiers.contains(KeyModifiers::SHIFT)
                {
                    if let Some(this) = weak.upgrade() {
                        this.set_canvas_focused(!this.canvas_focused());
                        event.set_handled(true);
                    }
                    return;
                }
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
        let can_arrange = session.can_manipulate() && !session.selection().is_empty();
        self.root
            .get_control::<Button>("ArrangeSelection")
            .set_is_enabled(can_arrange);
        for name in [
            "AlignLeft",
            "AlignCenter",
            "AlignRight",
            "AlignTop",
            "AlignMiddle",
            "AlignBottom",
        ] {
            self.root
                .get_control::<MenuItem>(name)
                .set_is_enabled(can_arrange && session.selection().len() > 1);
        }
        // Playback updates shell status each frame. Structural command planning depends on
        // authored state/selection/access, so it must not walk the AST at animation frequency.
        let ast = session.document().ast_snapshot();
        let can_edit = can_arrange
            && !session.is_read_only()
            && !self.pending_source.get()
            && self.outline_mode() == OutlineMode::Source;
        let commands_changed = self.shell.arrange_enabled.get() != can_edit
            || self.shell.arrange_selection.borrow().as_slice() != session.selection()
            || !self
                .shell
                .arrange_ast
                .borrow()
                .as_ref()
                .is_some_and(|old| Arc::ptr_eq(old, &ast));
        if commands_changed {
            for (name, command) in outline_commands() {
                self.root
                    .get_control::<MenuItem>(name)
                    .set_is_enabled(can_edit && session.can_edit_outline(command));
            }
            *self.shell.arrange_ast.borrow_mut() = Some(ast);
            *self.shell.arrange_selection.borrow_mut() = session.selection().to_vec();
            self.shell.arrange_enabled.set(can_edit);
        }
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
        let session = self.session.borrow();
        let uri = session.document_uri();
        self.root
            .get_control::<TextBlock>("ArtboardTitle")
            .set_text(Some(
                uri.rsplit('/')
                    .next()
                    .filter(|name| !name.is_empty())
                    .unwrap_or("Artboard"),
            ));
        self.root
            .get_control::<TextBlock>("OutlineHint")
            .set_text(Some(if self.outline_mode() == OutlineMode::Source {
                "Drag to reorder · Alt + arrows to nest"
            } else {
                "Live inspection · select to reveal source"
            }));
    }
}

fn outline_commands() -> [(&'static str, OutlineCommand); 6] {
    [
        ("OutlineMoveFirst", OutlineCommand::MoveFirst),
        ("OutlineMoveLast", OutlineCommand::MoveLast),
        ("OutlineMoveUp", OutlineCommand::MoveUp),
        ("OutlineMoveDown", OutlineCommand::MoveDown),
        ("OutlineIndent", OutlineCommand::Indent),
        ("OutlineOutdent", OutlineCommand::Outdent),
    ]
}
