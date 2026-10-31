#[cfg(target_os = "ios")]
mod app;
fn main() {
    #[cfg(target_os = "ios")]
    ferroui_ios::run_application_with(std::rc::Rc::new(app::Delegate));
}
