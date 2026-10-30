// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::tests::{run, update};
use crate::*;
use ferroui_base::data::TemplateBinding;
use ferroui_base::layout::Orientation;
use ferroui_base::media::Brushes;
use ferroui_base::{Rect, Ref, Size};
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::templates::{
    FuncControlTemplate, FuncTemplate, FuncTemplateNameScopeExtensions,
};
use ferroui_controls::{Border, Control, ItemsControl, ItemsSource, Panel, Window};
use ferroui_markup_xaml::templates::ItemsPanelTemplate;

#[test]
fn lays_out_children_items_control_without_a_control_template() {
    run(|| {
        // Preserve the original source case: no ItemsControl theme/template is
        // installed, so none of the supplied source controls are arranged.
        let outer = ItemsControl::new();
        outer.set_width(1000.0);
        outer.set_height(500.0);
        let horizontal = ProportionalStackPanel::new();
        horizontal.set_orientation(Orientation::Horizontal);
        let outer_template = ItemsPanelTemplate::new();
        outer_template.set_content(Some(Control::boxed(horizontal)));
        outer.set_items_panel(outer_template.as_template());

        let inner = ItemsControl::new();
        let vertical = ProportionalStackPanel::new();
        vertical.set_orientation(Orientation::Vertical);
        let inner_template = ItemsPanelTemplate::new();
        inner_template.set_content(Some(Control::boxed(vertical)));
        inner.set_items_panel(inner_template.as_template());
        let green = Border::new();
        green.set_background(Some(Brushes::green()));
        let blue = Border::new();
        blue.set_background(Some(Brushes::blue()));
        let red = Border::new();
        red.set_background(Some(Brushes::red()));
        let inner_items: [Ref<Control>; 4] = [
            green.upcast(),
            ProportionalStackPanelSplitter::new().upcast(),
            blue.upcast(),
            red.upcast(),
        ];
        inner.set_items_source(Some(ItemsSource::from_items(
            inner_items
                .iter()
                .cloned()
                .map(|item| Some(Control::boxed(item))),
        )));
        let green = Border::new();
        green.set_background(Some(Brushes::green()));
        let blue = Border::new();
        blue.set_background(Some(Brushes::blue()));
        let outer_items: [Ref<Control>; 5] = [
            green.upcast(),
            ProportionalStackPanelSplitter::new().upcast(),
            blue.upcast(),
            ProportionalStackPanelSplitter::new().upcast(),
            inner.clone().upcast(),
        ];
        outer.set_items_source(Some(ItemsSource::from_items(
            outer_items
                .iter()
                .cloned()
                .map(|item| Some(Control::boxed(item))),
        )));
        outer.apply_template();
        outer.measure(Size::INFINITY);
        outer.arrange(Rect::from_size(outer.desired_size()));
        assert_eq!(outer.bounds().size(), Size::new(1000.0, 500.0));
        for item in outer_items.iter().chain(inner_items.iter()) {
            assert_eq!(item.bounds(), Rect::default());
        }
        assert_eq!(inner.bounds().size(), Size::new(0.0, 0.0));
        assert!(outer.items_panel_root().is_none());
    });
}

fn install_items_template(control: &ItemsControl, orientation: Orientation) {
    control.set_items_panel(FuncTemplate::<Option<Ref<Panel>>>::new(move || {
        let panel = ProportionalStackPanel::new();
        panel.set_orientation(orientation);
        Some(panel.upcast())
    }));
    control.set_template(Some(FuncControlTemplate::for_type::<ItemsControl>(
        |_, scope| {
            let presenter = ItemsPresenter::new();
            presenter.set_name(Some("PART_ItemsPresenter".into()));
            let property = ItemsControl::items_panel_property().as_property();
            presenter.bind_binding(property, &TemplateBinding::new(property));
            presenter.register_in_name_scope(&**scope).upcast()
        },
    )));
}

#[test]
fn generated_items_panels_arrange_nested_controls_and_splitters() {
    run(|| {
        let outer = ItemsControl::new();
        install_items_template(&outer, Orientation::Horizontal);
        let inner = ItemsControl::new();
        install_items_template(&inner, Orientation::Vertical);
        let top = Border::new();
        let middle = ProportionalStackPanelSplitter::new();
        let bottom = Border::new();
        inner.set_items_source(Some(ItemsSource::from_items([
            Some(Control::boxed(top.clone())),
            Some(Control::boxed(middle.clone())),
            Some(Control::boxed(bottom.clone())),
        ])));
        let left = Border::new();
        let splitter = ProportionalStackPanelSplitter::new();
        outer.set_items_source(Some(ItemsSource::from_items([
            Some(Control::boxed(left.clone())),
            Some(Control::boxed(splitter.clone())),
            Some(Control::boxed(inner.clone())),
        ])));
        let window = Window::new();
        window.set_width(1000.0);
        window.set_height(500.0);
        window.set_content(Some(Control::boxed(outer.clone())));
        window.show();
        update(&window);
        let outer_panel = outer
            .items_panel_root()
            .unwrap()
            .cast::<ProportionalStackPanel>()
            .unwrap();
        let inner_panel = inner
            .items_panel_root()
            .unwrap()
            .cast::<ProportionalStackPanel>()
            .unwrap();
        assert_eq!(outer_panel.children().count(), 3);
        assert_eq!(inner_panel.children().count(), 3);
        assert_eq!(left.bounds(), Rect::new(0.0, 0.0, 498.0, 500.0));
        assert_eq!(splitter.bounds(), Rect::new(498.0, 0.0, 4.0, 500.0));
        assert_eq!(inner.bounds(), Rect::new(502.0, 0.0, 498.0, 500.0));
        assert_eq!(top.bounds(), Rect::new(0.0, 0.0, 498.0, 248.0));
        assert_eq!(middle.bounds(), Rect::new(0.0, 248.0, 498.0, 4.0));
        assert_eq!(bottom.bounds(), Rect::new(0.0, 252.0, 498.0, 248.0));
        window.close();
    });
}
