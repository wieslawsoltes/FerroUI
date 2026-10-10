//! Port of upstream's `Controls/PipsPagerTests.cs`.
//!
//! The template of the pips pager is built in code, as upstream builds it: `target[!Property] =
//! source[!Property]` is `target.bind_indexer(&Property.bind(), &source.indexer(&Property.bind()))`, and the
//! two-way form `~Property` is `!Property.bind()`.

use crate::test_base::{test_font_family, CompareOptions, TestBase};
use ferroui_base::data::ReflectionBinding;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroObject, Rect, Ref, Size, Thickness, Visual};
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::primitives::{SelectingItemsControl, TemplatedControl};
use ferroui_controls::shapes::{Rectangle, Shape};
use ferroui_controls::templates::{
    FuncControlTemplate, FuncTemplate, FuncTemplateNameScopeExtensions, IControlTemplate, ITemplateOf,
};
use ferroui_controls::{
    Border, Button, ContentControl, Control, ItemsControl, ListBox, ListBoxItem, Panel, PipsPager, StackPanel,
    TextBlock,
};
use std::rc::Rc;

fn base() -> TestBase {
    TestBase::new(r"Controls\PipsPager")
}

fn create_pips_pager_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<PipsPager>(|control, scope| {
        let stack_panel = StackPanel::new();
        stack_panel.set_name(Some("PART_RootPanel".to_string()));
        stack_panel.set_spacing(4.0);
        stack_panel.bind_indexer(
            &StackPanel::orientation_property().as_property().bind(),
            &control.indexer(&PipsPager::orientation_property().as_property().bind()),
        );
        stack_panel.set_horizontal_alignment(HorizontalAlignment::Center);
        stack_panel.set_vertical_alignment(VerticalAlignment::Center);

        let button_template: Rc<dyn IControlTemplate> = FuncControlTemplate::for_type::<Button>(|b, _s| {
            let border = Border::new();
            border.set_background(Some(Brushes::light_gray()));
            let text = TextBlock::new();
            text.bind_indexer(
                &TextBlock::text_property().as_property().bind(),
                &b.indexer(&ContentControl::content_property().as_property().bind()),
            );
            text.set_font_family(test_font_family());
            text.set_vertical_alignment(VerticalAlignment::Center);
            text.set_horizontal_alignment(HorizontalAlignment::Center);
            border.set_child(text);
            border.upcast()
        });

        let prev_button = Button::new();
        prev_button.set_name(Some("PART_PreviousButton".to_string()));
        prev_button.set_content(Some(Rc::new("<".to_string())));
        prev_button.set_template(Some(button_template.clone()));
        prev_button.set_width(20.0);
        prev_button.set_height(20.0);
        prev_button.bind_indexer(
            &Visual::is_visible_property().as_property().bind(),
            &control.indexer(&PipsPager::is_previous_button_visible_property().as_property().bind()),
        );
        let prev_button = prev_button.register_in_name_scope(&**scope);

        let next_button = Button::new();
        next_button.set_name(Some("PART_NextButton".to_string()));
        next_button.set_content(Some(Rc::new(">".to_string())));
        next_button.set_template(Some(button_template.clone()));
        next_button.set_width(20.0);
        next_button.set_height(20.0);
        next_button.bind_indexer(
            &Visual::is_visible_property().as_property().bind(),
            &control.indexer(&PipsPager::is_next_button_visible_property().as_property().bind()),
        );
        let next_button = next_button.register_in_name_scope(&**scope);

        let list_box_template: Rc<dyn IControlTemplate> = FuncControlTemplate::for_type::<ListBox>(|lb, s| {
            let presenter = ItemsPresenter::new();
            presenter.set_name(Some("PART_ItemsPresenter".to_string()));
            presenter.bind_indexer(
                &!ItemsPresenter::items_panel_property().as_property().bind(),
                &lb.indexer(&!ItemsControl::items_panel_property().as_property().bind()),
            );
            presenter.register_in_name_scope(&**s).upcast()
        });

        let pips_list = ListBox::new();
        pips_list.set_name(Some("PART_PipsPagerList".to_string()));
        pips_list.set_template(Some(list_box_template));
        pips_list.bind_indexer(
            &ItemsControl::items_source_property().as_property().bind(),
            &*ReflectionBinding::new("TemplateSettings.Pips")
                .with_source(Some(Rc::new(control.clone().upcast::<FerroObject>()))),
        );
        pips_list.bind_indexer(
            &SelectingItemsControl::selected_index_property().as_property().bind(),
            &control.indexer(&PipsPager::selected_page_index_property().as_property().bind()),
        );
        let items_panel_owner = control.clone();
        let items_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> = FuncTemplate::new(move || {
            let panel = StackPanel::new();
            panel.set_spacing(2.0);
            panel.bind_indexer(
                &StackPanel::orientation_property().as_property().bind(),
                &items_panel_owner.indexer(&PipsPager::orientation_property().as_property().bind()),
            );
            Some(panel.upcast::<Panel>())
        });
        pips_list.set_items_panel(items_panel);
        let pips_list = pips_list.register_in_name_scope(&**scope);

        let item_style = Style::with_selector(Selectors::of_type::<ListBoxItem>());
        let item_template: Rc<dyn IControlTemplate> = FuncControlTemplate::for_type::<ListBoxItem>(|_item, s| {
            let pip = Rectangle::new();
            pip.set_name(Some("Pip".to_string()));
            pip.set_width(10.0);
            pip.set_height(10.0);
            pip.register_in_name_scope(&**s).upcast()
        });
        item_style.setters().add(Setter::new(TemplatedControl::template_property(), Some(item_template)));

        let default_pip_style = Style::with_selector(Selectors::of_type::<ListBoxItem>().template().name("Pip"));
        let gray: Rc<dyn IBrush> = Brushes::gray();
        default_pip_style.setters().add(Setter::new(Shape::fill_property(), Some(gray)));

        let selected_style =
            Style::with_selector(Selectors::of_type::<ListBoxItem>().class(":selected").template().name("Pip"));
        let red: Rc<dyn IBrush> = Brushes::red();
        selected_style.setters().add(Setter::new(Shape::fill_property(), Some(red)));

        pips_list.styles().add(item_style);
        pips_list.styles().add(default_pip_style);
        pips_list.styles().add(selected_style);

        stack_panel.children().add(prev_button);
        stack_panel.children().add(pips_list);
        stack_panel.children().add(next_button);

        stack_panel.upcast::<Control>()
    })
}

