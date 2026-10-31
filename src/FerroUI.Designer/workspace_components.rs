//! Document-backed library, reusable definitions and template editing navigation.
//! Views emit commands; all mutations pass through the shared component planner.
use super::DesignerWorkspace;
use crate::component_browser::{ComponentLibrary, ComponentLibraryFilter, ComponentLibraryItem};
use crate::resource_editor::{
    DefinitionCreationKind, DefinitionEditorCommand, DefinitionPreviewSettings,
    DefinitionPropertyField, DefinitionReference, ResourceDefinitionEditor,
    ResourceDefinitionModel, kind_title,
};
use crate::{PropertyEditorContext, PropertyValueEdit, PropertyValueSource};
use ferroui_base::input::{
    DataFormat, DataTransfer, DataTransferItem, DragDrop, DragDropEffects, IDataTransfer,
    KeyModifiers, PointerPressedEventArgs,
};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::media::{FontWeight, SolidColorBrush, TextWrapping};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{CornerRadius, FerroProperty, Thickness, TypeInfo};
use ferroui_controls::{Border, Button, ContentControl, Control, StackPanel, TextBlock};
use ferroui_designer_core::{Ast, Document, MarkupValue, NodeId, NodeKind};
use ferroui_designer_tools::{ComponentAction, ComponentCatalog, ComponentEntry, ComponentKind};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::Arc;

#[path = "workspace_component_types.rs"]
mod control_types;
#[path = "workspace_component_projects.rs"]
mod project_sources;
#[path = "workspace_component_import.rs"]
mod source_import;

/// Exact XAML supplied by a library or a project. The designer does not reconstruct
/// a compiled template from live visuals. The source is copied through an AST edit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesignerDefinitionAsset {
    pub id: String,
    pub label: String,
    pub kind: ComponentKind,
    pub target_type: Option<String>,
    pub source: String,
    pub origin_uri: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum LibrarySelection {
    Document(NodeId),
    Definition(String),
    Control(String),
}

#[derive(Clone)]
struct ComponentContext {
    uri: String,
    revision: u64,
    ast: Arc<Ast>,
    scope: Option<NodeId>,
    scope_epoch: u64,
    selection: Vec<NodeId>,
}
impl ComponentContext {
    fn capture(workspace: &DesignerWorkspace) -> Self {
        let session = workspace.session.borrow();
        Self {
            uri: session.document_uri().into(),
            revision: session.document().revision(),
            ast: session.document().ast_snapshot(),
            scope: session.definition_scope().map(|scope| scope.node),
            scope_epoch: session.definition_scope_epoch(),
            selection: session.selection().to_vec(),
        }
    }
    fn check(&self, workspace: &DesignerWorkspace) -> Result<(), String> {
        let session = workspace.session.borrow();
        if workspace.pending_source.get()
            || !session.document().is_valid()
            || self.uri != session.document_uri()
            || self.revision != session.document().revision()
            || !Arc::ptr_eq(&self.ast, &session.document().ast_snapshot())
            || self.scope != session.definition_scope().map(|scope| scope.node)
            || self.scope_epoch != session.definition_scope_epoch()
            || self.selection != session.selection()
        {
            Err("The document changed. Reopen the definition before applying this edit.".into())
        } else {
            Ok(())
        }
    }
}

struct ComponentStamp {
    ast: Arc<Ast>,
    selection: Option<LibrarySelection>,
    create: bool,
    writable: bool,
    scope_epoch: u64,
    targets: Vec<NodeId>,
}

struct ComponentDragContext {
    uri: String,
    ast: Arc<Ast>,
    scope_epoch: u64,
    writable: bool,
}

#[derive(Default)]
pub(super) struct ComponentUi {
    browser: Option<Rc<ComponentLibrary>>,
    editor: Option<Rc<ResourceDefinitionEditor>>,
    selection: Option<LibrarySelection>,
    editor_open: bool,
    create_open: bool,
    document_uri: String,
    stamp: Option<ComponentStamp>,
    drag_context: Option<ComponentDragContext>,
    definitions: BTreeMap<String, DesignerDefinitionAsset>,
    sources: BTreeMap<String, source_import::DefinitionSource>,
    documents: project_sources::ComponentDocumentIndex,
}

impl DesignerWorkspace {
    pub fn component_library(&self) -> Option<Rc<ComponentLibrary>> {
        self.components.borrow().browser.clone()
    }

    /// Register actual source for a custom control's design-time template or theme.
    /// Registration changes the library, never the active document or its history.
    pub fn register_component_definition(
        self: &Rc<Self>,
        mut asset: DesignerDefinitionAsset,
    ) -> Result<(), String> {
        validate_definition_asset(&asset)?;
        if let Some(uri) = asset.origin_uri.as_deref() {
            asset.origin_uri = Some(super::project::document_identity(uri)?);
        }
        {
            let mut state = self.components.borrow_mut();
            state.documents.claim_explicit_asset(&asset.id);
            state.sources.remove(&asset.id);
            state.definitions.insert(asset.id.clone(), asset);
            state.stamp = None;
        }
        self.refresh_components();
        Ok(())
    }

    /// Catalog control roots, templates, themes and styles from a registered XAML document. Inherited
    /// namespace declarations are retained in each independently copyable fragment.
    pub fn register_component_document(
        self: &Rc<Self>,
        uri: &str,
        source: &str,
    ) -> Result<usize, String> {
        self.register_component_documents([(uri, source)])
    }

    /// Validate and register a group of source providers atomically, then refresh
    /// the Library once. An invalid source leaves every existing provider intact.
    pub fn register_component_documents<'a>(
        self: &Rc<Self>,
        documents: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<usize, String> {
        let mut prepared = Vec::new();
        let mut uris = BTreeSet::new();
        for (uri, source) in documents {
            let uri = super::project::document_identity(uri)?;
            if !uris.insert(uri.clone()) {
                return Err(format!(
                    "Component source {uri} occurs more than once in the batch"
                ));
            }
            let document = Document::new(source)
                .map_err(|error| format!("Component source {uri}: {error}"))?;
            prepared.push(
                project_sources::prepare_component_document(uri.clone(), Rc::new(document), false)
                    .map_err(|error| format!("Component source {uri}: {error}"))?,
            );
        }
        if prepared.is_empty() {
            return Ok(0);
        }
        let count = prepared
            .into_iter()
            .map(|document| self.replace_component_document_snapshot(document, false))
            .sum();
        self.refresh_components();
        Ok(count)
    }

    /// Create an independent authored copy of a registered definition and the local
    /// resources it requires. The complete import shares one undo/collaboration edit.
    /// Opening the resulting canvas scope remains a separate presentation operation.
    pub fn copy_component_definition(
        &self,
        id: &str,
        owner: NodeId,
        key: &str,
    ) -> Result<NodeId, String> {
        if self.pending_source.get() {
            return Err("Apply the pending XAML before copying a definition".into());
        }
        let asset = self
            .components
            .borrow()
            .definitions
            .get(id)
            .cloned()
            .ok_or("The definition provider is no longer available")?;
        self.copy_provided_definition(&asset, owner, key)?;
        self.session
            .borrow()
            .component_catalog()?
            .entries
            .into_iter()
            .find(|entry| {
                entry.key.as_deref() == Some(key)
                    && (entry.owner == Some(owner) || entry.scope == owner)
            })
            .map(|entry| entry.node)
            .ok_or_else(|| "The copied definition was not found".into())
    }

