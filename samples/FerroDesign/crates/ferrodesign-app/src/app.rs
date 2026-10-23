use crate::shell::EditorShell;
use ferroui_base::media::FontManagerOptions;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::{
    AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Control, NewApplication, Window,
};
use ferroui_fonts_inter::AppBuilderExtension;
use ferroui_themes_fluent::FluentTheme;
use std::{cell::RefCell, rc::Rc};

ferroui_base::ferro_thread_local! { static SHELL: RefCell<Option<Rc<EditorShell>>> = const { RefCell::new(None) }; }

#[repr(C)]
pub struct App {
    base: Application,
}
ferro_class!(App: Application);
ferro_impl_classes!(App: FerroObjectImpl);
impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        instantiate(Self {
            base: Application::construct(),
        })
    }
}
impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        crate::register_types();
        this.styles().add(FluentTheme::new().as_style());
    }
    fn on_framework_initialization_completed(this: &Self) {
        let shell = EditorShell::new();
        let lifetime = this.application_lifetime();
        if let Some(single) = lifetime
            .as_ref()
            .and_then(|l| l.as_single_view_application_lifetime())
        {
            single.set_main_view(Some(shell.root.clone().upcast::<Control>()));
        } else if let Some(desktop) = lifetime
            .as_ref()
            .and_then(|l| l.as_classic_desktop_style_application_lifetime())
        {
            let window = Window::new();
            window.set_title(Some("FerroDesign · Orbit".into()));
            window.set_width(1500.0);
            window.set_height(960.0);
            window.set_min_width(980.0);
            window.set_min_height(650.0);
            window.set_content(Some(Control::boxed(shell.root.clone())));
            let weak = Rc::downgrade(&shell);
            window.closing(move |event| {
                if let Some(shell) = weak.upgrade() {
                    if !shell.command("commit-edit") { event.set_cancel(true); }
                }
            });
            desktop.set_main_window(Some(window));
        }
        SHELL.with(|slot| *slot.borrow_mut() = Some(shell));
        Self::parent_on_framework_initialization_completed(this);
    }
}
pub fn build_app() -> AppBuilder {
    AppBuilder::configure::<App>()
        .with_inter_font()
        .with(Rc::new(FontManagerOptions {
            default_family_name: Some("fonts:Inter#Inter".into()),
            ..Default::default()
        }))
}
pub fn with_shell<R>(f: impl FnOnce(&Rc<EditorShell>) -> R) -> Option<R> {
    SHELL.with(|slot| slot.borrow().as_ref().map(f))
}
