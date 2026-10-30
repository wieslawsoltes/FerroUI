// All source helper bodies are present and tested. The native source item classes
// that own this helper are the next dependency; keep the allowance scoped here.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod item_drag_helper;

// Source consumers are being ported; retain allowances only on those internal modules.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod dock_window_state_helper;
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod drag_dockable_resolver;
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod drag_preview_context;
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod host_window_content_cleanup;
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod tool_menu_lifetime;
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod visual_root_compat_extensions;

#[cfg(test)]
mod host_helpers_tests;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod dock_data_template_helper;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod scroll_viewer_mouse_wheel_hook_helper;
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod tab_strip_mouse_wheel_scroll_helper;
#[cfg(test)]
mod tab_strip_mouse_wheel_tests;

pub(crate) mod managed_host_window_access;

pub(crate) mod event_type;
pub(crate) mod window_activation_helper;
pub(crate) mod window_drag_helper;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod managed_window_registry;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod dock_helpers;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod global_docking_helper;

pub(crate) mod services;

pub(crate) mod drag_preview_helper;

pub(crate) mod dock_control_state;

pub(crate) mod adorner_helper;

pub(crate) mod dock_manager_state;

pub(crate) mod default_drag_offset_calculator;

pub(crate) mod managed_host_window_state;

pub(crate) mod host_window_state;
