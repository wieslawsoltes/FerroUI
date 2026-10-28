use ferroui_base::animation::{IClock, PlayState, TimeSpan};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingPriority;
use ferroui_base::reactive::IDisposable;
use ferroui_base::reactive::{Disposable, IObservable, IObserver};
use ferroui_base::{BoxedValue, FerroObject, Rect, Ref, StyledElement, Visual, WeakRef};
use ferroui_controls::{ContentControl, Control};
use ferroui_designer_animation::Sample;
use ferroui_designer_core::{Document, NodeId};
use ferroui_inspect::{Inspector, TreeKind};
use ferroui_markup_xaml::diagnostics::XamlSourceInfo;
use ferroui_markup_xaml::FromXamlObject;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_PREVIEW_OPTIONS: AtomicU64 = AtomicU64::new(1);

#[cfg(debug_assertions)]
struct ReloadDriver {
    session: ferroui_hot_reload::HotReloadSession,
    target: Rc<crate::preview_host::FactoryReloadTarget>,
    use_factory: Rc<std::cell::Cell<bool>>,
}

/// The authored document and preview have independent acceptance states.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewStatus {
    Ready { revision: u64 },
    InvalidSource { revision: u64 },
    Failed { revision: u64, message: String },
}

/// One live-tree row; inspector identities remain shared with MCP and DevTools.
#[derive(Clone, Debug)]
pub struct RuntimeNode {
    pub inspector_id: u64,
    pub source_id: Option<NodeId>,
    pub parent: Option<u64>,
    pub depth: usize,
    pub type_name: String,
    pub name: Option<String>,
    pub bounds: Option<Rect>,
}

pub(crate) struct RuntimePreview {
    pub host: Ref<ContentControl>,
    uri: String,
    options: crate::preview_host::PreviewHostOptions,
    options_epoch: u64,
    isolated: bool,
    native_window: Option<Ref<ferroui_controls::Window>>,
    window_projection: bool,
    pub(crate) definition_scope: Option<crate::DefinitionScope>,
    definition_projection: Option<crate::definition_projection::DefinitionProjection>,
    design_projection: Option<crate::design_time::DesignTimeProjection>,
    design_samples: crate::design_time_samples::SampleState,
    pub status: PreviewStatus,
    pub notices: Vec<String>,
    objects: BTreeMap<NodeId, WeakRef<Visual>>,
    reverse_objects: BTreeMap<u64, NodeId>,
    animation_values: Vec<Rc<dyn IDisposable>>,
    #[cfg(debug_assertions)]
    reload_driver: Option<ReloadDriver>,
}

impl RuntimePreview {
    pub fn new(
        host: Ref<ContentControl>,
        uri: String,
        document: &Document,
    ) -> Result<Self, String> {
        Self::new_with_options(host, uri, document, Default::default())
    }
    pub fn new_with_options(
        host: Ref<ContentControl>,
        uri: String,
        document: &Document,
        options: crate::preview_host::PreviewHostOptions,
    ) -> Result<Self, String> {
        Self::construct(host, uri, document, options, None, false)
    }

    /// Independent secondary previews never publish a competing source overlay for
    /// the authored URI. They load a complete native candidate with the original
    /// resource base URI and registered factory request, in both debug and release.
    pub(crate) fn new_isolated(
        host: Ref<ContentControl>,
        uri: String,
        document: &Document,
        options: crate::preview_host::PreviewHostOptions,
        scope: Option<crate::DefinitionScope>,
    ) -> Result<Self, String> {
        Self::construct(host, uri, document, options, scope, true)
    }

