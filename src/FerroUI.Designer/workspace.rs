use crate::{
    DesignGesture, DesignSurface, DesignerSession, PreviewStatus, PropertyEditorContext,
    PropertyEditorFactory, RuntimeOutlineMode,
};
use ferroui_base::input::{InputElement, Key, KeyModifiers};
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::media::{MatrixTransform, SolidColorBrush};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::{Ref, Thickness, WeakRef};
use ferroui_controls::{
    Button, ContentControl, Control, Grid, Slider, StackPanel, TextBlock, TextBox,
};
use ferroui_designer_animation::Easing;
use ferroui_designer_core::NodeId;
use ferroui_inspect::Inspector;
use ferroui_markup_xaml::FromXamlObject;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;
use std::time::Duration;

#[path = "workspace_shell.rs"]
pub(crate) mod shell;

pub use inspector_drag::{OutlineCommand, OutlinePlacement};
pub use selection_tools::{CanvasLayerChoice, CanvasSelectionSnapshot};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutlineMode {
    #[default]
    Source,
    Visual,
    Logical,
}

/// A native, embeddable workspace. The host retains the returned `Rc` for the lifetime of the
/// view. Event callbacks capture it weakly, and dropping it stops the preview timer.
pub struct DesignerWorkspace {
    root: Ref<Control>,
    session: Rc<RefCell<DesignerSession>>,
    surface: Ref<DesignSurface>,
    artboard: Ref<Grid>,
    artboard_transform: Ref<Control>,
    source: Ref<TextBox>,
    inspectors: inspectors::InspectorControls,
    tracks: Ref<StackPanel>,
    status: Ref<TextBlock>,
    selection_title: Ref<TextBlock>,
    scrubber: Ref<Slider>,
    time: Ref<TextBox>,
    duration: Ref<TextBox>,
    guard: Cell<bool>,
    pending_source: Cell<bool>,
    source_changed_at: Cell<i64>,
    last_tick: Cell<i64>,
    last_revision: Cell<u64>,
    last_selection: RefCell<Vec<NodeId>>,
    last_generation: Cell<u64>,
    runtime_outline_mode: Cell<Option<RuntimeOutlineMode>>,
    runtime_selection: Cell<Option<u64>>,
    navigation: navigation::NavigationUi,
    guides: guides::GuidesUi,
    timer: RefCell<Option<Rc<DispatcherTimer>>>,
    property_editors: RefCell<Vec<Rc<dyn PropertyEditorFactory>>>,
    property_transfer: property_transfer::PropertyTransferUi,
    timeline_lanes: RefCell<Vec<WeakRef<crate::timeline_lane::TimelineLane>>>,
    toolbox: RefCell<Option<Rc<crate::asset_toolbox::AssetToolbox>>>,
    timeline_ui: RefCell<timeline::TimelineUi>,
    layout: RefCell<layout::LayoutUi>,
    file: RefCell<Option<Rc<dyn ferroui_base::platform::storage::IStorageFile>>>,
    saved_source: RefCell<String>,
    pending_open: RefCell<Option<files::PendingDocument>>,
    pending_close: RefCell<Option<Rc<dyn Fn()>>>,
    file_busy: Cell<bool>,
    project: RefCell<project::ProjectUi>,
    source_tools: RefCell<source_tools::SourceTools>,
    authoring: RefCell<authoring::AuthoringUi>,
    components: RefCell<components::ComponentUi>,
    animation_program_ids: RefCell<Vec<u64>>,
    #[cfg(feature = "collaboration-client")]
    sharing: RefCell<Option<Rc<crate::sharing::SharingController>>>,
    #[cfg(feature = "collaboration-client")]
    sharing_panel: RefCell<Option<Rc<sharing::SharingPanel>>>,
    shell: shell::WorkspaceShell,
    preview_environment: RefCell<preview_environment::PreviewEnvironmentUi>,
    preview_stages: preview_stages::PreviewStagesUi,
    prototype: prototype::PrototypeUi,
}

