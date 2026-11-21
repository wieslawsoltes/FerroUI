//! Native metadata for unchanged Core model APIs. Copyright Wiesław Šoltés, MIT.
use super::*;
use ferroribbon_core::model::{
    RibbonIcon, RibbonMenuHeaderModel, RibbonMenuItemModel, RibbonMenuItemModelImpl,
    RibbonMenuSeparatorModel, RibbonNodeModelImpl,
};

pub struct RibbonMenuItemModelMetadata;
ferroui_base::ferro_markup_type!(class RibbonMenuItemModelMetadata as "RibbonMenuItemModel" {
 namespace:"FerroRibbon.Model",this:Rc<dyn RibbonMenuItemModelImpl>,handles:[Rc<dyn RibbonMenuItemModelImpl>,Option<Rc<dyn RibbonMenuItemModelImpl>>,Rc<RibbonMenuItemModel>,RibbonMenuItemModel],base:Rc<dyn RibbonNodeModelImpl>,
 constructors:[()=>||RibbonMenuItemModel::with_options(None,None,None,None) as Rc<dyn RibbonMenuItemModelImpl>,(Option<String>)=>|v0:Option<String>|RibbonMenuItemModel::with_options(v0.map(Into::into),None,None,None) as Rc<dyn RibbonMenuItemModelImpl>,(Option<String>,Option<String>)=>|v0:Option<String>,v1:Option<String>|RibbonMenuItemModel::with_options(v0.map(Into::into),v1.map(Into::into),None,None) as Rc<dyn RibbonMenuItemModelImpl>,(Option<String>,Option<String>,Option<Rc<RibbonIcon>>)=>|v0:Option<String>,v1:Option<String>,v2:Option<Rc<RibbonIcon>>|RibbonMenuItemModel::with_options(v0.map(Into::into),v1.map(Into::into),v2,None) as Rc<dyn RibbonMenuItemModelImpl>,(Option<String>,Option<String>,Option<Rc<RibbonIcon>>,Option<Rc<dyn ferroribbon_core::commands::ICommand>>)=>|v0:Option<String>,v1:Option<String>,v2:Option<Rc<RibbonIcon>>,v3:Option<Rc<dyn ferroribbon_core::commands::ICommand>>|RibbonMenuItemModel::with_options(v0.map(Into::into),v1.map(Into::into),v2,v3) as Rc<dyn RibbonMenuItemModelImpl>],
 properties:[CommandId:Option<String>{try_get:|v:&Rc<dyn RibbonMenuItemModelImpl>|string(v.command_id()),set:|v:&Rc<dyn RibbonMenuItemModelImpl>,x:Option<String>|v.set_command_id(x.map(Into::into))},Command:Option<Rc<dyn ferroribbon_core::commands::ICommand>>{get:|v:&Rc<dyn RibbonMenuItemModelImpl>|v.command(),set:|v:&Rc<dyn RibbonMenuItemModelImpl>,x:Option<Rc<dyn ferroribbon_core::commands::ICommand>>|v.set_command(x)},CommandParameter:Option<ferroui_base::BoxedValue>{try_get:|v:&Rc<dyn RibbonMenuItemModelImpl>|crate::metadata::model_objects::to_native(v.command_parameter()),set:|v:&Rc<dyn RibbonMenuItemModelImpl>,x:Option<ferroui_base::BoxedValue>|v.set_command_parameter(crate::metadata::model_objects::from_native_replacing(x,||v.command_parameter()))},IsCheckable:bool{get:|v:&Rc<dyn RibbonMenuItemModelImpl>|v.is_checkable(),set:|v:&Rc<dyn RibbonMenuItemModelImpl>,x:bool|v.set_is_checkable(x)},IsChecked:bool{get:|v:&Rc<dyn RibbonMenuItemModelImpl>|v.is_checked(),set:|v:&Rc<dyn RibbonMenuItemModelImpl>,x:bool|v.set_is_checked(x)},GroupName:Option<String>{try_get:|v:&Rc<dyn RibbonMenuItemModelImpl>|string(v.group_name()),set:|v:&Rc<dyn RibbonMenuItemModelImpl>,x:Option<String>|v.set_group_name(x.map(Into::into))},Shortcut:Option<String>{try_get:|v:&Rc<dyn RibbonMenuItemModelImpl>|string(v.shortcut()),set:|v:&Rc<dyn RibbonMenuItemModelImpl>,x:Option<String>|v.set_shortcut(x.map(Into::into))},Items:ModelNodes{get:|v:&Rc<dyn RibbonMenuItemModelImpl>|v.items()}],
});
pub struct RibbonMenuSeparatorModelMetadata;
ferroui_base::ferro_markup_type!(class RibbonMenuSeparatorModelMetadata as "RibbonMenuSeparatorModel" {
 namespace:"FerroRibbon.Model",this:Rc<RibbonMenuSeparatorModel>,handles:[Rc<RibbonMenuSeparatorModel>,Option<Rc<RibbonMenuSeparatorModel>>,RibbonMenuSeparatorModel],base:Rc<dyn RibbonNodeModelImpl>,
 constructors:[()=>||RibbonMenuSeparatorModel::new() as Rc<RibbonMenuSeparatorModel>],
 properties:[],
});
pub struct RibbonMenuHeaderModelMetadata;
ferroui_base::ferro_markup_type!(class RibbonMenuHeaderModelMetadata as "RibbonMenuHeaderModel" {
 namespace:"FerroRibbon.Model",this:Rc<RibbonMenuHeaderModel>,handles:[Rc<RibbonMenuHeaderModel>,Option<Rc<RibbonMenuHeaderModel>>,RibbonMenuHeaderModel],base:Rc<dyn RibbonNodeModelImpl>,
 constructors:[(Option<String>)=>|label:Option<String>|RibbonMenuHeaderModel::new(label.map(Into::into)) as Rc<RibbonMenuHeaderModel>],
 properties:[],
});
