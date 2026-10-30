//! The pages of the gallery (namespace `BehaviorsGallery.Views.Pages`): one module and one
//! document per page (`<page>.rs` and `<Page>.xaml`), and the table of the pages ([`PAGES`]), in
//! the order of the tab items of the main view of the reference.
//!
//! A page module declares its class and its entry of the table (`PAGE`, a [`PageInfo`]): its
//! header, the reference file it is ported from, the function that creates it, and the types it
//! registers (its class, and the view models, behaviors and other types only it uses). A page not
//! ported yet is an entry made by [`PageInfo::placeholder`]: the gallery shows its name and that
//! it is not ported yet.
//!
//! Porting a page (`PAGES.md`, "How to port a page") changes two lines of this file, the module
//! line of the page (the comment marker is removed) and its line of the table (the placeholder
//! becomes `<page>::PAGE`); the two lists are in the same order and grouped the same way.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::{Ref, TypeInfo};
use ferroui_controls::Control;

pub mod placeholder;

// The modules of the pages, in the order of the table; a page not ported yet is a comment.
// Group A (pages 1 to 86)
pub mod select_all_on_focus_behavior_view;
pub mod focus_next_element_action_view;
pub mod window_drag_move_behavior_view;
pub mod tilt_effect_behavior_view;
pub mod orbit_effect_behavior_view;
pub mod parallax_behavior_view;
pub mod conditional_action_view;
pub mod switch_case_action_view;
pub mod switch_case_behavior_view;
pub mod conditional_behavior_view;
pub mod lose_focus_on_enter_behavior_view;
pub mod auto_scroll_to_bottom_behavior_view;
pub mod call_method_action_view;
pub mod debounce_throttle_action_view;
pub mod observable_stream_behavior_view;
pub mod async_action_group_view;
pub mod focus_trap_behavior_view;
pub mod screen_reader_announce_action_view;
pub mod global_hotkey_behavior_view;
pub mod visual_debug_behavior_view;
pub mod log_action_view;
pub mod network_status_trigger_view;
pub mod clipboard_monitor_behavior_view;
pub mod change_property_action_view;
pub mod data_trigger_behavior_view;
pub mod data_trigger_behavior_advanced_view;
pub mod multi_data_trigger_behavior_view;
pub mod binding_trigger_behavior_view;
pub mod value_changed_trigger_behavior_view;
pub mod property_changed_trigger_view;
pub mod event_trigger_behavior_view;
pub mod timer_trigger_view;
pub mod task_completed_trigger_view;
pub mod loaded_trigger_view;
pub mod invoke_command_action_view;
pub mod auto_focus_behavior_view;
pub mod auto_select_behavior_view;
pub mod text_box_validation_behavior_view;
pub mod numeric_up_down_validation_behavior_view;
pub mod not_null_validation_rule_view;
pub mod property_validation_behavior_view;
pub mod slider_validation_behavior_view;
pub mod date_picker_validation_behavior_view;
pub mod combo_box_validation_behavior_view;
pub mod focus_control_behavior_view;
pub mod focus_on_attached_behavior_view;
pub mod focus_on_attached_to_visual_tree_behavior_view;
pub mod focus_on_pointer_moved_behavior_view;
pub mod focus_on_pointer_pressed_behavior_view;
pub mod focus_on_visible_behavior_view;
pub mod focus_selected_item_behavior_view;
pub mod routed_event_trigger_behavior_view;
pub mod change_ferro_property_action_view;
pub mod view_model_property_actions_view;
pub mod custom_action_view;
pub mod custom_behavior_view;
pub mod button_click_event_trigger_behavior_view;
pub mod click_event_trigger_view;
pub mod advanced_view;
pub mod add_remove_class_action_view;
pub mod remove_element_action_view;
pub mod remove_item_in_list_box_action_view;
pub mod remove_item_in_items_control_action_view;
pub mod add_item_to_items_control_action_view;
pub mod insert_item_to_items_control_action_view;
pub mod object_template_view;
pub mod clear_items_control_action_view;
pub mod add_range_action_view;
pub mod remove_range_action_view;
pub mod move_item_in_items_control_action_view;
pub mod move_element_actions_view;
pub mod collection_changed_trigger_behavior_view;
pub mod items_control_container_triggers_view;
pub mod remove_items_sample_view;
pub mod adaptive_behavior_view;
pub mod aspect_ratio_behavior_view;
pub mod split_view_state_behavior_view;
pub mod editable_list_box_view;
pub mod editable_draggable_list_box_view;
pub mod editable_double_tapped_list_box_view;
pub mod editable_tree_view_view;
pub mod editable_double_tapped_tree_view_view;
pub mod editable_drag_tree_view_view;
pub mod sliding_animation_view;
pub mod fade_animation_view;
pub mod scale_animation_view;
// Group B (pages 87 to 144)
pub mod rotate_animation_view;
pub mod entrance_animations_view;
pub mod attention_animations_view;
pub mod exit_animations_view;
pub mod framer_motion_animations_view;
pub mod special_animations_view;
pub mod animation_behavior_view;
pub mod animation_completed_trigger_view;
pub mod start_animation_action_view;
pub mod animation_builder_view;
pub mod start_built_animation_action_view;
pub mod run_animation_trigger_view;
pub mod fluid_move_behavior_view;
pub mod selecting_items_control_behavior_view;
pub mod list_box_selection_behaviors_view;
pub mod behavior_collection_template_view;
pub mod action_collection_template_view;
pub mod template_binding_view;
pub mod one_time_binding;
pub mod pointer_triggers_view;
pub mod key_gesture_trigger_view;
pub mod execute_command_behaviors_view;
pub mod button_execute_command_on_key_down_behavior_view;
pub mod storage_provider_view;
pub mod picker_can_execute_command_view;
pub mod clipboard_view;
pub mod get_clipboard_data_action_view;
pub mod get_clipboard_formats_action_view;
pub mod set_clipboard_data_object_action_view;
pub mod focus_control_action_view;
pub mod hide_show_control_action_view;
pub mod show_hide_flyout_action_view;
pub mod show_context_menu_action_view;
pub mod context_dialog_view;
pub mod show_hide_popup_action_view;
pub mod show_on_event_behaviors_view;
pub mod set_enabled_action_view;
pub mod show_notification_action_view;
pub mod dialog_action_view;
pub mod file_drop_handler_view;
pub mod upload_file_view;
pub mod content_control_files_drop_behavior_view;
pub mod drag_and_drop_view;
pub mod drag_drop_commands_view;
pub mod files_preview_view;
pub mod typed_drag_behavior_view;
pub mod draggable_view;
pub mod mouse_drag_behavior_view;
pub mod canvas_drag_behavior_view;
pub mod auto_scroll_during_drag_behavior_view;
pub mod drag_between_panels_view;
pub mod cursor_view;
pub mod drawn_cursor_view;
pub mod if_else_trigger_view;
// pub mod execute_script_action_view;
// pub mod observable_trigger_behavior_view;
pub mod interaction_trigger_behavior_view;
pub mod observable_trigger_behavior_view;
pub mod flyout_binding_view;
// Group C (pages 145 to 234)
pub mod key_trigger_view;
pub mod tool_tip_helpers_view;
pub mod writeable_bitmap_view;
pub mod render_target_bitmap_view;
pub mod launch_uri_or_file_action_view;
pub mod reactive_navigation_view;
pub mod home_page_view; // owner: ReactiveNavigationView
pub mod detail_page_view; // owner: ReactiveNavigationView
pub mod transitions_behavior_view;
pub mod transitions_actions_view;
pub mod transitions_changed_trigger_view;
pub mod notifications_view;
pub mod carousel_navigation_view;
pub mod carousel_selection_changed_trigger_view;
pub mod auto_complete_box_view;
pub mod focus_auto_complete_box_text_box_behavior_view;
pub mod tab_control_navigation_view;
pub mod tab_control_selection_changed_trigger_view;
pub mod theme_variant_view;
pub mod screen_view;
pub mod bind_pointer_over_behavior_view;
pub mod bind_tag_to_visual_root_data_context_behavior_view;
pub mod bounds_observer_behavior_view;
pub mod hide_attached_flyout_behavior_view;
pub mod button_hide_flyout_behavior_view;
pub mod button_hide_flyout_on_click_behavior_view;
pub mod hide_on_key_pressed_behavior_view;
pub mod hide_on_lost_focus_behavior_view;
pub mod size_changed_trigger_view;
pub mod fade_in_behavior_view;
pub mod delayed_load_behavior_view;
pub mod delayed_load_trigger_view;
pub mod delayed_show_control_action_view;
pub mod animate_on_attached_behavior_view;
pub mod async_loading_view;
pub mod theme_variant_trigger_view;
pub mod actual_theme_variant_changed_trigger_view;
pub mod toggle_is_expanded_on_double_tapped_behavior_view;
pub mod tree_view_filter_view;
pub mod viewport_behavior_view;
pub mod scroll_changed_trigger_view;
pub mod scroll_viewer_offset_behavior_view;
pub mod scroll_to_offset_action_view;
pub mod scroll_behaviors_view;
pub mod window_interactions_view;
pub mod events_behaviors_view;
pub mod focus_event_triggers_view;
pub mod key_event_triggers_view;
pub mod pointer_event_triggers_view;
pub mod pointer_events_trigger_view;
pub mod drag_leave_event_trigger_view;
pub mod gesture_event_triggers_view;
pub mod gesture_advanced_triggers_view;
pub mod pull_gesture_triggers_view;
pub mod scroll_gesture_triggers_view;
pub mod touch_pad_gesture_triggers_view;
pub mod popup_event_triggers_view;
pub mod focus_triggers_view;
pub mod key_input_triggers_view;
pub mod pointer_extra_triggers_view;
pub mod gesture_triggers_view;
pub mod icon_view;
pub mod automation_view;
pub mod actual_theme_variant_changed_behavior_view;
pub mod attached_to_logical_tree_behavior_view;
pub mod attached_to_logical_tree_trigger_view;
pub mod attached_to_visual_tree_trigger_view;
pub mod binding_behavior_view;
pub mod data_context_changed_behavior_view;
pub mod data_context_changed_trigger_view;
pub mod detached_from_logical_tree_trigger_view;
pub mod detached_from_visual_tree_trigger_view;
pub mod disposing_behavior_view;
pub mod disposing_trigger_view;
pub mod disposable_action_view;
pub mod initialized_behavior_view;
pub mod initialized_trigger_view;
pub mod invoke_command_action_base_view;
pub mod loaded_behavior_view;
pub mod resources_changed_behavior_view;
pub mod resources_changed_trigger_view;
pub mod routed_event_trigger_base_view;
pub mod unloaded_trigger_view;
pub mod file_system_view;
pub mod network_view;
pub mod toggle_class_action_view;
pub mod launch_uri_action_view;
pub mod screenshot_action_view;
pub mod scroll_to_control_action_view;
pub mod console_beep_action_view;
pub mod break_action_view;
pub mod inactivity_trigger_view;

