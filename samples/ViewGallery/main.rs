//! The gallery in a window of its own, in the Fluent theme with the themes of the colour picker
//! and the TreeDataGrid.

use ferroui_base::styling::{IStyle, Styles};
use ferroui_base::utilities::Uri;
use ferroui_base::metadata::from_markup_value;
use ferroui_markup_xaml::FerroXamlLoader;
use std::rc::Rc;

fn style_at(uri: &str) -> Rc<dyn IStyle> {
    let uri = Uri::absolute(uri).expect("a URI");
    let loaded = FerroXamlLoader::load(&uri, None).unwrap_or_else(|error| panic!("{error}"));
    from_markup_value::<Rc<dyn IStyle>>(&Some(loaded)).expect("a style")
}

fn main() {
    ferroui_controls_color_picker::register_types();
    ferroui_controls_treedatagrid::register_types();
    let start_dark = std::env::args().any(|arg| arg == "--dark");
    let name = if start_dark { "view-gallery-dark" } else { "view-gallery" };
    ferroui_view::launch_themed(
        || {
            let styles = Styles::new();
            styles.add(ferroui_themes_fluent::FluentTheme::new().as_style());
            styles.add(style_at("ferres://FerroUI.Controls.ColorPicker/Themes/Fluent/Fluent.xaml"));
            styles.add(style_at("ferres://FerroUI.Controls.TreeDataGrid/Themes/Fluent.xaml"));
            let style: Rc<dyn IStyle> = styles.into();
            style
        },
        move || {
            use ferroui_view::prelude::*;
            window()
                .title("ferroui-view gallery")
                .width(1000.0)
                .height(640.0)
                .child(view_gallery::gallery().start_dark(start_dark))
                .into_element()
        },
        move |window| {
            if std::env::args().any(|arg| arg == "--inspector") {
                // The inspector of the components of the gallery, as its picture.
                let inspector = ferroui_view::open_inspector();
                // The bar selected, so that its properties show.
                if let Some(list) = inspector.get_logical_descendants().find_map(|d| d.cast::<ferroui_controls::ListBox>()) {
                    list.set_selected_index(13);
                }
                seven_guis_model::capture_if_asked(&inspector, "view-inspector");
            } else {
                seven_guis_model::capture_if_asked(window, name);
            }
        },
    );
}
