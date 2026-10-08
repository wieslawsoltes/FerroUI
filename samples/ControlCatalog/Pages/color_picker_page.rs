//! Port of `Pages/ColorPickerPage.xaml.cs`: the class of the document
//! `Pages/ColorPickerPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::media::Colors;
use ferroui_base::{ferro_class_info, instantiate, Ref, Thickness};
use ferroui_controls::{ContentPage, Grid};
use ferroui_controls_color_picker::{ColorPicker, IColorPalette, MaterialHalfColorPalette};
use std::rc::Rc;

#[repr(C)]
pub struct ColorPickerPage {
    base: ContentPage,
}

content_page_class!(ColorPickerPage);
ferro_class_info!(ColorPickerPage { new: ColorPickerPage::new });
xaml_class!(ColorPickerPage, "/Pages/ColorPickerPage.xaml");

impl ColorPickerPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // ColorPicker added from code-behind
        let color_picker = ColorPicker::new();
        color_picker.set_color(Colors::BLUE);
        color_picker.set_margin(Thickness::new(0.0, 50.0, 0.0, 0.0));
        color_picker.set_horizontal_alignment(HorizontalAlignment::Center);
        let palette: Rc<dyn IColorPalette> = Rc::new(MaterialHalfColorPalette::new());
        color_picker.set_palette(Some(palette));
        Grid::set_column(&color_picker, 2);
        Grid::set_row(&color_picker, 1);

        this.layout_root().children().add(color_picker.upcast::<ferroui_controls::Control>());
        this
    }

    /// The element `LayoutRoot` of the document.
    fn layout_root(&self) -> Ref<Grid> {
        self.get_control::<Grid>("LayoutRoot")
    }
}