/// A page of the gallery: an entry of the page table.
pub struct PageInfo {
    /// The header of the tab of the page (the header of its tab item in the reference).
    pub header: &'static str,
    /// The reference file of the page, below the directory of the reference sample, without its
    /// extension (the document and its code-behind).
    pub reference: &'static str,
    /// Creates the page (its constructor loads its document, and panics if it does not load);
    /// `None` for a page not ported yet.
    pub create: Option<fn() -> Ref<Control>>,
    /// The classes the page registers: its class, and the classes only it uses (`X::TYPE`).
    pub types: &'static [&'static TypeInfo],
    /// The classes with a document the page declares (`&X::XAML_CLASS`).
    pub classes: &'static [&'static XamlClass],
    /// The types declared with `ferro_markup_type!` only the page uses: its view models, drop
    /// handlers and converters (`<X as MarkupTyped>::MARKUP`).
    pub markup_types: &'static [&'static MarkupType],
    /// Registers what the untyped value conversions must know about the types only the page uses
    /// (`ValueTypes::register_reference::<X>()`, casts to contracts).
    pub register_value_types: fn(),
}

impl PageInfo {
    /// The entry of a page not ported yet: its header and its reference file.
    pub const fn placeholder(header: &'static str, reference: &'static str) -> Self {
        Self {
            header,
            reference,
            create: None,
            types: &[],
            classes: &[],
            markup_types: &[],
            register_value_types: no_value_types,
        }
    }

