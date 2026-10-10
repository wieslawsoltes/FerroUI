//! The delegate of a window scene: it creates the window of the scene
//! with the view of the application in it.
//!
//! Stage 2 of `docs/porting/ios-platform.md` adds the activations of the
//! reference's delegate: the user activities and the URLs a scene is
//! connected with or receives later, which become file and protocol
//! activations of the activatable lifetime.

use crate::ferro_app_delegate::{FerroAppDelegate, IFerroAppInternalDelegate};
use crate::ferro_view::FerroView;
use crate::single_view_lifetime::SingleViewLifetime;
use crate::view_controller::DefaultFerroViewController;
use ferroui_controls::Application;
use objc2::rc::{Allocated, Retained};
use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly, Message};
use objc2::runtime::AnyObject;
use objc2_foundation::{NSObjectProtocol, NSSet, NSUserActivity};
use objc2_ui_kit::{UIApplication, UIOpenURLContext};
use objc2_ui_kit::{
    UIResponder, UIScene, UISceneConnectionOptions, UISceneDelegate, UISceneSession, UIWindow, UIWindowScene,
    UIWindowSceneDelegate, UIWindowSceneSessionRoleApplication,
};
use std::cell::RefCell;

/// The instance variables of the delegate.
#[derive(Default)]
pub struct FerroSceneDelegateIvars {
    window: RefCell<Option<Retained<UIWindow>>>,
}

define_class!(
    // SAFETY: `UIResponder` has no requirements on a subclass that only
    // adds methods of the scene delegate protocols, and the class does not
    // implement `Drop`.
    #[unsafe(super(UIResponder))]
    #[thread_kind = MainThreadOnly]
    #[name = "FerroSceneDelegate"]
    #[ivars = FerroSceneDelegateIvars]
    pub struct FerroSceneDelegate;

    impl FerroSceneDelegate {
        #[unsafe(method_id(init))]
        fn init(this: Allocated<Self>) -> Option<Retained<Self>> {
            let this = this.set_ivars(FerroSceneDelegateIvars::default());
            // SAFETY: `init` of the superclass, on the object that was just
            // allocated and whose instance variables are set.
            unsafe { msg_send![super(this), init] }
        }

        #[unsafe(method_id(window))]
        fn window(&self) -> Option<Retained<UIWindow>> {
            self.ivars().window.borrow().clone()
        }

        #[unsafe(method(setWindow:))]
        fn set_window(&self, window: Option<&UIWindow>) {
            *self.ivars().window.borrow_mut() = window.map(|window| window.retain());
        }
    }

    unsafe impl NSObjectProtocol for FerroSceneDelegate {}

    unsafe impl UISceneDelegate for FerroSceneDelegate {
        #[unsafe(method(scene:willConnectToSession:options:))]
        fn will_connect(
            &self,
            scene: &UIScene,
            session: &UISceneSession,
            connection_options: &UISceneConnectionOptions,
        ) {
            let mtm = MainThreadMarker::from(self);

            // Protect against non-application scenes, which can be created
            // by the system for external displays, CarPlay, etc.
            // SAFETY: the role is a constant of UIKit.
            let application_role = unsafe { UIWindowSceneSessionRoleApplication };
            if !session.role().isEqualToString(application_role) || session.configuration().name().is_some() {
                return;
            }
            let Some(window_scene) = scene.downcast_ref::<UIWindowScene>() else {
                return;
            };
            let Some(lifetime) = Application::current().and_then(|application| application.application_lifetime())
            else {
                return;
            };
            let Some(lifetime) = lifetime.as_any().downcast_ref::<SingleViewLifetime>() else {
                return;
            };

            let window = UIWindow::initWithWindowScene(mtm.alloc(), window_scene);
            init_window(&window, lifetime, mtm);
            *self.ivars().window.borrow_mut() = Some(window.clone());

            window.makeKeyAndVisible();

            dispatch_connection_options(connection_options, mtm);
        }

        #[unsafe(method(scene:continueUserActivity:))]
        fn continue_user_activity(&self, _scene: &UIScene, user_activity: &NSUserActivity) {
            with_app_delegate(self.mtm(), |app_delegate| {
                app_delegate.continue_user_activity(user_activity);
            });
        }

        #[unsafe(method(scene:openURLContexts:))]
        fn open_url_contexts(&self, _scene: &UIScene, url_contexts: &NSSet<UIOpenURLContext>) {
            with_app_delegate(self.mtm(), |app_delegate| {
                for ctx in url_contexts.iter() {
                    app_delegate.open_url(&ctx.URL());
                }
            });
        }
    }

    unsafe impl UIWindowSceneDelegate for FerroSceneDelegate {}
);

/// Calls `action` with the delegate of the application when it is the
/// delegate of the platform.
fn with_app_delegate(mtm: MainThreadMarker, action: impl FnOnce(&FerroAppDelegate)) {
    // SAFETY: the delegate of the shared application, read on the main
    // thread.
    let delegate = unsafe { UIApplication::sharedApplication(mtm).delegate() };
    let delegate: Option<&AnyObject> = delegate.as_deref().map(|delegate| delegate.as_ref());
    if let Some(app_delegate) = delegate.and_then(|delegate| delegate.downcast_ref::<FerroAppDelegate>()) {
        action(app_delegate);
    }
}

/// The activations a scene is connected with: its user activities and
/// the URLs it is to open.
fn dispatch_connection_options(connection_options: &UISceneConnectionOptions, mtm: MainThreadMarker) {
    with_app_delegate(mtm, |app_delegate| {
        // The headers declare the two sets as never null, and the
        // bindings fail on a null one; UIKit returns null for a scene
        // that is connected without any (seen in the simulator, iOS
        // 26.4), which the reference allows for. So the two properties
        // are read as optional.
        // SAFETY: the two properties of the connection options, each a
        // set of the type of its elements or null.
        let (activities, url_contexts): (Option<Retained<NSSet<NSUserActivity>>>, Option<Retained<NSSet<UIOpenURLContext>>>) =
            unsafe { (msg_send![connection_options, userActivities], msg_send![connection_options, URLContexts]) };

        if let Some(activities) = activities {
            for activity in activities.iter() {
                app_delegate.continue_user_activity(&activity);
            }
        }

        if let Some(url_contexts) = url_contexts {
            for ctx in url_contexts.iter() {
                app_delegate.open_url(&ctx.URL());
            }
        }
    });
}

/// Gives `window` a view with the main view of the lifetime, under a view
/// controller of the platform.
pub(crate) fn init_window(window: &UIWindow, lifetime: &SingleViewLifetime, mtm: MainThreadMarker) {
    let view = FerroView::new(mtm);
    lifetime.set_view(Some(view.clone()));

    let controller = DefaultFerroViewController::new(mtm);
    controller.setView(Some(&view));
    window.setRootViewController(Some(&controller));
    view.init_with_controller(controller.controller());
}