    /// Browsing retains the canvas selection so a resource can be applied to it.
    pub fn inspect_component(self: &Rc<Self>, node: NodeId) -> Result<(), String> {
        if !self
            .session
            .borrow()
            .component_catalog()?
            .entries
            .iter()
            .any(|entry| entry.node == node)
        {
            return Err("Select an authored component or resource".into());
        }
        {
            let mut state = self.components.borrow_mut();
            state.selection = Some(LibrarySelection::Document(node));
            state.create_open = false;
            state.editor_open = true;
            state.stamp = None;
        }
        self.set_navigation_pane(crate::NavigationPane::Library);
        if let Some(browser) = self.component_library() {
            browser.select(&format!("document:{node}"));
        }
        self.refresh_components();
        Ok(())
    }

    pub fn create_component(self: &Rc<Self>) {
        {
            let mut state = self.components.borrow_mut();
            state.create_open = true;
            state.editor_open = true;
            state.stamp = None;
        }
        self.set_navigation_pane(crate::NavigationPane::Library);
        self.refresh_components();
    }

    pub fn edit_component_on_canvas(self: &Rc<Self>, node: NodeId) -> Result<(), String> {
        self.enter_component_canvas(node, crate::DefinitionPreviewOptions::default())
    }

    fn enter_component_canvas(
        self: &Rc<Self>,
        node: NodeId,
        options: crate::DefinitionPreviewOptions,
    ) -> Result<(), String> {
        if self.pending_source.get() {
            return Err("Apply the pending XAML before opening a definition".into());
        }
        self.session
            .borrow_mut()
            .enter_definition_scope_with_options(node, options)?;
        self.components.borrow_mut().editor_open = false;
        self.root
            .get_control::<Button>("CloseAuthoring")
            .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        self.set_view_mode(crate::WorkspaceViewMode::Design);
        self.refresh(true);
        Ok(())
    }

    pub fn return_to_document(self: &Rc<Self>) -> Result<(), String> {
        self.session.borrow_mut().exit_definition_scope()?;
        self.components.borrow_mut().editor_open = false;
        self.root
            .get_control::<Button>("CloseAuthoring")
            .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        self.refresh(true);
        Ok(())
    }

