// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::BoxedValue;

/// The `IToolTemplate` docking contract.
pub trait IToolTemplate: 'static {
    /// Exposes an implemented property-change contract to presentation adapters.
    fn as_notify_property_changed(
        &self,
    ) -> Option<&dyn ferroui_system::component_model::INotifyPropertyChanged> {
        None
    }

    /// Gets or sets tool content.
    fn content(&self) -> Option<BoxedValue>;
    /// Sets `Content`.
    fn set_content(&self, value: Option<BoxedValue>);
}

impl PartialEq for dyn IToolTemplate {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
