//! Gallery factory and exact parameter adapter from RibbonElementFactory.cs.
use super::ribbon_element_factory_parameters::gallery_preview_parameter;
use super::{RibbonElementFactory, RibbonModelBindings};
use crate::controls::{RibbonGallery, RibbonGalleryItemEventArgs};
use ferroribbon_core::model::*;
use ferroui_base::{BoxedValue, Ref, metadata::MarkupDelegate};
use ferroui_controls::Control;
use std::rc::Rc;
impl RibbonElementFactory {
    pub(super) fn build_gallery(
        &self,
        model: Rc<dyn RibbonGalleryModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        let gallery = RibbonGallery::new();
        gallery.set_items_source(Some(Rc::new(model.items())));
        let m = model.clone();
        let g = gallery.clone();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || {
                g.set_min_columns(m.min_columns());
                g.set_max_columns(m.max_columns());
                g.set_drop_down_columns(m.drop_down_columns());
                g.set_item_width(m.item_width());
                g.set_item_height(m.item_height());
                g.set_is_filter_enabled(m.is_filter_enabled());
                g.set_show_item_labels(m.show_labels());
                g.set_rows(m.rows());
            })),
            Some(
                [
                    "MinColumns",
                    "MaxColumns",
                    "DropDownColumns",
                    "ItemWidth",
                    "ItemHeight",
                    "IsFilterEnabled",
                    "ShowLabels",
                    "Rows",
                ]
                .into_iter()
                .map(|s| Some(s.into()))
                .collect(),
            ),
        );
        let m = model.clone();
        let g = gallery.clone();
        let mm = model.clone();
        let gg = gallery.clone();
        bindings.two_way(
            Some(model.clone()),
            Some("SelectedItem".into()),
            Some(Rc::new(move || {
                g.set_selected_item(m.selected_item().map(|v| Rc::new(v) as BoxedValue))
            })),
            Some(gallery.clone().upcast()),
            Some(RibbonGallery::selected_item_property().as_property()),
            Some(Rc::new(move || {
                let value = crate::commands::command_parameter(gg.selected_item().as_ref());
                let selected = value.and_then(|v| {
                    if is_registered_model_type(v.as_ref().type_id()) {
                        try_cast_model::<dyn RibbonGalleryItemModelImpl>(Some(v))
                            .expect("SelectedItem model cast")
                    } else {
                        None
                    }
                });
                mm.set_selected_item(selected);
            })),
        );
        let m = model.clone();
        let handler = MarkupDelegate::new(move |a| {
            let e = a[1]
                .as_ref()
                .unwrap()
                .downcast_ref::<Rc<RibbonGalleryItemEventArgs>>()
                .expect("Gallery preview event");
            let command = m.preview_command();
            let value = gallery_preview_parameter(e.item().as_ref());
            Self::execute(command.as_deref(), value.as_deref());
            None
        });
        gallery.add_item_preview(Some(handler.clone()));
        let g = gallery.clone();
        bindings.add(Some(Rc::new(move || {
            g.remove_item_preview(Some(handler.clone()))
        })));
        let factory = self.to_ref();
        bindings.items(
            model.menu_items(),
            Rc::new(gallery.footer_items()),
            Rc::new(move |entry, b| factory.create_menu_element(entry, Some(b))),
            false,
            None,
            None,
        );
        gallery.upcast()
    }
}