    pub(super) fn connect_components(self: &Rc<Self>) {
        let browser = ComponentLibrary::new();
        let editor = ResourceDefinitionEditor::new();
        self.root
            .get_control::<ContentControl>("ComponentLibraryHost")
            .set_content(Some(Control::boxed(browser.control())));
        self.root
            .get_control::<ContentControl>("ComponentDefinitionHost")
            .set_content(Some(Control::boxed(editor.control())));
        {
            let mut state = self.components.borrow_mut();
            state.browser = Some(browser.clone());
            state.editor = Some(editor);
        }
        let weak = Rc::downgrade(self);
        browser.on_select(move |item| {
            if let Some(this) = weak.upgrade() {
                this.select_library_item(item);
            }
        });
        let weak = Rc::downgrade(self);
        browser.on_activate(move |item| {
            if let Some(this) = weak.upgrade() {
                this.select_library_item(item);
                if let Some(node) = item.node {
                    if matches!(
                        item.filter,
                        ComponentLibraryFilter::Components
                            | ComponentLibraryFilter::Templates
                            | ComponentLibraryFilter::Themes
                    ) {
                        this.report(this.edit_component_on_canvas(node));
                    }
                } else if let Some(id) = item.asset_id.as_deref() {
                    if let Some(asset) = this.registered_control(id) {
                        this.surface.arm_insert(asset.source);
                        this.status
                            .set_text(Some(&format!("Click the canvas to place {}", asset.title)));
                    }
                } else if let Some(id) = item.id.strip_prefix("definition:") {
                    let is_control =
                        this.components
                            .borrow()
                            .definitions
                            .get(id)
                            .is_some_and(|asset| {
                                matches!(
                                    asset.kind,
                                    ComponentKind::UserControl | ComponentKind::CustomControl
                                )
                            });
                    if is_control {
                        this.report(this.open_component_document(id));
                    }
                }
            }
        });
        let weak = Rc::downgrade(self);
        browser.on_create(move || {
            if let Some(this) = weak.upgrade() {
                this.create_component();
            }
        });
        let weak = Rc::downgrade(self);
        browser.on_begin_drag(move |item, event| {
            if let Some(this) = weak.upgrade() {
                this.report(this.begin_component_drag(item, event));
            }
        });
        for name in [
            "CloseAuthoring",
            "StatesMode",
            "BehaviorsMode",
            "AnimationSettings",
        ] {
            let weak = Rc::downgrade(self);
            self.root.get_control::<Button>(name).click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.components.borrow_mut().editor_open = false;
                    this.rebuild_authoring();
                    this.refresh_components();
                }
            });
        }
        let weak = Rc::downgrade(self);
        self.root
            .get_control::<Button>("DefinitionMode")
            .click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let selected = this.components.borrow().selection.is_some();
                    if selected {
                        let mut state = this.components.borrow_mut();
                        state.editor_open = true;
                        state.stamp = None;
                        drop(state);
                        this.refresh_components();
                    } else {
                        this.create_component();
                    }
                }
            });
        let weak = Rc::downgrade(self);
        self.root
            .get_control::<Button>("ComponentScopeBack")
            .click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.report(this.return_to_document());
                }
            });
        let weak = Rc::downgrade(self);
        self.root
            .get_control::<Button>("ComponentScopeDefinition")
            .click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let node = this
                        .session
                        .borrow()
                        .definition_scope()
                        .map(|scope| scope.node);
                    if let Some(node) = node {
                        this.report(this.inspect_component(node));
                    }
                }
            });
    }

    fn registered_control(&self, id: &str) -> Option<crate::DesignerAsset> {
        self.asset_toolbox()?
            .assets()
            .into_iter()
            .find(|asset| asset.id == id)
    }

    fn select_library_item(self: &Rc<Self>, item: &ComponentLibraryItem) {
        let selection = if let Some(node) = item.node {
            LibrarySelection::Document(node)
        } else if let Some(id) = item.asset_id.as_ref() {
            LibrarySelection::Control(id.clone())
        } else {
            LibrarySelection::Definition(item.id.trim_start_matches("definition:").to_owned())
        };
        {
            let mut state = self.components.borrow_mut();
            state.selection = Some(selection);
            state.create_open = false;
            state.editor_open = true;
            state.stamp = None;
        }
        self.refresh_components();
    }

    pub(super) fn refresh_components(self: &Rc<Self>) {
        self.sync_project_components();
        let (browser, editor) = {
            let state = self.components.borrow();
            let (Some(browser), Some(editor)) = (&state.browser, &state.editor) else {
                return;
            };
            (browser.clone(), editor.clone())
        };
        let session = self.session.borrow();
        let document = session.document();
        let uri = session.document_uri().to_owned();
        let canonical_uri = super::project::document_identity(&uri).unwrap_or_else(|_| uri.clone());
        let ast = document.ast_snapshot();
        let writable = !session.is_read_only() && document.is_valid() && !self.pending_source.get();
        let catalog = session.component_catalog().unwrap_or_default();
        let mut items = catalog
            .entries
            .iter()
            .map(|entry| library_item(document, entry))
            .collect::<Vec<_>>();
        let scope = session.definition_scope().cloned();
        let scope_epoch = session.definition_scope_epoch();
        let targets = session.selection().to_vec();
        drop(session);
        let cancel_drag = self
            .components
            .borrow()
            .drag_context
            .as_ref()
            .is_none_or(|old| {
                old.uri != uri
                    || !Arc::ptr_eq(&old.ast, &ast)
                    || old.scope_epoch != scope_epoch
                    || old.writable != writable
            });
        if cancel_drag {
            browser.cancel_pending_drag();
        }
        {
            let mut state = self.components.borrow_mut();
            state.drag_context = Some(ComponentDragContext {
                uri: uri.clone(),
                ast: ast.clone(),
                scope_epoch,
                writable,
            });
            if state.document_uri != uri {
                state.document_uri = uri;
                state.selection = None;
                state.editor_open = false;
                state.create_open = false;
                state.stamp = None;
            }
            if let Some(LibrarySelection::Document(node)) = state.selection.as_ref() {
                if !catalog.entries.iter().any(|entry| entry.node == *node) {
                    state.selection = None;
                    state.editor_open = false;
                    state.stamp = None;
                }
            }
            for asset in state.definitions.values() {
                if asset.origin_uri.as_deref() == Some(canonical_uri.as_str()) {
                    continue;
                }
                items.push(ComponentLibraryItem {
                    id: format!("definition:{}", asset.id),
                    title: asset.label.clone(),
                    kind: format!("{} · Library", display_kind(asset.kind)),
                    detail: asset.origin_uri.clone().unwrap_or_default(),
                    filter: library_filter(asset.kind),
                    node: None,
                    asset_id: None,
                    swatch: None,
                    usages: 0,
                });
            }
        }
        if let Some(toolbox) = self.asset_toolbox() {
            let built_in = crate::default_designer_assets()
                .into_iter()
                .map(|asset| asset.id)
                .collect::<BTreeSet<_>>();
            for asset in toolbox
                .assets()
                .into_iter()
                .filter(|asset| !built_in.contains(&asset.id))
            {
                items.push(ComponentLibraryItem {
                    id: format!("control:{}", asset.id),
                    title: asset.title,
                    kind: "Custom control · Asset".into(),
                    detail: format!("{} {}", asset.category, asset.description),
                    filter: ComponentLibraryFilter::Components,
                    node: None,
                    asset_id: Some(asset.id),
                    swatch: None,
                    usages: 0,
                });
            }
        }
        browser.replace_items(items);
        let (selection, open, create, rebuild) = {
            let mut state = self.components.borrow_mut();
            let same = state.stamp.as_ref().is_some_and(|stamp| {
                Arc::ptr_eq(&stamp.ast, &ast)
                    && stamp.selection == state.selection
                    && stamp.create == state.create_open
                    && stamp.writable == writable
                    && stamp.scope_epoch == scope_epoch
                    && stamp.targets == targets
            });
            let rebuild = state.editor_open && !same;
            if rebuild {
                state.stamp = Some(ComponentStamp {
                    ast,
                    selection: state.selection.clone(),
                    create: state.create_open,
                    writable,
                    scope_epoch,
                    targets,
                });
            }
            (
                state.selection.clone(),
                state.editor_open,
                state.create_open,
                rebuild,
            )
        };
        self.root
            .get_control::<Control>("ComponentInspectorPane")
            .set_is_visible(open);
        self.root
            .get_control::<Button>("DefinitionMode")
            .classes()
            .set("active", open);
        if open {
            for name in ["InspectorDesignPane", "AuthoringScroll"] {
                self.root.get_control::<Control>(name).set_is_visible(false);
            }
            for name in ["CloseAuthoring", "StatesMode", "BehaviorsMode"] {
                self.root
                    .get_control::<Button>(name)
                    .classes()
                    .set("active", false);
            }
        }
        if rebuild {
            let context = ComponentContext::capture(self);
            let weak = Rc::downgrade(self);
            let selected = selection.clone();
            let guard = context.clone();
            editor.on_command(move |command| {
                let this = weak.upgrade().ok_or("The designer workspace was closed")?;
                guard.check(&this)?;
                this.component_command(selected.clone(), command)
            });
            let owners = self.component_owners();
            if create {
                editor.configure_create(owners, writable);
            } else {
                match selection {
                    Some(LibrarySelection::Document(node)) => {
                        if let Some(entry) = catalog.entries.iter().find(|entry| entry.node == node)
                        {
                            match self.definition_model(entry, writable, &context) {
                                Ok(model) => editor.configure(model),
                                Err(error) => editor.show_error(Some(&error)),
                            }
                        }
                    }
                    Some(LibrarySelection::Definition(id)) => {
                        if let Some(asset) = self.components.borrow().definitions.get(&id).cloned()
                        {
                            editor.configure_external(
                                &asset.label,
                                asset.kind,
                                asset.origin_uri.as_deref(),
                                &asset.source,
                                owners,
                                writable,
                            );
                        }
                    }
                    Some(LibrarySelection::Control(id)) => {
                        if let Some(asset) = self.registered_control(&id) {
                            editor.configure_external(
                                &asset.title,
                                ComponentKind::CustomControl,
                                Some(&asset.category),
                                &asset.source,
                                owners,
                                writable,
                            );
                        }
                    }
                    None => editor.configure_create(owners, writable),
                }
            }
        }
        let bar = self.root.get_control::<Control>("ComponentScopeBar");
        bar.set_is_visible(scope.is_some());
        if let Some(scope) = scope {
            self.root
                .get_control::<TextBlock>("ComponentScopeTitle")
                .set_text(Some(&format!(
                    "{}  /  {}",
                    display_kind(scope.kind),
                    scope.title
                )));
        }
        self.refresh_component_instance(&catalog);
    }

    pub(super) fn component_owners(&self) -> Vec<(NodeId, String)> {
        let session = self.session.borrow();
        let document = session.document();
        let mut ids = Vec::new();
        if let Some(node) = session.selection().first() {
            let mut current = Some(*node);
            while let Some(id) = current {
                let Some(node) = document.ast().node(id) else {
                    break;
                };
                if node.kind == NodeKind::Element
                    && !matches!(
                        node.local_name(),
                        "Setter" | "Style" | "ControlTheme" | "ControlTemplate" | "DataTemplate"
                    )
                {
                    ids.push(id);
                }
                current = node.parent;
            }
        }
        ids.extend(document.ast().roots().iter().copied().filter(|id| {
            document
                .ast()
                .node(*id)
                .is_some_and(|node| node.kind == NodeKind::Element)
        }));
        ids.extend(
            document
                .ast()
                .elements()
                .filter(|node| node.local_name() == "ResourceDictionary")
                .map(|node| node.id),
        );
        let mut seen = BTreeSet::new();
        ids.retain(|id| seen.insert(*id));
        ids.into_iter()
            .map(|id| {
                (
                    id,
                    format!(
                        "{} · {}",
                        node_title(document, id),
                        if document.ast().roots().contains(&id) {
                            "Document"
                        } else {
                            "Element scope"
                        }
                    ),
                )
            })
            .collect()
    }

    fn definition_model(
        self: &Rc<Self>,
        entry: &ComponentEntry,
        writable: bool,
        context: &ComponentContext,
    ) -> Result<ResourceDefinitionModel, String> {
        let session = self.session.borrow();
        let document = session.document();
        let node = document
            .ast()
            .node(entry.node)
            .ok_or("The definition no longer exists")?;
        let source = ferroui_designer_tools::component_fragment(document, entry.node)
            .map_err(|error| error.to_string())?;
        let references = session
            .component_resource_references(entry.node)?
            .into_iter()
            .map(|reference| DefinitionReference {
                node: reference.node,
                title: node_title(document, reference.node),
                property: reference.property,
            })
            .collect();
        let runtime_type = definition_type(document, entry);
        let is_setter = matches!(
            entry.kind,
            ComponentKind::ControlTheme | ComponentKind::Style
        );
        let definition_names: &[&str] = match entry.kind {
            ComponentKind::ControlTheme => &["TargetType", "BasedOn"],
            ComponentKind::ControlTemplate => &["TargetType"],
            ComponentKind::DataTemplate => &["DataType"],
            ComponentKind::Style => &["Selector"],
            _ => &[],
        };
        let identity = definition_names
            .iter()
            .map(|name| {
                ferroui_designer_tools::component_property_attribute(document, entry.node, name)
                    .map(|attribute| {
                        (
                            (*name).to_owned(),
                            attribute.map(|attribute| attribute.value.clone()),
                        )
                    })
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, String>>()?;
        let mut fields = Vec::new();
        if is_setter {
            for child in document.ast().element_children(entry.node) {
                let Some(setter) = document.ast().node(child) else {
                    continue;
                };
                if setter.local_name() != "Setter" {
                    continue;
                }
                let Some(property) = ferroui_designer_tools::component_property_attribute(
                    document, child, "Property",
                )
                .map_err(|error| error.to_string())?
                .map(|attribute| attribute.value.as_str()) else {
                    continue;
                };
                let object =
                    ferroui_designer_tools::component_property_object(document, child, "Value")
                        .map_err(|error| error.to_string())?;
                fields.push((
                    property.to_owned(),
                    ferroui_designer_tools::component_property_attribute(document, child, "Value")
                        .map_err(|error| error.to_string())?
                        .map(|attribute| attribute.value.clone()),
                    object,
                    child,
                ));
            }
        } else {
            let mut names = node
                .attributes
                .iter()
                .filter(|attribute| {
                    !attribute.is_namespace()
                        && attribute.namespace_uri.as_deref()
                            != Some("http://schemas.microsoft.com/winfx/2006/xaml")
                        && !matches!(
                            attribute.name.as_str(),
                            "Name" | "TargetType" | "DataType" | "Selector"
                        )
                })
                .map(|attribute| attribute.name.clone())
                .collect::<BTreeSet<_>>();
            for child in document.ast().element_children(entry.node) {
                if let Some(child) = document
                    .ast()
                    .node(child)
                    .filter(|node| node.kind == NodeKind::PropertyElement)
                {
                    if let Some((owner, member)) = child.local_name().split_once('.') {
                        let property = if owner == node.local_name()
                            && child.namespace_uri == node.namespace_uri
                        {
                            member
                        } else {
                            &child.name
                        };
                        if !matches!(property, "Resources" | "Styles" | "Children") {
                            names.insert(property.to_owned());
                        }
                    }
                }
            }
            if entry.kind == ComponentKind::Resource {
                if let Some(type_) = runtime_type {
                    names.extend(
                        type_
                            .properties()
                            .iter()
                            .filter(|property| !property.is_read_only())
                            .map(|property| property.name().to_owned()),
                    );
                }
            }
            for property in names {
                let object = ferroui_designer_tools::component_property_object(
                    document, entry.node, &property,
                )
                .map_err(|error| error.to_string())?;
                fields.push((
                    property.clone(),
                    ferroui_designer_tools::component_property_attribute(
                        document, entry.node, &property,
                    )
                    .map_err(|error| error.to_string())?
                    .map(|attribute| attribute.value.clone()),
                    object,
                    entry.node,
                ));
            }
        }
        let primitive_value = if node.children.iter().any(|id| {
            document.ast().node(*id).is_some_and(|node| {
                matches!(node.kind, NodeKind::Text | NodeKind::CData)
                    && node
                        .text
                        .as_deref()
                        .is_some_and(|text| !text.trim().is_empty())
            })
        }) && document.ast().element_children(entry.node).is_empty()
            && entry.kind == ComponentKind::Resource
        {
            Some(
                node.children
                    .iter()
                    .filter_map(|id| document.ast().node(*id))
                    .filter(|node| matches!(node.kind, NodeKind::Text | NodeKind::CData))
                    .filter_map(|node| node.text.as_deref())
                    .collect::<String>(),
            )
        } else {
            None
        };
        let mut preview = DefinitionPreviewSettings {
            target_type: preview_target_type(entry),
            ..Default::default()
        };
        if let Some(scope) = session
            .definition_scope()
            .filter(|scope| scope.node == entry.node)
        {
            preview = DefinitionPreviewSettings {
                target_type: scope.options.target_type.clone(),
                sample_content: scope.options.sample_content.clone(),
                width: scope.options.width,
                height: scope.options.height,
            };
        }
        let scope = node_title(document, entry.scope);
        let type_name = node.name.clone();
        drop(session);
        let definition_properties = identity
            .into_iter()
            .map(|(property, value)| {
                self.definition_property(
                    entry.node, entry.node, &property, value, None, false, None, context,
                )
            })
            .collect();
        let properties = fields
            .into_iter()
            .map(|(property, value, object, target)| {
                self.definition_property(
                    entry.node,
                    target,
                    &property,
                    value,
                    object,
                    is_setter,
                    runtime_type.and_then(|type_| type_.find_property(&property)),
                    context,
                )
            })
            .collect();
        let setter_properties = runtime_type
            .map(|type_| {
                let mut names = type_
                    .properties()
                    .iter()
                    .filter(|property| !property.is_read_only())
                    .map(|property| property.name().to_owned())
                    .collect::<Vec<_>>();
                names.sort();
                names.dedup();
                names
            })
            .unwrap_or_default();
        Ok(ResourceDefinitionModel {
            node: entry.node,
            title: entry.display_name.clone(),
            type_name,
            kind: entry.kind,
            key: entry.key.clone(),
            scope,
            source,
            writable,
            can_preview: matches!(
                entry.kind,
                ComponentKind::UserControl
                    | ComponentKind::CustomControl
                    | ComponentKind::ControlTemplate
                    | ComponentKind::DataTemplate
                    | ComponentKind::ControlTheme
                    | ComponentKind::Style
                    | ComponentKind::Resource
                    | ComponentKind::ResourceDictionary
            ),
            preview,
            apply_property: default_apply_property(entry).into(),
            properties,
            definition_properties,
            references,
            primitive_value,
            setter_properties,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn definition_property(
        self: &Rc<Self>,
        owner: NodeId,
        target: NodeId,
        property: &str,
        value: Option<String>,
        object: Option<String>,
        setter: bool,
        native: Option<&'static FerroProperty>,
        context: &ComponentContext,
    ) -> DefinitionPropertyField {
        let weak = Rc::downgrade(self);
        let guard = context.clone();
        let name = property.to_owned();
        let commit: crate::PropertyValueCommit = Rc::new(move |edit| {
            let this = weak.upgrade().ok_or("The designer workspace was closed")?;
            guard.check(&this)?;
            let action = match edit {
                PropertyValueEdit::Literal(value) | PropertyValueEdit::Expression(value) => {
                    if setter {
                        ComponentAction::SetSetter {
                            owner,
                            property: name.clone(),
                            value: Some(value),
                        }
                    } else {
                        ComponentAction::SetProperty {
                            node: owner,
                            property: name.clone(),
                            value: Some(value),
                        }
                    }
                }
                PropertyValueEdit::Reset => {
                    if setter {
                        ComponentAction::SetSetter {
                            owner,
                            property: name.clone(),
                            value: None,
                        }
                    } else {
                        ComponentAction::SetProperty {
                            node: owner,
                            property: name.clone(),
                            value: None,
                        }
                    }
                }
                PropertyValueEdit::ExtractResource {
                    owner: resource_owner,
                    key,
                } => ComponentAction::ExtractProperty {
                    node: target,
                    property: if setter { "Value".into() } else { name.clone() },
                    owner: resource_owner,
                    key,
                },
                PropertyValueEdit::Object(source) => {
                    if setter {
                        ComponentAction::SetSetterObject {
                            owner,
                            property: name.clone(),
                            source: Some(source),
                        }
                    } else {
                        ComponentAction::SetObjectProperty {
                            node: owner,
                            property: name.clone(),
                            source: Some(source),
                        }
                    }
                }
                PropertyValueEdit::Resource { node, dynamic } => {
                    if let Some(native) = native {
                        let session = this.session.borrow();
                        let resource = session
                            .document()
                            .ast()
                            .node(node)
                            .ok_or("This resource is no longer available")?;
                        if crate::editors::resource_is_compatible(native, resource) == Some(false) {
                            return Err(format!("This resource cannot be assigned to {}", name));
                        }
                    }
                    if setter {
                        let session = this.session.borrow();
                        let resource = session
                            .visible_component_resources(target)?
                            .into_iter()
                            .find(|entry| entry.node == node)
                            .ok_or("This resource is no longer visible to the setter")?;
                        let expression = ferroui_designer_tools::component_resource_expression(
                            session.document(),
                            target,
                            resource
                                .key
                                .as_deref()
                                .ok_or("Choose an explicit resource key")?,
                            dynamic,
                        )
                        .map_err(|error| error.to_string())?;
                        ComponentAction::SetSetter {
                            owner,
                            property: name.clone(),
                            value: Some(expression),
                        }
                    } else {
                        ComponentAction::ApplyResource {
                            resource: node,
                            targets: vec![owner],
                            property: name.clone(),
                            dynamic,
                        }
                    }
                }
            };
            this.session.borrow_mut().edit_component(action)?;
            this.components.borrow_mut().stamp = None;
            this.refresh(true);
            Ok(())
        });
        let weak = Rc::downgrade(self);
        let fallback = commit.clone();
        let context = PropertyEditorContext {
            property: property.into(),
            targets: vec![target],
            value,
            mixed: false,
            runtime_type: native.map(|property| property.property_type_name().to_owned()),
            apply: Rc::new(move |value| {
                let result = fallback(value.map_or(PropertyValueEdit::Reset, |value| {
                    if value.trim_start().starts_with('{') && !value.trim_start().starts_with("{}")
                    {
                        PropertyValueEdit::Expression(value)
                    } else {
                        PropertyValueEdit::Literal(value)
                    }
                }));
                if let Some(this) = weak.upgrade() {
                    this.report(result);
                }
            }),
        };
        let mut metadata = crate::resolve_property_editor_metadata(&context, native);
        metadata.commit = Some(commit);
        metadata.object_source = object;
        metadata.binding_extension = crate::component_session::property_binding_extension(
            self.session.borrow().document(),
            target,
            context.value.as_deref(),
        );
        metadata.binding_relative_source = crate::component_session::property_relative_source_extension(self.session.borrow().document(), target);
        if metadata.object_source.is_some() {
            metadata.source = PropertyValueSource::Object;
        }
        if matches!(
            metadata.source,
            PropertyValueSource::Local | PropertyValueSource::Object
        ) && (!setter || metadata.object_source.is_some())
        {
            // Scalar setter values require the target property's native resource type;
            // object setters already carry that identity in their authored graph.
            metadata.resource_owners = self.component_owners();
        }
        metadata.resources = self
            .session
            .borrow()
            .visible_component_resources(target)
            .unwrap_or_default()
            .into_iter()
            .filter(|entry| {
                native.is_none_or(|native| {
                    self.session
                        .borrow()
                        .document()
                        .ast()
                        .node(entry.node)
                        .is_none_or(|resource| {
                            crate::editors::resource_is_compatible(native, resource) != Some(false)
                        })
                })
            })
            .filter_map(|entry| {
                entry.key.map(|key| crate::PropertyResourceChoice {
                    node: entry.node,
                    label: format!("{} · {}", key, display_kind(entry.kind)),
                    key,
                })
            })
            .collect();
        {
            let session = self.session.borrow();
            let ast = session.document().ast();
            let namescope = ast.name_scope(target);
            metadata.named_elements = ast
                .elements()
                .filter(|node| ast.name_scope(node.id) == namescope)
                .filter_map(|node| node.xaml_name().map(str::to_owned))
                .collect();
        }
        let editor =
            crate::create_property_editor(&context, &metadata, &self.property_editors.borrow());
        editor.set_name(Some(format!(
            "DefinitionProperty_{}",
            property.replace(['.', ':'], "_")
        )));
        DefinitionPropertyField {
            name: property.into(),
            editor,
            description: Some(
                if setter {
                    "Theme setter · edits all linked instances"
                } else {
                    "Authored definition property"
                }
                .into(),
            ),
        }
    }

    fn component_command(
        self: &Rc<Self>,
        selection: Option<LibrarySelection>,
        command: DefinitionEditorCommand,
    ) -> Result<(), String> {
        if let DefinitionEditorCommand::CopyProvided { owner, key } = &command {
            let Some(LibrarySelection::Definition(id)) = selection else {
                return Err("Choose a source-backed library definition".into());
            };
            let node = self.copy_component_definition(&id, *owner, key)?;
            {
                let mut state = self.components.borrow_mut();
                state.selection = Some(LibrarySelection::Document(node));
                state.stamp = None;
                state.create_open = false;
            }
            self.refresh(true);
            self.edit_component_on_canvas(node)?;
            return Ok(());
        }
        if let DefinitionEditorCommand::OpenProvidedSource = command {
            let Some(LibrarySelection::Definition(id)) = selection else {
                return Err("Choose a source-backed definition".into());
            };
            return self.open_component_document(&id);
        }
        if let DefinitionEditorCommand::InsertControl = command {
            let (title, source) = match selection {
                Some(LibrarySelection::Control(id)) => {
                    let asset = self
                        .registered_control(&id)
                        .ok_or("The control asset is no longer registered")?;
                    (asset.title, asset.source)
                }
                Some(LibrarySelection::Definition(id)) => {
                    let asset = self
                        .components
                        .borrow()
                        .definitions
                        .get(&id)
                        .cloned()
                        .filter(|asset| {
                            matches!(
                                asset.kind,
                                ComponentKind::CustomControl | ComponentKind::UserControl
                            )
                        })
                        .ok_or("Select a registered control definition")?;
                    let source = control_types::control_instance_source(&asset)?;
                    (asset.label, source)
                }
                _ => return Err("Select a registered control asset".into()),
            };
            if self.session.borrow().is_read_only() {
                return Err("The document is read-only".into());
            }
            self.surface.arm_insert(source);
            self.status
                .set_text(Some(&format!("Click the canvas to place {title}")));
            return Ok(());
        }
        let mut created = None;
        let mut open_canvas = false;
        let action = match command {
            DefinitionEditorCommand::Create {
                owner,
                key,
                kind,
                target_type,
                source,
            } => {
                created = Some((owner, key.clone()));
                match kind {
                    DefinitionCreationKind::Resource => {
                        ComponentAction::CreateResource { owner, key, source }
                    }
                    DefinitionCreationKind::ControlTheme => ComponentAction::CreateTheme {
                        owner,
                        key,
                        target_type,
                        based_on: None,
                    },
                    DefinitionCreationKind::ControlTemplate
                    | DefinitionCreationKind::DataTemplate => ComponentAction::CreateTemplate {
                        owner,
                        key,
                        target_type: (!target_type.trim().is_empty()).then_some(target_type),
                        content: source,
                        data_template: kind == DefinitionCreationKind::DataTemplate,
                    },
                }
            }
            DefinitionEditorCommand::CopyProvided { .. } => unreachable!(),
            command => {
                let Some(LibrarySelection::Document(node)) = selection else {
                    return Err("Select a definition in the active document".into());
                };
                let entry = self
                    .session
                    .borrow()
                    .component_catalog()?
                    .entries
                    .into_iter()
                    .find(|entry| entry.node == node)
                    .ok_or("The definition no longer exists")?;
                match command {
                    DefinitionEditorCommand::Preview(settings) => {
                        return self.enter_component_canvas(
                            node,
                            crate::DefinitionPreviewOptions {
                                target_type: settings.target_type,
                                sample_content: settings.sample_content,
                                width: settings.width,
                                height: settings.height,
                            },
                        );
                    }
                    DefinitionEditorCommand::Source => {
                        let span = self
                            .session
                            .borrow()
                            .document()
                            .ast()
                            .node(node)
                            .ok_or("The definition no longer exists")?
                            .span;
                        self.select_source_span(span);
                        return Ok(());
                    }
                    DefinitionEditorCommand::SelectReference(reference) => {
                        self.session.borrow_mut().exit_definition_scope()?;
                        self.session.borrow_mut().select([reference]);
                        self.components.borrow_mut().editor_open = false;
                        self.root
                            .get_control::<Button>("CloseAuthoring")
                            .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
                        self.refresh(true);
                        let span = self
                            .session
                            .borrow()
                            .document()
                            .ast()
                            .node(reference)
                            .map(|node| node.span);
                        if let Some(span) = span {
                            self.select_source_span(span);
                        }
                        return Ok(());
                    }
                    DefinitionEditorCommand::Rename(key) => ComponentAction::RenameResource {
                        resource: node,
                        key,
                    },
                    DefinitionEditorCommand::Duplicate(key) => {
                        let owner = entry.owner.ok_or("Choose a resource in a dictionary")?;
                        let key = self.unique_component_key(owner, &key);
                        created = Some((owner, key.clone()));
                        ComponentAction::DuplicateResource {
                            resource: node,
                            key,
                        }
                    }
                    DefinitionEditorCommand::CopyTemplate(key) => {
                        let owner = entry
                            .owner
                            .unwrap_or_else(|| self.session.borrow().document().ast().root());
                        let key = self.unique_component_key(owner, &key);
                        created = Some((owner, key.clone()));
                        open_canvas = true;
                        ComponentAction::CopyTemplate {
                            template: node,
                            owner,
                            key,
                        }
                    }
                    DefinitionEditorCommand::Remove => {
                        ComponentAction::RemoveResource { resource: node }
                    }
                    DefinitionEditorCommand::Apply { property, dynamic } => {
                        ComponentAction::ApplyResource {
                            resource: node,
                            targets: self.session.borrow().selection().to_vec(),
                            property,
                            dynamic,
                        }
                    }
                    DefinitionEditorCommand::SetSetter { property, value } => {
                        ComponentAction::SetSetter {
                            owner: node,
                            property,
                            value: Some(value),
                        }
                    }
                    DefinitionEditorCommand::SetResourceValue(value) => {
                        ComponentAction::SetResourceValue {
                            resource: node,
                            value,
                        }
                    }
                    DefinitionEditorCommand::ReplaceSource(source) => {
                        ComponentAction::ReplaceDefinition { node, source }
                    }
                    DefinitionEditorCommand::Create { .. }
                    | DefinitionEditorCommand::CopyProvided { .. }
                    | DefinitionEditorCommand::OpenProvidedSource
                    | DefinitionEditorCommand::InsertControl => unreachable!(),
                }
            }
        };
        self.session.borrow_mut().edit_component(action)?;
        {
            let mut state = self.components.borrow_mut();
            state.stamp = None;
            state.create_open = false;
        }
        let new_node = created.and_then(|(owner, key)| {
            self.session
                .borrow()
                .component_catalog()
                .ok()?
                .entries
                .into_iter()
                .find(|entry| {
                    entry.key.as_deref() == Some(&key)
                        && (entry.owner == Some(owner) || entry.scope == owner)
                })
                .map(|entry| entry.node)
        });
        if let Some(node) = new_node {
            self.components.borrow_mut().selection = Some(LibrarySelection::Document(node));
        }
        self.refresh(true);
        if open_canvas {
            if let Some(node) = new_node {
                self.edit_component_on_canvas(node)?;
            }
        }
        Ok(())
    }

    fn unique_component_key(&self, owner: NodeId, suggested: &str) -> String {
        let keys = self
            .session
            .borrow()
            .component_catalog()
            .unwrap_or_default()
            .entries
            .into_iter()
            .filter(|entry| entry.owner == Some(owner) || entry.scope == owner)
            .filter_map(|entry| entry.key)
            .collect::<BTreeSet<_>>();
        if !keys.contains(suggested) {
            return suggested.into();
        }
        let mut index = 2;
        loop {
            let key = format!("{suggested}{index}");
            if !keys.contains(&key) {
                return key;
            }
            index += 1;
        }
    }

    fn begin_component_drag(
        &self,
        item: &ComponentLibraryItem,
        event: &PointerPressedEventArgs,
    ) -> Result<(), String> {
        if let Some(id) = &item.asset_id {
            let asset = self
                .registered_control(id)
                .ok_or("The control asset is no longer registered")?;
            self.surface.begin_asset_drag(asset.source, event);
            return Ok(());
        }
        if let Some(asset) = item
            .id
            .strip_prefix("definition:")
            .and_then(|id| self.components.borrow().definitions.get(id).cloned())
            .filter(|asset| {
                matches!(
                    asset.kind,
                    ComponentKind::UserControl | ComponentKind::CustomControl
                )
            })
        {
            self.surface
                .begin_asset_drag(control_types::control_instance_source(&asset)?, event);
            return Ok(());
        }
        let Some(node) = item.node else {
            return Err("Create an editable copy before dragging a library definition".into());
        };
        let session = self.session.borrow();
        let document = session.document();
        let entry = session
            .component_catalog()?
            .entries
            .into_iter()
            .find(|entry| entry.node == node)
            .ok_or("The resource no longer exists")?;
        let key = entry
            .key
            .ok_or("Only named resources can be dropped on property fields")?;
        let value = ferroui_designer_tools::component_resource_expression(
            document,
            node,
            &key,
            event.key_modifiers().contains(KeyModifiers::SHIFT),
        )
        .map_err(|error| error.to_string())?;
        let reference = crate::property_controls::PropertyReference {
            document_uri: session.document_uri().into(),
            revision: document.revision(),
            ast: document.ast_snapshot(),
            node,
            kind: crate::property_controls::PropertyReferenceKind::Resource,
        };
        let item =
            DataTransferItem::create(&crate::property_value_data_format(), Some(value.clone()));
        item.set(
            &crate::property_controls::property_reference_data_format(),
            Some(reference),
        );
        item.set(&DataFormat::text(), Some(value));
        let data = DataTransfer::new();
        data.add(item);
        drop(session);
        let operation = DragDrop::do_drag_drop_async(
            event,
            data as Rc<dyn IDataTransfer>,
            DragDropEffects::COPY,
        );
        drop(
            Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                let _ = operation.await;
            }),
        );
        Ok(())
    }

    fn refresh_component_instance(self: &Rc<Self>, catalog: &ComponentCatalog) {
        let host = self
            .root
            .get_control::<ContentControl>("ComponentInstanceHost");
        let session = self.session.borrow();
        let Some(selected) = session.selection().first().copied() else {
            host.set_is_visible(false);
            host.set_content(None);
            return;
        };
        let selected_type = session.document().ast().node(selected).and_then(|node| {
            control_types::registered_type(session.document(), selected, &node.name)
        });
        let writable =
            !session.is_read_only() && session.document().is_valid() && !self.pending_source.get();
        let references = catalog
            .entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.kind,
                    ComponentKind::ControlTheme
                        | ComponentKind::ControlTemplate
                        | ComponentKind::DataTemplate
                        | ComponentKind::Style
                )
            })
            .filter_map(|entry| {
                session
                    .component_resource_references(entry.node)
                    .ok()?
                    .into_iter()
                    .find(|reference| reference.node == selected)
                    .map(|reference| (entry.node, entry.display_name.clone(), reference.property))
            })
            .collect::<Vec<_>>();
        let inline = catalog
            .entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.kind,
                    ComponentKind::ControlTemplate | ComponentKind::DataTemplate
                ) && session
                    .document()
                    .ast()
                    .node(entry.node)
                    .and_then(|node| node.parent)
                    .and_then(|parent| session.document().ast().node(parent))
                    .is_some_and(|parent| {
                        parent.kind == NodeKind::PropertyElement && parent.parent == Some(selected)
                    })
            })
            .map(|entry| {
                (
                    entry.node,
                    entry.display_name.clone(),
                    session
                        .document()
                        .ast()
                        .node(entry.node)
                        .and_then(|node| node.parent)
                        .and_then(|id| session.document().ast().node(id))
                        .map(|parent| {
                            parent
                                .local_name()
                                .rsplit('.')
                                .next()
                                .unwrap_or("Template")
                                .to_owned()
                        })
                        .unwrap_or_else(|| "Template".into()),
                )
            })
            .collect::<Vec<_>>();
        drop(session);
        let state = self.components.borrow();
        let control_definitions = state
            .definitions
            .values()
            .filter(|asset| {
                matches!(
                    asset.kind,
                    ComponentKind::UserControl | ComponentKind::CustomControl
                )
            })
            .filter_map(|asset| {
                // Document providers already carry a validated AST, including the
                // declaration's inherited namespaces. Reuse it during frequent
                // selection refreshes instead of reparsing every project control.
                let parsed;
                let (document, node) = if let Some(provider) = state.sources.get(&asset.id) {
                    (provider.document.as_ref(), provider.node)
                } else {
                    parsed = Document::new(&asset.source).ok()?;
                    let node = parsed.ast().root();
                    (&parsed, node)
                };
                // An anonymous UserControl snippet is a reusable fragment, not a
                // declaration for every UserControl type in the document.
                if asset.kind != ComponentKind::CustomControl
                    && asset.target_type.is_none()
                    && control_types::class_name(document.ast().node(node)?).is_none()
                {
                    return None;
                }
                let definition = control_types::definition_type_in_document(
                    document,
                    node,
                    asset.target_type.as_deref(),
                )?;
                std::ptr::eq(definition, selected_type?)
                    .then(|| (asset.id.clone(), asset.label.clone()))
            })
            .collect::<Vec<_>>();
        drop(state);
        let references = references.into_iter().chain(inline).collect::<Vec<_>>();
        host.set_is_visible(!references.is_empty() || !control_definitions.is_empty());
        if references.is_empty() && control_definitions.is_empty() {
            host.set_content(None);
            return;
        }
        let border = Border::new();
        border.set_background(Some(SolidColorBrush::from_uint32(0xff302a40).into()));
        border.set_corner_radius(CornerRadius::uniform(5.0));
        border.set_padding(Thickness::uniform(8.0));
        let panel = StackPanel::new();
        panel.set_spacing(4.0);
        let title = TextBlock::new();
        title.set_text(Some("LINKED DEFINITIONS"));
        title.set_font_size(9.0);
        title.set_font_weight(FontWeight::SemiBold);
        title.set_foreground(Some(SolidColorBrush::from_uint32(0xffc9b8f1).into()));
        panel.children().add(title.upcast::<Control>());
        for (node, name, property) in references {
            let row = Button::new();
            row.classes().add("nav");
            row.set_horizontal_alignment(HorizontalAlignment::Stretch);
            row.set_horizontal_content_alignment(HorizontalAlignment::Left);
            let label = TextBlock::new();
            label.set_text(Some(&format!("{} · {}  ›", property, name)));
            label.set_font_size(11.0);
            label.set_text_wrapping(TextWrapping::Wrap);
            row.set_content(Some(Control::boxed(&label)));
            let weak = Rc::downgrade(self);
            row.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.report(this.inspect_component(node));
                }
            });
            panel.children().add(row.upcast::<Control>());
            let actions = StackPanel::new();
            actions.set_orientation(ferroui_base::layout::Orientation::Horizontal);
            actions.set_spacing(6.0);
            let edit = Button::new();
            edit.classes().add("nav");
            edit.set_name(Some(format!("LinkedDefinitionEdit_{node}")));
            edit.set_content(Some(Rc::new("Edit current".to_owned())));
            let weak = Rc::downgrade(self);
            let guard = ComponentContext::capture(self);
            edit.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let result = guard
                        .check(&this)
                        .and_then(|_| this.edit_component_on_canvas(node));
                    this.report(result);
                }
            });
            actions.children().add(edit.upcast::<Control>());
            if let Some(entry) = catalog
                .entries
                .iter()
                .find(|entry| entry.node == node)
                .filter(|entry| {
                    matches!(
                        entry.kind,
                        ComponentKind::ControlTemplate
                            | ComponentKind::DataTemplate
                            | ComponentKind::ControlTheme
                    )
                })
            {
                let copy = Button::new();
                copy.classes().add("nav");
                copy.set_name(Some(format!("LinkedDefinitionCopy_{node}")));
                copy.set_content(Some(Rc::new("Edit a copy".to_owned())));
                copy.set_is_enabled(writable);
                let owner = entry.owner.unwrap_or(selected);
                let stem = entry.key.as_deref().unwrap_or("Template").to_owned() + "Copy";
                let guard = ComponentContext::capture(self);
                let weak = Rc::downgrade(self);
                copy.click(move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        let result = guard.check(&this).and_then(|_| {
                            let key = this.unique_component_key(owner, &stem);
                            let copied = this
                                .session
                                .borrow_mut()
                                .copy_definition_for_instance(node, selected, &property, &key)?;
                            this.refresh(true);
                            this.edit_component_on_canvas(copied)
                        });
                        this.report(result);
                    }
                });
                actions.children().add(copy.upcast::<Control>());
            }
            panel.children().add(actions.upcast::<Control>());
        }
        for (id, name) in control_definitions {
            let row = Button::new();
            row.classes().add("nav");
            row.set_horizontal_alignment(HorizontalAlignment::Stretch);
            row.set_horizontal_content_alignment(HorizontalAlignment::Left);
            row.set_content(Some(Rc::new(format!("Control definition · {name}  ›"))));
            let weak = Rc::downgrade(self);
            row.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.report(this.open_component_document(&id));
                }
            });
            panel.children().add(row.upcast::<Control>());
        }
        border.set_child(Some(panel.upcast::<Control>()));
        host.set_content(Some(Control::boxed(&border)));
    }
}

