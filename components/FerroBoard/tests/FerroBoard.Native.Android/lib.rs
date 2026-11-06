//! Native downstream proof: public dependency-compiled XAML and mobile services, no sample.
const _: () = assert!(
    cfg!(feature = "native-password-checks") as usize
        + cfg!(feature = "native-emoji-panel-checks") as usize
        + cfg!(feature = "native-clipboard-panel-checks") as usize
        + cfg!(feature = "native-lifecycle-checks") as usize
        + cfg!(feature = "native-glide-checks") as usize
        + cfg!(feature = "native-selection-checks") as usize
        <= 1,
    "Select one native fixture feature at a time"
);
#[cfg(all(target_os = "android", feature = "native-clipboard-panel-checks"))]
mod clipboard_panel_probe;
#[cfg(all(
    target_os = "android",
    not(any(
        feature = "native-password-checks",
        feature = "native-emoji-panel-checks",
        feature = "native-clipboard-panel-checks",
        feature = "native-lifecycle-checks",
        feature = "native-glide-checks",
        feature = "native-selection-checks"
    ))
))]
mod clipboard_probe;
#[cfg(all(target_os = "android", feature = "native-emoji-panel-checks"))]
mod emoji_probe;
#[cfg(all(target_os = "android", feature = "native-glide-checks"))]
mod glide_probe;
#[cfg(all(target_os = "android", feature = "native-lifecycle-checks"))]
mod lifecycle_probe;
#[cfg(all(target_os = "android", feature = "native-password-checks"))]
mod password_probe;
#[cfg(all(target_os = "android", feature = "native-selection-checks"))]
mod selection_probe;
#[cfg(target_os = "android")]
mod host {
    use ferroboard_controls::services::KeyboardHapticService;
    #[cfg(not(any(
        feature = "native-password-checks",
        feature = "native-emoji-panel-checks",
        feature = "native-clipboard-panel-checks",
        feature = "native-lifecycle-checks",
        feature = "native-glide-checks",
        feature = "native-selection-checks"
    )))]
    use ferroboard_controls::{KeyboardVisibilityMode, ScreenKeyboardHost};
    use ferroui_android::AndroidApplicationExtensions;
    #[cfg(not(any(
        feature = "native-password-checks",
        feature = "native-emoji-panel-checks",
        feature = "native-clipboard-panel-checks",
        feature = "native-lifecycle-checks",
        feature = "native-glide-checks",
        feature = "native-selection-checks"
    )))]
    use ferroui_android::log::{self, LogPriority};
    use ferroui_base::{FerroObjectImpl, Ref};
    #[cfg(not(any(
        feature = "native-password-checks",
        feature = "native-emoji-panel-checks",
        feature = "native-clipboard-panel-checks",
        feature = "native-lifecycle-checks",
        feature = "native-glide-checks",
        feature = "native-selection-checks"
    )))]
    use ferroui_controls::TextBox;
    use ferroui_controls::{
        AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Control, NewApplication,
    };
    use ferroui_themes_winui::WinUITheme;
    use std::rc::Rc;
    #[repr(C)]
    struct ConsumerApp {
        base: Application,
    }
    ferroui_base::ferro_class!(ConsumerApp:Application);
    ferroui_base::ferro_class_info!(ConsumerApp {});
    ferroui_base::ferro_impl_classes!(ConsumerApp:FerroObjectImpl);
    impl NewApplication for ConsumerApp {
        fn new_application() -> Ref<Self> {
            ferroui_base::instantiate(Self {
                base: Application::construct(),
            })
        }
    }
    impl ApplicationImpl for ConsumerApp {
        fn initialize(this: &Self) {
            ferroboard_controls::register_types();
            ferroboard_native_consumer::consumer::register_types();
            ferroboard_native_consumer::compiled_markup::register();
            this.styles().add(WinUITheme::new().as_style());
        }
        fn on_framework_initialization_completed(this: &Self) {
            this.application_lifetime()
                .unwrap()
                .as_activity_application_lifetime()
                .unwrap()
                .set_main_view_factory(Some(Rc::new(main_view)));
            Self::parent_on_framework_initialization_completed(this);
        }
    }
    #[cfg(all(
        feature = "native-password-checks",
        not(any(
            feature = "native-emoji-panel-checks",
            feature = "native-clipboard-panel-checks",
            feature = "native-lifecycle-checks",
            feature = "native-glide-checks",
            feature = "native-selection-checks"
        ))
    ))]
    fn main_view() -> Ref<Control> {
        crate::password_probe::view()
    }
    #[cfg(all(
        feature = "native-emoji-panel-checks",
        not(any(
            feature = "native-clipboard-panel-checks",
            feature = "native-lifecycle-checks",
            feature = "native-glide-checks",
            feature = "native-selection-checks"
        ))
    ))]
    fn main_view() -> Ref<Control> {
        crate::emoji_probe::view()
    }
    #[cfg(all(
        feature = "native-clipboard-panel-checks",
        not(any(feature = "native-lifecycle-checks", feature = "native-glide-checks"))
    ))]
    fn main_view() -> Ref<Control> {
        crate::clipboard_panel_probe::view()
    }
    #[cfg(all(
        feature = "native-lifecycle-checks",
        not(feature = "native-glide-checks")
    ))]
    fn main_view() -> Ref<Control> {
        crate::lifecycle_probe::view()
    }
    #[cfg(feature = "native-glide-checks")]
    fn main_view() -> Ref<Control> {
        crate::glide_probe::view()
    }
    #[cfg(not(any(
        feature = "native-password-checks",
        feature = "native-emoji-panel-checks",
        feature = "native-clipboard-panel-checks",
        feature = "native-lifecycle-checks",
        feature = "native-glide-checks",
        feature = "native-selection-checks"
    )))]
    fn main_view() -> Ref<Control> {
        let value = ferroboard_native_consumer::compiled_xaml::try_load(
            None,
            &format!(
                "{}Hosted.xaml",
                ferroboard_native_consumer::compiled_xaml::ROOT_URI
            ),
        )
        .unwrap()
        .expect("dependency compiled host");
        let host = value
            .downcast_ref::<Ref<ScreenKeyboardHost>>()
            .unwrap()
            .clone();
        host.set_visibility_mode(KeyboardVisibilityMode::Auto);
        let field = host.content().unwrap().cast::<TextBox>().unwrap();
        field.set_name(Some("IndependentMail".into()));
        field.set_height(100.0);
        crate::clipboard_probe::install(&host.keyboard(), &field);
        let field = field.downgrade();
        host.loaded(move |_, _| {
            if let Some(field) = field.upgrade() {
                field.focus();
            }
        });
        host.keyboard()
            .enter_action_requested()
            .add(Rc::new(|args| {
                args.set_handled(true);
                log::write(
                    LogPriority::Info,
                    "ferroboard-consumer",
                    &format!("ENTER {:?}", args.action()),
                );
            }));
        host.upcast()
    }
    #[cfg(feature = "native-selection-checks")]
    fn main_view() -> Ref<Control> {
        crate::selection_probe::view()
    }
    pub fn build() -> AppBuilder {
        let locale =
            ferroboard_core::text::locale_text::system_locale_name().expect("native locale");
        ferroui_base::utilities::CultureInfo::set_current_formatting(
            ferroui_base::utilities::CultureInfo::get_culture_info(&locale),
        );
        AppBuilder::configure::<ConsumerApp>()
            .use_android()
            .with(Rc::new(KeyboardHapticService::native()))
    }
}
/// # Safety
/// Called only by Android with its actual process VM.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn JNI_OnLoad(
    vm: *mut std::ffi::c_void,
    _reserved: *mut std::ffi::c_void,
) -> i32 {
    // SAFETY: the real process VM lives through all UI operations.
    if unsafe { ferroboard_core::text::locale_text::install_android_java_vm(vm) }.is_err() {
        return -1;
    }
    // SAFETY: forward Android's VM to its real framework bootstrap.
    unsafe { ferroui_android::on_load(vm, host::build) }
}
