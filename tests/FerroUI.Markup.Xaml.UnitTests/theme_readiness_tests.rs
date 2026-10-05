//! Theme readiness: documents of the Simple theme (the files under
//! `theme_readiness/`, brought over with only their default namespace
//! changed) loaded through the run-time loader as a group, the way the
//! theme links its documents (`MergeResourceInclude`), and used by controls
//! of a window: the control theme is found by type, the template is
//! applied, the template parts exist and the values of template bindings
//! and dynamic resources arrive.

use ferroui_base::controls::ResourceDictionary;
use ferroui_base::media::{Color, IBrush};
use ferroui_base::styling::ControlTheme;
use ferroui_base::{Ref, Thickness};
use ferroui_controls::presenters::{ContentPresenter, ItemsPresenter, ScrollContentPresenter};
use ferroui_controls::primitives::ScrollBar;
use ferroui_controls::shapes::Path;
use ferroui_controls::{Border, Button, CheckBox, ListBox, ListBoxItem, RadioButton, ScrollViewer, Window};
use ferroui_markup_xaml::RuntimeXamlLoaderDocument;
use std::rc::Rc;

use crate::support::app::styled_window_application;
use crate::support::helpers::assert_value_is_type;
use crate::support::loader::{document, document_without_uri, group_item, load_as, load_group};

const BASE: &str = include_str!("theme_readiness/Base.xaml");
const BUTTON: &str = include_str!("theme_readiness/Button.xaml");
const CHECK_BOX: &str = include_str!("theme_readiness/CheckBox.xaml");
const RADIO_BUTTON: &str = include_str!("theme_readiness/RadioButton.xaml");
const LIST_BOX: &str = include_str!("theme_readiness/ListBox.xaml");
const LIST_BOX_ITEM: &str = include_str!("theme_readiness/ListBoxItem.xaml");
const SCROLL_VIEWER: &str = include_str!("theme_readiness/ScrollViewer.xaml");

const ROOT: &str = "ferres://FerroUI.Markup.Xaml.UnitTests/theme_readiness";

/// The documents `names` of the theme and a window that merges them into
/// its resources and has `content`; returns the loaded window.
fn window_with(theme_documents: &[(&str, &str)], content: &str) -> Ref<Window> {
    let mut documents: Vec<RuntimeXamlLoaderDocument> =
        theme_documents.iter().map(|(name, xaml)| document(&format!("{ROOT}/{name}.xaml"), xaml)).collect();
    let includes: String = theme_documents
        .iter()
        .map(|(name, _)| format!("<MergeResourceInclude Source='{ROOT}/{name}.xaml'/>"))
        .collect();
    documents.push(document_without_uri(&format!(
        "<Window xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
           <Window.Resources>
             <ResourceDictionary>
               <ResourceDictionary.MergedDictionaries>{includes}</ResourceDictionary.MergedDictionaries>
             </ResourceDictionary>
           </Window.Resources>
           {content}
         </Window>"
    )));
    let loaded = load_group(documents, None);
    group_item::<Ref<Window>>(&loaded, theme_documents.len())
}

fn color_of(brush: Option<Rc<dyn IBrush>>) -> Color {
    brush.expect("a brush").as_solid_color_brush().expect("a solid color brush").color()
}

#[test]
fn accent_dictionary_loads_with_its_theme_dictionaries() {
    let _app = styled_window_application();
    let dictionary = load_as::<Ref<ResourceDictionary>>(BASE);
    // The entries of the dictionary itself and of the two theme dictionaries.
    assert_eq!(dictionary.theme_dictionaries().count(), 2);
    let thickness = dictionary.get(&"ThemeBorderThickness".into()).expect("the thickness");
    assert_eq!(crate::support::loader::cast::<Thickness>(&thickness), Thickness::uniform(1.0));
    let opacity = dictionary.get(&"ThemeDisabledOpacity".into()).expect("the opacity");
    assert_eq!(crate::support::loader::cast::<f64>(&opacity), 0.5);
}

#[test]
fn control_theme_document_is_a_dictionary_with_a_theme_keyed_by_type() {
    let _app = styled_window_application();
    let dictionary = load_as::<Ref<ResourceDictionary>>(BUTTON);
    let theme = assert_value_is_type::<ControlTheme>(&dictionary.get(&Button::TYPE.into()));
    assert!(theme.target_type().is_some_and(|type_| std::ptr::eq(type_, Button::TYPE)));
    // Seven setters and the template; four nested styles.
    assert_eq!(theme.setters().count(), 8);
    assert_eq!(theme.children().count(), 4);
}

#[test]
fn button_theme_applies_template_with_template_bindings_and_dynamic_resources() {
    let _app = styled_window_application();
    let window = window_with(&[("Base", BASE), ("Button", BUTTON)], "<Button Name='button' Content='Hello'/>");
    let button = window.get_control::<Button>("button");
    window.show();

    assert!(button.template().is_some(), "the theme gave the button its template");
    let presenter = button.find_descendant_of_type::<ContentPresenter>(false).expect("the content presenter of the template");
    assert_eq!(presenter.name().as_deref(), Some("PART_ContentPresenter"));
    // A setter of the theme, through a template binding.
    assert_eq!(presenter.padding(), Thickness::uniform(4.0));
    // Dynamic resources of the theme setters, resolved in the theme dictionary
    // of the variant of the window, through template bindings.
    assert_eq!(color_of(presenter.background()), Color::parse("#FFF5F5F5").expect("a color"));
    assert_eq!(color_of(presenter.border_brush()), Color::parse("#FFAAAAAA").expect("a color"));
    assert_eq!(presenter.border_thickness(), Thickness::uniform(1.0));
    // A nested style of the theme that targets a template part.
    assert!(presenter.recognizes_access_key());
    // The content arrives through its template binding.
    assert!(presenter.child().is_some());
}