    fn construct(
        host: Ref<ContentControl>,
        uri: String,
        document: &Document,
        mut options: crate::preview_host::PreviewHostOptions,
        scope: Option<crate::DefinitionScope>,
        isolated: bool,
    ) -> Result<Self, String> {
        FerroRuntimeXamlLoader::register();
        ferroui_markup_xaml::register_types();
        for register in &options.register_types {
            register();
        }
        options.configuration.design_mode = true;
        options.configuration.set_create_source_info(true);
        host.set_clock(Some(Rc::new(DesignClock)));
        #[cfg(debug_assertions)]
        let reload_driver = if isolated {
            None
        } else {
            let mut session = ferroui_hot_reload::HotReloadSession::for_runtime_documents();
            session
                .register_document(
                    uri.clone(),
                    crate::preview_source::project(document)
                        .map(|projection| projection.source)
                        .unwrap_or_else(|_| document.valid_source().to_owned()),
                )
                .map_err(|error| format!("{error:?}"))?;
            let use_factory = Rc::new(std::cell::Cell::new(false));
            let target = Rc::new(crate::preview_host::FactoryReloadTarget::new(
                &host,
                &uri,
                options.clone(),
                use_factory.clone(),
            )?);
            let registered: Rc<dyn ferroui_hot_reload::ReloadTarget> = target.clone();
            session.register_target(&registered);
            Some(ReloadDriver {
                session,
                target,
                use_factory,
            })
        };
        let mut this = Self {
            host,
            uri,
            options,
            options_epoch: NEXT_PREVIEW_OPTIONS.fetch_add(1, Ordering::Relaxed),
            isolated,
            native_window: None,
            window_projection: false,
            definition_scope: scope,
            definition_projection: None,
            design_projection: None,
            design_samples: Default::default(),
            status: PreviewStatus::Ready { revision: 0 },
            notices: Vec::new(),
            objects: BTreeMap::new(),
            reverse_objects: BTreeMap::new(),
            animation_values: Vec::new(),
            #[cfg(debug_assertions)]
            reload_driver,
        };
        this.reload_document(document, true);
        Ok(this)
    }

