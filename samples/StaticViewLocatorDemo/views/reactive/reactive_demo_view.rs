//! Port of `Views/MainWindow` (markup) of the ReactiveUI demo: the same navigation shown through the
//! generated data template (a content control) and through the generated view locator of
//! ReactiveUI (a view model view host).

use crate::class_prelude::*;
use crate::located_view;

use crate::reactive_view_locator::ReactiveViewLocator;
use crate::view_models::reactive::reactive_demo_view_model::Navigation;
use crate::view_models::reactive::ReactiveDemoViewModel;
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::FontWeight;
use ferroui_base::Ref;
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::ContentControl;
use ferroui_reactiveui::IViewLocator;
use ferroui_reactiveui_controls::ViewModelViewHost;
use ferroui_view::prelude::*;
use std::rc::Rc;

located_view!(ReactiveDemoView, ReactiveDemoViewModel, reactive_demo);

fn reactive_demo(view_model: &Rc<ReactiveDemoViewModel>) -> Element {
    let locator = Rc::new(ReactiveViewLocator);
    let content = NodeRef::<ContentControl>::new();
    let host = NodeRef::<ViewModelViewHost>::new();
    let navigate = move |navigation: Navigation| {
        if let Some(content) = content.get() {
            content.set_content(Some(navigation.content.clone()));
        }
        if let Some(host) = host.get() {
            host.set_view_model(Some(navigation.view_model));
        }
    };
    let button_of = |label: &'static str, navigation: fn(&ReactiveDemoViewModel) -> Navigation| {
        let view_model = view_model.clone();
        button().content(label).on_click(move |_| navigate(navigation(&view_model)))
    };
    let start = view_model.show_home();
    let template: Rc<dyn IDataTemplate> = locator.clone();
    let view_locator: Rc<dyn IViewLocator> = locator;
    grid()
        .margin(16.0)
        .row_definitions("Auto,*")
        .child(
            stack_panel()
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .child(button_of("Home", ReactiveDemoViewModel::show_home))
                .child(button_of("Settings", ReactiveDemoViewModel::show_settings))
                .child(button_of("Fallback wrapper", ReactiveDemoViewModel::show_wrapped_settings)),
        )
        .child(
            grid()
                .grid_row(1)
                .margin((0.0, 16.0, 0.0, 0.0))
                .column_definitions("*,*")
                .child(panel_of(
                    0,
                    "Data template",
                    content_control()
                        .node_ref(content)
                        .horizontal_content_alignment(HorizontalAlignment::Stretch)
                        .vertical_content_alignment(VerticalAlignment::Stretch)
                        .content(Some(start.content.clone()))
                        .with(move |control: &Ref<ContentControl>| control.data_templates().add(template.clone()))
                        .into(),
                ))
                .child(panel_of(
                    1,
                    "ReactiveUI view locator",
                    view_model_view_host()
                        .node_ref(host)
                        .with(move |host: &Ref<ViewModelViewHost>| {
                            host.set_view_locator(Some(view_locator.clone()));
                            host.set_view_model(Some(start.view_model.clone()));
                        })
                        .into(),
                )),
        )
        .into()
}

/// A framed panel of the page, with its title.
fn panel_of(column: i32, title: &str, content: Element) -> Element {
    border()
        .grid_column(column)
        .margin(4.0)
        .padding(16.0)
        .border_thickness(1.0)
        .corner_radius(6.0)
        .child(
            grid()
                .row_definitions("Auto,*")
                .child(text_block().text(title).font_weight(FontWeight::SemiBold))
                .child(content_control().grid_row(1).margin((0.0, 12.0, 0.0, 0.0)).content(Some(ferroui_controls::Control::boxed(&content.mount())))),
        )
        .into()
}
