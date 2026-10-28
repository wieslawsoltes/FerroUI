//! Explicit canvas layer choice and matching-type selection. These commands change only
//! selection; opening a menu never changes source, history, or the primary selection.
use super::*;
use crate::CanvasViewport;
use ferroui_base::{Matrix, Point, Rect, Size, TypeInfo, Visual};
use ferroui_controls::{ContextMenu, MenuItem, PlacementMode, Separator, ToolTip};
use ferroui_designer_core::{Ast, NodeKind};
use std::rc::Weak;
use std::sync::Arc;

/// One authored control in the native front-to-back hit stack. The path distinguishes
/// unnamed controls and repeated names in separate XAML namescopes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanvasLayerChoice {
    pub node: NodeId,
    pub label: String,
    pub path: String,
}

#[derive(Clone)]
struct AcceptedLayerGeometry {
    visual: WeakRef<Visual>,
    bounds: Rect,
    transform: Matrix,
}

#[derive(Default)]
struct SelectionMenuHost {
    active: RefCell<Option<Ref<ContextMenu>>>,
}

impl Drop for SelectionMenuHost {
    fn drop(&mut self) {
        if let Some(menu) = self.active.get_mut().take() {
            menu.close();
        }
    }
}

/// Accepted canvas context. Native identities and geometry supplement document/view
/// epochs: runtime-only layout or template replacement also retires a retained menu.
#[derive(Clone)]
pub struct CanvasSelectionSnapshot {
    session: Weak<RefCell<DesignerSession>>,
    uri: String,
    syntax: Arc<Ast>,
    revision: u64,
    generation: u64,
    selection: Vec<NodeId>,
    runtime_selection: Option<u64>,
    scope_epoch: u64,
    layer_epoch: u64,
    viewport: CanvasViewport,
    viewport_size: Size,
    surface_transform: Matrix,
    position: Point,
    choices: Vec<CanvasLayerChoice>,
    geometry: Vec<AcceptedLayerGeometry>,
}

impl CanvasSelectionSnapshot {
    pub fn choices(&self) -> &[CanvasLayerChoice] {
        &self.choices
    }

    /// Point in the DesignSurface's artboard coordinate space, before camera projection.
    pub fn position(&self) -> Point {
        self.position
    }
}

impl DesignerSession {
    fn matching_native_type_seed(&self) -> Result<(NodeId, &'static TypeInfo), String> {
        require_current_preview(self)?;
        let seed = self
            .selection()
            .first()
            .copied()
            .ok_or("Select an authored control to find the same control type")?;
        if !authored_layer_available(self, seed) {
            return Err("Select a visible, unlocked control in the current canvas scope".into());
        }
        let visual = self
            .preview
            .visual(seed)
            .ok_or("The selected control has no live preview")?;
        Ok((seed, visual.get_type()))
    }

    /// Finds the primary selection's exact native type in the current authored preview
    /// scope. Namespace aliases converge through TypeInfo identity; derived and unrelated
    /// same-named application types remain distinct. The primary seed stays first.
    pub fn matching_native_type_selection(&self) -> Result<Vec<NodeId>, String> {
        let (seed, type_info) = self.matching_native_type_seed()?;
        let mut result = vec![seed];
        result.extend(
            self.document()
                .ast()
                .nodes_in_source_order()
                .filter(|node| node.id != seed && node.kind == NodeKind::Element)
                .filter(|node| authored_layer_available(self, node.id))
                .filter(|node| {
                    self.preview
                        .visual(node.id)
                        .is_some_and(|visual| visual.get_type() == type_info)
                })
                .map(|node| node.id),
        );
        Ok(result)
    }

    /// Local selection command, including in read-only/viewer sessions. No authored
    /// properties, source history, animation keys, or collaboration edits are produced.
    pub fn select_matching_native_type(&mut self) -> Result<(), String> {
        let selection = self.matching_native_type_selection()?;
        if self.selection() != selection {
            self.select(selection);
        }
        Ok(())
    }
}