impl DesignerWorkspace {
    pub fn new(source: impl Into<String>) -> Result<Rc<Self>, String> {
        Self::create(source.into(), None)
    }
    /// Preserves the original document URI for relative resource and include resolution.
    pub fn with_uri(source: impl Into<String>, uri: String) -> Result<Rc<Self>, String> {
        Self::create(source.into(), Some(uri))
    }
    fn create(source: String, uri: Option<String>) -> Result<Rc<Self>, String> {
        FerroRuntimeXamlLoader::register();
        ferroui_markup_xaml::register_types();
        ferroui_controls_treedatagrid::register_types();
        let loaded =
            FerroRuntimeXamlLoader::load(include_str!("Workspace.xaml"), None, None, None, false)
                .map_err(|error| error.to_string())?;
        let root = Ref::<Control>::from_xaml_object(&loaded)
            .ok_or("Designer shell did not load a control")?;
        root.styles().add(crate::designer_chrome_style()?);
        let preview = root.get_control::<ContentControl>("Preview");
        let session = Rc::new(RefCell::new(match uri {
            Some(uri) if crate::rsx_document::RustDocument::for_uri(&uri) => {
                DesignerSession::with_rust_source(source, preview, uri)?
            }
            Some(uri) => DesignerSession::with_uri(source, preview, uri)?,
            None => DesignerSession::new(source, preview)?,
        }));
        let artboard = root.get_control::<Grid>("Artboard");
        let surface = DesignSurface::new();
        surface.attach(&session);
        artboard.children().add(surface.clone().upcast::<Control>());
        let now = Dispatcher::ui_thread().now();
        let saved_source = session.borrow().authored_source().to_owned();
        let this = Rc::new(Self {
            source: root.get_control("Source"),
            inspectors: inspectors::InspectorControls::new(&root),
            tracks: root.get_control("Tracks"),
            status: root.get_control("Status"),
            selection_title: root.get_control("SelectionTitle"),
            scrubber: root.get_control("Scrubber"),
            time: root.get_control("Time"),
            duration: root.get_control("Duration"),
            artboard_transform: root.get_control("ArtboardTransform"),
            root,
            session,
            surface,
            artboard,
            guard: Cell::new(false),
            pending_source: Cell::new(false),
            source_changed_at: Cell::new(now),
            last_tick: Cell::new(now),
            last_revision: Cell::new(u64::MAX),
            last_selection: RefCell::new(Vec::new()),
            last_generation: Cell::new(u64::MAX),
            runtime_outline_mode: Cell::new(None),
            runtime_selection: Cell::new(None),
            navigation: navigation::NavigationUi::default(),
            guides: guides::GuidesUi::default(),
            timer: RefCell::new(None),
            property_editors: RefCell::new(Vec::new()),
            property_transfer: property_transfer::PropertyTransferUi::default(),
            timeline_lanes: RefCell::new(Vec::new()),
            toolbox: RefCell::new(None),
            timeline_ui: RefCell::new(Default::default()),
            layout: RefCell::new(Default::default()),
            file: RefCell::new(None),
            saved_source: RefCell::new(saved_source),
            pending_open: RefCell::new(None),
            pending_close: RefCell::new(None),
            file_busy: Cell::new(false),
            project: RefCell::new(Default::default()),
            source_tools: RefCell::new(Default::default()),
            authoring: RefCell::new(Default::default()),
            components: RefCell::new(Default::default()),
            animation_program_ids: RefCell::new(Vec::new()),
            #[cfg(feature = "collaboration-client")]
            sharing: RefCell::new(None),
            #[cfg(feature = "collaboration-client")]
            sharing_panel: RefCell::new(None),
            shell: shell::WorkspaceShell::default(),
            preview_environment: RefCell::new(Default::default()),
            preview_stages: preview_stages::PreviewStagesUi::default(),
            prototype: prototype::PrototypeUi::default(),
        });
        this.connect();
        this.connect_inspectors();
        this.connect_inspector_drag();
        this.connect_components();
        this.connect_preview_environment();
        this.connect_preview_stages();
        this.connect_layer_commands();
        this.connect_selection_tools();
        this.connect_selection_layout();
        this.connect_property_transfer_commands();
        this.connect_focus_order_commands();
        this.connect_preview_checks();
        this.connect_columns();
        this.connect_rust_styles();
        this.connect_item_templates();
        this.connect_command_bindings();
        this.connect_design_time_commands();
        this.refresh(true);
        Ok(this)
    }

    pub fn register_property_editor(self: &Rc<Self>, editor: Rc<dyn PropertyEditorFactory>) {
        self.property_editors.borrow_mut().insert(0, editor);
        self.rebuild_properties();
        self.rebuild_layout();
    }
    pub fn control(&self) -> Ref<Control> {
        self.root.clone()
    }
    pub fn session(&self) -> Rc<RefCell<DesignerSession>> {
        self.session.clone()
    }
    pub fn design_surface(&self) -> Ref<DesignSurface> {
        self.surface.clone()
    }
    pub fn source_editor(&self) -> Ref<TextBox> {
        self.source.clone()
    }

