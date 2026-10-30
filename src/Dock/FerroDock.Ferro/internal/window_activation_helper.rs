// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::managed_host_window_access::managed_host;
use ferrodock_model::core::IFactory;
use ferroui_base::{Ref, Visual};
use ferroui_controls::{Control, Window};
use std::{any::TypeId, collections::HashSet};
/// Original window activation ordering used while dragging a managed window.
pub(crate) struct WindowActivationHelper;
impl WindowActivationHelper {
    pub(crate) fn activate_all_windows(factory: &dyn IFactory, initiator: Option<&Visual>) {
        let mut windows: Vec<Ref<Window>> = Vec::new();
        let mut known_windows = HashSet::new();
        let mut managed_hosts = Vec::new();
        for host in factory.host_windows().iter() {
            if let Some(window) = host
                .clone()
                .boxed_interface(TypeId::of::<Ref<Window>>())
                .and_then(|v| v.downcast_ref::<Ref<Window>>().cloned())
            {
                if known_windows.insert(window.clone()) {
                    windows.push(window);
                }
            }
        }
        for host in factory.host_windows().iter() {
            if let Some(managed) = managed_host(&host) {
                managed_hosts.push((host, managed));
            }
        }
        for dock in factory.dock_controls().iter() {
            if let Some(window) = dock
                .boxed_interface(TypeId::of::<Ref<Control>>())
                .and_then(|v| v.downcast_ref::<Ref<Control>>().cloned())
                .and_then(|c| c.visual_root())
                .and_then(|r| r.cast::<Window>())
            {
                if known_windows.insert(window.clone()) {
                    windows.push(window);
                }
            }
        }
        // Preserve the original Window root test. The native drawn root can instead be
        // TopLevelHost; that framework/runtime difference is explicitly tracked.
        let root_window = initiator
            .and_then(Visual::visual_root)
            .and_then(|r| r.cast::<Window>());
        if let Some(root) = &root_window {
            windows.retain(|w| w != root);
        }
        Window::sort_windows_by_z_order(&mut windows);
        for window in windows {
            if window.is_visible() {
                window.activate();
            }
        }
        let mut sorted_hosts: Vec<_> = managed_hosts
            .into_iter()
            .map(|(host, managed)| (managed.managed_z_index(), host))
            .collect();
        sorted_hosts.sort_by_key(|(z, _)| *z);
        for (_, host) in sorted_hosts {
            host.set_active();
        }
        if let Some(window) = root_window.filter(|w| w.is_visible()) {
            window.activate();
        }
    }
}