    /// The name of the view of the page (the class of the reference).
    pub fn view_name(&self) -> &'static str {
        self.reference.rsplit('/').next().unwrap_or(self.reference)
    }

    /// Whether the page is ported (its entry creates its view).
    pub fn is_ported(&self) -> bool {
        self.create.is_some()
    }

    /// Creates the page, or the placeholder of a page not ported yet.
    pub fn create_view(&'static self) -> Ref<Control> {
        match self.create {
            Some(create) => create(),
            None => placeholder::create(self),
        }
    }

    /// The page with the header or the view name `name` (without regard to case).
    pub fn find(name: &str) -> Option<&'static PageInfo> {
        let name = name.trim();
        PAGES.iter().find(|page| {
            page.header.eq_ignore_ascii_case(name) || page.view_name().eq_ignore_ascii_case(name)
        })
    }
}

/// `register_value_types` of a page whose types need no registration.
pub fn no_value_types() {}

/// The pages, in the order of the reference (a constant, so that the table of the classes is built
/// from it at compile time).
#[rustfmt::skip]
pub(crate) const PAGE_TABLE: &[PageInfo] = &[
    // Group A (pages 1 to 86)
    select_all_on_focus_behavior_view::PAGE,
    focus_next_element_action_view::PAGE,
    window_drag_move_behavior_view::PAGE,
    tilt_effect_behavior_view::PAGE,
    orbit_effect_behavior_view::PAGE,
    parallax_behavior_view::PAGE,
    conditional_action_view::PAGE,
    switch_case_action_view::PAGE,
    switch_case_behavior_view::PAGE,
    conditional_behavior_view::PAGE,
    lose_focus_on_enter_behavior_view::PAGE,
    auto_scroll_to_bottom_behavior_view::PAGE,
    call_method_action_view::PAGE,
    debounce_throttle_action_view::PAGE,
    observable_stream_behavior_view::PAGE,
    async_action_group_view::PAGE,
    focus_trap_behavior_view::PAGE,
    screen_reader_announce_action_view::PAGE,
    global_hotkey_behavior_view::PAGE,
    visual_debug_behavior_view::PAGE,
    log_action_view::PAGE,
    network_status_trigger_view::PAGE,
    clipboard_monitor_behavior_view::PAGE,
    change_property_action_view::PAGE,
    data_trigger_behavior_view::PAGE,
    data_trigger_behavior_advanced_view::PAGE,
    multi_data_trigger_behavior_view::PAGE,
    binding_trigger_behavior_view::PAGE,
    value_changed_trigger_behavior_view::PAGE,
    property_changed_trigger_view::PAGE,
    event_trigger_behavior_view::PAGE,
    timer_trigger_view::PAGE,
    task_completed_trigger_view::PAGE,
    loaded_trigger_view::PAGE,
    invoke_command_action_view::PAGE,
    auto_focus_behavior_view::PAGE,
    auto_select_behavior_view::PAGE,
    text_box_validation_behavior_view::PAGE,
    numeric_up_down_validation_behavior_view::PAGE,
    not_null_validation_rule_view::PAGE,
    property_validation_behavior_view::PAGE,
    slider_validation_behavior_view::PAGE,
    date_picker_validation_behavior_view::PAGE,
    combo_box_validation_behavior_view::PAGE,
    focus_control_behavior_view::PAGE,
    focus_on_attached_behavior_view::PAGE,
    focus_on_attached_to_visual_tree_behavior_view::PAGE,
    focus_on_pointer_moved_behavior_view::PAGE,
    focus_on_pointer_pressed_behavior_view::PAGE,
    focus_on_visible_behavior_view::PAGE,
    focus_selected_item_behavior_view::PAGE,
    routed_event_trigger_behavior_view::PAGE,
    change_ferro_property_action_view::PAGE,
    view_model_property_actions_view::PAGE,
    custom_action_view::PAGE,
    custom_behavior_view::PAGE,
    button_click_event_trigger_behavior_view::PAGE,
    click_event_trigger_view::PAGE,
    advanced_view::PAGE,
    add_remove_class_action_view::PAGE,
    remove_element_action_view::PAGE,
    remove_item_in_list_box_action_view::PAGE,
    remove_item_in_items_control_action_view::PAGE,
    add_item_to_items_control_action_view::PAGE,
    insert_item_to_items_control_action_view::PAGE,
    object_template_view::PAGE,
    clear_items_control_action_view::PAGE,
    add_range_action_view::PAGE,
    remove_range_action_view::PAGE,
    move_item_in_items_control_action_view::PAGE,
    move_element_actions_view::PAGE,
    collection_changed_trigger_behavior_view::PAGE,
    items_control_container_triggers_view::PAGE,
    remove_items_sample_view::PAGE,
    adaptive_behavior_view::PAGE,
    aspect_ratio_behavior_view::PAGE,
    split_view_state_behavior_view::PAGE,
    editable_list_box_view::PAGE,
    editable_draggable_list_box_view::PAGE,
    editable_double_tapped_list_box_view::PAGE,
    editable_tree_view_view::PAGE,
    editable_double_tapped_tree_view_view::PAGE,
    editable_drag_tree_view_view::PAGE,
    sliding_animation_view::PAGE,
    fade_animation_view::PAGE,
    scale_animation_view::PAGE,
    // Group B (pages 87 to 144)
    rotate_animation_view::PAGE,
    entrance_animations_view::PAGE,
    attention_animations_view::PAGE,
    exit_animations_view::PAGE,
    framer_motion_animations_view::PAGE,
    special_animations_view::PAGE,
    animation_behavior_view::PAGE,
    animation_completed_trigger_view::PAGE,
    start_animation_action_view::PAGE,
    animation_builder_view::PAGE,
    start_built_animation_action_view::PAGE,
    run_animation_trigger_view::PAGE,
    fluid_move_behavior_view::PAGE,
    selecting_items_control_behavior_view::PAGE,
    list_box_selection_behaviors_view::PAGE,
    behavior_collection_template_view::PAGE,
    action_collection_template_view::PAGE,
    template_binding_view::PAGE,
    one_time_binding::PAGE,
    pointer_triggers_view::PAGE,
    key_gesture_trigger_view::PAGE,
    execute_command_behaviors_view::PAGE,
    button_execute_command_on_key_down_behavior_view::PAGE,
    storage_provider_view::PAGE,
    picker_can_execute_command_view::PAGE,
    clipboard_view::PAGE,
    get_clipboard_data_action_view::PAGE,
    get_clipboard_formats_action_view::PAGE,
    set_clipboard_data_object_action_view::PAGE,
    focus_control_action_view::PAGE,
    hide_show_control_action_view::PAGE,
    show_hide_flyout_action_view::PAGE,
    show_context_menu_action_view::PAGE,
    context_dialog_view::PAGE,
    show_hide_popup_action_view::PAGE,
    show_on_event_behaviors_view::PAGE,
    set_enabled_action_view::PAGE,
    show_notification_action_view::PAGE,
    dialog_action_view::PAGE,
    file_drop_handler_view::PAGE,
    upload_file_view::PAGE,
    content_control_files_drop_behavior_view::PAGE,
    drag_and_drop_view::PAGE,
    drag_drop_commands_view::PAGE,
    files_preview_view::PAGE,
    typed_drag_behavior_view::PAGE,
    draggable_view::PAGE,
    mouse_drag_behavior_view::PAGE,
    canvas_drag_behavior_view::PAGE,
    auto_scroll_during_drag_behavior_view::PAGE,
    drag_between_panels_view::PAGE,
    cursor_view::PAGE,
    drawn_cursor_view::PAGE,
    if_else_trigger_view::PAGE,
    PageInfo::placeholder("ExecuteScriptAction", "Views/Pages/ExecuteScriptActionView"),
    observable_trigger_behavior_view::PAGE,
    interaction_trigger_behavior_view::PAGE,
    flyout_binding_view::PAGE,
    // Group C (pages 145 to 234)
    key_trigger_view::PAGE,
    tool_tip_helpers_view::PAGE,
    writeable_bitmap_view::PAGE,
    render_target_bitmap_view::PAGE,
    launch_uri_or_file_action_view::PAGE,
    reactive_navigation_view::PAGE,
    transitions_behavior_view::PAGE,
    transitions_actions_view::PAGE,
    transitions_changed_trigger_view::PAGE,
    notifications_view::PAGE,
    carousel_navigation_view::PAGE,
    carousel_selection_changed_trigger_view::PAGE,
    auto_complete_box_view::PAGE,
    focus_auto_complete_box_text_box_behavior_view::PAGE,
    tab_control_navigation_view::PAGE,
    tab_control_selection_changed_trigger_view::PAGE,
    theme_variant_view::PAGE,
    screen_view::PAGE,
    bind_pointer_over_behavior_view::PAGE,
    bind_tag_to_visual_root_data_context_behavior_view::PAGE,
    bounds_observer_behavior_view::PAGE,
    hide_attached_flyout_behavior_view::PAGE,
    button_hide_flyout_behavior_view::PAGE,
    button_hide_flyout_on_click_behavior_view::PAGE,
    hide_on_key_pressed_behavior_view::PAGE,
    hide_on_lost_focus_behavior_view::PAGE,
    size_changed_trigger_view::PAGE,
    fade_in_behavior_view::PAGE,
    delayed_load_behavior_view::PAGE,
    delayed_load_trigger_view::PAGE,
    delayed_show_control_action_view::PAGE,
    animate_on_attached_behavior_view::PAGE,
    async_loading_view::PAGE,
    theme_variant_trigger_view::PAGE,
    actual_theme_variant_changed_trigger_view::PAGE,
    toggle_is_expanded_on_double_tapped_behavior_view::PAGE,
    tree_view_filter_view::PAGE,
    viewport_behavior_view::PAGE,
    scroll_changed_trigger_view::PAGE,
    scroll_viewer_offset_behavior_view::PAGE,
    scroll_to_offset_action_view::PAGE,
    scroll_behaviors_view::PAGE,
    window_interactions_view::PAGE,
    events_behaviors_view::PAGE,
    focus_event_triggers_view::PAGE,
    key_event_triggers_view::PAGE,
    pointer_event_triggers_view::PAGE,
    pointer_events_trigger_view::PAGE,
    drag_leave_event_trigger_view::PAGE,
    gesture_event_triggers_view::PAGE,
    gesture_advanced_triggers_view::PAGE,
    pull_gesture_triggers_view::PAGE,
    scroll_gesture_triggers_view::PAGE,
    touch_pad_gesture_triggers_view::PAGE,
    popup_event_triggers_view::PAGE,
    focus_triggers_view::PAGE,
    key_input_triggers_view::PAGE,
    pointer_extra_triggers_view::PAGE,
    gesture_triggers_view::PAGE,
    icon_view::PAGE,
    automation_view::PAGE,
    actual_theme_variant_changed_behavior_view::PAGE,
    attached_to_logical_tree_behavior_view::PAGE,
    attached_to_logical_tree_trigger_view::PAGE,
    attached_to_visual_tree_trigger_view::PAGE,
    binding_behavior_view::PAGE,
    data_context_changed_behavior_view::PAGE,
    data_context_changed_trigger_view::PAGE,
    detached_from_logical_tree_trigger_view::PAGE,
    detached_from_visual_tree_trigger_view::PAGE,
    disposing_behavior_view::PAGE,
    disposing_trigger_view::PAGE,
    disposable_action_view::PAGE,
    initialized_behavior_view::PAGE,
    initialized_trigger_view::PAGE,
    invoke_command_action_base_view::PAGE,
    loaded_behavior_view::PAGE,
    resources_changed_behavior_view::PAGE,
    resources_changed_trigger_view::PAGE,
    routed_event_trigger_base_view::PAGE,
    unloaded_trigger_view::PAGE,
    file_system_view::PAGE,
    network_view::PAGE,
    toggle_class_action_view::PAGE,
    launch_uri_action_view::PAGE,
    screenshot_action_view::PAGE,
    scroll_to_control_action_view::PAGE,
    console_beep_action_view::PAGE,
    break_action_view::PAGE,
    inactivity_trigger_view::PAGE,
];

/// The pages, in the order of the reference.
pub static PAGES: &[PageInfo] = PAGE_TABLE;
