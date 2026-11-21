// Copyright (c) 2026 Wiesław Šoltés. MIT. Narrow native bindings for Word presentation.
use crate::word_view_model::{WordAlignment, WordViewModel};
use ferroui_base::media::{
    Color, FontFamily, FontStyle, FontWeight, IBrush, ITransform, ScaleTransform, SolidColorBrush,
    TextAlignment, TextDecorationCollection,
};
use std::rc::Rc;
fn brush(argb: u32) -> Rc<dyn IBrush> {
    SolidColorBrush::with_color(Color::from_argb(
        (argb >> 24) as u8,
        (argb >> 16) as u8,
        (argb >> 8) as u8,
        argb as u8,
    ))
    .into()
}
fn weight(bold: bool) -> FontWeight {
    if bold {
        FontWeight::Bold
    } else {
        FontWeight::Normal
    }
}
fn style(italic: bool) -> FontStyle {
    if italic {
        FontStyle::Italic
    } else {
        FontStyle::Normal
    }
}
fn decorations(vm: &WordViewModel) -> Option<TextDecorationCollection> {
    let text = match (vm.body_underline(), vm.body_strike()) {
        (false, false) => return None,
        (true, false) => "Underline",
        (false, true) => "Strikethrough",
        (true, true) => "Underline,Strikethrough",
    };
    Some(TextDecorationCollection::parse(text).expect("native text decorations"))
}
ferroui_base::ferro_markup_type!(class WordViewModel{
namespace:"FerroRibbon.Sample",this:Rc<WordViewModel>,handles:[Rc<WordViewModel>,Option<Rc<WordViewModel>>],notify_property_changed:WordViewModel,
properties:[
Bold:Option<bool>{get:|vm:&Rc<WordViewModel>|vm.checked(0),set:|vm:&Rc<WordViewModel>,v|vm.set_checked(0,v)},
Italic:Option<bool>{get:|vm:&Rc<WordViewModel>|vm.checked(1),set:|vm:&Rc<WordViewModel>,v|vm.set_checked(1,v)},
Underline:Option<bool>{get:|vm:&Rc<WordViewModel>|vm.checked(2),set:|vm:&Rc<WordViewModel>,v|vm.set_checked(2,v)},
Strike:Option<bool>{get:|vm:&Rc<WordViewModel>|vm.checked(3),set:|vm:&Rc<WordViewModel>,v|vm.set_checked(3,v)},
BodyFontSize:f64{get:|vm:&Rc<WordViewModel>|vm.body_font_size(),set:|vm:&Rc<WordViewModel>,v|vm.set_body_font_size(v)},
BodyFontFamily:FontFamily{get:|vm:&Rc<WordViewModel>|FontFamily::new(&vm.body_font_family())},
BodyFontWeight:FontWeight{get:|vm:&Rc<WordViewModel>|weight(vm.body_bold())},
BodyFontStyle:FontStyle{get:|vm:&Rc<WordViewModel>|style(vm.body_italic())},
BodyForeground:Rc<dyn IBrush>{get:|vm:&Rc<WordViewModel>|brush(vm.body_foreground())},
BodyDecorations:Option<TextDecorationCollection>{get:|vm:&Rc<WordViewModel>|decorations(vm)},
BodyAlignment:TextAlignment{get:|vm:&Rc<WordViewModel>|match vm.body_alignment(){WordAlignment::Left=>TextAlignment::Left,WordAlignment::Center=>TextAlignment::Center,WordAlignment::Right=>TextAlignment::Right,WordAlignment::Justify=>TextAlignment::Justify}},
TitleFontSize:f64{get:|vm:&Rc<WordViewModel>|vm.title_font_size()},
TitleForeground:Rc<dyn IBrush>{get:|vm:&Rc<WordViewModel>|brush(vm.title_foreground())},
TitleFontWeight:FontWeight{get:|vm:&Rc<WordViewModel>|weight(vm.title_bold())},
TitleFontStyle:FontStyle{get:|vm:&Rc<WordViewModel>|style(vm.title_italic())},
PageBackground:Rc<dyn IBrush>{get:|vm:&Rc<WordViewModel>|brush(vm.page_background())},
TableVisible:bool{get:|vm:&Rc<WordViewModel>|vm.table_visible()},
FontSizeText:Option<String>{get:|vm:&Rc<WordViewModel>|vm.font_size_text(),set:|vm:&Rc<WordViewModel>,v|vm.set_font_size_text(v)},
ZoomTransform:Option<Rc<dyn ITransform>>{get:|vm:&Rc<WordViewModel>|vm.zoom_scale().map(|s|ScaleTransform::with_scale(s,s).into())},
LastCommand:String{get:|vm:&Rc<WordViewModel>|vm.last_command()},
ToggleTableAction:Rc<dyn ferroui_base::input::ICommand>{get:|vm:&Rc<WordViewModel>|crate::command_adapter::adapt(vm.button_action(0))},
ToggleDarkAction:Rc<dyn ferroui_base::input::ICommand>{get:|vm:&Rc<WordViewModel>|crate::command_adapter::adapt(vm.button_action(1))},
BackstageSaveAction:Rc<dyn ferroui_base::input::ICommand>{get:|vm:&Rc<WordViewModel>|crate::command_adapter::adapt(vm.button_action(2))}
]});