impl DesignerWorkspace {
    /// Capture an explicit layer picker without selecting the control under the pointer.
    /// All coordinates and candidates come from the same native hit-testing path used
    /// by ordinary canvas selection and overlap cycling.
    pub fn capture_canvas_selection(
        &self,
        position: Point,
    ) -> Result<CanvasSelectionSnapshot, String> {
        self.ensure_canvas_picker_available()?;
        if !position.x.is_finite() || !position.y.is_finite() {
            return Err("Canvas selection requires a finite artboard position".into());
        }
        self.session.borrow_mut().refresh_runtime_mapping();
        let nodes = self.selectable_canvas_stack(position);
        if nodes.is_empty() {
            return Err("There are no selectable authored controls at this position".into());
        }
        let relative = self.surface.clone().upcast::<Visual>();
        let session = self.session.borrow();
        let mut choices = Vec::with_capacity(nodes.len());
        let mut geometry = Vec::with_capacity(nodes.len());
        for node in nodes {
            let authored = session
                .document()
                .ast()
                .node(node)
                .ok_or("Layer no longer exists")?;
            let visual = session
                .preview
                .visual(node)
                .ok_or("Layer has no native preview")?;
            let type_name = visual.get_type().name();
            let path = authored_layer_path(session.document().ast(), node);
            let label = authored.xaml_name().map_or_else(
                || format!("{type_name} · {path}"),
                |name| format!("{type_name} · {name}"),
            );
            choices.push(CanvasLayerChoice { node, label, path });
            geometry.push(AcceptedLayerGeometry {
                visual: visual.downgrade(),
                bounds: visual.bounds(),
                transform: visual
                    .transform_to_visual(&relative)
                    .ok_or("Layer is detached from the canvas")?,
            });
        }
        Ok(CanvasSelectionSnapshot {
            session: Rc::downgrade(&self.session),
            uri: session.document_uri().to_owned(),
            syntax: session.document().ast_snapshot(),
            revision: session.document().revision(),
            generation: session.generation(),
            selection: session.selection().to_vec(),
            runtime_selection: self.runtime_selection.get(),
            scope_epoch: session.definition_scope_epoch(),
            layer_epoch: session.layer_view_epoch(),
            viewport: self.canvas_viewport(),
            viewport_size: self
                .root
                .get_control::<Control>("ArtboardScroll")
                .bounds()
                .size(),
            surface_transform: self
                .surface
                .transform_to_visual(&self.root.clone().upcast::<Visual>())
                .ok_or("The canvas is detached from its workspace")?,
            position,
            choices,
            geometry,
        })
    }

    /// Accept only a candidate from the captured hit stack while the complete source,
    /// view, selection, and native geometry still describe that context.
    pub fn select_canvas_layer(
        self: &Rc<Self>,
        snapshot: &CanvasSelectionSnapshot,
        node: NodeId,
    ) -> Result<(), String> {
        self.validate_canvas_selection(snapshot)?;
        if !snapshot.choices.iter().any(|choice| choice.node == node) {
            return Err("Choose a layer from this canvas selection menu".into());
        }
        self.session.borrow_mut().select([node]);
        self.runtime_selection.set(None);
        self.reveal_source(node);
        self.refresh(true);
        Ok(())
    }

    pub fn select_matching_native_type(self: &Rc<Self>) -> Result<(), String> {
        self.ensure_selection_source_ready()?;
        if self.runtime_selection.get().is_some() {
            return Err("Select an authored control before finding the same control type".into());
        }
        self.session.borrow_mut().select_matching_native_type()?;
        self.runtime_selection.set(None);
        self.refresh(true);
        Ok(())
    }

    /// Build a native menu for a host-owned context presentation. Opening or dismissing
    /// it is observational; the menu's callbacks retain the accepted selection snapshot.
    pub fn create_canvas_selection_menu(
        self: &Rc<Self>,
        snapshot: &CanvasSelectionSnapshot,
    ) -> Result<Ref<ContextMenu>, String> {
        self.validate_canvas_selection(snapshot)?;
        let snapshot = Rc::new(snapshot.clone());
        let menu = ContextMenu::new();
        crate::chrome::apply_designer_menu_resources(&menu, &self.root);
        let layers = MenuItem::new();
        layers.set_header(Some(Rc::new("Select layer".to_owned())));
        for choice in snapshot.choices() {
            let item = MenuItem::new();
            item.set_header(Some(Rc::new(choice.label.clone())));
            ToolTip::set_tip(&item, Some(Rc::new(choice.path.clone())));
            ferroui_controls::automation::AutomationProperties::set_name(
                &item,
                Some(&format!("{} · {}", choice.label, choice.path)),
            );
            let node = choice.node;
            let snapshot = snapshot.clone();
            let weak = Rc::downgrade(self);
            item.click(move |_, event| {
                if let Some(workspace) = weak.upgrade() {
                    workspace.report(workspace.select_canvas_layer(&snapshot, node));
                }
                event.set_handled(true);
            });
            layers.items().add(Some(Control::boxed(&item)));
        }
        menu.items().add(Some(Control::boxed(&layers)));
        menu.items().add(Some(Control::boxed(Separator::new())));
        let matching = MenuItem::new();
        matching.set_header(Some(Rc::new("Select same control type".to_owned())));
        matching.set_is_enabled(
            self.runtime_selection.get().is_none()
                && self.session.borrow().matching_native_type_seed().is_ok(),
        );
        let snapshot = snapshot.clone();
        let weak = Rc::downgrade(self);
        matching.click(move |_, event| {
            if let Some(workspace) = weak.upgrade() {
                let result = workspace
                    .validate_canvas_selection(&snapshot)
                    .and_then(|()| workspace.select_matching_native_type());
                workspace.report(result);
            }
            event.set_handled(true);
        });
        menu.items().add(Some(Control::boxed(&matching)));
        Ok(menu)
    }