    /// Shared ownership guard for workspace-bound selection authoring automation.
    /// A session-only host has no source-editor draft or runtime-inspection selection.
    #[cfg(feature = "mcp")]
    pub(crate) fn ensure_selection_workflow_context(&self) -> Result<(), String> {
        if self.pending_source.get() {
            return Err("Apply or cancel the XAML draft before using selection authoring tools".into());
        }
        if self.runtime_selection.get().is_some() {
            return Err("Select authored control layers before using selection authoring tools".into());
        }
        Ok(())
    }
    /// Returns the outline mode available to existing hosts. Combined uses the visual hierarchy.
    pub fn outline_mode(&self) -> OutlineMode {
        match self.runtime_outline_mode() {
            None => OutlineMode::Source,
            Some(RuntimeOutlineMode::Visual | RuntimeOutlineMode::Combined) => OutlineMode::Visual,
            Some(RuntimeOutlineMode::Logical) => OutlineMode::Logical,
        }
    }
    pub fn set_outline_mode(self: &Rc<Self>, mode: OutlineMode) {
        self.set_runtime_outline_mode(match mode {
            OutlineMode::Source => None,
            OutlineMode::Visual => Some(RuntimeOutlineMode::Visual),
            OutlineMode::Logical => Some(RuntimeOutlineMode::Logical),
        });
    }
    /// The runtime hierarchy shown by the Outline. Source mode returns `None`.
    pub fn runtime_outline_mode(&self) -> Option<RuntimeOutlineMode> {
        self.runtime_outline_mode.get()
    }
    /// Select the runtime hierarchy or pass `None` to return to the authored outline.
    pub fn set_runtime_outline_mode(self: &Rc<Self>, mode: Option<RuntimeOutlineMode>) {
        self.runtime_outline_mode.set(mode);
        if mode.is_none() {
            self.runtime_selection.set(None);
        }
        self.rebuild_outline();
        self.rebuild_properties();
        self.rebuild_layout();
        self.refresh_shell();
    }
    pub fn artboard_size(&self) -> (f64, f64) {
        (self.artboard.width(), self.artboard.height())
    }
    pub fn set_artboard_size(&self, width: f64, height: f64) -> Result<(), String> {
        self.set_preview_environment(crate::PreviewEnvironment {
            width,
            height,
            ..self.preview_environment()
        })
    }
    pub fn zoom(&self) -> f64 {
        self.canvas_viewport().zoom()
    }
    pub fn set_zoom(&self, zoom: f64) {
        let zoom = if zoom.is_finite() {
            zoom.max(crate::CanvasViewport::MIN_ZOOM)
        } else {
            1.0
        };
        if self.set_unmeasured_canvas_zoom(zoom) {
            return;
        }
        let canvas = self
            .root
            .get_control::<Control>("ArtboardScroll")
            .bounds()
            .size();
        let _ = self.zoom_canvas_at(
            zoom,
            ferroui_base::Point::new(canvas.width / 2.0, canvas.height / 2.0),
        );
    }

