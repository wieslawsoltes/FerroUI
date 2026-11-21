use super::{
    CONSUMER_ASSEMBLY, compiled_xaml, group_items_consumer::ConsumerGroupItemsPanel, tests::run,
};
use ferroribbon::primitives::RibbonMetrics;
use ferroribbon_core::collections::RibbonItems;
use ferroribbon_core::{RibbonGroupItemsLayout, collections::RibbonReadOnlyList};
use ferroui_base::{Rect, Size, data::core::ValueTypes, input::KeyboardNavigation};
use std::sync::Arc;
#[test]
fn group_items_panel_compiles_loads_and_inherits_layout_across_assemblies() {
    run(|| {
        let compiled =
            compiled_xaml::try_load(None, "ferres://ferroribbon_markup_consumer/GroupItems.xaml")
                .unwrap()
                .unwrap();
        let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
            include_str!("GroupItems.xaml"),
            Some(&CONSUMER_ASSEMBLY),
        )
        .unwrap();
        for value in [compiled, runtime] {
            let p = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ConsumerGroupItemsPanel>()
                .unwrap();
            p.set_use_layout_rounding(false);
            assert_eq!(p.items_layout(), RibbonGroupItemsLayout::Rows);
            assert_eq!(p.row_count(), 2);
            assert!(Arc::ptr_eq(
                p.metrics().as_ref().unwrap(),
                &RibbonMetrics::compact()
            ));
            let retained = p.children().snapshot();
            for c in retained.iter() {
                c.set_use_layout_rounding(false);
            }
            p.measure(Size::INFINITY);
            assert_eq!(p.desired_size().width, 34.25);
            assert_eq!(p.desired_size().height, 64.);
            assert!(p.measures.get() > 0);
            let mask = RibbonItems::from_values(vec![false, true, false]) as RibbonReadOnlyList<_>;
            p.set_in_line_mask(Some(mask.clone()));
            p.set_is_simplified(true);
            p.invalidate_measure();
            p.measure(Size::INFINITY);
            p.arrange(Rect::from_size(p.desired_size()));
            assert!(std::rc::Rc::ptr_eq(&p.in_line_mask().unwrap(), &mask));
            assert!(!KeyboardNavigation::get_is_tab_stop(&retained[0]));
            assert!(KeyboardNavigation::get_is_tab_stop(&retained[1]));
            assert!(!KeyboardNavigation::get_is_tab_stop(&retained[2]));
            p.set_is_simplified(false);
            p.set_items_layout(RibbonGroupItemsLayout::Columns);
            p.invalidate_measure();
            p.measure(Size::INFINITY);
            p.arrange(Rect::from_size(p.desired_size()));
            for (i, c) in retained.iter().enumerate() {
                assert_eq!(p.children().get(i), *c);
                assert!(KeyboardNavigation::get_is_tab_stop(c));
            }
        }
    });
}