    pub(super) fn connect_selection_tools(self: &Rc<Self>) {
        for name in [
            "LayerActions",
            "ArrangeSelection",
            "PropertyTransferOptions",
        ] {
            crate::chrome::apply_designer_menu_resources(
                &self.root.get_control::<Button>(name),
                &self.root,
            );
        }
        let weak = Rc::downgrade(self);
        self.root
            .get_control::<MenuItem>("LayerSelectMatchingType")
            .click(move |_, event| {
                if let Some(workspace) = weak.upgrade() {
                    workspace.report(workspace.select_matching_native_type());
                }
                event.set_handled(true);
            });

        // Retain only the current detached menu. Attaching it as Control.ContextMenu
        // would let the default handler reopen stale candidates in Pan/Interact mode.
        let menus = SelectionMenuHost::default();
        let weak = Rc::downgrade(self);
        self.surface
            .add_handler(InputElement::context_requested_event(), move |_, event| {
                let Some(workspace) = weak.upgrade() else {
                    return;
                };
                if event.handled()
                    || !event.is_source(&workspace.surface)
                    || workspace.ensure_canvas_picker_available().is_err()
                {
                    return;
                }
                let position = event
                    .try_get_position(Some(&workspace.surface))
                    .or_else(|| {
                        let session = workspace.session.borrow();
                        let node = *session.selection().first()?;
                        let bounds = session
                            .preview
                            .bounds(node, &workspace.surface.clone().upcast())?;
                        Some(Point::new(
                            bounds.x + bounds.width / 2.0,
                            bounds.y + bounds.height / 2.0,
                        ))
                    });
                let Some(position) = position else { return };
                let result = workspace
                    .capture_canvas_selection(position)
                    .and_then(|snapshot| workspace.create_canvas_selection_menu(&snapshot));
                match result {
                    Ok(menu) => {
                        if let Some(previous) = menus.active.borrow_mut().take() {
                            previous.close();
                        }
                        workspace.open_canvas_selection_menu(&menu, position);
                        *menus.active.borrow_mut() = Some(menu);
                        event.set_handled(true);
                    }
                    Err(message) => workspace.report(Err(message)),
                }
            });
    }

    pub(super) fn open_canvas_selection_menu(&self, menu: &ContextMenu, position: Point) {
        // Native popup positioning intersects this target-relative rectangle with
        // the surface bounds before transforming it. A zero-area anchor collapses
        // to the origin; use the same nonempty point rectangle as pointer popups.
        menu.set_placement(PlacementMode::BottomEdgeAlignedLeft);
        menu.set_placement_rect(Some(Rect::new(position.x, position.y, 1.0, 1.0)));
        menu.open_at(Some(&self.surface.clone().upcast()));
    }

    pub(super) fn refresh_selection_tools(&self) {
        let enabled = self.ensure_selection_source_ready().is_ok()
            && self.runtime_selection.get().is_none()
            && self.session.borrow().matching_native_type_seed().is_ok();
        self.root
            .get_control::<MenuItem>("LayerSelectMatchingType")
            .set_is_enabled(enabled);
    }

    fn ensure_selection_source_ready(&self) -> Result<(), String> {
        if self.pending_source.get() {
            return Err("Apply the pending XAML source before selecting canvas layers".into());
        }
        require_current_preview(&self.session.borrow())
    }

    fn ensure_canvas_picker_available(&self) -> Result<(), String> {
        self.ensure_selection_source_ready()?;
        if !self.surface.is_hit_test_visible()
            || !self.surface.is_effectively_visible()
            || self.canvas_navigation_active()
            || self.surface.has_pending_interaction()
        {
            return Err("Finish the active canvas operation and use the Select tool".into());
        }
        Ok(())
    }

    fn selectable_canvas_stack(&self, position: Point) -> Vec<NodeId> {
        let hits = self.surface.hit_stack(position);
        let session = self.session.borrow();
        hits.into_iter()
            .filter(|node| authored_layer_available(&session, *node))
            .collect()
    }

