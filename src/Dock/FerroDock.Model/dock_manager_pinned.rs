// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;

fn strip(
    root: &dyn IRootDock,
    alignment: Alignment,
) -> Option<crate::collections::ListHandle<Option<Rc<dyn IDockable>>>> {
    match alignment {
        Alignment::Left | Alignment::Unset => root.left_pinned_dockables(),
        Alignment::Right => root.right_pinned_dockables(),
        Alignment::Top => root.top_pinned_dockables(),
        Alignment::Bottom => root.bottom_pinned_dockables(),
    }
}
impl DockManager {
    fn pinned_alignment(item: Rc<dyn IDockable>, factory: Rc<dyn IFactory>) -> Alignment {
        let Some(root) = factory.find_root(Some(item.clone()), Some(Rc::new(|_| true))) else {
            return Alignment::Unset;
        };
        for alignment in [
            Alignment::Left,
            Alignment::Right,
            Alignment::Top,
            Alignment::Bottom,
        ] {
            if strip(root.as_ref(), alignment)
                .is_some_and(|items| items.contains(&Some((item).clone())))
            {
                return alignment;
            }
        }
        Alignment::Unset
    }
    pub(super) fn tool_to_tool(
        &self,
        source: Rc<dyn ITool>,
        target: Rc<dyn ITool>,
        action: DragAction,
        execute: bool,
    ) -> bool {
        if self.prevent_size_conflicts() && Self::size_conflict(source.as_ref(), target.as_ref()) {
            return false;
        }
        if let Some(factory) = target.owner().and_then(|owner| owner.factory()) {
            if factory.is_dockable_pinned(Some(target.clone()), None) {
                let target_alignment = Self::pinned_alignment(target.clone(), factory.clone());
                let source_alignment = Self::pinned_alignment(source.clone(), factory.clone());
                if factory.is_dockable_pinned(Some(source.clone()), None) {
                    if source_alignment == target_alignment {
                        return false;
                    }
                    if execute {
                        Self::move_pinned(source, source_alignment, target_alignment, factory);
                    }
                } else if execute {
                    if let Some(owner) = source.owner().and_then(|owner| owner.as_tool_dock()) {
                        owner.set_alignment(target_alignment);
                    }
                    factory.pin_dockable(Some(source));
                }
                return true;
            }
        }
        self.service
            .dock_dockable_into_dockable(source, target, action, execute)
    }
    fn move_pinned(
        item: Rc<dyn IDockable>,
        source: Alignment,
        target: Alignment,
        factory: Rc<dyn IFactory>,
    ) {
        let Some(root) = factory.find_root(Some(item.clone()), Some(Rc::new(|_| true))) else {
            return;
        };
        if source != Alignment::Unset {
            if let Some(items) = strip(root.as_ref(), source) {
                items.remove(&Some((item).clone()));
            }
        }
        if strip(root.as_ref(), target).is_none() {
            let items = Some(factory.create_dockable_list(Vec::new()));
            match target {
                Alignment::Unset | Alignment::Left => root.set_left_pinned_dockables(items),
                Alignment::Right => root.set_right_pinned_dockables(items),
                Alignment::Top => root.set_top_pinned_dockables(items),
                Alignment::Bottom => root.set_bottom_pinned_dockables(items),
            }
        }
        if let Some(items) = strip(root.as_ref(), target) {
            items.add(Some(item.clone()));
        }
        if factory.clone().as_factory_base().is_some() {
            factory.on_dockable_moved(Some(item));
        }
    }
}