    fn connect(self: &Rc<Self>) {
        self.connect_shell();
        self.connect_canvas_navigation();
        self.connect_canvas_guides();
        self.connect_file_commands();
        self.connect_project();
        self.connect_source_tools();
        self.connect_event_handlers();
        self.connect_dictionary_extraction();
        self.connect_file_move();
        self.connect_registered_rename();
        self.connect_resource_move();
        self.connect_rust_source();
        self.connect_authoring();
        self.connect_timeline_curves();
        self.connect_sharing();
        for (name, action) in [
            ("Undo", Action::Undo),
            ("Redo", Action::Redo),
            ("Delete", Action::Delete),
            ("Duplicate", Action::Duplicate),
            ("ZoomOut", Action::ZoomOut),
            ("ZoomIn", Action::ZoomIn),
            ("ZoomReset", Action::ZoomReset),
            ("SelectTool", Action::Select),
            ("PanTool", Action::Pan),
            ("PreviewTool", Action::Interact),
            ("ApplySource", Action::Apply),
            ("SourceMode", Action::Source),
            ("VisualMode", Action::Visual),
            ("LogicalMode", Action::Logical),
            ("CombinedMode", Action::Combined),
            ("Play", Action::Play),
            ("Stop", Action::Stop),
            ("Record", Action::Record),
            ("Keyframe", Action::Keyframe),
            ("CommitAnimation", Action::CommitAnimation),
        ] {
            let weak = Rc::downgrade(self);
            self.root.get_control::<Button>(name).click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.perform(action);
                }
            });
        }
        let weak = Rc::downgrade(self);
        self.root
            .get_control::<TextBox>("PropertySearch")
            .text_changing(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.rebuild_properties();
                }
            });
        // Synchronize role changes before TextBox's native edit handlers run. Its
        // keyboard handler still allows caret navigation and selection while read-only.
        let weak = Rc::downgrade(self);
        self.source.add_handler_with(
            InputElement::key_down_event(),
            move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.source
                        .set_is_read_only(this.session.borrow().is_read_only());
                }
            },
            ferroui_base::interactivity::RoutingStrategies::TUNNEL,
            false,
        );
        let weak = Rc::downgrade(self);
        self.source.add_handler_with(
            InputElement::text_input_event(),
            move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.source
                        .set_is_read_only(this.session.borrow().is_read_only());
                }
            },
            ferroui_base::interactivity::RoutingStrategies::TUNNEL,
            false,
        );
        let weak = Rc::downgrade(self);
        self.source.text_changing(move |_, _| {
            if let Some(this) = weak.upgrade() {
                if !this.guard.get() {
                    let retained_source = {
                        let session = this.session.borrow();
                        session
                            .is_read_only()
                            .then(|| session.document().source().to_owned())
                    };
                    if let Some(source) = retained_source {
                        // A role change can precede the next UI refresh; programmatic text
                        // replacement also bypasses TextBox.IsReadOnly. Preserve the accepted
                        // source/draft and pending state without creating an authored edit.
                        this.guard.set(true);
                        this.source.set_is_read_only(true);
                        this.source.set_text(Some(&source));
                        this.guard.set(false);
                        return;
                    }
                    this.pending_source.set(true);
                    this.session
                        .borrow_mut()
                        .stage_source(this.source.text().unwrap_or_default());
                    this.source_changed_at.set(Dispatcher::ui_thread().now());
                }
            }
        });
        let weak = Rc::downgrade(self);
        self.scrubber.value_changed(move |_, _| {
            if let Some(this) = weak.upgrade() {
                if !this.guard.get() {
                    let result = this.seek_workspace_timeline(this.scrubber.value());
                    this.report(result);
                    this.surface.invalidate_visual();
                }
            }
        });
        for (field, is_duration) in [(self.time.clone(), false), (self.duration.clone(), true)] {
            let weak = Rc::downgrade(self);
            let weak_field = field.downgrade();
            field.add_handler(InputElement::key_down_event(), move |_, event| {
                if event.key != Key::Enter {
                    return;
                }
                if let (Some(this), Some(field)) = (weak.upgrade(), weak_field.upgrade()) {
                    let result = field
                        .text()
                        .unwrap_or_default()
                        .parse::<f64>()
                        .map_err(|_| "Enter a valid number of seconds".to_string())
                        .and_then(|value| {
                            if is_duration {
                                if this.comparing_animation_clips() { Ok(()) } else { this.session.borrow_mut().set_duration(value) }
                            } else {
                                this.seek_workspace_timeline(value)
                            }
                        });
                    this.report(result);
                    this.refresh(is_duration);
                    event.set_handled(true);
                }
            });
        }
        let weak = Rc::downgrade(self);
        self.surface.on_selection_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.runtime_selection.set(None);
                this.refresh(false);
            }
        });
        let weak = Rc::downgrade(self);
        self.surface.on_commit(move |gesture| {
            if let Some(this) = weak.upgrade() {
                let result = this.commit_gesture(gesture);
                this.report(result);
                this.refresh(true);
            }
        });
        let weak = Rc::downgrade(self);
        self.root
            .add_handler(InputElement::key_down_event(), move |_, event| {
                let Some(this) = weak.upgrade() else {
                    return;
                };
                let control = event
                    .key_modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::META);
                if control && matches!(event.key, Key::S | Key::O) {
                    if event.key == Key::O {
                        this.request_open();
                    } else {
                        this.request_save(event.key_modifiers.contains(KeyModifiers::SHIFT), false);
                    }
                    event.set_handled(true);
                    return;
                }
                if this.text_input_focused() {
                    return;
                }
                if event.key == Key::H && event.key_modifiers.is_empty() {
                    this.set_layout_helpers_visible(!this.layout_helpers_visible());
                    event.set_handled(true);
                    return;
                }
                let action = match (
                    event.key,
                    control,
                    event.key_modifiers.contains(KeyModifiers::SHIFT),
                ) {
                    (Key::Z, true, false) => Some(Action::Undo),
                    (Key::Z, true, true) | (Key::Y, true, _) => Some(Action::Redo),
                    (Key::Delete, false, _) => Some(Action::Delete),
                    (Key::D, true, _) => Some(Action::Duplicate),
                    (Key::V, false, false) if event.key_modifiers.is_empty() => {
                        Some(Action::Select)
                    }
                    (Key::I, false, false) if event.key_modifiers.is_empty() => {
                        Some(Action::Interact)
                    }
                    (Key::O, true, _) => {
                        this.request_open();
                        event.set_handled(true);
                        None
                    }
                    (Key::S, true, shift) => {
                        this.request_save(shift, false);
                        event.set_handled(true);
                        None
                    }
                    _ => None,
                };
                if let Some(action) = action {
                    this.perform(action);
                    event.set_handled(true);
                }
            });
        self.build_assets();
        let weak = Rc::downgrade(self);
        *self.timer.borrow_mut() = Some(DispatcherTimer::with_callback(
            Duration::from_millis(32),
            DispatcherPriority::BACKGROUND,
            move |timer| {
                let Some(this) = weak.upgrade() else {
                    timer.stop();
                    return;
                };
                let now = Dispatcher::ui_thread().now();
                this.poll_project();
                #[cfg(feature = "collaboration-client")]
                this.tick_sharing();
                let delta = (now - this.last_tick.replace(now)).max(0) as f64 / 1000.0;
                if this.pending_source.get() && now - this.source_changed_at.get() >= 180 {
                    this.apply_source();
                }
                if this.session.borrow().is_playing() {
                    let result = this.session.borrow_mut().advance(delta.min(0.25));
                    if let Err(error) = result {
                        this.session.borrow_mut().pause();
                        this.status.set_text(Some(&error));
                    }
                }
                if this.session.borrow().collaboration_version().is_some() {
                    let count = this.session.borrow().current_presence(30_000).len();
                    this.root
                        .get_control::<TextBlock>("PeerCount")
                        .set_text(Some(&format!(
                            "{count} peer{}",
                            if count == 1 { "" } else { "s" }
                        )));
                    this.surface.invalidate_visual();
                } else {
                    this.root
                        .get_control::<TextBlock>("PeerCount")
                        .set_text(Some(""));
                }
                this.refresh(false);
            },
        ));
    }

    fn perform(self: &Rc<Self>, action: Action) {
        let view_only = matches!(
            action,
            Action::ZoomOut
                | Action::ZoomIn
                | Action::ZoomReset
                | Action::Select
                | Action::Pan
                | Action::Interact
                | Action::Source
                | Action::Visual
                | Action::Logical
                | Action::Combined
        );
        if self.pending_source.get() && !view_only {
            self.apply_source();
        }
        let result = match action {
            Action::Undo => self.source_history(false),
            Action::Redo => self.source_history(true),
            Action::Delete => crate::workspace_tools::delete(&mut self.session.borrow_mut()),
            Action::Duplicate => self.duplicate(),
            Action::ZoomOut => {
                self.set_zoom(self.zoom() / 1.2);
                Ok(())
            }
            Action::ZoomIn => {
                self.set_zoom(self.zoom() * 1.2);
                Ok(())
            }
            Action::ZoomReset => {
                self.set_zoom(1.0);
                Ok(())
            }
            Action::Select => {
                self.set_canvas_pan_tool(false);
                self.surface.set_interactive(false);
                Ok(())
            }
            Action::Pan => {
                self.set_canvas_pan_tool(true);
                Ok(())
            }
            Action::Interact => {
                self.set_canvas_pan_tool(false);
                self.surface.set_interactive(true);
                Ok(())
            }
            Action::Apply => {
                self.apply_source();
                Ok(())
            }
            Action::Source => {
                self.set_outline_mode(OutlineMode::Source);
                Ok(())
            }
            Action::Visual => {
                self.set_outline_mode(OutlineMode::Visual);
                Ok(())
            }
            Action::Logical => {
                self.set_outline_mode(OutlineMode::Logical);
                Ok(())
            }
            Action::Combined => {
                self.set_runtime_outline_mode(Some(RuntimeOutlineMode::Combined));
                Ok(())
            }
            Action::Play => {
                let mut session = self.session.borrow_mut();
                if session.is_playing() {
                    session.pause();
                } else if let Some(range) = self.preview_range_for_view(&session) {
                    let result = session.start_preview_range(None, self.comparing_animation_clips(), range);
                    drop(session);
                    self.refresh(true);
                    self.report(result);
                    return;
                } else if self.comparing_animation_clips() {
                    let restart = session.animation.playback_completed.then_some(0.0);
                    let result = session.start_combined_playback(None, restart);
                    drop(session);
                    self.refresh(false);
                    self.report(result);
                    return;
                } else {
                    session.play();
                }
                Ok(())
            }
            Action::Stop => {
                self.session.borrow_mut().stop();
                Ok(())
            }
            Action::Record => {
                let mut session = self.session.borrow_mut();
                let value = !session.recording();
                session.set_recording(value);
                Ok(())
            }
            Action::Keyframe => self.record_selected_keyframe(),
            Action::CommitAnimation => self.session.borrow_mut().commit_animation(),
        };
        self.refresh(true);
        self.report(result);
    }
    fn report(&self, result: Result<(), String>) {
        if let Err(error) = result {
            self.status.set_text(Some(&error));
        }
    }
    fn apply_source(self: &Rc<Self>) {
        self.pending_source.set(false);
        let result = self.session.borrow_mut().apply_staged_preview();
        self.report(result);
        self.refresh(false);
    }

    pub fn refresh(self: &Rc<Self>, force: bool) {
        self.refresh_preview_stages();
        self.refresh_canvas_navigation();
        self.refresh_canvas_guides();
        let session = self.session.borrow();
        let revision = session.document().revision();
        let generation = session.generation();
        let selected = session.selection().to_vec();
        let changed = force
            || self.last_revision.get() != revision
            || *self.last_selection.borrow() != selected;
        let generation_changed = force || self.last_generation.get() != generation;
        if !changed && !generation_changed {
            return;
        }
        self.source.set_is_read_only(session.is_read_only());
        if changed && !self.pending_source.get() {
            let text = session.authored_source();
            if self.source.text().as_deref() != Some(text) {
                self.guard.set(true);
                self.source.set_text(Some(text));
                self.guard.set(false);
            }
        }
        let range = self.preview_range_for_view(&session);
        self.root.get_control::<Button>("TimelineRange").classes().set("active", range.is_some());
        self.root.get_control::<Button>("TimelineRange").set_is_enabled(!session.is_playing());
        for lane in self
            .timeline_lanes
            .borrow()
            .iter()
            .filter_map(WeakRef::upgrade)
        {
            lane.set_playhead(session.time());
            if lane.is_ruler() {
                lane.set_preview_range(range);
            }
        }
        self.guard.set(true);
        let duration = self.timeline_view_duration();
        self.scrubber.set_maximum(duration);
        self.scrubber
            .set_range_value(session.time().min(duration));
        if !self.time.is_focused() {
            self.time.set_text(Some(&format!("{:.2}", session.time())));
        }
        if !self.duration.is_focused() {
            self.duration
                .set_text(Some(&format!("{duration}")));
        }
        self.guard.set(false);
        self.root
            .get_control::<TextBlock>("RecordLabel")
            .set_text(Some(if session.recording() {
                "Recording"
            } else {
                "Record"
            }));
        self.root
            .get_control::<Button>("Record")
            .classes()
            .set("recording", session.recording());
        self.root
            .get_control::<Control>("PlayIcon")
            .set_is_visible(!session.is_playing());
        self.root
            .get_control::<Control>("PauseIcon")
            .set_is_visible(session.is_playing());
        self.root
            .get_control::<Button>("Undo")
            .set_is_enabled(session.can_undo());
        self.root
            .get_control::<Button>("Redo")
            .set_is_enabled(session.can_redo());
        let status = match session.preview_status() {
            PreviewStatus::Ready { revision } => format!(
                "Revision {revision}  ·  {} elements  ·  {} selected  ·  {} animation tracks",
                session.document().ast().elements().count(),
                selected.len(),
                session.timeline().tracks.len()
            ),
            PreviewStatus::InvalidSource { .. } => session
                .authored_diagnostics()
                .first()
                .map(|diagnostic| {
                    format!(
                        "{} {}:{}  {}  ·  preview retains last accepted document",
                        if session.uses_rust_source() { "Rust" } else { "XAML" },
                        diagnostic.line, diagnostic.column, diagnostic.message
                    )
                })
                .unwrap_or_else(|| "Invalid source".into()),
            PreviewStatus::Failed { message, .. } => format!("Preview: {message}"),
        };
        let status = if session.preview_notices().is_empty() {
            status
        } else {
            format!("{status}  ·  {}", session.preview_notices().join("; "))
        };
        let status = session
            .collaboration_error()
            .map(|error| format!("{status}  ·  {error}"))
            .unwrap_or(status);
        self.status.set_text(Some(&status));
        self.refresh_shell();
        self.last_revision.set(revision);
        self.last_generation.set(generation);
        *self.last_selection.borrow_mut() = selected;
        drop(session);
        self.refresh_layer_commands();
        self.refresh_selection_tools();
        self.refresh_selection_layout();
        self.refresh_property_transfer_commands();
        self.refresh_columns_command();
        self.refresh_rust_styles_command();
        self.refresh_item_template_command();
        self.refresh_command_binding();
        self.refresh_file_title();
        if changed || generation_changed {
            self.rebuild_outline();
        }
        if changed {
            self.rebuild_properties();
            self.rebuild_layout();
            self.rebuild_tracks();
            self.rebuild_document_tabs();
            self.refresh_source_tools();
            self.rebuild_authoring();
            self.refresh_components();
            self.refresh_animation_programs();
        }
        self.refresh_preview_markers();
        self.surface.invalidate_layout_helpers();
        self.surface.invalidate_visual();
    }

    fn build_assets(self: &Rc<Self>) {
        let toolbox = crate::asset_toolbox::AssetToolbox::new(
            crate::asset_toolbox::default_designer_assets(),
        );
        let weak = Rc::downgrade(self);
        toolbox.on_begin_drag(move |asset, event| {
            if let Some(this) = weak.upgrade() {
                this.surface.begin_asset_drag(asset.source.clone(), event);
                this.status
                    .set_text(Some(&format!("Drag {} onto the artboard", asset.title)));
            }
        });
        let weak = Rc::downgrade(self);
        toolbox.on_activate(move |asset| {
            if let Some(this) = weak.upgrade() {
                this.surface.arm_insert(asset.source.clone());
                this.status.set_text(Some(&format!(
                    "Click the artboard to place {}",
                    asset.title
                )));
            }
        });
        self.root
            .get_control::<ContentControl>("AssetHost")
            .set_content(Some(Control::boxed(toolbox.control())));
        *self.toolbox.borrow_mut() = Some(toolbox);
    }

    /// Register project-specific controls in the native searchable asset catalog.
    pub fn register_asset(
        self: &Rc<Self>,
        asset: crate::asset_toolbox::DesignerAsset,
    ) -> Result<(), String> {
        self.toolbox
            .borrow()
            .as_ref()
            .ok_or("The asset toolbox is not initialized")?
            .register(asset)
    }

    /// Access the reusable catalog to choose Grid/List presentation or configure filtering.
    /// Browsing state is independent of the authored document and its undo history.
    pub fn asset_toolbox(&self) -> Option<Rc<crate::AssetToolbox>> {
        self.toolbox.borrow().clone()
    }

    fn reveal_source(&self, id: NodeId) {
        let session = self.session.borrow();
        if let Some(span) = session.authored_span(id) {
            let source = session.authored_source();
            let begin = source_tools::utf16_from_byte(source, span.start);
            let end = source_tools::utf16_from_byte(source, span.end);
            self.source.set_selection_start(begin);
            self.source.set_selection_end(end);
        }
    }

    fn text_input_focused(&self) -> bool {
        let mut focused = ferroui_base::input::FocusManager::get_focus_manager(&self.root)
            .and_then(|manager| manager.get_focused_element())
            .map(|element| element.upcast::<ferroui_base::Visual>());
        while let Some(visual) = focused {
            if visual.cast::<TextBox>().is_some() {
                return true;
            }
            focused = visual.visual_parent();
        }
        false
    }
    fn commit_gesture(self: &Rc<Self>, gesture: DesignGesture) -> Result<(), String> {
        let revision = self.session.borrow().document().revision();
        if self.pending_source.get() {
            self.apply_source();
            if self.session.borrow().document().revision() != revision {
                return Err(
                    "Source changed before the gesture could commit; repeat the gesture".into(),
                );
            }
        }
        crate::workspace_tools::commit(&mut self.session.borrow_mut(), gesture)
    }
    fn duplicate(&self) -> Result<(), String> {
        crate::workspace_tools::duplicate(&mut self.session.borrow_mut())
    }
}
impl Drop for DesignerWorkspace {
    fn drop(&mut self) {
        #[cfg(feature = "collaboration-client")]
        if let Some(controller) = self.sharing.get_mut().take() {
            controller.disconnect();
        }
        if let Some(timer) = self.timer.get_mut().take() {
            timer.stop();
        }
    }
}
#[derive(Clone, Copy)]
enum Action {
    Undo,
    Redo,
    Delete,
    Duplicate,
    ZoomOut,
    ZoomIn,
    ZoomReset,
    Select,
    Pan,
    Interact,
    Apply,
    Source,
    Visual,
    Logical,
    Combined,
    Play,
    Stop,
    Record,
    Keyframe,
    CommitAnimation,
}
fn button(label: &str) -> Ref<Button> {
    let button = Button::new();
    button.set_content(Some(Rc::new(label.to_owned())));
    button
}

