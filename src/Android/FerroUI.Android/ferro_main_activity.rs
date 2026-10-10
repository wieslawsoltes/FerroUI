//! The main activity of an application: its view shows the main view of
//! the application lifetime.
//!
//! The reference derives the class from the activity class and overrides
//! three members. Here the main activity is an activity that states it is
//! the main one (`FerroMainActivity` of the Java layer), and the three
//! overrides are the functions of this file, which the activity calls
//! where the reference dispatches to the override.

use crate::ferro_activity::FerroActivity;
use crate::ferro_android_application::FerroAndroidApplication;
use crate::ferro_view::FerroView;
use crate::i_ferro_activity::IFerroActivity;
use crate::platform::AndroidActivatableLifetime;
use ferroui_base::{BoxedValue, FerroLocator, LocatorExtensions};
use ferroui_controls::application_lifetimes::IActivityApplicationLifetime;
use ferroui_controls::Control;
use std::rc::Rc;

/// The overrides of the main activity.
pub struct FerroMainActivity;

impl FerroMainActivity {
    pub(crate) fn initialize_view(activity: &Rc<FerroActivity>, initial_content: Option<BoxedValue>) {
        if let Some(lifetime) = FerroAndroidApplication::lifetime() {
            let initial_content =
                initial_content.or_else(|| lifetime.main_view_factory().map(|factory| Control::boxed(factory())));

            activity.set_view(Some(FerroView::new(activity.java_object())));

            activity.set_content(initial_content);
        }

        if activity.view().is_none() {
            panic!("Unknown error: the initialization of the view has failed.");
        }
    }

    pub(crate) fn on_resume(activity: &Rc<FerroActivity>) {
        if let Some(activatable_lifetime) = FerroLocator::current().get_service::<AndroidActivatableLifetime>() {
            let activity: Rc<dyn IFerroActivity> = activity.clone();
            activatable_lifetime.set_current_main_activity(Some(activity));
        }
    }

    pub(crate) fn on_destroy(activity: &Rc<FerroActivity>) {
        if let Some(activatable_lifetime) = FerroLocator::current().get_service::<AndroidActivatableLifetime>() {
            let is_current = activatable_lifetime.current_main_activity().is_some_and(|current| {
                current.as_ferro_activity_handle().is_some()
                    && current.as_ferro_activity_handle() == activity.as_ferro_activity_handle()
            });
            if is_current {
                activatable_lifetime.set_current_main_activity(None);
            }
        }
    }
}
