//! Exact source Gallery model metadata; Core storage and native values retain identity.
use super::*;
pub struct RibbonGalleryItemModelMetadata;
ferroui_base::ferro_markup_type!(class RibbonGalleryItemModelMetadata as "RibbonGalleryItemModel" {
 namespace:"FerroRibbon.Model",this:Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>,handles:[Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>,Option<Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>>,Rc<ferroribbon_core::model::RibbonGalleryItemModel>,ferroribbon_core::model::RibbonGalleryItemModel],base:Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>,
 constructors:[()=>||ferroribbon_core::model::RibbonGalleryItemModel::new(None,None) as Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>,(Option<String>)=>|id:Option<String>|ferroribbon_core::model::RibbonGalleryItemModel::new(id.map(Into::into),None) as Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>,(Option<String>,Option<String>)=>|id:Option<String>,label:Option<String>|ferroribbon_core::model::RibbonGalleryItemModel::new(id.map(Into::into),label.map(Into::into)) as Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>,(Option<String>,Option<String>,Option<Rc<RibbonIcon>>)=>|id:Option<String>,label:Option<String>,icon|ferroribbon_core::model::RibbonGalleryItemModel::with_options(id.map(Into::into),label.map(Into::into),icon,None,None) as Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>,(Option<String>,Option<String>,Option<Rc<RibbonIcon>>,Option<String>)=>|id:Option<String>,label:Option<String>,icon,category:Option<String>|ferroribbon_core::model::RibbonGalleryItemModel::with_options(id.map(Into::into),label.map(Into::into),icon,category.map(Into::into),None) as Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>,(Option<String>,Option<String>,Option<Rc<RibbonIcon>>,Option<String>,Option<ferroui_base::BoxedValue>)=>|id:Option<String>,label:Option<String>,icon,category:Option<String>,value|ferroribbon_core::model::RibbonGalleryItemModel::with_options(id.map(Into::into),label.map(Into::into),icon,category.map(Into::into),crate::metadata::model_objects::from_native(value)) as Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>],
 properties:[Category:Option<String>{try_get:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>|string(v.category()),set:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>,x:Option<String>|v.set_category(x.map(Into::into))},Content:Option<ferroui_base::BoxedValue>{try_get:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>|crate::metadata::model_objects::to_native(v.content()),set:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>,x|v.set_content(crate::metadata::model_objects::from_native_replacing(x,||v.content()))},Value:Option<ferroui_base::BoxedValue>{try_get:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>|crate::metadata::model_objects::to_native(v.value()),set:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>,x|v.set_value(crate::metadata::model_objects::from_native_replacing(x,||v.value()))}]
});
pub struct RibbonGalleryModelMetadata;
ferroui_base::ferro_markup_type!(class RibbonGalleryModelMetadata as "RibbonGalleryModel" {
 namespace:"FerroRibbon.Model",this:Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>,handles:[Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>,Option<Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>>,Rc<ferroribbon_core::model::RibbonGalleryModel>,ferroribbon_core::model::RibbonGalleryModel],base:Rc<dyn ferroribbon_core::model::RibbonItemModelImpl>,
 constructors:[()=>||ferroribbon_core::model::RibbonGalleryModel::new(None,None) as Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>,(Option<String>)=>|id:Option<String>|ferroribbon_core::model::RibbonGalleryModel::new(id.map(Into::into),None) as Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>,(Option<String>,Option<String>)=>|id:Option<String>,label:Option<String>|ferroribbon_core::model::RibbonGalleryModel::new(id.map(Into::into),label.map(Into::into)) as Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>],
 properties:[Items:ModelGalleryItems{get:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>|v.items()},MenuItems:ModelNodes{get:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>|v.menu_items()},SelectedItem:Option<Rc<dyn ferroribbon_core::model::RibbonGalleryItemModelImpl>>{get:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>|v.selected_item(),set:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>,x|v.set_selected_item(x)},PreviewCommand:Option<Rc<dyn ferroribbon_core::commands::ICommand>>{get:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>|v.preview_command(),set:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>,x|v.set_preview_command(x)},
MinColumns:i32{get:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>|v.min_columns(),set:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>,x:i32|v.set_min_columns(x)},
MaxColumns:i32{get:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>|v.max_columns(),set:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>,x:i32|v.set_max_columns(x)},
DropDownColumns:i32{get:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>|v.drop_down_columns(),set:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>,x:i32|v.set_drop_down_columns(x)},
ItemWidth:f64{get:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>|v.item_width(),set:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>,x:f64|v.set_item_width(x)},
ItemHeight:f64{get:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>|v.item_height(),set:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>,x:f64|v.set_item_height(x)},
Rows:i32{get:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>|v.rows(),set:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>,x:i32|v.set_rows(x)},
IsFilterEnabled:bool{get:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>|v.is_filter_enabled(),set:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>,x:bool|v.set_is_filter_enabled(x)},
ShowLabels:bool{get:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>|v.show_labels(),set:|v:&Rc<dyn ferroribbon_core::model::RibbonGalleryModelImpl>,x:bool|v.set_show_labels(x)},
]});

// Closed erased collection view used by Gallery ItemsSource. No payload copies.
pub(super) fn register_views() {
    ValueTypes::register_cast::<
        ModelGalleryItems,
        crate::metadata::combo_items::RibbonObjectEnumerable,
    >(|items| {
        let items = items.clone();
        ferroribbon_core::model::RibbonItemsSource::new(move || {
            Box::new(
                items
                    .iter()
                    .map(|v| v.map(|v| std::rc::Rc::new(v) as ferroui_base::BoxedValue)),
            )
        })
    });
}