#[path = "workspace_animation.rs"]
mod authoring;
#[path = "workspace_components.rs"]
mod components;
pub use components::DesignerDefinitionAsset;
#[path = "workspace_design_time.rs"]
mod design_time;
#[path = "workspace_files.rs"]
mod files;
#[path = "workspace_rsx.rs"]
mod rust_source;
#[path = "workspace_focus_order.rs"]
mod focus_order;
#[path = "workspace_preview_checks.rs"]
mod preview_checks;
#[path = "workspace_columns.rs"]
mod columns;
#[path = "workspace_rust_styles.rs"]
mod rust_styles;
#[path = "inspector_actions.rs"]
mod inspector_actions;
#[path = "workspace_property_trace.rs"]
mod property_trace_view;
#[path = "workspace_item_templates.rs"]
mod item_templates;
#[path = "inspector_drag.rs"]
mod inspector_drag;
#[path = "inspector_controls.rs"]
mod inspectors;
#[path = "workspace_layers.rs"]
mod layer_commands;
#[path = "workspace_selection_tools.rs"]
mod selection_tools;
#[path = "workspace_selection_layout.rs"]
mod selection_layout;
#[path = "workspace_property_transfer.rs"]
mod property_transfer;
#[path = "workspace_layout.rs"]
mod layout;
#[path = "workspace_navigation.rs"]
mod navigation;
#[path = "workspace_guides.rs"]
mod guides;
pub use guides::{CanvasGuideEditSnapshot, CanvasGuideViewSnapshot};
#[path = "workspace_preview_environment.rs"]
mod preview_environment;
#[path = "workspace_preview_stages.rs"]
pub(crate) mod preview_stages;
#[path = "workspace_stage_sources.rs"]
mod stage_sources;
#[path = "workspace_stage_files.rs"]
mod stage_files;
#[path = "workspace_prototype.rs"]
pub(crate) mod prototype;
pub use prototype::PreviewPrototypePlaybackInfo;
#[path = "workspace_project.rs"]
mod project;
#[cfg(feature = "mcp")]
#[path = "workspace_project_library.rs"]
mod project_library;
#[path = "workspace_history.rs"]
mod history;
#[path = "workspace_event_handlers.rs"]
mod event_handlers;
#[path = "workspace_command_bindings.rs"]
mod command_bindings;
#[path = "workspace_dictionary_extraction.rs"]
mod dictionary_extraction;
#[path = "workspace_file_move.rs"]
mod file_move;
#[path = "workspace_reviewed_sources.rs"]
mod reviewed_sources;
#[path = "workspace_resource_rename.rs"]
mod resource_rename;
#[path = "workspace_resource_move.rs"]
mod resource_move;
#[path = "workspace_rust_review.rs"]
mod rust_review;
#[path = "workspace_registered_rename.rs"]
mod registered_rename;
#[path = "workspace_type_rename.rs"]
mod type_rename;
#[cfg(feature = "collaboration-client")]
#[path = "workspace_sharing.rs"]
mod sharing;

