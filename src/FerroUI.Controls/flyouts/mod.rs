//! Flyouts: light-dismissable content shown in a popup next to a control.

mod flyout;
mod flyout_base;
mod flyout_presenter;
mod flyout_show_mode;
mod menu_flyout;
mod menu_flyout_presenter;
mod popup_flyout_base;

pub use flyout::Flyout;
pub use flyout_base::{FlyoutBase, FlyoutBaseImpl, FlyoutBaseImplExt, FlyoutBaseVTable};
pub use flyout_presenter::FlyoutPresenter;
pub use flyout_show_mode::FlyoutShowMode;
pub use menu_flyout::MenuFlyout;
pub use menu_flyout_presenter::MenuFlyoutPresenter;
pub use popup_flyout_base::{
    PopupFlyoutBase, PopupFlyoutBaseImpl, PopupFlyoutBaseImplExt, PopupFlyoutBaseVTable,
};

#[cfg(test)]
mod flyout_tests;
