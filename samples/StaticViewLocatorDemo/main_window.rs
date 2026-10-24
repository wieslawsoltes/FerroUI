//! Port of `Views/MainWindow` (markup): a tab control with a tab per page, whose content the
//! generated data template of the window shows.

use crate::view_models::MainWindowViewModel;
use crate::ViewLocator;
use ferroui_base::Ref;
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::Window;
use ferroui_view::prelude::*;
use std::rc::Rc;

/// The main window of the sample.
pub fn main_window() -> Element {
    let view_model = MainWindowViewModel::new();
    let template: Rc<dyn IDataTemplate> = Rc::new(ViewLocator);
    window()
        .title("Static View Locator Demo")
        .width(900.0)
        .height(560.0)
        .with(move |window: &Ref<Window>| window.data_templates().add(template.clone()))
        .child(tab_control().children(
            view_model
                .pages()
                .iter()
                .map(|page| tab_item().header(Some(Rc::new(page.title.clone()) as ferroui_base::BoxedValue)).content(Some(page.view_model.clone())))
                .collect::<Vec<_>>(),
        ))
        .into()
}
