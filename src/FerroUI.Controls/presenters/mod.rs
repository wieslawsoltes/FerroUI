//! Presenters: the controls that display the content of templated controls.

mod content_presenter;
mod i_content_presenter_host;

pub use content_presenter::ContentPresenter;
pub use i_content_presenter_host::{as_content_presenter_host, register_content_presenter_host, IContentPresenterHost};

#[cfg(test)]
mod content_presenter_tests;

mod scroll_content_presenter;
pub use scroll_content_presenter::ScrollContentPresenter;
#[cfg(test)]
mod scroll_content_presenter_tests;
#[cfg(test)]
mod scroll_content_presenter_tests_i_logical_scrollable;

mod text_presenter;
pub use text_presenter::{TextPresenter, TextPresenterImpl, TextPresenterImplExt, TextPresenterVTable};
#[cfg(test)]
mod content_presenter_text_tests;
#[cfg(test)]
mod text_presenter_tests;

mod items_presenter;
pub(crate) mod panel_container_generator;

pub use items_presenter::ItemsPresenter;

#[cfg(test)]
mod items_presenter_tests;