    /// Change the host factory without registering a second owner for the same URI.
    /// A failed factory leaves the accepted graph and previous factory configuration intact.
    pub(crate) fn configure(
        &mut self,
        mut options: crate::preview_host::PreviewHostOptions,
        document: &Document,
    ) -> Result<(), String> {
        for register in &options.register_types {
            register();
        }
        options.configuration.design_mode = true;
        options.configuration.set_create_source_info(true);
        #[cfg(debug_assertions)]
        let previous_target = self.reload_driver.as_ref().map(|driver| {
            driver
                .target
                .replace_configuration(&self.host, options.clone())
        });
        let previous = std::mem::replace(&mut self.options, options);
        self.reload_document(document, true);
        if let PreviewStatus::Failed { message, .. } = &self.status {
            let error = message.clone();
            self.options = previous;
            #[cfg(debug_assertions)]
            if let (Some(driver), Some(previous_target)) = (&self.reload_driver, previous_target) {
                driver.target.restore_configuration(previous_target);
            }
            return Err(error);
        }
        self.options_epoch = NEXT_PREVIEW_OPTIONS.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub(crate) fn options(&self) -> crate::PreviewHostOptions {
        self.options.clone()
    }

    pub(crate) fn options_epoch(&self) -> u64 {
        self.options_epoch
    }

    pub(crate) fn replace_document(
        &mut self,
        uri: String,
        document: &Document,
    ) -> Result<(), String> {
        let canonical = |value: &str| {
            ferroui_base::utilities::Uri::absolute(value)
                .map(|uri| uri.absolute_uri().to_owned())
                .map_err(|error| error.to_string())
        };
        if canonical(&uri)? == canonical(&self.uri)? {
            self.uri = uri;
            self.definition_scope = None;
            self.definition_projection = None;
            self.reload_document(document, true);
        } else {
            let mut preview =
                Self::new_with_options(self.host.clone(), uri, document, self.options.clone())?;
            preview.options_epoch = self.options_epoch;
            *self = preview;
        }
        Ok(())
    }

    pub fn reload(&mut self, document: &Document) {
        self.reload_document(document, false);
    }

    pub(crate) fn design_time_enabled(&self) -> bool {
        self.options.design_time_data
    }

    pub(crate) fn set_design_time_enabled(
        &mut self,
        enabled: bool,
        document: &Document,
    ) -> Result<(), String> {
        if enabled == self.options.design_time_data {
            return Ok(());
        }
        let status = self.status.clone();
        let notices = self.notices.clone();
        let options_epoch = self.options_epoch;
        let mut options = self.options.clone();
        options.design_time_data = enabled;
        if let Err(error) = self.configure(options, document) {
            self.status = status;
            self.notices = notices;
            return Err(error);
        }
        // Each secondary stage owns its design-data choice. Focusing another main
        // preview is not a change to their shared factory or loader services.
        self.options_epoch = options_epoch;
        Ok(())
    }

    /// A scope change is presentation state. A rejected projection keeps both the last
    /// accepted native tree and its original-source identity map intact.
    pub(crate) fn set_definition_scope(
        &mut self,
        scope: Option<crate::DefinitionScope>,
        document: &Document,
    ) -> Result<(), String> {
        let previous_scope = std::mem::replace(&mut self.definition_scope, scope);
        let previous_status = self.status.clone();
        let previous_notices = self.notices.clone();
        let previous_window = self.window_projection;
        #[cfg(debug_assertions)]
        let previous_factory = self
            .reload_driver
            .as_ref()
            .map(|driver| driver.use_factory.get());
        self.reload_document(document, true);
        if let PreviewStatus::Failed { message, .. } = &self.status {
            let message = message.clone();
            self.definition_scope = previous_scope;
            self.status = previous_status;
            self.notices = previous_notices;
            self.window_projection = previous_window;
            #[cfg(debug_assertions)]
            if let (Some(driver), Some(previous_factory)) = (&self.reload_driver, previous_factory)
            {
                driver.use_factory.set(previous_factory);
            }
            return Err(message);
        }
        Ok(())
    }

    fn reload_document(&mut self, document: &Document, initialize: bool) {
        let _design_mode =
            crate::preview_host::DesignModeGuard::new(self.options.configuration.design_mode);
        self.clear_animation();
        if self
            .definition_scope
            .as_ref()
            .is_some_and(|scope| document.ast().node(scope.node).is_none())
        {
            self.definition_scope = None;
        }
        // A root control definition is already an independently loadable document.
        // Keep its application-owned constructor/code-behind and source coordinates;
        // only nested definitions need the synthetic sample host.
        let root_control_scope = self.definition_scope.as_ref().is_some_and(|scope| {
            scope.node == document.ast().root()
                && matches!(
                    scope.kind,
                    ferroui_designer_tools::ComponentKind::UserControl
                        | ferroui_designer_tools::ComponentKind::CustomControl
                )
        });
        let synthetic_scope = self.definition_scope.is_some() && !root_control_scope;
        self.window_projection = !synthetic_scope
            && document
                .ast()
                .node(document.ast().root())
                .is_some_and(|node| {
                    node.local_name() == "Window"
                        && node.namespace_uri.as_deref().is_some_and(|uri| {
                            matches!(
                                uri,
                                "https://github.com/ferroui"
                                    | "using:FerroUI.Controls"
                                    | "clr-namespace:FerroUI.Controls;assembly=FerroUI.Controls"
                            )
                        })
                });
        let invalid = !document.diagnostics().is_empty();
        if invalid && !initialize {
            self.status = PreviewStatus::InvalidSource {
                revision: document.revision(),
            };
            return;
        }
        let actual_factory =
            !synthetic_scope && self.options.factory(document).is_some() && !self.window_projection;
        #[cfg(debug_assertions)]
        if let Some(driver) = &self.reload_driver {
            driver.use_factory.set(
                !synthetic_scope
                    && (actual_factory
                        || self.options.resource_factory.is_some()
                        || document
                            .ast()
                            .node(document.ast().root())
                            .is_some_and(|node| {
                                matches!(
                                    node.local_name(),
                                    "Styles"
                                        | "Style"
                                        | "ResourceDictionary"
                                        | "ControlTheme"
                                        | "Application"
                                        | "DataTemplate"
                                )
                            })),
            );
        }
        let mut candidate_definition = None;
        let mut candidate_design = None;
        let result = if let Some(scope) = self.definition_scope.as_ref().filter(|_| synthetic_scope)
        {
            crate::definition_projection::project(document, scope).and_then(|projection| {
                let design = crate::design_time::project(
                    &projection.document,
                    projection.document.valid_source(),
                    self.options.design_time_data,
                )?;
                self.apply_source(
                    design
                        .as_ref()
                        .map_or(projection.document.valid_source(), |design| {
                            design.document.valid_source()
                        }),
                    false,
                )?;
                self.notices = projection.notices.clone();
                if let Some(design) = &design {
                    self.notices.extend(design.notices.clone());
                }
                candidate_design = design;
                candidate_definition = Some(projection);
                Ok(())
            })
        } else if actual_factory {
            self.notices = vec!["Preview uses the registered application root factory".into()];
            crate::design_time::project(
                document,
                document.valid_source(),
                self.options.design_time_data,
            )
            .and_then(|design| {
                // The factory receives exact authored source. PreviewRequest::populate
                // loads the same projection used here to map its native result.
                self.apply_source(document.valid_source(), true)?;
                if let Some(design) = &design {
                    self.notices.extend(design.notices.clone());
                }
                candidate_design = design;
                Ok(())
            })
        } else {
            crate::preview_source::project(document).and_then(|projection| {
                self.notices = projection.notices;
                let design = crate::design_time::project(
                    document,
                    &projection.source,
                    self.options.design_time_data,
                )?;
                self.apply_source(
                    design
                        .as_ref()
                        .map_or(projection.source.as_str(), |design| {
                            design.document.valid_source()
                        }),
                    // A Window's synthetic Border (or another projection root)
                    // must not accidentally match a factory registered for that
                    // different authored type. Actual factories use the branch above.
                    false,
                )?;
                if let Some(design) = &design {
                    self.notices.extend(design.notices.clone());
                }
                candidate_design = design;
                Ok(())
            })
        };
        if result.is_ok() {
            self.definition_projection = candidate_definition;
            self.design_projection = candidate_design;
            self.design_samples = Default::default();
            if root_control_scope {
                if let Some(scope) = &self.definition_scope {
                    self.notices.push(format!(
                        "Editing definition: {}. Changes author the original document.",
                        scope.title
                    ));
                }
            }
        }
        self.status = match result {
            Ok(()) => PreviewStatus::Ready {
                revision: document.revision(),
            },
            Err(message) => PreviewStatus::Failed {
                revision: document.revision(),
                message,
            },
        };
        if matches!(self.status, PreviewStatus::Ready { .. }) {
            self.host.update_layout();
            self.rebuild_map(document);
            if self
                .native_window
                .as_ref()
                .is_some_and(|window| window.is_visible())
            {
                if let Err(error) = self.open_native(document.valid_source().to_owned()) {
                    self.notices.push(format!("Native preview: {error}"));
                }
            }
            if invalid {
                self.status = PreviewStatus::InvalidSource {
                    revision: document.revision(),
                };
            }
        }
    }

    fn apply_source(&mut self, source: &str, allow_root_factories: bool) -> Result<(), String> {
        #[cfg(debug_assertions)]
        if let Some(driver) = &mut self.reload_driver {
            driver.session.submit(self.uri.clone(), source.to_owned());
            if let Some(outcome) = driver.session.apply_pending() {
                if outcome.status != ferroui_hot_reload::ReloadStatus::Applied {
                    return Err(outcome
                        .diagnostic
                        .unwrap_or_else(|| format!("{:?}", outcome.status)));
                }
            }
            return Ok(());
        }
        // Release previews and isolated comparison stages stage complete graphs.
        // The designer's sole authoring preview retains development state restoration.
        let mut options = self.options.clone();
        if !allow_root_factories {
            options.root_factories.clear();
        }
        let candidate = options.control(source, &self.uri)?;
        if self.isolated
            && (candidate.parent().is_some()
                || candidate.visual_parent().is_some()
                || candidate.cast::<ferroui_controls::TopLevel>().is_some())
        {
            return Err(
                "A preview factory must return a fresh, unparented control for each stage".into(),
            );
        }
        self.host.set_content(Some(Control::boxed(candidate)));
        Ok(())
    }

    pub fn is_window_projection(&self) -> bool {
        self.window_projection
    }
    pub fn open_native(&mut self, source: String) -> Result<Ref<ferroui_controls::Window>, String> {
        let _design_mode =
            crate::preview_host::DesignModeGuard::new(self.options.configuration.design_mode);
        let document = Document::new(&source).map_err(|error| error.to_string())?;
        let design =
            crate::design_time::project(&document, &source, self.options.design_time_data)?;
        let loaded = self.options.load(&source, &self.uri)?;
        let sample_root = Ref::<Control>::from_xaml_object(&loaded);
        let window =
            if let Some(window) = Ref::<ferroui_controls::Window>::from_xaml_object(&loaded) {
                window
            } else {
                let control = Ref::<Control>::from_xaml_object(&loaded)
                    .or_else(|| self.options.control(&source, &self.uri).ok())
                    .ok_or("The native preview requires a control or a configured resource view")?;
                let window = ferroui_controls::Window::new();
                window.set_content(Some(Control::boxed(control)));
                window
            };
        window.show();
        if let (Some(root), Some(projection)) = (sample_root, design) {
            let mut state = crate::design_time_samples::SampleState::default();
            self.notices
                .extend(crate::design_time_samples::apply_projection(
                    &root,
                    &projection,
                    &mut state,
                ));
        }
        if let Some(previous) = self.native_window.replace(window.clone()) {
            previous.close();
        }
        Ok(window)
    }
    pub fn close_native(&mut self) {
        if let Some(window) = self.native_window.take() {
            window.close();
        }
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn set_interactive(&mut self, interactive: bool) {
        self.clear_animation();
        self.host.set_clock(if interactive {
            None
        } else {
            Some(Rc::new(DesignClock))
        });
    }

    pub fn root(&self) -> Option<Ref<Control>> {
        self.host
            .content()
            .as_ref()
            .and_then(Ref::<Control>::from_xaml_object)
    }

    pub fn visual(&self, id: NodeId) -> Option<Ref<Visual>> {
        self.objects.get(&id).and_then(WeakRef::upgrade)
    }

    /// Stage scenarios address one authored owner. A template can produce several
    /// live instances of that source node; choosing the ordinary map's first item
    /// would apply a scenario to an arbitrary instance. Keep normal authoring map
    /// behavior unchanged and require a unique current object for this operation.
    pub(crate) fn unique_scenario_visual(&self, id: NodeId) -> Result<Option<Ref<Visual>>, String> {
        let Some(root) = self.root() else {
            return Ok(None);
        };
        let root_visual = root.clone().upcast::<Visual>();
        let root_logical = root.upcast::<StyledElement>();
        let inspector = Inspector::shared();
        let inspector = inspector.borrow();
        let mut found = None;
        for (inspector_id, source) in &self.reverse_objects {
            if *source != id {
                continue;
            }
            let Some(visual) = inspector
                .resolve(*inspector_id)
                .ok()
                .and_then(|object| object.cast::<Visual>())
            else {
                continue;
            };
            // A retained inspector object is not necessarily still part of this
            // graph. Accept either native tree, matching rebuild_map's traversal.
            let mut current = Some(visual.clone());
            let mut belongs = false;
            while let Some(ancestor) = current {
                if ancestor == root_visual {
                    belongs = true;
                    break;
                }
                current = ancestor.visual_parent();
            }
            if !belongs {
                let mut current = Some(visual.clone().upcast::<StyledElement>());
                while let Some(ancestor) = current {
                    if ancestor == root_logical {
                        belongs = true;
                        break;
                    }
                    current = ancestor.parent();
                }
            }
            if !belongs {
                continue;
            }
            if found.is_some() {
                return Err("The state owner has multiple native instances; open a definition stage to preview its visual state".into());
            }
            found = Some(visual);
        }
        Ok(found)
    }

    pub fn bounds(&self, id: NodeId, relative_to: &Ref<Visual>) -> Option<Rect> {
        let visual = self.visual(id)?;
        Inspector::box_model(&visual, relative_to)
            .ok()
            .map(|model| model.border)
    }

    pub fn source_id(&self, visual: &Visual) -> Option<NodeId> {
        let id = Inspector::shared().borrow_mut().node_id(visual);
        self.reverse_objects.get(&id).copied()
    }

    pub fn rebuild_map(&mut self, document: &Document) {
        let _design_mode =
            crate::preview_host::DesignModeGuard::new(self.options.configuration.design_mode);
        if !matches!(self.status, PreviewStatus::Ready { revision } if revision == document.revision())
        {
            return;
        }
        self.objects.clear();
        self.reverse_objects.clear();
        let Some(root) = self.root() else {
            return;
        };
        if let Some(projection) = &self.design_projection {
            self.notices
                .extend(crate::design_time_samples::apply_projection(
                    &root,
                    projection,
                    &mut self.design_samples,
                ));
        }
        // One scan of source positions, followed by indexed lookup for each runtime object.
        let mut names = BTreeMap::new();
        let mut locations: BTreeMap<(String, usize), Vec<(usize, NodeId)>> = BTreeMap::new();
        let source_document = self.design_projection.as_ref().map_or_else(
            || {
                self.definition_projection
                    .as_ref()
                    .map_or(document, |projection| &projection.document)
            },
            |projection| &projection.document,
        );
        let mut elements = source_document.ast().elements().collect::<Vec<_>>();
        elements.sort_by_key(|node| node.span.start);
        let source = source_document.valid_source();
        let (mut offset, mut line, mut column) = (0usize, 1usize, 1usize);
        for node in elements {
            for character in source[offset..node.span.start].chars() {
                if character == '\n' {
                    line += 1;
                    column = 1;
                } else {
                    column += 1;
                }
            }
            offset = node.span.start;
            if let Some(name) = node.xaml_name() {
                // Template and component namescopes can repeat names in the same
                // source document. Such a name is never a safe fallback identity.
                names
                    .entry(name.to_owned())
                    .and_modify(|identity| *identity = None)
                    .or_insert(Some(node.id));
            }
            locations
                .entry((node.local_name().to_owned(), line))
                .or_default()
                .push((column, node.id));
        }
        // The runtime loader names in-memory documents independently of their base URI.
        // Use the root's actual provenance, so a different template/resource document cannot
        // claim an authored node just because its namescope repeats the same Name.
        let root_value: BoxedValue = Rc::new(root.clone());
        let provenance = XamlSourceInfo::get_xaml_source_info(&root_value);
        let inspector = Inspector::shared();
        let root_id = inspector.borrow_mut().node_id(&root);
        if self.definition_projection.is_none()
            && self.design_projection.as_ref().is_none_or(|projection| {
                projection.identities.get(&projection.document.ast().root())
                    == Some(&document.ast().root())
            })
            && !document
                .ast()
                .node(document.ast().root())
                .is_some_and(|node| {
                    matches!(
                        node.local_name(),
                        "Styles"
                            | "Style"
                            | "ResourceDictionary"
                            | "ControlTheme"
                            | "Application"
                            | "DataTemplate"
                    )
                })
        {
            self.objects.insert(
                document.ast().root(),
                root.clone().upcast::<Visual>().downgrade(),
            );
            self.reverse_objects.insert(root_id, document.ast().root());
        }
        let mut pending = vec![root.upcast::<FerroObject>()];
        let mut seen = BTreeSet::new();
        while let Some(object) = pending.pop() {
            let id = inspector.borrow_mut().node_id(&object);
            if !seen.insert(id) {
                continue;
            }
            for tree in [TreeKind::Logical, TreeKind::Visual] {
                pending.extend(inspector.borrow().children(&object, tree));
            }
            let Some(visual) = object.cast::<Visual>() else {
                continue;
            };
            let value: BoxedValue = Rc::new(object.clone());
            let Some(info) = XamlSourceInfo::get_xaml_source_info(&value) else {
                continue;
            };
            if provenance
                .as_ref()
                .is_some_and(|root| root.source_uri() != info.source_uri())
            {
                continue;
            }
            let name = object
                .cast::<StyledElement>()
                .and_then(|element| element.name());
            let named = name
                .as_ref()
                .and_then(|name| names.get(name).copied().flatten());
            let located = || {
                locations
                    .get(&(
                        object.get_type().name().to_owned(),
                        info.line_number().max(1) as usize,
                    ))
                    .and_then(|entries| {
                        entries
                            .iter()
                            .min_by_key(|(column, _)| {
                                column.abs_diff(info.line_position().max(1) as usize)
                            })
                            .map(|(_, id)| *id)
                    })
            };
            // Names repeat in template namescopes. Generated preview locations are the
            // authoritative mapping; a synthetic sample control must never claim a
            // definition owner or a same-named part in another resource.
            let source = if self.design_projection.is_some() || self.definition_projection.is_some()
            {
                located().and_then(|source| self.authored_identity(source))
            } else {
                located().or(named)
            };
            if let Some(source) = source {
                self.objects
                    .entry(source)
                    .or_insert_with(|| visual.downgrade());
                self.reverse_objects.insert(id, source);
            }
        }
    }

    fn authored_identity(&self, mut node: NodeId) -> Option<NodeId> {
        if let Some(projection) = &self.design_projection {
            node = *projection.identities.get(&node)?;
        }
        if let Some(projection) = &self.definition_projection {
            node = *projection.identities.get(&node)?;
        }
        Some(node)
    }

    pub fn tree(&self, kind: TreeKind) -> Vec<RuntimeNode> {
        let Some(root) = self.root() else {
            return Vec::new();
        };
        let mut rows = Vec::new();
        let mut pending = vec![(root.upcast::<FerroObject>(), None, 0)];
        let mut seen = BTreeSet::new();
        let inspector = Inspector::shared();
        while let Some((object, parent, depth)) = pending.pop() {
            let id = inspector.borrow_mut().node_id(&object);
            if !seen.insert(id) {
                continue;
            }
            let visual = object.cast::<Visual>();
            let source_id = visual.as_ref().and_then(|visual| self.source_id(visual));
            rows.push(RuntimeNode {
                inspector_id: id,
                source_id,
                parent,
                depth,
                type_name: object.get_type().name().to_owned(),
                name: object
                    .cast::<StyledElement>()
                    .and_then(|element| element.name()),
                bounds: visual
                    .and_then(|visual| {
                        Inspector::box_model(&visual, &self.host.clone().upcast()).ok()
                    })
                    .map(|model| model.border),
            });
            let children = inspector.borrow().children(&object, kind);
            pending.extend(
                children
                    .into_iter()
                    .rev()
                    .map(|child| (child, Some(id), depth + 1)),
            );
        }
        rows
    }

    pub fn preview_samples(&mut self, samples: &[Sample]) -> Result<(), String> {
        // Validate every conversion before replacing the previous preview layer.
        let mut staged = Vec::new();
        for sample in samples {
            let object = self
                .visual(sample.target)
                .ok_or_else(|| format!("No live animation target {}", sample.target))?;
            let property = Inspector::property(&object, &sample.property)?;
            if property.is_direct() || property.is_read_only() {
                return Err(format!(
                    "{} does not support an animation value layer",
                    sample.property
                ));
            }
            let value: BoxedValue = Rc::new(sample.value.to_xaml());
            let converted = ValueTypes::try_convert(
                Some(&value),
                ValueType::new(property.property_type(), property.property_type_name()),
            )
            .flatten()
            .ok_or_else(|| format!("Cannot animate {} with this value", sample.property))?;
            if !property.is_valid_value(converted.as_any()) {
                return Err(format!("Invalid animation value for {}", sample.property));
            }
            staged.push((object, property, converted));
        }
        self.clear_animation();
        for (object, property, value) in staged {
            if let Some(guard) =
                object.set_value_untyped(property, value.as_any(), BindingPriority::Animation)
            {
                self.animation_values.push(guard);
            }
        }
        Ok(())
    }

    pub fn clear_animation(&mut self) {
        for guard in self.animation_values.drain(..) {
            guard.dispose();
        }
    }
}

impl Drop for RuntimePreview {
    fn drop(&mut self) {
        self.clear_animation();
        self.close_native();
    }
}

// Design mode owns time. Inherited runtime animations and transitions remain frozen until the
// host explicitly enables interaction; timeline sampling uses disposable animation layers.
struct DesignClock;
impl IObservable<TimeSpan> for DesignClock {
    fn subscribe(&self, _observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}
impl IClock for DesignClock {
    fn play_state(&self) -> PlayState {
        PlayState::Pause
    }
    fn set_play_state(&self, _value: PlayState) {}
}
