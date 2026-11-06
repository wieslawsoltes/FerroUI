fn main() {
    #[cfg(target_os = "ios")]
    ferroui_ios::run_application_with(std::rc::Rc::new(portable_media_mobile::AppDelegate));
}
