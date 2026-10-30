// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::model_projection;
use ferrodock_model::core::IDockable;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTypeKind};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::automation::peers::{AutomationPeer, ControlAutomationPeer};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{boxed_reference_equals, items_equal, Control, ItemsControl};
use std::fmt::Write;
use std::rc::Rc;
pub(crate) struct DockAutomationPeerHelper;
impl DockAutomationPeerHelper {
    pub(crate) fn resolve_name(
        owner: &Control,
        fallback: &str,
        dockable: Option<Rc<dyn IDockable>>,
    ) -> String {
        if let Some(name) = nonblank(AutomationProperties::get_name(owner)) {
            return name;
        }
        if let Some(title) = dockable.and_then(|d| nonblank(Some(d.title()))) {
            return title;
        }
        if let Some(title) = model_projection::dockable(owner.data_context().as_ref())
            .and_then(|d| nonblank(Some(d.title())))
        {
            return title;
        }
        fallback.to_owned()
    }
    pub(crate) fn resolve_automation_id(
        owner: &Control,
        dockable: Option<Rc<dyn IDockable>>,
    ) -> String {
        if let Some(id) = nonblank(AutomationProperties::get_automation_id(owner)) {
            return id;
        }
        if let Some(id) = dockable.and_then(|d| nonblank(Some(d.id()))) {
            return id;
        }
        if let Some(id) = model_projection::dockable(owner.data_context().as_ref())
            .and_then(|d| nonblank(Some(d.id())))
        {
            return id;
        }
        String::new()
    }
    pub(crate) fn format_state(states: &[(&str, Option<BoxedValue>)]) -> String {
        let mut builder = String::with_capacity(states.len() * 24);
        for (index, (key, value)) in states.iter().enumerate() {
            if index > 0 {
                builder.push_str("; ");
            }
            builder.push_str(key);
            builder.push('=');
            match value {
                None => builder.push_str("null"),
                Some(value) => {
                    if let Some(value) = value.downcast_ref::<bool>() {
                        builder.push_str(if *value { "true" } else { "false" });
                    } else if let Some(name) = enum_name(value) {
                        builder.push_str(&name);
                    } else {
                        write!(builder, "{}", ValueTypes::to_display_string(Some(value)))
                            .expect("String writing is infallible");
                    }
                }
            }
        }
        builder
    }
    #[cfg_attr(not(test), allow(dead_code))] // Used by source selection peers still being ported.
    pub(crate) fn try_activate_dockable(dockable: Option<Rc<dyn IDockable>>) -> bool {
        let Some(dockable) = dockable else {
            return false;
        };
        let Some(owner) = dockable.owner().and_then(|o| o.as_dock()) else {
            return false;
        };
        if owner.factory().is_none() {
            return false;
        }
        // The source reads Owner.Factory again after each user callback.
        owner
            .factory()
            .expect("source owner factory exists")
            .set_active_dockable(Some(dockable.clone()));
        let root = owner
            .factory()
            .expect("source owner factory exists")
            .find_root(Some(dockable.clone()), Some(Rc::new(|_| true)));
        if let Some(root) = root {
            owner
                .factory()
                .expect("source owner factory exists")
                .set_focused_dockable(Some(root), Some(dockable.clone()));
        }
        owner
            .factory()
            .expect("source owner factory exists")
            .activate_window(Some(dockable));
        true
    }
    pub(crate) fn try_get_container_peer(
        owner: &ItemsControl,
        item: Option<&BoxedValue>,
    ) -> Option<Ref<AutomationPeer>> {
        let item = item?;
        if let Some(container) = owner.container_from_item(&Some(item.clone())) {
            return Some(ControlAutomationPeer::create_peer_for_element(&container));
        }
        let items = owner.items();
        let mut index = 0;
        while index < items.count() {
            let candidate = items.get_at(index);
            if !candidate
                .as_ref()
                .is_some_and(|candidate| boxed_reference_equals(candidate, item))
                && !items_equal(&candidate, &Some(item.clone()))
            {
                index += 1;
                continue;
            }
            if let Some(container) = owner.container_from_index(index as i32) {
                return Some(ControlAutomationPeer::create_peer_for_element(&container));
            }
            break;
        }
        None
    }
    pub(crate) fn to_selection_list(peer: Option<Ref<AutomationPeer>>) -> Vec<Ref<AutomationPeer>> {
        match peer {
            Some(peer) => vec![peer],
            None => Vec::new(),
        }
    }
}
fn nonblank(value: Option<String>) -> Option<String> {
    value.filter(|s| !s.trim().is_empty())
}

// The source's explicit Enum branch precedes general object formatting.
fn enum_name(value: &BoxedValue) -> Option<String> {
    let metadata = MarkupType::find_by_handle((**value).any_value_as_any().type_id())?;
    if metadata.kind != MarkupTypeKind::Enum {
        return None;
    }
    for member in metadata.enum_members {
        if (member.get)().value_eq(&**value) {
            return Some(member.name.to_owned());
        }
    }
    metadata
        .enum_to_value
        .and_then(|convert| convert(&**value))
        .map(|number| number.to_string())
}
