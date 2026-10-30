// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::model_projection;
use ferrodock_controls_deferred_content_control::DeferredContentControl;
use ferrodock_controls_recycling::from_recycling_value;
use ferrodock_model::{
    collections::ListHandle,
    core::{IDock, IDockable},
};
use ferroui_base::{data::core::ValueTypes, Ref, StyledElement, Visual};
use ferroui_controls::{ContentControl, Control};
use std::collections::HashSet;
use std::rc::Rc;
/// Original close-time release of visual and detached logical model presenters.
pub(crate) struct HostWindowContentCleanup;
impl HostWindowContentCleanup {
    pub(crate) fn release_models(host: &Control, layout: Option<Rc<dyn IDock>>) {
        // Source snapshots first: clearing one presenter can remove descendants.
        let mut presenters = Vec::new();
        collect_visual_presenters(host, &mut presenters);
        if let Some(layout) = layout {
            collect_model_presenters(Some(layout), host, &mut presenters, &mut HashSet::new());
        }
        for presenter in presenters.iter().rev() {
            presenter.set_content(None);
            if let Some(deferred) = presenter.cast::<DeferredContentControl>() {
                deferred.clear_presentation();
            }
        }
    }
}
fn collect_visual_presenters(visual: &Visual, presenters: &mut Vec<Ref<ContentControl>>) {
    let children = visual.visual_children();
    let mut index = 0;
    while index < children.count() {
        let child = children.get(index);
        if let Some(presenter) = child.cast::<ContentControl>() {
            if model_projection::dockable(presenter.content().as_ref()).is_some() {
                presenters.push(presenter);
            }
        }
        collect_visual_presenters(&child, presenters);
        index += 1;
    }
}
fn collect_model_presenters(
    model: Option<Rc<dyn IDockable>>,
    host: &Control,
    presenters: &mut Vec<Ref<ContentControl>>,
    visited: &mut HashSet<Option<Rc<dyn IDockable>>>,
) {
    if !visited.insert(model.clone()) {
        return;
    }
    let Some(model) = model else { return };
    // The shared native class registry recovers ILogical/StyledElement identity
    // from original model handles; plain reactive models have no logical parent.
    let value = from_recycling_value(&model.clone().boxed_reference());
    if let Some(parent) = ValueTypes::as_object(&*value)
        .and_then(|object| object.cast::<StyledElement>())
        .and_then(|logical| logical.parent())
        .and_then(|parent| parent.cast::<ContentControl>())
    {
        if parent.clone().upcast::<Control>() != host.to_ref()
            && model_projection::dockable(parent.content().as_ref())
                .is_some_and(|content| Rc::ptr_eq(&content, &model))
            && parent
                .visual_root()
                .is_none_or(|root| root == host.to_ref().upcast::<Visual>())
            && !presenters.contains(&parent)
        {
            presenters.push(parent);
        }
    }
    if let Some(dock) = model.clone().as_dock() {
        collect(dock.visible_dockables(), host, presenters, visited);
    }
    if let Some(root) = model.clone().as_root_dock() {
        collect(root.hidden_dockables(), host, presenters, visited);
        collect(root.left_pinned_dockables(), host, presenters, visited);
        collect(root.right_pinned_dockables(), host, presenters, visited);
        collect(root.top_pinned_dockables(), host, presenters, visited);
        collect(root.bottom_pinned_dockables(), host, presenters, visited);
        if let Some(pinned) = root.pinned_dock() {
            collect_model_presenters(Some(pinned), host, presenters, visited);
        }
    }
    if let Some(split) = model.as_split_view_dock() {
        if let Some(pane) = split.pane_dockable() {
            collect_model_presenters(Some(pane), host, presenters, visited);
        }
        if let Some(content) = split.content_dockable() {
            collect_model_presenters(Some(content), host, presenters, visited);
        }
    }
}
fn collect(
    models: Option<ListHandle<Option<Rc<dyn IDockable>>>>,
    host: &Control,
    presenters: &mut Vec<Ref<ContentControl>>,
    visited: &mut HashSet<Option<Rc<dyn IDockable>>>,
) {
    let Some(models) = models else {
        return;
    };
    let mut index = 0;
    while index < models.count() {
        collect_model_presenters(models.get(index), host, presenters, visited);
        index += 1;
    }
}
