//! The application delegate: it builds the application when UIKit has
//! launched, gives each scene its delegate, and reports the activations of
//! the application.

use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_controls::application_lifetimes::ActivatedEventArgs;
use ferroui_controls::AppBuilder;
use std::rc::{Rc, Weak};

/// The events of an application delegate.
pub trait IFerroAppDelegate {
    /// The application was activated.
    fn activated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable>;

    /// The application was deactivated.
    fn deactivated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable>;
}

/// What an application overrides of the application delegate.
///
/// The delegate of the reference is a generic class an application derives
/// from; a class of the Objective-C runtime can be neither. The delegate
/// class of the port is one class ([`FerroAppDelegate`] on iOS), and the
/// two overridable members are this trait, given to [`run_application_with`].
pub trait FerroApplicationDelegate {
    /// Creates the builder of the application. `app_delegate` is what
    /// `use_ios_with_delegate` takes.
    fn create_app_builder(&self, app_delegate: Rc<dyn IFerroAppDelegate>) -> AppBuilder;

    /// Changes the builder before the application is set up.
    fn customize_app_builder(&self, builder: AppBuilder) -> AppBuilder {
        builder
    }
}

/// The two events of the application delegate, raised by the delegate class.
pub struct AppDelegateEvents {
    this: Weak<AppDelegateEvents>,
    on_activated: HandlerList<dyn Fn(&ActivatedEventArgs)>,
    on_deactivated: HandlerList<dyn Fn(&ActivatedEventArgs)>,
}

impl AppDelegateEvents {
    /// Creates the events without handlers.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            on_activated: HandlerList::new(),
            on_deactivated: HandlerList::new(),
        })
    }

    /// Raises the activated event.
    pub fn on_activated(&self, args: ActivatedEventArgs) {
        for (_, handler) in self.on_activated.snapshot().iter() {
            handler(&args);
        }
    }

    /// Raises the deactivated event.
    pub fn on_deactivated(&self, args: ActivatedEventArgs) {
        for (_, handler) in self.on_deactivated.snapshot().iter() {
            handler(&args);
        }
    }

    fn subscribe(
        &self,
        select: fn(&AppDelegateEvents) -> &HandlerList<dyn Fn(&ActivatedEventArgs)>,
        handler: Rc<dyn Fn(&ActivatedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        let token = select(self).add(handler);
        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                select(&this).remove(token);
            }
        })
    }
}

impl IFerroAppDelegate for AppDelegateEvents {
    fn activated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable> {
        self.subscribe(|this| &this.on_activated, handler)
    }

    fn deactivated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable> {
        self.subscribe(|this| &this.on_deactivated, handler)
    }
}

#[cfg(target_os = "ios")]
pub use uikit::{run_application, run_application_with, FerroAppDelegate};

#[cfg(target_os = "ios")]
mod uikit {
    use super::{AppDelegateEvents, FerroApplicationDelegate, IFerroAppDelegate};
    use crate::ferro_scene_delegate::FerroSceneDelegate;
    use crate::platform::IosApplicationExtensions;
    use crate::single_view_lifetime::SingleViewLifetime;
    use block2::RcBlock;
    use ferroui_controls::application_lifetimes::{ActivatedEventArgs, ActivationKind, IApplicationLifetime};
    use ferroui_controls::{AppBuilder, NewApplication};
    use objc2::rc::{Allocated, Retained};
    use objc2::runtime::AnyObject;
    use objc2::{define_class, msg_send, ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, Message};
    use objc2_foundation::{
        NSDictionary, NSNotification, NSNotificationCenter, NSNotificationName, NSObjectProtocol, NSString,
    };
    use objc2_ui_kit::{
        UIApplication, UIApplicationDelegate, UIApplicationDidEnterBackgroundNotification,
        UIApplicationLaunchOptionsKey, UIApplicationWillEnterForegroundNotification, UIResponder,
        UISceneConfiguration, UISceneConnectionOptions, UISceneSession, UIWindow,
    };
    use std::cell::RefCell;
    use std::marker::PhantomData;
    use std::ptr::NonNull;
    use std::rc::Rc;

    thread_local! {
        /// What the application gave `run_application`: UIKit creates the
        /// delegate object itself, from the name of its class, so the
        /// object finds here what a derived class would have overridden.
        static APPLICATION: RefCell<Option<Rc<dyn FerroApplicationDelegate>>> = const { RefCell::new(None) };
    }

    /// The instance variables of the delegate.
    pub struct FerroAppDelegateIvars {
        events: Rc<AppDelegateEvents>,
        window: RefCell<Option<Retained<UIWindow>>>,
    }