    fn validate_canvas_selection(&self, snapshot: &CanvasSelectionSnapshot) -> Result<(), String> {
        self.ensure_canvas_picker_available()?;
        let session = self.session.borrow();
        let unchanged = Weak::ptr_eq(&snapshot.session, &Rc::downgrade(&self.session))
            && snapshot.uri == session.document_uri()
            && snapshot.revision == session.document().revision()
            && Arc::ptr_eq(&snapshot.syntax, &session.document().ast_snapshot())
            && snapshot.generation == session.generation()
            && snapshot.selection == session.selection()
            && snapshot.runtime_selection == self.runtime_selection.get()
            && snapshot.scope_epoch == session.definition_scope_epoch()
            && snapshot.layer_epoch == session.layer_view_epoch()
            && snapshot.viewport == self.canvas_viewport()
            && snapshot.viewport_size
                == self
                    .root
                    .get_control::<Control>("ArtboardScroll")
                    .bounds()
                    .size()
            && Some(snapshot.surface_transform)
                == self
                    .surface
                    .transform_to_visual(&self.root.clone().upcast::<Visual>());
        let relative = self.surface.clone().upcast::<Visual>();
        let geometry_matches = unchanged
            && snapshot
                .choices
                .iter()
                .zip(&snapshot.geometry)
                .all(|(choice, accepted)| {
                    let Some(visual) = session.preview.visual(choice.node) else {
                        return false;
                    };
                    accepted.visual.upgrade().is_some_and(|original| {
                        std::ptr::eq(&*original, &*visual)
                            && accepted.bounds == visual.bounds()
                            && Some(accepted.transform) == visual.transform_to_visual(&relative)
                    })
                });
        drop(session);
        if !geometry_matches
            || self.selectable_canvas_stack(snapshot.position)
                != snapshot
                    .choices
                    .iter()
                    .map(|choice| choice.node)
                    .collect::<Vec<_>>()
        {
            return Err(
                "The source, selection, canvas view, or layers changed; reopen the layer menu"
                    .into(),
            );
        }
        Ok(())
    }
}

fn require_current_preview(session: &DesignerSession) -> Result<(), String> {
    if session.document().is_valid()
        && matches!(session.preview_status(), PreviewStatus::Ready { revision } if *revision == session.document().revision())
    {
        Ok(())
    } else {
        Err("Canvas selection requires a successful preview of the current XAML".into())
    }
}

fn authored_layer_available(session: &DesignerSession, node: NodeId) -> bool {
    if !session.can_canvas_select(node) || !in_authored_selection_scope(session, node) {
        return false;
    }
    let Some(visual) = session.preview.visual(node) else {
        return false;
    };
    if session.preview.source_id(&visual) != Some(node) {
        return false;
    }
    if session.definition_scope().is_none() && visual.templated_parent().is_some() {
        return false;
    }
    let Some(root) = session.preview.root().map(|root| root.upcast::<Visual>()) else {
        return false;
    };
    // Respect the application's visibility up to the preview root. The designer's
    // Code view may hide the artboard chrome without making source-layer matching invalid.
    let mut current = Some(visual);
    while let Some(visual) = current {
        if !visual.is_visible() {
            return false;
        }
        if std::ptr::eq(&*visual, &*root) {
            return true;
        }
        current = visual.visual_parent();
    }
    false
}

fn in_authored_selection_scope(session: &DesignerSession, node: NodeId) -> bool {
    let ast = session.document().ast();
    let scope = session.definition_scope().map(|scope| scope.node);
    let mut current = Some(node);
    while let Some(id) = current {
        if scope == Some(id) {
            return true;
        }
        let Some(element) = ast.node(id) else {
            return false;
        };
        let template = matches!(
            element.local_name(),
            "ControlTemplate" | "DataTemplate" | "TreeDataTemplate"
        ) && matches!(
            element.namespace_uri.as_deref(),
            Some(
                "https://github.com/ferroui"
                    | "using:FerroUI.Markup.Xaml.Templates"
                    | "clr-namespace:FerroUI.Markup.Xaml.Templates;assembly=FerroUI.Markup.Xaml"
            )
        );
        let keyed = element.attributes.iter().any(|attribute| {
            attribute.name.rsplit(':').next() == Some("Key")
                && attribute.namespace_uri.as_deref()
                    == Some("http://schemas.microsoft.com/winfx/2006/xaml")
        });
        if template || keyed {
            return false;
        }
        current = element.parent;
    }
    scope.is_none()
}

fn authored_layer_path(ast: &Ast, node: NodeId) -> String {
    let mut path = Vec::new();
    let mut current = Some(node);
    while let Some(id) = current {
        let Some(element) = ast.node(id) else { break };
        if element.is_element() {
            let ordinal = element
                .parent
                .and_then(|parent| ast.node(parent))
                .map_or(1, |parent| {
                    parent
                        .children
                        .iter()
                        .filter_map(|child| ast.node(*child))
                        .filter(|child| child.is_element())
                        .position(|child| child.id == id)
                        .map_or(1, |index| index + 1)
                });
            let segment = element.xaml_name().map_or_else(
                || format!("{}[{ordinal}]", element.name),
                |name| format!("{}#{name}", element.name),
            );
            path.push(segment);
        }
        current = element.parent;
    }
    path.reverse();
    path.join(" / ")
}

#[cfg(test)]
#[path = "workspace_selection_tools_tests.rs"]
mod tests;
