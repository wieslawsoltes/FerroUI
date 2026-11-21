use crate::view_model::ProbeViewModel;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(class ProbeViewModel {
    namespace:"FerroRibbon.NativeProbe",this:Rc<ProbeViewModel>,handles:[ProbeViewModel,Rc<ProbeViewModel>,Option<Rc<ProbeViewModel>>],
    notify_property_changed:ProbeViewModel,
    properties:[
        AvailableWidth:f64 {get:|this:&Rc<ProbeViewModel>|this.available_width(),set:|this:&Rc<ProbeViewModel>,value|this.set_available_width(value)},
        ColumnSetting:f64 {get:|this:&Rc<ProbeViewModel>|this.column_setting(),set:|this:&Rc<ProbeViewModel>,value|this.set_column_setting(value)},
        Columns:i32 {get:|this:&Rc<ProbeViewModel>|this.columns()},
        LayoutSummary:String {get:|this:&Rc<ProbeViewModel>|this.layout_summary()},
        GridSummary:String {get:|this:&Rc<ProbeViewModel>|this.grid_summary()},
        PresentationLabel:String {get:|this:&Rc<ProbeViewModel>|this.presentation_label(),set:|this:&Rc<ProbeViewModel>,value|this.set_presentation_label(value)},
        IsSimplified:bool {get:|this:&Rc<ProbeViewModel>|this.is_simplified(),set:|this:&Rc<ProbeViewModel>,value|this.set_is_simplified(value)},
    ],
});
