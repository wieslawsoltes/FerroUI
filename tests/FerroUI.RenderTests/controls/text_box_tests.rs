//! Port of upstream's `Controls/TextBoxTests.cs`.

use crate::test_base::{test_font_family, TestBase};
use ferroui_base::controls::NameScopeRef;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::{Brushes, TextOptions, TextRenderingMode};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::presenters::TextPresenter;
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use ferroui_controls::{Border, Control, Panel, TextBlock, TextBox};
use std::rc::Rc;

fn base() -> TestBase {
    TestBase::new(r"Controls\TextBox")
}

fn create_text_box_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TextBox>(|text_box: &Ref<TextBox>, scope: &NameScopeRef| -> Ref<Control> {
        let border = Border::new();
        border.set_background(text_box.background());
        border.set_border_brush(Some(Brushes::gray()));
        border.set_border_thickness(Thickness::uniform(1.0));
        border.set_padding(Thickness::uniform(4.0));

        let panel = Panel::new();

        // Use Antialias mode
        TextOptions::set_text_rendering_mode(&panel, TextRenderingMode::Antialias);

        let placeholder = TextBlock::new();
        placeholder.set_name(Some("PART_Placeholder".to_string()));
        placeholder.bind_indexer(
            &TextBlock::text_property().bind(),
            &text_box.indexer(&TextBox::placeholder_text_property().bind()),
        );
        placeholder.bind_indexer(
            &TextBlock::foreground_property().bind(),
            &text_box.indexer(&TextBox::placeholder_foreground_property().bind()),
        );
        placeholder.set_font_family(text_box.font_family());
        placeholder.set_font_size(text_box.font_size());
        placeholder.set_vertical_alignment(VerticalAlignment::Center);
        placeholder.set_opacity(0.5);
        let placeholder = placeholder.register_in_name_scope(&**scope);

        let presenter = TextPresenter::new();
        presenter.set_name(Some("PART_TextPresenter".to_string()));
        presenter.bind_indexer(
            &TextPresenter::text_property().bind(),
            &text_box.indexer(&TextBox::text_property().bind()),
        );
        presenter.bind_indexer(
            &TextPresenter::caret_index_property().bind(),
            &text_box.indexer(&TextBox::caret_index_property().bind()),
        );
        presenter.set_font_family(text_box.font_family());
        presenter.set_font_size(text_box.font_size());
        let presenter = presenter.register_in_name_scope(&**scope);

        panel.children().add(placeholder);
        panel.children().add(presenter);
        border.set_child(panel);

        border.upcast()
    })
}

#[test]
fn placeholder_with_red_foreground() {
    let t = base();
    let target = Border::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(50.0);
    target.set_background(Some(Brushes::white()));
    let child = TextBox::new();
    child.set_template(Some(create_text_box_template()));
    child.set_font_family(test_font_family());
    child.set_font_size(12.0);
    child.set_background(Some(Brushes::white()));
    child.set_placeholder_text(Some("Red placeholder"));
    child.set_placeholder_foreground(Some(Brushes::red()));
    target.set_child(child);

    t.render_to_file(&target, "Placeholder_With_Red_Foreground");
    t.compare_images("Placeholder_With_Red_Foreground");
}

#[test]
fn placeholder_with_blue_foreground() {
    let t = base();
    let target = Border::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(50.0);
    target.set_background(Some(Brushes::white()));
    let child = TextBox::new();
    child.set_template(Some(create_text_box_template()));
    child.set_font_family(test_font_family());
    child.set_font_size(12.0);
    child.set_background(Some(Brushes::white()));
    child.set_placeholder_text(Some("Blue placeholder"));
    child.set_placeholder_foreground(Some(Brushes::blue()));
    target.set_child(child);

    t.render_to_file(&target, "Placeholder_With_Blue_Foreground");
    t.compare_images("Placeholder_With_Blue_Foreground");
}

#[test]
fn placeholder_with_default_foreground() {
    let t = base();
    let target = Border::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(50.0);
    target.set_background(Some(Brushes::white()));
    let child = TextBox::new();
    child.set_template(Some(create_text_box_template()));
    child.set_font_family(test_font_family());
    child.set_font_size(12.0);
    child.set_background(Some(Brushes::white()));
    child.set_placeholder_text(Some("Default placeholder"));
    child.set_placeholder_foreground(Some(Brushes::gray()));
    target.set_child(child);

    t.render_to_file(&target, "Placeholder_With_Default_Foreground");
    t.compare_images("Placeholder_With_Default_Foreground");
}
