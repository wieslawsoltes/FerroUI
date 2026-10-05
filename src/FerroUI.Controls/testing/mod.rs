//! Test doubles for the tests of this crate and of the crates built on it.

mod compositor_test_services;
mod test_services;
mod unit_test_application;

pub use compositor_test_services::CompositorTestServices;
pub use test_services::{MockRuntimePlatform, TestServices};
pub use unit_test_application::{UnitTestApplication, UnitTestApplicationScope};

mod mock_screen;
mod mock_window_impl;
mod mock_windowing_platform;
mod null_renderer;
mod test_clipboard;
mod test_log_sink;

pub use mock_screen::{mock_screen, MockScreenImpl};
pub use mock_window_impl::{MockCall, MockImplKind, MockWindowImpl, SCREEN_SIZE};
pub use mock_windowing_platform::MockWindowingPlatform;
pub use null_renderer::NullRenderer;
pub use test_clipboard::{TestClipboardFailure, TestClipboardImpl};
pub use test_log_sink::{LogCallback, TestLogSink};
mod test_services_windowing;
mod test_theme;
pub use test_theme::{add_template_theme, create_test_theme, embeddable_control_root_template, top_level_template};
pub use test_theme::{
    decorations_template_theme, window_drawn_decorations_template, FuncWindowDrawnDecorationsTemplate,
};
pub use test_services_windowing::HeadlessCursorFactoryStub;
mod test_theme_command_bar;
pub use test_theme_command_bar::{
    add_command_bar_themes, command_bar_button_theme, command_bar_separator_theme, command_bar_template,
    command_bar_theme, command_bar_toggle_button_theme,
};
mod test_theme_split_view;
pub use test_theme_split_view::{
    add_split_view_list_box_themes, add_split_view_themes, create_split_view_list_box_theme, split_view_theme,
};
mod test_theme_autocomplete;
pub use test_theme_autocomplete::add_autocomplete_themes;
mod test_theme_notifications;
pub use test_theme_notifications::{
    add_notifications_themes, notification_card_template, notification_card_theme,
    window_notification_manager_template, window_notification_manager_theme,
};
mod test_theme_page;
pub use test_theme_page::{
    add_page_themes, carousel_page_template, carousel_template, content_page_template, create_page_test_theme,
};
