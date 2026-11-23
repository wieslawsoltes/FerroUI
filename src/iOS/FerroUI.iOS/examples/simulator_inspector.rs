//! An opted-in Live App and source inspection example for an iOS simulator.

#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!("simulator_inspector requires the iOS platform");
}

#[cfg(target_os = "ios")]
fn main() {
    ferroui_ios::run_application_with(std::rc::Rc::new(app::Delegate))
}

#[cfg(target_os = "ios")]
mod app {
    use ferroui_base::layout::VerticalAlignment;
    use ferroui_base::styling::Styles;
    use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
    use ferroui_controls::{
        AppBuilder, Application, ApplicationImpl, ApplicationImplExt, NewApplication,
    };
    use ferroui_ios::{FerroApplicationDelegate, IFerroAppDelegate, IosApplicationExtensions};
    use ferroui_themes_simple::SimpleTheme;
    use ferroui_view::prelude::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    pub struct Delegate;

    impl FerroApplicationDelegate for Delegate {
        fn create_app_builder(&self, delegate: Rc<dyn IFerroAppDelegate>) -> AppBuilder {
            AppBuilder::configure::<App>().use_ios_with_delegate(delegate)
        }
    }

    #[repr(C)]
    struct App {
        base: Application,
        view_owner: RefCell<Option<Owner>>,
        inspector: RefCell<Option<ferroui_mcp::ServerHandle>>,
    }

    ferro_class!(App: Application);
    ferro_impl_classes!(App: FerroObjectImpl);

    impl NewApplication for App {
        fn new_application() -> Ref<Self> {
            instantiate(Self {
                base: Application::construct(),
                view_owner: RefCell::new(None),
                inspector: RefCell::new(None),
            })
        }
    }

    impl ApplicationImpl for App {
        fn initialize(this: &Self) {
            Self::parent_initialize(this);
            this.set_name(Some("Contact review".into()));
            this.styles().add(SimpleTheme::new().upcast::<Styles>());
        }

        fn on_framework_initialization_completed(this: &Self) {
            let lifetime = this.application_lifetime();
            if let Some(lifetime) = lifetime
                .as_ref()
                .and_then(|lifetime| lifetime.as_single_view_application_lifetime())
            {
                let (view, owner) = rsx! { ContactReview {} }.mount_root();
                lifetime.set_main_view(Some(view));
                this.view_owner.replace(Some(owner));
            }
            Self::parent_on_framework_initialization_completed(this);

            if std::env::args().any(|argument| argument == "--mcp") {
                let options = ferroui_mcp::Options {
                    permissions: ferroui_mcp::Permissions {
                        allow_input: std::env::args().any(|argument| argument == "--mcp-input"),
                        ..Default::default()
                    },
                    ..Default::default()
                };
                match ferroui_mcp::serve(options) {
                    Ok(inspector) => {
                        this.inspector.replace(Some(inspector));
                    }
                    Err(error) => eprintln!("Could not start the FerroUI inspector: {error}"),
                }
            }
        }
    }

    #[component(rerender)]
    fn ContactReview() -> Element {
        let reviewed = use_signal(|| 0);
        rsx! {
            Border { background: "#EDF4F2", padding: 23.0,
                StackPanel { vertical_alignment: VerticalAlignment::Center, spacing: 16.0,
                    TextBlock { name: "Heading", text: "Contact review", foreground: "#164438", font_size: 28.0 }
                    Border { background: "#FFFFFF", corner_radius: 14.0, padding: 16.0,
                        StackPanel { spacing: 12.0,
                            TextBlock { text: "Renewal queue", foreground: "#214B40", font_size: 19.0 }
                            TextBox { name: "Search", placeholder_text: "Filter customers" }
                            Button { name: "ReviewContact", on_click: move |_| reviewed.update(|value| *value += 1), "Review next contact" }
                            TextBlock { name: "ReviewedCount", text: format!("{} contacts reviewed", reviewed.get()), foreground: "#3E6158" }
                        }
                    }
                }
            }
        }
    }
}
