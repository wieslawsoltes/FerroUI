//! A debuggable Android app for the VS Code Live App workflow.

#[cfg(target_os = "android")]
ferroui_android::android_application!(app::build);

#[cfg(target_os = "android")]
mod app {
    use ferroui_android::live_inspector::{self, LiveAppInspector};
    use ferroui_android::{
        AndroidApplicationExtensions, AndroidPlatformOptions, AndroidRenderingMode,
    };
    use ferroui_base::layout::VerticalAlignment;
    use ferroui_base::media::FontWeight;
    use ferroui_base::styling::Styles;
    use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
    use ferroui_controls::{
        AppBuilder, Application, ApplicationImpl, ApplicationImplExt, NewApplication,
    };
    use ferroui_themes_simple::SimpleTheme;
    use ferroui_view::prelude::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    pub fn build() -> AppBuilder {
        AppBuilder::configure::<App>()
            .with(Rc::new(AndroidPlatformOptions {
                rendering_mode: vec![AndroidRenderingMode::Software],
            }))
            .use_android()
    }

    #[repr(C)]
    struct App {
        base: Application,
        inspector: RefCell<Option<LiveAppInspector>>,
    }

    ferro_class!(App: Application);
    ferro_impl_classes!(App: FerroObjectImpl);

    impl NewApplication for App {
        fn new_application() -> Ref<Self> {
            instantiate(Self {
                base: Application::construct(),
                inspector: RefCell::new(None),
            })
        }
    }

    impl ApplicationImpl for App {
        fn initialize(this: &Self) {
            Self::parent_initialize(this);
            this.set_name(Some("Service Review".into()));
            this.styles().add(SimpleTheme::new().upcast::<Styles>());
            ferroui_view::hot::enable_editor_source_locations();
        }

        fn on_framework_initialization_completed(this: &Self) {
            let lifetime = this.application_lifetime();
            if let Some(lifetime) = lifetime
                .as_ref()
                .and_then(|lifetime| lifetime.as_activity_application_lifetime())
            {
                lifetime.set_main_view_factory(Some(Rc::new(|| {
                    let (view, owner) = rsx! { ServiceReview {} }.mount_root();
                    view.detached_from_visual_tree(move |_| owner.dispose())
                        .detach();
                    view
                })));
            }
            Self::parent_on_framework_initialization_completed(this);

            let options = ferroui_mcp::Options {
                permissions: ferroui_mcp::Permissions {
                    allow_input: true,
                    ..Default::default()
                },
                ..Default::default()
            };
            match live_inspector::serve(options) {
                Ok(inspector) => {
                    this.inspector.replace(Some(inspector));
                }
                Err(error) => ferroui_android::log::write(
                    ferroui_android::log::LogPriority::Warn,
                    "FerroUI",
                    &error.to_string(),
                ),
            }
        }
    }

    #[component(rerender)]
    fn ServiceReview() -> Element {
        let reviewed = use_signal(|| 0);
        rsx! {
            Border { background: "#EEF3FB", padding: 24.0,
                StackPanel { vertical_alignment: VerticalAlignment::Center, spacing: 14.0,
                    TextBlock { text: "Service review", font_size: 28.0, font_weight: FontWeight::SemiBold, foreground: "#1B3156" }
                    TextBlock { text: "Review open cases for your team", foreground: "#405679" }
                    Border { background: "#FFFFFF", corner_radius: 14.0, padding: 16.0,
                        StackPanel { spacing: 10.0,
                            TextBox { name: "Search", placeholder_text: "Find a case" }
                            Button { name: "ReviewCase", on_click: move |_| reviewed.update(|count| *count += 1), "Complete review" }
                            TextBlock { name: "ReviewedCount", text: format!("Reviewed cases: {}", reviewed.get()), foreground: "#2E5C6A" }
                        }
                    }
                }
            }
        }
    }
}