#[test]
fn pips_pager_default() {
    let t = base();
    let pips_pager = PipsPager::new();
    pips_pager.set_template(Some(create_pips_pager_template()));
    pips_pager.set_number_of_pages(5);
    pips_pager.set_selected_page_index(1);

    let target = Border::new();
    target.set_padding(Thickness::uniform(20.0));
    target.set_background(Some(Brushes::white()));
    target.set_child(pips_pager);
    target.set_width(400.0);
    target.set_height(150.0);

    target.measure(Size::new(400.0, 150.0));
    target.arrange(Rect::new(0.0, 0.0, 400.0, 150.0));
    Dispatcher::ui_thread().run_jobs(None);

    t.render_to_file(&target, "PipsPager_Default");
    t.compare_images_with("PipsPager_Default", CompareOptions { skip_immediate: true, ..Default::default() });
}

#[test]
fn pips_pager_preselected_index() {
    let t = base();
    let pips_pager = PipsPager::new();
    pips_pager.set_template(Some(create_pips_pager_template()));
    pips_pager.set_number_of_pages(5);
    pips_pager.set_selected_page_index(3);

    let target = Border::new();
    target.set_padding(Thickness::uniform(20.0));
    target.set_background(Some(Brushes::white()));
    target.set_child(pips_pager);
    target.set_width(400.0);
    target.set_height(150.0);

    target.measure(Size::new(400.0, 150.0));
    target.arrange(Rect::new(0.0, 0.0, 400.0, 150.0));
    Dispatcher::ui_thread().run_jobs(None);

    t.render_to_file(&target, "PipsPager_Preselected_Index");
    t.compare_images_with(
        "PipsPager_Preselected_Index",
        CompareOptions { skip_immediate: true, ..Default::default() },
    );
}
