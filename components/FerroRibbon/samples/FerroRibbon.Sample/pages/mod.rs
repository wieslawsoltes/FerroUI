pub mod cad_page;
pub use cad_page::CadPage;
mod settings_page;
mod settings_services;
pub use settings_page::SettingsPage;
mod word_page;
pub use word_page::WordPage;
mod cad_tooltip;
#[cfg(all(test, feature = "desktop"))]
mod settings_page_tests;
pub use cad_tooltip::CadToolTipError;
#[cfg(all(test, feature = "desktop"))]
mod cad_action_tests;
mod cad_actions;
#[cfg(all(test, feature = "desktop"))]
mod cad_live_tests;
mod cad_menus;
#[cfg(all(test, feature = "desktop"))]
mod cad_page_tests;
#[cfg(all(test, feature = "desktop"))]
mod cad_popup_coordinates_tests;
mod cad_session;
#[cfg(all(test, feature = "desktop"))]
mod cad_tab_tests;
mod cad_tabs;
#[cfg(all(test, feature = "desktop"))]
mod cad_tooltip_tests;
#[cfg(all(test, feature = "desktop"))]
mod word_page_tests;

mod powerpoint_page;
mod tools_page;
pub use powerpoint_page::PowerPointPage;
pub use tools_page::ToolsPage;
mod excel_page;
#[cfg(all(test, feature = "desktop"))]
mod office_page_tests;
pub use excel_page::ExcelPage;

#[cfg(all(test, feature = "desktop"))]
mod excel_page_tests;

pub(crate) use cad_tooltip::point_in_page_root;