    define_class!(
        // SAFETY: `UIResponder` has no requirements on a subclass that only
        // adds methods of the application delegate protocol, and the class
        // does not implement `Drop`.
        #[unsafe(super(UIResponder))]
        #[thread_kind = MainThreadOnly]
        #[name = "FerroAppDelegate"]
        #[ivars = FerroAppDelegateIvars]
        pub struct FerroAppDelegate;

        impl FerroAppDelegate {
            #[unsafe(method_id(init))]
            fn init(this: Allocated<Self>) -> Option<Retained<Self>> {
                let events = AppDelegateEvents::new();
                let this = this.set_ivars(FerroAppDelegateIvars { events: events.clone(), window: RefCell::new(None) });
                // SAFETY: `init` of the superclass, on the object that was
                // just allocated and whose instance variables are set.
                let this: Option<Retained<Self>> = unsafe { msg_send![super(this), init] };

                // SAFETY: the two names are constants of UIKit.
                let (entered_background, will_enter_foreground) = unsafe {
                    (UIApplicationDidEnterBackgroundNotification, UIApplicationWillEnterForegroundNotification)
                };
                observe(entered_background, {
                    let events = events.clone();
                    move || events.on_deactivated(ActivatedEventArgs::new(ActivationKind::Background))
                });
                observe(will_enter_foreground, move || {
                    events.on_activated(ActivatedEventArgs::new(ActivationKind::Background))
                });

                this
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

        unsafe impl NSObjectProtocol for FerroAppDelegate {}

        unsafe impl UIApplicationDelegate for FerroAppDelegate {
            #[unsafe(method_id(application:configurationForConnectingSceneSession:options:))]
            fn get_configuration(
                &self,
                _application: &UIApplication,
                connecting_scene_session: &UISceneSession,
                _options: &UISceneConnectionOptions,
            ) -> Retained<UISceneConfiguration> {
                let mtm = MainThreadMarker::from(self);
                let config = UISceneConfiguration::configurationWithName_sessionRole(
                    None,
                    &connecting_scene_session.role(),
                    mtm,
                );
                // SAFETY: the class is a scene delegate class
                // (`FerroSceneDelegate` implements the window scene
                // delegate protocol and has an `init`).
                unsafe { config.setDelegateClass(Some(FerroSceneDelegate::class())) };
                config
            }

            #[unsafe(method(application:didFinishLaunchingWithOptions:))]
            fn finished_launching(
                &self,
                _application: &UIApplication,
                _launch_options: Option<&NSDictionary<UIApplicationLaunchOptionsKey, AnyObject>>,
            ) -> bool {
                let application = match APPLICATION.with(|application| application.borrow().clone()) {
                    Some(application) => application,
                    None => panic!(
                        "The application delegate was created without an application. \
                         Make sure the application was started with run_application."
                    ),
                };

                let app_delegate: Rc<dyn IFerroAppDelegate> = self.ivars().events.clone();
                let builder = application.create_app_builder(app_delegate);
                let builder = application.customize_app_builder(builder);

                let lifetime = SingleViewLifetime::new();
                // The reference creates a window here on the systems
                // before iOS 13, which have no scenes; on later systems the
                // scene delegate creates it. The port needs iOS 13
                // (docs/porting/ios-platform.md, section 4), so nothing is
                // done after the setup of the application.
                let lifetime: Rc<dyn IApplicationLifetime> = lifetime;
                builder.setup_with_lifetime(lifetime);

                if let Some(window) = self.ivars().window.borrow().clone() {
                    window.makeKeyAndVisible();
                }

                true
            }
        }
    );

    /// Calls `handler` on the main thread for every notification of the
    /// name. The subscription lasts as long as the process, as the one of
    /// the reference does.
    fn observe(name: &NSNotificationName, handler: impl Fn() + 'static) {
        let block = RcBlock::new(move |_notification: NonNull<NSNotification>| handler());
        // SAFETY: the block takes the one argument of a notification block
        // and ignores it. No queue is given, so the block runs on the thread
        // that posts the notification, which for the notifications of the
        // application is the main thread: the block is therefore never
        // called on another thread than the one that made it.
        let observer = unsafe {
            NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                Some(name),
                None,
                None,
                &block,
            )
        };
        // The notification center holds the block for as long as the
        // observer is registered; the token is never used to remove it.
        std::mem::forget(observer);
    }

    /// The application delegate of an application class without changes
    /// to the builder.
    struct DefaultApplicationDelegate<TApp>(PhantomData<TApp>);

    impl<TApp: NewApplication> FerroApplicationDelegate for DefaultApplicationDelegate<TApp> {
        fn create_app_builder(&self, app_delegate: Rc<dyn IFerroAppDelegate>) -> AppBuilder {
            AppBuilder::configure::<TApp>().use_ios_with_delegate(app_delegate)
        }
    }

    /// Runs the application `TApp`: the entry point of an iOS
    /// application (`UIApplicationMain` with the delegate class of the
    /// platform). Does not return.
    ///
    /// # Panics
    /// Panics when called on another thread than the main thread.
    pub fn run_application<TApp: NewApplication + 'static>() -> ! {
        run_application_with(Rc::new(DefaultApplicationDelegate::<TApp>(PhantomData)))
    }

    /// Runs an application whose builder `application` creates and
    /// changes. Does not return.
    ///
    /// # Panics
    /// Panics when called on another thread than the main thread.
    pub fn run_application_with(application: Rc<dyn FerroApplicationDelegate>) -> ! {
        let Some(mtm) = MainThreadMarker::new() else {
            panic!("An application is run on the main thread.");
        };
        APPLICATION.with(|slot| *slot.borrow_mut() = Some(application));
        // Registers the class with the runtime before UIKit looks it up by
        // name.
        let delegate_class = FerroAppDelegate::class();
        let delegate_class_name = NSString::from_str(delegate_class.name().to_str().unwrap_or("FerroAppDelegate"));
        UIApplication::main(None, Some(&delegate_class_name), mtm)
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;
    use ferroui_controls::application_lifetimes::ActivationKind;
    use std::cell::Cell;

    #[test]
    fn a_handler_is_called_until_its_subscription_is_disposed() {
        let events = AppDelegateEvents::new();
        let count = Rc::new(Cell::new(0));
        let subscription = events.activated({
            let count = count.clone();
            Rc::new(move |_: &ActivatedEventArgs| count.set(count.get() + 1))
        });

        events.on_activated(ActivatedEventArgs::new(ActivationKind::Background));
        events.on_deactivated(ActivatedEventArgs::new(ActivationKind::Background));
        assert_eq!(1, count.get());

        subscription.dispose();
        events.on_activated(ActivatedEventArgs::new(ActivationKind::Background));
        assert_eq!(1, count.get());
    }
}
