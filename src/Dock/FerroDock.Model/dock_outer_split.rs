// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::rc::Rc;

pub(crate) fn direction(operation: DockOperation) -> DockOperation {
    match operation {
        DockOperation::OuterLeft => DockOperation::Left,
        DockOperation::OuterRight => DockOperation::Right,
        DockOperation::OuterTop => DockOperation::Top,
        DockOperation::OuterBottom => DockOperation::Bottom,
        _ => DockOperation::None,
    }
}
pub(crate) fn contains(source: Rc<dyn IDockable>, mut content: Option<Rc<dyn IDockable>>) -> bool {
    while let Some(item) = content {
        if Rc::ptr_eq(&source, &item) {
            return true;
        }
        content = item.owner();
    }
    false
}
pub(crate) fn has_other_content(
    region: Option<Rc<dyn IDockable>>,
    source: Rc<dyn IDockable>,
) -> bool {
    if region
        .as_ref()
        .is_some_and(|region| Rc::ptr_eq(region, &source))
        || region.clone().and_then(|item| item.as_splitter()).is_some()
    {
        return false;
    }
    let Some(dock) = region.and_then(|item| item.as_dock()) else {
        return true;
    };
    let Some(children) = dock.visible_dockables() else {
        return false;
    };
    let mut index = 0;
    while index < children.count() {
        if has_other_content(children.get(index), source.clone()) {
            return true;
        }
        index += 1;
    }
    false
}
fn selected_content(source: Option<Rc<dyn IDockable>>) -> Option<Rc<dyn IDockable>> {
    let Some(dock) = source.clone().and_then(|item| item.as_dock()) else {
        return source;
    };
    let children = dock.visible_dockables()?;
    if let Some(active) = dock
        .active_dockable()
        .filter(|active| children.contains(&Some((active).clone())))
    {
        return selected_content(Some(active));
    }
    for child in children.iter() {
        if child.clone().and_then(|item| item.as_splitter()).is_none() {
            if let Some(content) = selected_content(child) {
                return Some(content);
            }
        }
    }
    None
}
pub(crate) fn execute(source: Rc<dyn IDockable>, target: Rc<dyn IDock>, direction: DockOperation) {
    let Some(owner) = source.owner().and_then(|owner| owner.as_dock()) else {
        return;
    };
    let Some(factory) = target.factory() else {
        return;
    };
    let mut focused = factory
        .find_root(
            Some(source.clone()),
            Some(Rc::new(|root| root.is_focusable_root())),
        )
        .and_then(|root| root.focused_dockable());
    if !contains(source.clone(), focused.clone()) {
        focused = selected_content(Some(source.clone()));
    }
    if let Some(parent) = target
        .owner()
        .and_then(|owner| owner.as_proportional_dock())
    {
        if let Some(siblings) = parent.visible_dockables() {
            let mut count = 0;
            for sibling in siblings.iter() {
                if sibling.and_then(|item| item.as_splitter()).is_none() {
                    count += 1;
                }
            }
            if count == 1 {
                parent.set_orientation(
                    if matches!(direction, DockOperation::Left | DockOperation::Right) {
                        Orientation::Horizontal
                    } else {
                        Orientation::Vertical
                    },
                );
            }
        }
    }
    if let Some(dock) = source.clone().as_dock() {
        factory.remove_dockable(Some(dock.clone()), false);
        factory.split_to_dock(target, dock, direction);
        factory.collapse_dock(owner);
    } else {
        let created: Rc<dyn IDock> = if source.clone().as_tool().is_some() {
            factory.create_tool_dock()
        } else {
            factory.create_document_dock()
        };
        if let Some(tool) = (created.clone() as Rc<dyn IDockable>).as_tool_dock() {
            tool.set_alignment(direction.to_alignment());
        }
        created.set_visible_dockables(Some(factory.create_dockable_list(Vec::new())));
        created.set_dock_group(DockGroupValidator::get_effective_dock_group(source.clone()));
        factory.move_dockable_between_docks(Some(owner), Some(created.clone()), Some(source), None);
        factory.split_to_dock(target, created, direction);
    }
    if let Some(focused) = focused {
        if let Some(root) = factory.find_root(
            Some(focused.clone()),
            Some(Rc::new(|root| root.is_focusable_root())),
        ) {
            factory.set_focused_dockable(Some(root), Some(focused));
        }
    }
}
