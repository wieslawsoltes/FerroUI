// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::GripMode;
use ferroui_base::data::converters::{FuncValueConverter, IValueConverter};
use std::rc::Rc;
/// Original shared grip-mode value converters.
pub struct GripModeConverters;
impl GripModeConverters {
    /// Maps AutoHide to grid row one, other values to row zero.
    pub fn grid_row_auto_hide_converter() -> Rc<dyn IValueConverter> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<dyn IValueConverter>=Rc::new(FuncValueConverter::<GripMode,i32>::new(|v|if v==GripMode::AutoHide {1}else{0}));}
        INSTANCE.with(Rc::clone)
    }
    /// Tests Visible.
    pub fn is_visible_visible_converter() -> Rc<dyn IValueConverter> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<dyn IValueConverter>=Rc::new(FuncValueConverter::<GripMode,bool>::new(|v|v==GripMode::Visible));}
        INSTANCE.with(Rc::clone)
    }
    /// Tests Hidden or Visible.
    pub fn is_visible_visible_or_hidden_converter() -> Rc<dyn IValueConverter> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<dyn IValueConverter>=Rc::new(FuncValueConverter::<GripMode,bool>::new(|v|v==GripMode::Hidden||v==GripMode::Visible));}
        INSTANCE.with(Rc::clone)
    }
    /// Tests AutoHide or Visible.
    pub fn is_visible_auto_hide_or_visible_converter() -> Rc<dyn IValueConverter> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<dyn IValueConverter>=Rc::new(FuncValueConverter::<GripMode,bool>::new(|v|v==GripMode::AutoHide||v==GripMode::Visible));}
        INSTANCE.with(Rc::clone)
    }
    /// Tests AutoHide or Hidden.
    pub fn is_visible_auto_hide_or_hidden_converter() -> Rc<dyn IValueConverter> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<dyn IValueConverter>=Rc::new(FuncValueConverter::<GripMode,bool>::new(|v|v==GripMode::AutoHide||v==GripMode::Hidden));}
        INSTANCE.with(Rc::clone)
    }
}
ferroui_base::ferro_markup_type!(static GripModeConverters {
    namespace:"FerroDock.Ferro.Converters",fields:[
        GridRowAutoHideConverter:Rc<dyn IValueConverter> =>GripModeConverters::grid_row_auto_hide_converter,
        IsVisibleVisibleConverter:Rc<dyn IValueConverter> =>GripModeConverters::is_visible_visible_converter,
        IsVisibleVisibleOrHiddenConverter:Rc<dyn IValueConverter> =>GripModeConverters::is_visible_visible_or_hidden_converter,
        IsVisibleAutoHideOrVisibleConverter:Rc<dyn IValueConverter> =>GripModeConverters::is_visible_auto_hide_or_visible_converter,
        IsVisibleAutoHideOrHiddenConverter:Rc<dyn IValueConverter> =>GripModeConverters::is_visible_auto_hide_or_hidden_converter,
    ],
});