/// Without the feature `collaboration-client` the workspace has no sharing: the Share button of the shell is
/// hidden, and the sharing panel and its controller do not exist.
#[cfg(not(feature = "collaboration-client"))]
impl DesignerWorkspace {
    fn connect_sharing(self: &Rc<Self>) {
        self.root.get_control::<Button>("Share").set_is_visible(false);
    }
}
#[path = "source_editor.rs"]
mod source_tools;
#[path = "workspace_timeline.rs"]
mod timeline;

fn property_rank(name: &str) -> usize {
    const COMMON: &[&str] = &[
        "Name",
        "x:Name",
        "Width",
        "Height",
        "MinWidth",
        "MinHeight",
        "MaxWidth",
        "MaxHeight",
        "Margin",
        "Padding",
        "HorizontalAlignment",
        "VerticalAlignment",
        "Canvas.Left",
        "Canvas.Top",
        "Canvas.Right",
        "Canvas.Bottom",
        "Grid.Row",
        "Grid.Column",
        "Grid.RowSpan",
        "Grid.ColumnSpan",
        "Background",
        "Foreground",
        "BorderBrush",
        "BorderThickness",
        "CornerRadius",
        "Opacity",
        "IsVisible",
        "RenderTransform",
        "RenderTransformOrigin",
        "Content",
        "Text",
        "FontFamily",
        "FontSize",
        "FontWeight",
        "TextAlignment",
        "TextWrapping",
        "RowDefinitions",
        "ColumnDefinitions",
        "Orientation",
        "Spacing",
        "Rows",
        "Columns",
        "LastChildFill",
    ];
    COMMON
        .iter()
        .position(|candidate| *candidate == name)
        .unwrap_or_else(|| {
            if name.starts_with("AutomationProperties.")
                || name.starts_with("AccessKey")
                || name.starts_with("XamlSourceInfo.")
            {
                200
            } else {
                100
            }
        })
}
