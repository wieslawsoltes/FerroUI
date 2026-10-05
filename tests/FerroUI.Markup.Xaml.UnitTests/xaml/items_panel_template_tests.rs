//! Port of `Xaml/ItemsPanelTemplateTests.cs`.

use std::rc::Rc;

use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::Ref;
use ferroui_controls::presenters::ItemsPresenter;
use ferroui_controls::{ItemsSource, ListBox, Window};

use crate::support::app::styled_window_application;
use crate::support::helpers::{assert_value_is_type, value_of, value_type_name};
use crate::support::loader::load_as;

#[test]
fn items_panel_template_in_style_allows_template_binding() {
    let _app = styled_window_application();
    let window = load_as::<Ref<Window>>(
        r#"<Window xmlns="https://github.com/ferroui"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
    <Window.Styles>
        <Style Selector="ListBox">
            <Setter Property="Template">
                <ControlTemplate>
                    <ItemsPresenter Name="PART_ItemsPresenter"
                                    ItemsPanel="{TemplateBinding ItemsPanel}" />
                </ControlTemplate>
            </Setter>
            <Setter Property="ItemsPanel">
                <ItemsPanelTemplate>
                    <Panel Background="{TemplateBinding Background}"
                           Tag="{TemplateBinding ItemsSource}" />
                </ItemsPanelTemplate>
            </Setter>
        </Style>
    </Window.Styles>
    <ListBox Background="DodgerBlue" />
</Window>"#,
    );
    let list_box = assert_value_is_type::<ListBox>(&window.content());
    let items = ItemsSource::from_strs(["foo", "bar"]);
    list_box.set_items_source(Some(items.clone()));

    window.apply_template();
    list_box.apply_template();

    let items_presenter = list_box.find_descendant_of_type::<ItemsPresenter>(false);
    let items_presenter = items_presenter.expect("the items presenter is null");
    items_presenter.apply_template();

    let panel = items_presenter.panel();
    let panel = panel.expect("the panel is null");
    let dodger_blue: Rc<dyn IBrush> = Brushes::dodger_blue();
    assert_eq!(Some(dodger_blue), panel.background());
    let tag = value_of::<ItemsSource>(&panel.tag())
        .unwrap_or_else(|| panic!("the tag is not the items source: {}", value_type_name(&panel.tag())));
    assert!(items.ptr_eq(&tag), "the tag is not the same collection as the items source");
}

#[test]
fn items_panel_template_in_control_allows_template_binding() {
    let _app = styled_window_application();
    let window = load_as::<Ref<Window>>(
        r#"<Window xmlns="https://github.com/ferroui"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
    <ListBox Background="DodgerBlue">
        <ListBox.Template>
            <ControlTemplate>
                <ItemsPresenter Name="PART_ItemsPresenter"
                                ItemsPanel="{TemplateBinding ItemsPanel}" />
            </ControlTemplate>
        </ListBox.Template>
        <ListBox.ItemsPanel>
            <ItemsPanelTemplate>
                <Panel Background="{TemplateBinding Background}"
                       Tag="{TemplateBinding ItemsSource}" />
            </ItemsPanelTemplate>
        </ListBox.ItemsPanel>
    </ListBox>
</Window>"#,
    );
    let list_box = assert_value_is_type::<ListBox>(&window.content());
    let items = ItemsSource::from_strs(["foo", "bar"]);
    list_box.set_items_source(Some(items.clone()));

    window.apply_template();
    list_box.apply_template();

    let items_presenter = list_box.find_descendant_of_type::<ItemsPresenter>(false);
    let items_presenter = items_presenter.expect("the items presenter is null");
    items_presenter.apply_template();

    let panel = items_presenter.panel();
    let panel = panel.expect("the panel is null");
    let dodger_blue: Rc<dyn IBrush> = Brushes::dodger_blue();
    assert_eq!(Some(dodger_blue), panel.background());
    let tag = value_of::<ItemsSource>(&panel.tag())
        .unwrap_or_else(|| panic!("the tag is not the items source: {}", value_type_name(&panel.tag())));
    assert!(items.ptr_eq(&tag), "the tag is not the same collection as the items source");
}