fn validate_definition_asset(asset: &DesignerDefinitionAsset) -> Result<(), String> {
    if asset.id.trim().is_empty() || asset.label.trim().is_empty() {
        return Err("A component definition requires an id and a label".into());
    }
    let document = Document::new(&asset.source).map_err(|error| error.to_string())?;
    if document
        .ast()
        .roots()
        .iter()
        .filter(|node| {
            document
                .ast()
                .node(**node)
                .is_some_and(|node| node.kind == NodeKind::Element)
        })
        .count()
        != 1
    {
        return Err("A library definition requires one XAML object".into());
    }
    let root = document
        .ast()
        .roots()
        .iter()
        .copied()
        .find(|node| {
            document
                .ast()
                .node(*node)
                .is_some_and(|node| node.kind == NodeKind::Element)
        })
        .ok_or("A library definition requires a XAML object")?;
    let entry = ComponentCatalog::from_document(&document)
        .map_err(|error| error.to_string())?
        .entries
        .into_iter()
        .find(|entry| entry.node == root)
        .ok_or("The registered source is not a component definition")?;
    if entry.kind != asset.kind {
        return Err("The registered component kind must match its actual XAML source".into());
    }
    Ok(())
}

fn library_filter(kind: ComponentKind) -> ComponentLibraryFilter {
    match kind {
        ComponentKind::UserControl | ComponentKind::CustomControl => {
            ComponentLibraryFilter::Components
        }
        ComponentKind::ControlTemplate | ComponentKind::DataTemplate => {
            ComponentLibraryFilter::Templates
        }
        ComponentKind::ControlTheme | ComponentKind::Style => ComponentLibraryFilter::Themes,
        ComponentKind::Resource => ComponentLibraryFilter::Resources,
        ComponentKind::ResourceDictionary => ComponentLibraryFilter::Dictionaries,
    }
}
fn display_kind(kind: ComponentKind) -> String {
    kind_title(kind)
        .split_whitespace()
        .map(|word| {
            let mut value = word.to_ascii_lowercase();
            if let Some(first) = value.get_mut(..1) {
                first.make_ascii_uppercase();
            }
            value
        })
        .collect::<Vec<_>>()
        .join(" ")
}
fn node_title(document: &Document, node: NodeId) -> String {
    document
        .ast()
        .node(node)
        .map(|node| node.xaml_name().unwrap_or(node.local_name()).to_owned())
        .unwrap_or_else(|| "Document".into())
}
fn library_item(document: &Document, entry: &ComponentEntry) -> ComponentLibraryItem {
    let node = document.ast().node(entry.node);
    let swatch = node
        .filter(|node| node.local_name() == "SolidColorBrush")
        .and_then(|node| node.attribute("Color"))
        .map(str::to_owned);
    ComponentLibraryItem {
        id: format!("document:{}", entry.node),
        title: entry.display_name.clone(),
        kind: display_kind(entry.kind),
        detail: format!(
            "{} · {}",
            entry.type_name,
            node_title(document, entry.scope)
        ),
        filter: library_filter(entry.kind),
        node: Some(entry.node),
        asset_id: None,
        swatch,
        usages: entry.usages,
    }
}
fn default_apply_property(entry: &ComponentEntry) -> &'static str {
    match entry.kind {
        ComponentKind::ControlTheme => "Theme",
        ComponentKind::ControlTemplate => "Template",
        ComponentKind::DataTemplate => "ContentTemplate",
        ComponentKind::Style => "Styles",
        _ if entry.type_name.ends_with("Brush") => "Background",
        _ => "Content",
    }
}
fn type_token(value: &str) -> Option<String> {
    match MarkupValue::parse(value).ok()? {
        MarkupValue::Literal(value) => Some(value),
        MarkupValue::Extension(extension) if extension.name.rsplit(':').next() == Some("Type") => {
            extension.arguments.into_iter().find_map(|argument| {
                if argument.name.is_none() || argument.name.as_deref() == Some("TypeName") {
                    if let MarkupValue::Literal(value) = argument.value {
                        Some(value)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
        }
        _ => None,
    }
}
fn preview_target_type(entry: &ComponentEntry) -> Option<String> {
    if entry.kind == ComponentKind::DataTemplate {
        return None;
    }
    if entry.kind == ComponentKind::Style {
        return entry
            .target_type
            .as_deref()?
            .split(|character: char| {
                character.is_whitespace() || matches!(character, '.' | '#' | ':' | '>')
            })
            .next()
            .filter(|value| !value.is_empty() && *value != "^")
            .map(|value| value.replace('|', ":"));
    }
    entry.target_type.as_deref().and_then(type_token)
}
fn definition_type(document: &Document, entry: &ComponentEntry) -> Option<&'static TypeInfo> {
    let node = document.ast().node(entry.node)?;
    let token = if matches!(
        entry.kind,
        ComponentKind::ControlTheme | ComponentKind::Style
    ) {
        preview_target_type(entry)?
    } else {
        node.name.clone()
    };
    control_types::registered_type(document, entry.node, &token)
}

#[cfg(test)]
#[path = "workspace_components_tests.rs"]
mod tests;
