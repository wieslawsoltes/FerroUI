//! The delegate of a window scene: it creates the window of the scene
//! with the view of the application in it.
//!
//! Stage 2 of `docs/porting/ios-platform.md` adds the activations of the
//! reference's delegate: the user activities and the URLs a scene is
//! connected with or receives later, which become file and protocol
//! activations of the activatable lifetime.

use crate::ferro_view::FerroView;
use crate::single_view_lifetime::SingleViewLifetime;
use crate::view_controller::DefaultFerroViewController;
use ferroui_controls::Application;
use objc2::rc::{Allocated, Retained};
use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly, Message};
use objc2_foundation::NSObjectProtocol;
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
            _connection_options: &UISceneConnectionOptions,
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
        }
    }

    unsafe impl UIWindowSceneDelegate for FerroSceneDelegate {}
);

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
