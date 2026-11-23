//! A debug browser target for the VS Code Live App workflow.

#![cfg_attr(target_os = "emscripten", no_main)]

#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
mod app {
    use ferroui_base::layout::VerticalAlignment;
    use ferroui_base::media::{FontManagerOptions, FontWeight};
    use ferroui_base::styling::Styles;
    use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
    use ferroui_browser::live_inspector::{self, LiveInspector, Permissions};
    use ferroui_browser::{BrowserAppBuilder, BrowserPlatformOptions, BrowserRenderingMode};
    use ferroui_controls::{
        AppBuilder, Application, ApplicationImpl, ApplicationImplExt, NewApplication,
    };
    use ferroui_fonts_inter::AppBuilderExtension;
    use ferroui_themes_simple::SimpleTheme;
    use ferroui_view::prelude::*;
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::prelude::*;

    #[repr(C)]
    struct App {
        base: Application,
        owner: RefCell<Option<Owner>>,
        inspector: RefCell<Option<LiveInspector>>,
    }

    ferro_class!(App: Application);
    ferro_impl_classes!(App: FerroObjectImpl);

    impl NewApplication for App {
        fn new_application() -> Ref<Self> {
            instantiate(Self {
                base: Application::construct(),
                owner: RefCell::new(None),
                inspector: RefCell::new(None),
            })
        }
    }

    impl ApplicationImpl for App {
        fn initialize(this: &Self) {
            Self::parent_initialize(this);
            this.set_name(Some("Dispatch Desk".into()));
            this.styles().add(SimpleTheme::new().upcast::<Styles>());
            ferroui_view::hot::enable_editor_source_locations();
        }

        fn on_framework_initialization_completed(this: &Self) {
            let lifetime = this.application_lifetime();
            if let Some(view) = lifetime
                .as_ref()
                .and_then(|lifetime| lifetime.as_single_view_application_lifetime())
            {
                let (content, owner) = rsx! { DispatchDesk {} }.mount_root();
                view.set_main_view(Some(content));
                this.owner.replace(Some(owner));
            }
            Self::parent_on_framework_initialization_completed(this);
            let permissions = Permissions {
                allow_input: true,
                ..Default::default()
            };
            this.inspector.replace(Some(
                live_inspector::install("out", permissions).expect("Install the browser inspector"),
            ));
        }
    }

    #[component(rerender)]
    fn DispatchDesk() -> Element {
        let dispatched = use_signal(|| 0);
        rsx! {
            Border { background: "#EDF3F9", padding: 30.0,
                StackPanel { vertical_alignment: VerticalAlignment::Center, spacing: 16.0,
                    TextBlock { name: "Heading", text: "Dispatch desk", font_size: 34.0,
                        font_weight: FontWeight::SemiBold, foreground: "#1C3B5B" }
                    TextBlock { text: "Confirm the orders ready for pickup", foreground: "#3C607D" }
                    Border { background: "#FFFFFF", corner_radius: 14.0, padding: 19.0,
                        StackPanel { spacing: 12.0,
                            TextBox { name: "Search", placeholder_text: "Find an order" }
                            Button { name: "DispatchOrder", on_click: move |_| dispatched.update(|count| *count += 1), "Mark dispatched" }
                            TextBlock { name: "DispatchedCount", text: format!("Dispatched orders: {}", dispatched.get()), foreground: "#246455" }
                        }
                    }
                }
            }
        }
    }

    #[wasm_bindgen(js_name = runMain)]
    pub fn run_main() {
        AppBuilder::configure::<App>()
            .with_inter_font()
            .with(Rc::new(FontManagerOptions {
                default_family_name: Some("fonts:Inter#Inter".into()),
                ..Default::default()
            }))
            .start_browser_app(
                "out",
                Some(BrowserPlatformOptions {
                    rendering_mode: vec![BrowserRenderingMode::Software2D],
                    ..Default::default()
                }),
            );
    }
}
