use crate::controls::{IRibbonItem, RibbonItemLayout, RibbonLabel, RibbonSeparator};
use crate::primitives::{RibbonGroupItemsPanel, RibbonMetrics};
use ferroui_base::{
    BoxedValue, Rect, Ref, Size, StaticType, Thickness,
    controls::ResourceDictionary,
    data::core::{ValueType, ValueTypes},
    input::KeyboardNavigation,
    layout::{HorizontalAlignment, Orientation, VerticalAlignment},
    styling::ControlTheme,
};
use ferroui_controls::{Application, Control, TextBlock};
use std::rc::Rc;
fn run(test: impl FnOnce() + Send + 'static) {
    crate::tests::run(move || {
        let app = Application::current().unwrap();
        let theme = ferroui_themes_simple::SimpleTheme::new().as_style();
        app.styles().add(theme.clone());
        test();
        app.styles().remove(theme);
    })
}
fn item(value: BoxedValue) -> Rc<dyn IRibbonItem> {
    ValueTypes::try_convert(Some(&value), ValueType::of::<Rc<dyn IRibbonItem>>())
        .flatten()
        .unwrap()
        .downcast_ref::<Rc<dyn IRibbonItem>>()
        .unwrap()
        .clone()
}
fn resources() -> Ref<ResourceDictionary> {
    let value =
        crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/SimpleItems.xaml")
            .unwrap()
            .unwrap();
    ValueTypes::as_object(&*value)
        .unwrap()
        .cast::<ResourceDictionary>()
        .unwrap()
}
fn theme<T: StaticType>(resources: &ResourceDictionary) -> Ref<ControlTheme> {
    let value = resources
        .try_get_resource(&T::TYPE.into(), None)
        .flatten()
        .unwrap();
    ValueTypes::as_object(&*value)
        .unwrap()
        .cast::<ControlTheme>()
        .unwrap()
}
fn line(separator: &RibbonSeparator) -> Ref<Control> {
    separator
        .get_visual_descendants()
        .find(|c| {
            c.cast::<ferroui_base::StyledElement>()
                .is_some_and(|c| c.name().as_deref() == Some("PART_Line"))
        })
        .unwrap()
        .cast()
        .unwrap()
}
#[test]
fn production_separator_default_interface_and_source_template_dimensions() {
    run(|| {
        let separator = RibbonSeparator::new();
        assert_eq!(separator.orientation(), Orientation::Vertical);
        assert_eq!(separator.size(), ferroribbon_core::RibbonItemSize::Large);
        assert!(!separator.can_add_to_quick_access());
        assert!(!KeyboardNavigation::get_is_tab_stop(&separator));
        let item = item(Control::boxed(&separator));
        assert!(!item.invoke());
        let first = item.create_overflow_menu_items();
        let a = first.iter().next().unwrap().unwrap();
        let b = first.iter().next().unwrap().unwrap();
        assert!(a.ptr_eq(&b));
        let next = item
            .create_overflow_menu_items()
            .iter()
            .next()
            .unwrap()
            .unwrap();
        assert!(!a.ptr_eq(&next));
        assert!(a.cast::<ferroui_controls::Separator>().is_some());
        separator.set_theme(theme::<RibbonSeparator>(&resources()));
        let window = ferroui_controls::Window::new();
        window.set_content(Some(Control::boxed(&separator)));
        window.show();
        window.measure(Size::new(300., 200.));
        window.arrange(Rect::new(0., 0., 300., 200.));
        for simplified in [false, true] {
            for metrics in [
                RibbonMetrics::compact(),
                RibbonMetrics::comfortable(),
                RibbonMetrics::touch(),
            ] {
                item.apply_layout(RibbonItemLayout {
                    size: ferroribbon_core::RibbonItemSize::Small,
                    metrics: Some(metrics.clone()),
                    is_simplified: simplified,
                    ..RibbonItemLayout::default()
                });
                let line = line(&separator);
                assert_eq!(line.width(), 1.);
                assert_eq!(
                    line.height(),
                    if simplified {
                        metrics.simplified_item_height() - 8.
                    } else {
                        metrics.group_content_height() - 6.
                    }
                );
                assert_eq!(line.margin(), Thickness::uniform(3.));
            }
        }
        separator.set_orientation(Orientation::Horizontal);
        let line = line(&separator);
        assert!(line.width().is_nan());
        assert_eq!(line.height(), 1.);
        assert_eq!(line.margin(), Thickness::new(4., 3., 4., 3.));
        assert_eq!(
            separator.horizontal_alignment(),
            HorizontalAlignment::Stretch
        );
        assert_eq!(separator.vertical_alignment(), VerticalAlignment::Top);
        window.close();
    })
}
#[test]
fn source_separator_branch_precedes_large_item_in_real_group_panel() {
    run(|| {
        let separator = RibbonSeparator::new();
        let panel = RibbonGroupItemsPanel::new();
        assert!(panel.is_full_height(&separator));
        separator.set_orientation(Orientation::Horizontal);
        assert!(!panel.is_full_height(&separator));
        separator.set_height(1000.);
        separator.measure(Size::INFINITY);
        separator.arrange(Rect::from_size(separator.desired_size()));
        assert!(!panel.is_full_height(&separator));
        separator.set_orientation(Orientation::Vertical);
        separator.set_current_size(ferroribbon_core::RibbonItemSize::Small);
        assert!(panel.is_full_height(&separator));
    })
}
#[test]
fn production_label_uses_source_markup_label_binding_and_empty_overflow() {
    run(|| {
        let label = RibbonLabel::new();
        label.set_label(Some("Status".into()));
        assert!(!label.can_add_to_quick_access());
        assert!(!KeyboardNavigation::get_is_tab_stop(&label));
        let i = item(Control::boxed(&label));
        assert!(!i.invoke());
        assert!(i.create_overflow_menu_items().iter().next().is_none());
        assert!(i.create_overflow_menu_items() == i.create_overflow_menu_items());
        assert!(
            i.create_overflow_menu_items()
                == item(Control::boxed(RibbonLabel::new())).create_overflow_menu_items()
        );
        label.set_theme(theme::<RibbonLabel>(&resources()));
        let window = ferroui_controls::Window::new();
        window.set_content(Some(Control::boxed(&label)));
        window.show();
        window.measure(Size::new(300., 200.));
        window.arrange(Rect::new(0., 0., 300., 200.));
        let text = label
            .get_visual_descendants()
            .find_map(|c| c.cast::<TextBlock>())
            .unwrap();
        assert_eq!(text.text().as_deref(), Some("Status"));
        label.set_label(Some("Ready".into()));
        assert_eq!(text.text().as_deref(), Some("Ready"));
        window.close();
    })
}
