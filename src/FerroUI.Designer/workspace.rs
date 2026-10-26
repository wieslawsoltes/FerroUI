use crate::{
    DesignGesture, DesignSurface, DesignerSession, PreviewStatus, PropertyEditorContext,
    PropertyEditorFactory,
};
use ferroui_base::input::{InputElement, Key, KeyModifiers};
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::media::{Brushes, MatrixTransform, SolidColorBrush};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::{Matrix, Ref, Thickness, WeakRef};
use ferroui_controls::{
    Button, ContentControl, Control, Grid, LayoutTransformControl, Slider, StackPanel, TextBlock,
    TextBox, WrapPanel,
};
use ferroui_designer_animation::{AnimationValue, Easing};
use ferroui_designer_core::NodeId;
use ferroui_inspect::{Inspector, TreeKind};
use ferroui_markup_xaml::FromXamlObject;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;
use std::time::Duration;

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
    artboard_transform: Ref<LayoutTransformControl>,
    source: Ref<TextBox>,
    outline: Ref<StackPanel>,
    properties: Ref<StackPanel>,
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
    outline_mode: Cell<OutlineMode>,
    runtime_selection: Cell<Option<u64>>,
    zoom: Cell<f64>,
    timer: RefCell<Option<Rc<DispatcherTimer>>>,
    property_editors: RefCell<Vec<Rc<dyn PropertyEditorFactory>>>,
    timeline_lanes: RefCell<Vec<WeakRef<crate::timeline_lane::TimelineLane>>>,
    file: RefCell<Option<Rc<dyn ferroui_base::platform::storage::IStorageFile>>>,
    saved_source: RefCell<String>,
    pending_open: RefCell<Option<files::PendingDocument>>,
    pending_close: RefCell<Option<Rc<dyn Fn()>>>,
    file_busy: Cell<bool>,
    project: RefCell<project::ProjectUi>,
    source_tools: RefCell<source_tools::SourceTools>,
    authoring: RefCell<authoring::AuthoringUi>,
    animation_program_ids: RefCell<Vec<u64>>,
    sharing: RefCell<Option<Rc<crate::sharing::SharingController>>>,
    sharing_panel: RefCell<Option<Rc<sharing::SharingPanel>>>,
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
        let loaded =
            FerroRuntimeXamlLoader::load(include_str!("Workspace.xaml"), None, None, None, false)
                .map_err(|error| error.to_string())?;
        let root = Ref::<Control>::from_xaml_object(&loaded)
            .ok_or("Designer shell did not load a control")?;
        let preview = root.get_control::<ContentControl>("Preview");
        let session = Rc::new(RefCell::new(match uri {
            Some(uri) => DesignerSession::with_uri(source, preview, uri)?,
            None => DesignerSession::new(source, preview)?,
        }));
        let artboard = root.get_control::<Grid>("Artboard");
        let surface = DesignSurface::new();
        surface.attach(&session);
        artboard.children().add(surface.clone().upcast::<Control>());
        let now = Dispatcher::ui_thread().now();
        let saved_source = session.borrow().document().source().to_owned();
        let this = Rc::new(Self {
            source: root.get_control("Source"),
            outline: root.get_control("Outline"),
            properties: root.get_control("Properties"),
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
            outline_mode: Cell::new(OutlineMode::Source),
            runtime_selection: Cell::new(None),
            zoom: Cell::new(1.0),
            timer: RefCell::new(None),
            property_editors: RefCell::new(Vec::new()),
            timeline_lanes: RefCell::new(Vec::new()),
            file: RefCell::new(None),
            saved_source: RefCell::new(saved_source),
            pending_open: RefCell::new(None),
            pending_close: RefCell::new(None),
            file_busy: Cell::new(false),
            project: RefCell::new(Default::default()),
            source_tools: RefCell::new(Default::default()),
            authoring: RefCell::new(Default::default()),
            animation_program_ids: RefCell::new(Vec::new()),
            sharing: RefCell::new(None),
            sharing_panel: RefCell::new(None),
        });
        this.connect();
        this.refresh(true);
        Ok(this)
    }

    pub fn register_property_editor(self: &Rc<Self>, editor: Rc<dyn PropertyEditorFactory>) {
        self.property_editors.borrow_mut().insert(0, editor);
        self.rebuild_properties();
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
    pub fn outline_mode(&self) -> OutlineMode {
        self.outline_mode.get()
    }
    pub fn set_outline_mode(self: &Rc<Self>, mode: OutlineMode) {
        self.outline_mode.set(mode);
        if mode == OutlineMode::Source {
            self.runtime_selection.set(None);
        }
        self.rebuild_outline();
        self.rebuild_properties();
    }
    pub fn artboard_size(&self) -> (f64, f64) {
        (self.artboard.width(), self.artboard.height())
    }
    pub fn set_artboard_size(&self, width: f64, height: f64) -> Result<(), String> {
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err("Artboard dimensions must be finite and positive".into());
        }
        self.artboard.set_width(width);
        self.artboard.set_height(height);
        Ok(())
    }
    pub fn zoom(&self) -> f64 {
        self.zoom.get()
    }
    pub fn set_zoom(&self, zoom: f64) {
        let zoom = if zoom.is_finite() {
            zoom.clamp(0.1, 4.0)
        } else {
            1.0
        };
        self.zoom.set(zoom);
        self.surface.set_zoom(zoom);
        self.artboard_transform.set_layout_transform(Some(
            MatrixTransform::with_matrix(Matrix::create_scale(zoom, zoom)).into(),
        ));
        self.root
            .get_control::<Button>("ZoomReset")
            .set_content(Some(Rc::new(format!("{}%", (zoom * 100.0).round() as i32))));
        self.surface.invalidate_visual();
    }

    fn connect(self: &Rc<Self>) {
        self.connect_file_commands();
        self.connect_project();
        self.connect_source_tools();
        self.connect_authoring();
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
            ("PreviewTool", Action::Interact),
            ("ApplySource", Action::Apply),
            ("SourceMode", Action::Source),
            ("VisualMode", Action::Visual),
            ("LogicalMode", Action::Logical),
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
        let weak = Rc::downgrade(self);
        self.source.text_changing(move |_, _| {
            if let Some(this) = weak.upgrade() {
                if !this.guard.get() {
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
                    let result = this.session.borrow_mut().seek(this.scrubber.value());
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
                                this.session.borrow_mut().set_duration(value)
                            } else {
                                this.session.borrow_mut().seek(value)
                            }
                        });
                    this.report(result);
                    this.refresh(true);
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
                let action = match (
                    event.key,
                    control,
                    event.key_modifiers.contains(KeyModifiers::SHIFT),
                ) {
                    (Key::Z, true, false) => Some(Action::Undo),
                    (Key::Z, true, true) | (Key::Y, true, _) => Some(Action::Redo),
                    (Key::Delete, false, _) => Some(Action::Delete),
                    (Key::D, true, _) => Some(Action::Duplicate),
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
        if self.pending_source.get() {
            self.apply_source();
        }
        let result = match action {
            Action::Undo => self
                .session
                .borrow_mut()
                .undo()
                .map(|_| ())
                .map_err(|error| error.to_string()),
            Action::Redo => self
                .session
                .borrow_mut()
                .redo()
                .map(|_| ())
                .map_err(|error| error.to_string()),
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
                self.surface.set_interactive(false);
                Ok(())
            }
            Action::Interact => {
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
            Action::Play => {
                let mut session = self.session.borrow_mut();
                if session.is_playing() {
                    session.pause();
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
            Action::Keyframe => {
                let target = self.session.borrow().selection().first().copied();
                if let Some(target) = target {
                    let value = self
                        .session
                        .borrow()
                        .preview
                        .visual(target)
                        .map(|visual| visual.opacity())
                        .ok_or_else(|| {
                            "The selected element has no current live preview".to_owned()
                        });
                    value.and_then(|value| {
                        self.session.borrow_mut().record_keyframe(
                            target,
                            "Opacity",
                            AnimationValue::Number(value),
                        )
                    })
                } else {
                    Err("Select an element to keyframe its opacity; Record also captures edited properties".into())
                }
            }
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
            let text = session.document().source();
            if self.source.text().as_deref() != Some(text) {
                self.guard.set(true);
                self.source.set_text(Some(text));
                self.guard.set(false);
            }
        }
        for lane in self
            .timeline_lanes
            .borrow()
            .iter()
            .filter_map(WeakRef::upgrade)
        {
            lane.set_playhead(session.time());
        }
        self.guard.set(true);
        self.scrubber.set_maximum(session.timeline().duration);
        self.scrubber
            .set_range_value(session.time().min(session.timeline().duration));
        if !self.time.is_focused() {
            self.time.set_text(Some(&format!("{:.2}", session.time())));
        }
        if !self.duration.is_focused() {
            self.duration
                .set_text(Some(&format!("{}", session.timeline().duration)));
        }
        self.guard.set(false);
        self.root
            .get_control::<Button>("Record")
            .set_content(Some(Rc::new(if session.recording() {
                "● Recording".to_owned()
            } else {
                "Record off".to_owned()
            })));
        self.root
            .get_control::<Button>("Play")
            .set_content(Some(Rc::new(if session.is_playing() {
                "Pause".to_owned()
            } else {
                "Play".to_owned()
            })));
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
                .document()
                .diagnostics()
                .first()
                .map(|diagnostic| {
                    format!(
                        "XAML {}:{}  {}  ·  preview retains last accepted document",
                        diagnostic.line, diagnostic.column, diagnostic.message
                    )
                })
                .unwrap_or_else(|| "Invalid XAML".into()),
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
        self.last_revision.set(revision);
        self.last_generation.set(generation);
        *self.last_selection.borrow_mut() = selected;
        drop(session);
        self.refresh_file_title();
        if changed {
            self.rebuild_outline();
            self.rebuild_properties();
            self.rebuild_tracks();
            self.rebuild_document_tabs();
            self.refresh_source_tools();
            self.rebuild_authoring();
            self.refresh_animation_programs();
        }
        self.surface.invalidate_visual();
    }

    fn build_assets(self: &Rc<Self>) {
        let assets = self.root.get_control::<WrapPanel>("Assets");
        for (name, source) in [
            (
                "Button",
                "<Button Content=\"Button\" Width=\"120\" Height=\"36\" />",
            ),
            ("TextBlock", "<TextBlock Text=\"Text\" FontSize=\"18\" />"),
            ("TextBox", "<TextBox Text=\"Edit me\" Width=\"180\" />"),
            ("CheckBox", "<CheckBox Content=\"Option\" />"),
            (
                "Border",
                "<Border Width=\"180\" Height=\"100\" Background=\"#FFE0E7FF\" CornerRadius=\"8\" />",
            ),
            (
                "Grid",
                "<Grid Width=\"220\" Height=\"160\" Background=\"#FFF1F5F9\" RowDefinitions=\"*,*\" ColumnDefinitions=\"*,*\" />",
            ),
            (
                "StackPanel",
                "<StackPanel Width=\"180\" Height=\"140\" Background=\"#FFF1F5F9\" Spacing=\"8\" />",
            ),
            (
                "Canvas",
                "<Canvas Width=\"240\" Height=\"160\" Background=\"#FFF1F5F9\" />",
            ),
            (
                "WrapPanel",
                "<WrapPanel Width=\"240\" Height=\"160\" Background=\"#FFF1F5F9\" />",
            ),
            (
                "DockPanel",
                "<DockPanel Width=\"240\" Height=\"160\" Background=\"#FFF1F5F9\" />",
            ),
            (
                "Rectangle",
                "<Rectangle Width=\"120\" Height=\"80\" Fill=\"#FF6D91FF\" />",
            ),
            (
                "Ellipse",
                "<Ellipse Width=\"100\" Height=\"100\" Fill=\"#FFA78BFA\" />",
            ),
        ] {
            let button = button(name);
            button.set_margin(Thickness::uniform(2.0));
            let weak = Rc::downgrade(self);
            let source = source.to_owned();
            button.add_handler(InputElement::pointer_pressed_event(), move |_, event| {
                if let Some(this) = weak.upgrade() {
                    this.surface.begin_asset_drag(source.clone(), event);
                    this.status.set_text(Some(
                        "Drag onto the artboard, or click a location to insert",
                    ));
                }
            });
            assets.children().add(button.upcast::<Control>());
        }
    }

    fn rebuild_outline(self: &Rc<Self>) {
        self.outline.children().clear();
        let session = self.session.borrow();
        let selected = session.selection().to_vec();
        let rows = match self.outline_mode.get() {
            OutlineMode::Source => {
                let mut elements = session.document().ast().elements().collect::<Vec<_>>();
                elements.sort_by_key(|node| node.span.start);
                elements
                    .into_iter()
                    .map(|node| {
                        let mut depth = 0;
                        let mut parent = node.parent;
                        while let Some(id) = parent {
                            depth += 1;
                            parent = session
                                .document()
                                .ast()
                                .node(id)
                                .and_then(|node| node.parent);
                        }
                        (
                            Some(node.id),
                            None,
                            depth,
                            format!(
                                "{}{}",
                                node.name,
                                node.xaml_name()
                                    .map(|name| format!("  {name}"))
                                    .unwrap_or_default()
                            ),
                        )
                    })
                    .collect::<Vec<_>>()
            }
            mode => session
                .runtime_tree(if mode == OutlineMode::Visual {
                    TreeKind::Visual
                } else {
                    TreeKind::Logical
                })
                .into_iter()
                .map(|row| {
                    (
                        row.source_id,
                        Some(row.inspector_id),
                        row.depth,
                        format!(
                            "{}{}{}",
                            row.type_name,
                            row.name.map(|name| format!("  {name}")).unwrap_or_default(),
                            if row.source_id.is_none() {
                                "  [runtime]"
                            } else {
                                ""
                            }
                        ),
                    )
                })
                .collect(),
        };
        drop(session);
        for (id, runtime, depth, title) in rows {
            let row = button(&title);
            row.set_horizontal_alignment(HorizontalAlignment::Stretch);
            row.set_horizontal_content_alignment(HorizontalAlignment::Left);
            row.set_margin(Thickness::new((depth * 12).min(100) as f64, 0.0, 0.0, 0.0));
            row.set_font_size(11.0);
            if id.is_some_and(|id| selected.contains(&id))
                || (runtime.is_some() && runtime == self.runtime_selection.get())
            {
                row.set_background(Some(SolidColorBrush::from_uint32(0xff334568).into()));
            }
            let weak = Rc::downgrade(self);
            row.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.runtime_selection.set(runtime);
                    if let Some(id) = id {
                        this.session.borrow_mut().select([id]);
                        this.reveal_source(id);
                    }
                    this.refresh(true);
                }
            });
            self.outline.children().add(row.upcast::<Control>());
        }
    }

    fn reveal_source(&self, id: NodeId) {
        let session = self.session.borrow();
        if !session.document().diagnostics().is_empty() {
            return;
        }
        if let Some(node) = session.document().ast().node(id) {
            let source = session.document().source();
            let begin = source[..node.span.start].encode_utf16().count() as i32;
            let end = source[..node.span.end].encode_utf16().count() as i32;
            self.source.set_selection_start(begin);
            self.source.set_selection_end(end);
        }
    }

    fn rebuild_properties(self: &Rc<Self>) {
        self.properties.children().clear();
        if let Some(id) = self.runtime_selection.get() {
            if self.rebuild_runtime_properties(id) {
                return;
            }
            self.runtime_selection.set(None);
        }
        let session = self.session.borrow();
        let selected = session.selection().to_vec();
        let Some(first) = selected
            .first()
            .and_then(|id| session.document().ast().node(*id))
        else {
            self.selection_title.set_text(Some("Select an element"));
            return;
        };
        self.selection_title.set_text(Some(&if selected.len() == 1 {
            first.name.clone()
        } else {
            format!("{} elements", selected.len())
        }));
        let mut properties = vec![
            "Name",
            "Width",
            "Height",
            "MinWidth",
            "MinHeight",
            "Margin",
            "Padding",
            "HorizontalAlignment",
            "VerticalAlignment",
            "Opacity",
            "Background",
            "Foreground",
            "Text",
            "Content",
            "FontSize",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        properties.extend(
            first
                .attributes
                .iter()
                .filter(|attribute| !attribute.name.starts_with("xmlns"))
                .map(|attribute| attribute.name.clone()),
        );
        if let Some(parent) = first
            .parent
            .and_then(|id| session.document().ast().node(id))
        {
            match parent.name.rsplit(':').next().unwrap_or_default() {
                "Canvas" => properties.extend(
                    ["Canvas.Left", "Canvas.Top", "Canvas.Right", "Canvas.Bottom"]
                        .map(str::to_owned),
                ),
                "Grid" => properties.extend(
                    ["Grid.Row", "Grid.Column", "Grid.RowSpan", "Grid.ColumnSpan"]
                        .map(str::to_owned),
                ),
                "DockPanel" => properties.push("DockPanel.Dock".into()),
                _ => {}
            }
        }
        match first.name.rsplit(':').next().unwrap_or_default() {
            "Grid" => properties.extend(["RowDefinitions", "ColumnDefinitions"].map(str::to_owned)),
            "StackPanel" | "WrapPanel" => {
                properties.extend(["Orientation", "Spacing"].map(str::to_owned))
            }
            "UniformGrid" => {
                properties.extend(["Rows", "Columns", "FirstColumn"].map(str::to_owned))
            }
            "DockPanel" => properties.push("LastChildFill".into()),
            _ => {}
        }
        let live = session.preview.visual(first.id);
        let infos = live
            .as_ref()
            .map(|object| Inspector::properties(object))
            .unwrap_or_default();
        let known = live.as_ref().map(|_| {
            infos
                .iter()
                .filter(|info| !info.read_only)
                .map(|info| info.name.clone())
                .collect::<BTreeSet<_>>()
        });
        if let Some(known) = &known {
            properties.extend(known.iter().cloned());
        }
        let effective = infos
            .iter()
            .map(|info| {
                (
                    info.name.clone(),
                    (
                        ferroui_inspect::display_value(Some(&info.value)),
                        format!(
                            "{}{}",
                            info.priority_name(),
                            if info.has_binding { " · binding" } else { "" }
                        ),
                    ),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        properties.sort();
        properties.dedup();
        properties.sort_by(|a, b| property_rank(a).cmp(&property_rank(b)).then(a.cmp(b)));
        let runtime_types = infos
            .iter()
            .map(|info| {
                (
                    info.name.clone(),
                    info.property.property_type_name().to_owned(),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let filter = self
            .root
            .get_control::<TextBox>("PropertySearch")
            .text()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let rows = properties
            .into_iter()
            .filter(|name| {
                first.attribute(name).is_some()
                    || name.contains('.')
                    || name == "Name"
                    || known.as_ref().is_none_or(|known| known.contains(name))
            })
            .filter(|name| filter.is_empty() || name.to_ascii_lowercase().contains(&filter))
            .map(|name| {
                let first_value = first.attribute(&name).map(str::to_owned);
                let mixed = selected.iter().skip(1).any(|id| {
                    session
                        .document()
                        .ast()
                        .node(*id)
                        .and_then(|node| node.attribute(&name))
                        != first_value.as_deref()
                });
                (name, if mixed { None } else { first_value }, mixed)
            })
            .collect::<Vec<_>>();
        drop(session);
        for (name, value, mixed) in rows {
            let panel = StackPanel::new();
            panel.set_spacing(3.0);
            let label = TextBlock::new();
            label.set_text(Some(&name));
            label.set_font_size(11.0);
            label.set_foreground(Some(SolidColorBrush::from_uint32(0xffaebbd3).into()));
            let weak = Rc::downgrade(self);
            let property = name.clone();
            let ids = selected.clone();
            let context = PropertyEditorContext {
                property: name.clone(),
                targets: selected.clone(),
                value,
                mixed,
                runtime_type: runtime_types.get(&name).cloned(),
                apply: Rc::new(move |value| {
                    let Some(this) = weak.upgrade() else {
                        return;
                    };
                    if this.pending_source.get() {
                        this.apply_source();
                    }
                    let result = match value {
                        None => this.session.borrow_mut().reset_property(&ids, &property),
                        Some(value) => this
                            .session
                            .borrow_mut()
                            .set_property(&ids, &property, &value),
                    };
                    this.refresh(true);
                    this.report(result);
                }),
            };
            let factories = self.property_editors.borrow().clone();
            let editor = factories
                .iter()
                .find_map(|factory| factory.create(&context))
                .unwrap_or_else(|| crate::editors::built_in(&context));
            let heading = StackPanel::new();
            heading.set_orientation(Orientation::Horizontal);
            heading.set_spacing(8.0);
            heading.children().add(label.upcast::<Control>());
            let reset = button("Reset");
            reset.set_font_size(9.0);
            reset.set_padding(Thickness::new(5.0, 1.0, 5.0, 1.0));
            let apply = context.apply.clone();
            reset.click(move |_, _| apply(None));
            heading.children().add(reset.upcast::<Control>());
            panel.children().add(heading.upcast::<Control>());
            panel.children().add(editor);
            if let Some((value, origin)) = effective.get(&name) {
                let detail = TextBlock::new();
                detail.set_text(Some(&format!("{origin}  ·  {value}")));
                detail.set_font_size(9.0);
                detail.set_foreground(Some(Brushes::gray()));
                panel.children().add(detail.upcast::<Control>());
            }
            self.properties.children().add(panel.upcast::<Control>());
        }
        let hint = TextBlock::new();
        hint.set_text(Some(
            "Enter applies • Reset inherits • Record creates keyframes",
        ));
        hint.set_font_size(10.0);
        hint.set_foreground(Some(Brushes::gray()));
        self.properties.children().add(hint.upcast::<Control>());
    }

    fn rebuild_runtime_properties(self: &Rc<Self>, id: u64) -> bool {
        let Ok(object) = Inspector::shared().borrow().resolve(id) else {
            return false;
        };
        self.selection_title.set_text(Some(&format!(
            "{}  ·  runtime #{id}",
            object.get_type().name()
        )));
        if !self.session.borrow().selection().is_empty() {
            let edit = button("Edit authored properties");
            let weak = Rc::downgrade(self);
            edit.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.runtime_selection.set(None);
                    this.rebuild_properties();
                }
            });
            self.properties.children().add(edit.upcast::<Control>());
        }
        let filter = self
            .root
            .get_control::<TextBox>("PropertySearch")
            .text()
            .unwrap_or_default()
            .to_ascii_lowercase();
        for info in Inspector::properties(&object)
            .into_iter()
            .filter(|info| filter.is_empty() || info.name.to_ascii_lowercase().contains(&filter))
        {
            let row = StackPanel::new();
            row.set_spacing(2.0);
            let label = TextBlock::new();
            label.set_text(Some(&info.name));
            label.set_font_size(11.0);
            label.set_foreground(Some(SolidColorBrush::from_uint32(0xffaebbd3).into()));
            let value = TextBox::new();
            value.set_text(Some(&ferroui_inspect::display_value(Some(&info.value))));
            value.set_is_read_only(true);
            value.set_font_size(11.0);
            let origin = TextBlock::new();
            origin.set_text(Some(&format!(
                "{}{}{}",
                info.priority_name(),
                if info.has_binding { " · binding" } else { "" },
                if info.read_only { " · read only" } else { "" }
            )));
            origin.set_font_size(10.0);
            origin.set_foreground(Some(Brushes::gray()));
            row.children().add(label.upcast::<Control>());
            row.children().add(value.upcast::<Control>());
            row.children().add(origin.upcast::<Control>());
            self.properties.children().add(row.upcast::<Control>());
        }
        for frame in Inspector::styles(&object) {
            let text = TextBlock::new();
            text.set_text(Some(&format!(
                "{}: {} · {}",
                frame.kind,
                frame.selector.unwrap_or_default(),
                if frame.active { "active" } else { "inactive" }
            )));
            text.set_font_size(10.0);
            text.set_foreground(Some(Brushes::gray()));
            self.properties.children().add(text.upcast::<Control>());
        }
        true
    }

    fn rebuild_tracks(self: &Rc<Self>) {
        self.tracks.children().clear();
        self.timeline_lanes.borrow_mut().clear();
        let session = self.session.borrow();
        let tracks = session.timeline().tracks.clone();
        let warnings = session.animation_warnings().to_vec();
        let duration = session.timeline().duration;
        let revision = session.document().revision();
        let playhead = session.time();
        drop(session);
        for warning in warnings {
            let label = TextBlock::new();
            label.set_text(Some(&warning));
            label.set_font_size(10.0);
            label.set_foreground(Some(SolidColorBrush::from_uint32(0xffe3b879).into()));
            label.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
            self.tracks.children().add(label.upcast::<Control>());
        }
        for track in tracks {
            let target_name = self
                .session
                .borrow()
                .document()
                .ast()
                .node(track.target)
                .map(|node| {
                    node.xaml_name()
                        .unwrap_or_else(|| node.local_name())
                        .to_owned()
                })
                .unwrap_or_else(|| "Missing target".into());
            let track_title = format!("{} / {}", target_name, track.property);
            let plot = Grid::new();
            plot.set_column_definitions(
                ferroui_controls::ColumnDefinitions::parse("170,*")
                    .expect("valid timeline columns"),
            );
            let name = TextBlock::new();
            name.set_text(Some(&track_title));
            name.set_font_size(11.0);
            name.set_foreground(Some(SolidColorBrush::from_uint32(0xffc4d1ec).into()));
            name.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
            plot.children().add(name.upcast::<Control>());
            let lane = crate::timeline_lane::TimelineLane::new();
            lane.configure(
                duration,
                track.keyframes.iter().map(|key| key.time).collect(),
            );
            lane.set_playhead(playhead);
            let weak = Rc::downgrade(self);
            let target = track.target;
            lane.on_activate(move |time| {
                if let Some(this) = weak.upgrade() {
                    this.session.borrow_mut().select([target]);
                    let result = this.session.borrow_mut().seek(time);
                    this.refresh(true);
                    this.report(result);
                }
            });
            let weak = Rc::downgrade(self);
            let property = track.property.clone();
            lane.on_move(move |from,to| {if let Some(this)=weak.upgrade(){
                let result=if this.session.borrow().document().revision()!=revision {Err("The timeline changed during the drag; retry with the current keyframes".into())}else{this.session.borrow_mut().move_keyframe(target,&property,from,to)};
                this.refresh(true);this.report(result);
            }});
            self.timeline_lanes.borrow_mut().push(lane.downgrade());
            Grid::set_column(&lane, 1);
            plot.children().add(lane.upcast::<Control>());
            self.tracks.children().add(plot.upcast::<Control>());
            let row = StackPanel::new();
            row.set_orientation(Orientation::Horizontal);
            row.set_spacing(6.0);
            let title = TextBlock::new();
            title.set_text(Some(&track_title));
            title.set_width(170.0);
            title.set_foreground(Some(SolidColorBrush::from_uint32(0xffd0d9ed).into()));
            title.set_font_size(11.0);
            row.children().add(title.upcast::<Control>());
            for key in track.keyframes {
                let key_button = button(&format!("◆ {:.2}s", key.time));
                key_button.set_font_size(11.0);
                let weak = Rc::downgrade(self);
                let target = track.target;
                let property = track.property.clone();
                let time = key.time;
                key_button.click(move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        this.session.borrow_mut().select([target]);
                        let result = this.session.borrow_mut().seek(time);
                        this.report(result);
                        this.refresh(true);
                    }
                });
                row.children().add(key_button.upcast::<Control>());
                let move_time = TextBox::new();
                move_time.set_width(58.0);
                move_time.set_text(Some(&key.time.to_string()));
                move_time.set_font_size(10.0);
                let weak = Rc::downgrade(self);
                let weak_time = move_time.downgrade();
                let move_property = property.clone();
                move_time.add_handler(InputElement::key_down_event(), move |_, event| {
                    if event.key != Key::Enter {
                        return;
                    }
                    if let (Some(this), Some(input)) = (weak.upgrade(), weak_time.upgrade()) {
                        let result = input
                            .text()
                            .unwrap_or_default()
                            .parse::<f64>()
                            .map_err(|_| "Invalid keyframe time".to_owned())
                            .and_then(|to| {
                                if (0.0..=duration).contains(&to) {
                                    this.session.borrow_mut().move_keyframe(
                                        target,
                                        &move_property,
                                        time,
                                        to,
                                    )
                                } else {
                                    Err("Keyframe is outside the timeline".into())
                                }
                            });
                        this.refresh(true);
                        this.report(result);
                        event.set_handled(true);
                    }
                });
                row.children().add(move_time.upcast::<Control>());
                let curve = TextBox::new();
                curve.set_width(130.0);
                curve.set_font_size(10.0);
                curve.set_text(Some(&key.easing.to_xaml()));
                let easing = button("Apply curve");
                easing.set_font_size(10.0);
                let weak = Rc::downgrade(self);
                let weak_curve = curve.downgrade();
                let easing_property = property.clone();
                let apply_curve: Rc<dyn Fn()> = Rc::new(move || {
                    if let (Some(this), Some(input)) = (weak.upgrade(), weak_curve.upgrade()) {
                        let result = Easing::parse(&input.text().unwrap_or_default())
                            .map_err(|error| error.to_string())
                            .and_then(|easing| {
                                this.session.borrow_mut().set_keyframe_easing(
                                    target,
                                    &easing_property,
                                    time,
                                    easing,
                                )
                            });
                        this.refresh(true);
                        this.report(result);
                    }
                });
                let click_curve = apply_curve.clone();
                easing.click(move |_, _| click_curve());
                curve.add_handler(InputElement::key_down_event(), move |_, event| {
                    if event.key == Key::Enter {
                        apply_curve();
                        event.set_handled(true);
                    }
                });
                row.children().add(curve.upcast::<Control>());
                row.children().add(easing.upcast::<Control>());
                let remove = button("×");
                remove.set_font_size(10.0);
                let weak = Rc::downgrade(self);
                remove.click(move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        let result = this
                            .session
                            .borrow_mut()
                            .remove_keyframe(target, &property, time);
                        this.refresh(true);
                        this.report(result);
                    }
                });
                row.children().add(remove.upcast::<Control>());
            }
            self.tracks.children().add(row.upcast::<Control>());
        }
        if self.tracks.children().is_empty() {
            let hint = TextBlock::new();
            hint.set_text(Some("Select an element, enable Record, then edit a property at different times. Keyframes are saved as native FerroUI XAML."));
            hint.set_foreground(Some(SolidColorBrush::from_uint32(0xff9aa4b9).into()));
            hint.set_font_size(12.0);
            self.tracks.children().add(hint.upcast::<Control>());
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
    Interact,
    Apply,
    Source,
    Visual,
    Logical,
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
#[path = "workspace_files.rs"]
mod files;
#[path = "workspace_project.rs"]
mod project;
#[path = "workspace_sharing.rs"]
mod sharing;
#[path = "source_editor.rs"]
mod source_tools;

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
