//! `ResolvesMultipleViewModelsAndRespectsOrdering`: the table of the views is sorted by view model,
//! and a view model without a view gets the text of its missing view.

use crate::prelude::*;

pub mod view_models {
    #[derive(PartialEq)]
    pub struct ViewModelBase;

    #[derive(PartialEq)]
    pub struct HomeViewModel;

    #[derive(PartialEq)]
    pub struct ReportsViewModel;

    #[derive(PartialEq)]
    pub struct SettingsViewModel;

    #[derive(PartialEq)]
    pub struct WorkspaceViewModel;
}

pub mod views {
    use crate::prelude::*;

    test_view!(HomeView);
    test_view!(ReportsView);
}

#[static_view_locator]
#[derive(Default)]
pub struct PortalViewLocator;

#[cfg(test)]
mod tests {
    use super::view_models::{HomeViewModel, ReportsViewModel, SettingsViewModel, WorkspaceViewModel};
    use super::PortalViewLocator;
    use crate::support::{class_name, start, value};
    use ferroui_controls::TextBlock;
    use std::any::TypeId;

    #[test]
    fn resolves_multiple_view_models_and_respects_ordering() {
        let _app = start();
        // Every locator of the crate maps every view model of the crate: the entries of this module
        // are the ones in the order of the reference.
        let portal = [TypeId::of::<HomeViewModel>(), TypeId::of::<ReportsViewModel>(), TypeId::of::<SettingsViewModel>(), TypeId::of::<WorkspaceViewModel>()];
        let order: Vec<TypeId> = PortalViewLocator::s_views()
            .entries()
            .iter()
            .map(|entry| entry.type_id())
            .filter(|type_id| portal.contains(type_id))
            .collect();
        assert_eq!(order, [TypeId::of::<HomeViewModel>(), TypeId::of::<ReportsViewModel>()]);

        let locator = PortalViewLocator;
        let home = locator.build(Some(&value(HomeViewModel))).expect("a view");
        let reports = locator.build(Some(&value(ReportsViewModel))).expect("a view");
        let settings = locator.build(Some(&value(SettingsViewModel))).expect("a view");

        assert_eq!(class_name(&home), "HomeView");
        assert_eq!(class_name(&reports), "ReportsView");
        let fallback = settings.cast::<TextBlock>().expect("a text block");
        assert_eq!(fallback.text().as_deref(), Some("Not Found: ferroui_static_view_locator_tests::portal::views::SettingsView"));
        assert!(!PortalViewLocator::s_views().contains(TypeId::of::<WorkspaceViewModel>()));
    }
}
