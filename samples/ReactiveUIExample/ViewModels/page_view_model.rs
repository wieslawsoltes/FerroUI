//! Port of `ViewModels/PageViewModel.cs`: the base routed page view model.

use super::ViewModelBase;
use ferroui_reactiveui::{IScreen, ReactiveObject};
use std::rc::{Rc, Weak};

/// The base routed page view model (`PageViewModel`): the route segment, the owning screen, the
/// display title and the summary of a page.
///
/// Deviation (DEVIATIONS.md, samples): as [`ViewModelBase`], the base class is a part the page holds
/// (the field `page`); `page_view_model!` writes the traits a
/// derived class inherits (`ReactiveObject`, `IActivatableViewModel`, `IRoutableViewModel`). The page
/// holds its screen weakly: the screen owns the router, whose stack owns the page.
pub struct PageViewModel {
    base: ViewModelBase,
    /// The route segment (`UrlPathSegment`).
    url_path_segment: String,
    /// The owning screen (`HostScreen`).
    host_screen: Weak<dyn IScreen>,
    /// The display title (`Title`).
    title: String,
    /// The page summary (`Summary`).
    summary: String,
}

impl AsRef<ReactiveObject> for PageViewModel {
    fn as_ref(&self) -> &ReactiveObject {
        self.base.as_ref()
    }
}

impl PageViewModel {
    /// The page of `host_screen` with its route segment, title and summary
    /// (`PageViewModel(IScreen, string, string, string)`).
    pub fn new(host_screen: &Rc<dyn IScreen>, url_path_segment: &str, title: &str, summary: &str) -> Self {
        Self {
            base: ViewModelBase::new(),
            url_path_segment: url_path_segment.to_owned(),
            host_screen: Rc::downgrade(host_screen),
            title: title.to_owned(),
            summary: summary.to_owned(),
        }
    }

    /// The base view model.
    pub fn base(&self) -> &ViewModelBase {
        &self.base
    }

    /// The route segment (`UrlPathSegment`).
    pub fn url_path_segment(&self) -> String {
        self.url_path_segment.clone()
    }

    /// The owning screen, while it is alive (`HostScreen`).
    pub fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        self.host_screen.upgrade()
    }

    /// The display title (`Title`).
    pub fn title(&self) -> String {
        self.title.clone()
    }

    /// The page summary (`Summary`).
    pub fn summary(&self) -> String {
        self.summary.clone()
    }
}

/// Writes what a routed page inherits from [`PageViewModel`] for a type that holds one in the field
/// `page`: the traits of a reactive object, `IActivatableViewModel`, `IRoutableViewModel`, the
/// comparison by identity of a view model, and the accessors `title` and `summary`.
macro_rules! page_view_model {
    ($type_:ident) => {
        ::ferroui_reactiveui::reactive_object!($type_, page);

        impl ::ferroui_reactiveui::IActivatableViewModel for $type_ {
            fn activator(&self) -> &::ferroui_reactiveui::ViewModelActivator {
                self.page.base().activator()
            }
        }

        impl ::ferroui_reactiveui::IRoutableViewModel for $type_ {
            fn url_path_segment(&self) -> ::core::option::Option<::std::string::String> {
                ::core::option::Option::Some(self.page.url_path_segment())
            }

            fn host_screen(&self) -> ::core::option::Option<::std::rc::Rc<dyn ::ferroui_reactiveui::IScreen>> {
                self.page.host_screen()
            }
        }

        impl ::core::cmp::PartialEq for $type_ {
            /// The same view model (reference equality, as for the objects of the reference).
            fn eq(&self, other: &Self) -> bool {
                ::std::ptr::eq(self, other)
            }
        }

        impl $type_ {
            /// The display title (`Title`).
            #[allow(dead_code)]
            pub fn title(&self) -> ::std::string::String {
                self.page.title()
            }

            /// The page summary (`Summary`).
            #[allow(dead_code)]
            pub fn summary(&self) -> ::std::string::String {
                self.page.summary()
            }
        }
    };
}
pub(crate) use page_view_model;
