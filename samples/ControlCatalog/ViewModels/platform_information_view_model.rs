//! Port of `ViewModels/PlatformInformationViewModel.cs`.

use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::platform::{IRuntimePlatform, RuntimePlatformInfo};
use ferroui_base::{ferro_markup_type, FerroLocator, LocatorExtensions};
use mini_mvvm::ViewModelBase;
use std::rc::Rc;

/// The view model of the platform information page.
pub struct PlatformInformationViewModel {
    base: ViewModelBase,
    platform_info: Option<String>,
}

impl PartialEq for PlatformInformationViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for PlatformInformationViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl PlatformInformationViewModel {
    pub fn new() -> Rc<PlatformInformationViewModel> {
        // The runtime information of the platform is not meant for applications; a real
        // application asks the standard library what it is compiled for.
        let runtime_info =
            FerroLocator::current().get_service::<dyn IRuntimePlatform>().map(|platform| platform.get_runtime_info());

        Rc::new(Self {
            base: ViewModelBase::new(),
            platform_info: runtime_info.map(|info| Self::describe(info, cfg!(target_family = "wasm")).to_string()),
        })
    }

    /// The text for the runtime information `info` of a platform that is
    /// (`is_browser`) or is not a browser.
    fn describe(info: RuntimePlatformInfo, is_browser: bool) -> &'static str {
        if is_browser {
            if info.is_desktop {
                "Platform: Desktop (browser)"
            } else if info.is_mobile {
                "Platform: Mobile (browser)"
            } else {
                "Platform: Unknown (browser) - please report"
            }
        } else if info.is_desktop {
            "Platform: Desktop (native)"
        } else if info.is_mobile {
            "Platform: Mobile (native)"
        } else {
            "Platform: Unknown (native) - please report"
        }
    }

    pub fn platform_info(&self) -> Option<String> {
        self.platform_info.clone()
    }
}

ferro_markup_type!(class PlatformInformationViewModel {
    this: Rc<PlatformInformationViewModel>,
    handles: [
        PlatformInformationViewModel,
        Rc<PlatformInformationViewModel>,
        Option<Rc<PlatformInformationViewModel>>
    ],
    constructors: [() => PlatformInformationViewModel::new],
    properties: [
        PlatformInfo: Option<String> { get: |this: &Rc<PlatformInformationViewModel>| this.platform_info() },
    ],
    notify_property_changed: PlatformInformationViewModel,
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn the_text_names_the_form_factor_and_the_kind_of_platform() {
        let desktop = RuntimePlatformInfo { is_desktop: true, ..Default::default() };
        let mobile = RuntimePlatformInfo { is_mobile: true, ..Default::default() };
        let tv = RuntimePlatformInfo { is_tv: true, ..Default::default() };
        assert_eq!("Platform: Desktop (native)", PlatformInformationViewModel::describe(desktop, false));
        assert_eq!("Platform: Mobile (native)", PlatformInformationViewModel::describe(mobile, false));
        assert_eq!("Platform: Unknown (native) - please report", PlatformInformationViewModel::describe(tv, false));
        assert_eq!("Platform: Desktop (browser)", PlatformInformationViewModel::describe(desktop, true));
        assert_eq!("Platform: Mobile (browser)", PlatformInformationViewModel::describe(mobile, true));
        assert_eq!("Platform: Unknown (browser) - please report", PlatformInformationViewModel::describe(tv, true));
    }
}