#[test]
fn check_box_theme_applies_template_with_named_parts() {
    let _app = styled_window_application();
    let window =
        window_with(&[("Base", BASE), ("CheckBox", CHECK_BOX)], "<CheckBox Name='check' IsChecked='True' Content='Check'/>");
    let check_box = window.get_control::<CheckBox>("check");
    window.show();

    let border = check_box.find_descendant_of_type::<Border>(false).expect("the border of the template");
    assert_eq!(border.name().as_deref(), Some("border"));
    assert_eq!(color_of(border.border_brush()), Color::parse("#FF888888").expect("a color"));
    let check_mark = check_box.find_descendant_of_type::<Path>(false).expect("the check mark of the template");
    assert_eq!(check_mark.name().as_deref(), Some("checkMark"));
    // `^:checked /template/ Path#checkMark` wins over `^ /template/ Path#checkMark`.
    assert!(check_mark.is_visible());
    assert_eq!(color_of(check_mark.fill()), Color::parse("#FF086F9E").expect("a color"));
    let presenter = check_box.find_descendant_of_type::<ContentPresenter>(false).expect("the content presenter");
    assert_eq!(presenter.margin(), Thickness::new(4.0, 0.0, 0.0, 0.0));
    // `IsVisible="{TemplateBinding Content, Converter={x:Static ObjectConverters.IsNotNull}}"`.
    assert!(presenter.is_visible());
}

#[test]
fn radio_button_theme_applies_template() {
    let _app = styled_window_application();
    let window = window_with(&[("Base", BASE), ("RadioButton", RADIO_BUTTON)], "<RadioButton Name='radio' Content='Radio'/>");
    let radio = window.get_control::<RadioButton>("radio");
    window.show();
    assert!(radio.template().is_some());
    assert!(radio.find_descendant_of_type::<ContentPresenter>(false).is_some());
}

#[test]
fn list_box_theme_applies_templates_of_the_list_and_its_items() {
    let _app = styled_window_application();
    let window = window_with(
        &[("Base", BASE), ("ListBox", LIST_BOX), ("ListBoxItem", LIST_BOX_ITEM)],
        "<ListBox Name='list'><ListBoxItem>One</ListBoxItem><ListBoxItem>Two</ListBoxItem></ListBox>",
    );
    let list = window.get_control::<ListBox>("list");
    window.show();

    let border = list.find_descendant_of_type::<Border>(false).expect("the border of the template");
    assert_eq!(border.name().as_deref(), Some("border"));
    assert_eq!(color_of(border.border_brush()), Color::parse("#FF888888").expect("a color"));
    let scroll_viewer = list.find_descendant_of_type::<ScrollViewer>(false).expect("the scroll viewer of the template");
    assert_eq!(scroll_viewer.name().as_deref(), Some("PART_ScrollViewer"));
    // `Background="{TemplateBinding Background}"` of a dynamic resource setter.
    assert_eq!(color_of(scroll_viewer.background()), Color::parse("#FFFFFFFF").expect("a color"));
    // The items presenter is the content of the scroll viewer: it is in the visual
    // tree once the scroll viewer has a template of its own (not part of this test).
    let presenter = scroll_viewer
        .content()
        .and_then(|content| ferroui_controls::Control::from_boxed(&content))
        .and_then(|control| control.cast::<ItemsPresenter>())
        .expect("the items presenter of the template");
    assert_eq!(presenter.name().as_deref(), Some("PART_ItemsPresenter"));
    assert_eq!(presenter.margin(), Thickness::uniform(4.0));
}

#[test]
fn scroll_viewer_theme_applies_template_with_its_parts() {
    let _app = styled_window_application();
    let window = window_with(
        &[("Base", BASE), ("ScrollViewer", SCROLL_VIEWER)],
        "<ScrollViewer Name='scroll' Padding='3'><Border Width='10' Height='10'/></ScrollViewer>",
    );
    let scroll = window.get_control::<ScrollViewer>("scroll");
    window.show();

    let presenter =
        scroll.find_descendant_of_type::<ScrollContentPresenter>(false).expect("the content presenter of the template");
    assert_eq!(presenter.name().as_deref(), Some("PART_ContentPresenter"));
    assert_eq!(presenter.padding(), Thickness::uniform(3.0));
    assert!(scroll.find_descendant_of_type::<ScrollBar>(false).is_some());
}

#[test]
fn list_box_with_scroll_viewer_theme_realises_item_containers() {
    let _app = styled_window_application();
    let window = window_with(
        &[
            ("Base", BASE),
            ("ListBox", LIST_BOX),
            ("ListBoxItem", LIST_BOX_ITEM),
            ("ScrollViewer", SCROLL_VIEWER),
        ],
        "<ListBox Name='list'><ListBoxItem>One</ListBoxItem><ListBoxItem>Two</ListBoxItem></ListBox>",
    );
    let list = window.get_control::<ListBox>("list");
    window.show();
    window.update_layout();

    let item = list.find_descendant_of_type::<ListBoxItem>(false).expect("an item container in the visual tree");
    let presenter = item.find_descendant_of_type::<ContentPresenter>(false).expect("the presenter of the item template");
    assert_eq!(presenter.padding(), Thickness::new(2.0, 1.0, 2.0, 1.0));
}
