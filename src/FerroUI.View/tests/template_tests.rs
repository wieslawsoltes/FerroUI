//! Hosting elements in other trees of controls (`ComponentHost`) and data
//! templates whose items are components (`template`).

use crate::generated::builders::*;
use crate::reactive::{on_cleanup, Owner};
use crate::{rerender, template, use_data_context, use_effect_with, ComponentHost, IntoElement};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::{BoxedValue, FerroLocator, LocatorExtensions, Ref};
use ferroui_controls::testing::{create_split_view_list_box_theme, TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Control, ItemsSource, ListBox, TextBlock, Window};
use std::cell::Cell;
use std::rc::Rc;

/// The test application with windows, a theme that has the list box, its
/// items and its scroll viewer, and text that lays out.
struct App {
    _app: UnitTestApplicationScope,
    _text: TextTestScope,
}

fn app() -> App {
    let text = TextTestScope::new();
    let render_interface =
        FerroLocator::current().get_service::<dyn IPlatformRenderInterface>().expect("the text services have a render interface");
    let app = UnitTestApplication::start(
        TestServices::styled_window().with_theme(create_split_view_list_box_theme).with_render_interface(render_interface),
    );
    App { _app: app, _text: text }
}

fn window_with(content: &Ref<Control>, width: f64, height: f64) -> Ref<Window> {
    let window = Window::new();
    window.set_width(width);
    window.set_height(height);
    window.set_content(Some(Control::boxed(content)));
    window.show();
    window.layout_manager().execute_layout_pass();
    window
}

fn texts_in(control: &Ref<Control>) -> Vec<String> {
    control.get_visual_descendants().filter_map(|visual| visual.cast::<TextBlock>()).filter_map(|text| text.text()).collect()
}

#[test]
fn a_host_shows_its_component_with_the_data_context_and_ends_it_when_detached() {
    let _app = app();
    let cleaned = Rc::new(Cell::new(0));
    let counted = cleaned.clone();
    let host = ComponentHost::with_view(move || {
        let counted = counted.clone();
        rerender(move || {
            let counted = counted.clone();
            use_effect_with((), move |_| on_cleanup(move || counted.set(counted.get() + 1)));
            let name = use_data_context::<String>().unwrap_or_else(|| String::from("nobody"));
            text_block().text(format!("Hello, {name}"))
        })
    });
    let control: Ref<Control> = host.clone().upcast();
    let window = window_with(&control, 300.0, 200.0);
    assert_eq!(texts_in(&control), vec!["Hello, nobody"]);

    window.set_data_context(Some(Rc::new(String::from("Ada")) as BoxedValue));
    assert_eq!(texts_in(&control), vec!["Hello, Ada"], "the component follows the data context of the host");

    window.set_content(None);
    assert_eq!(cleaned.get(), 1, "detaching the host ends its component");
    assert!(host.child().is_none());

    window.set_content(Some(Control::boxed(&host)));
    window.layout_manager().execute_layout_pass();
    assert_eq!(texts_in(&control), vec!["Hello, Ada"], "attaching it again mounts it again");
    window.close();
}

#[test]
fn a_registered_component_is_shown_by_name() {
    let _app = app();
    crate::register_component("Greeting", || text_block().text("registered"));
    let host = ComponentHost::new();
    host.set_component(Some(String::from("Greeting")));
    let control: Ref<Control> = host.upcast();
    let window = window_with(&control, 300.0, 200.0);
    assert_eq!(texts_in(&control), vec!["registered"]);
    window.close();
}

#[test]
fn a_virtualized_list_of_ten_thousand_items_recycles_its_component_items() {
    let _app = app();
    let mounts = Rc::new(Cell::new(0));
    let runs = Rc::new(Cell::new(0));
    let owner = Owner::new_root();
    let (counted_mounts, counted_runs) = (mounts.clone(), runs.clone());
    let list = owner.with(|| {
        let items = (0..10_000).map(|n| Some(Rc::new(n) as BoxedValue));
        list_box()
            .items_source(ItemsSource::from_items(items))
            .item_template(template(move |n: i32| {
                counted_runs.set(counted_runs.get() + 1);
                let mounts = counted_mounts.clone();
                use_effect_with((), move |_| mounts.set(mounts.get() + 1));
                text_block().text(format!("Item {n}"))
            }))
            .into_element()
            .mount()
    });
    let window = window_with(&list, 200.0, 300.0);
    let list_box = list.cast::<ListBox>().unwrap();

    let realized = list_box.get_realized_containers().len();
    assert!(realized > 0 && realized < 100, "only the visible items are realized ({realized})");
    assert!(texts_in(&list).contains(&String::from("Item 0")));
    let mounted_first = mounts.get();
    assert!(mounted_first <= realized + 2, "one component per realized container ({mounted_first} for {realized})");

    for index in [2_000, 5_000, 9_999] {
        list_box.scroll_into_view(index);
        window.layout_manager().execute_layout_pass();
        assert!(texts_in(&list).contains(&format!("Item {index}")), "item {index} is shown after scrolling");
    }
    assert!(
        mounts.get() <= realized + 2,
        "scrolling takes hosts from the pool and runs their components again instead of mounting new ones ({} mounts for {realized} containers)",
        mounts.get()
    );
    assert!(runs.get() > 3 * realized, "the recycled hosts ran with the items of each screen ({} runs)", runs.get());
    window.close();
    owner.dispose();
}

#[test]
fn xaml_hosts_a_registered_component() {
    let _app = app();
    crate::register_component("XamlGreeting", || {
        rerender(|| {
            let name = use_data_context::<String>().unwrap_or_default();
            text_block().text(format!("Component in XAML for {name}"))
        })
    });
    ferroui_markup_xaml::register_types();
    crate::register_types();
    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::register();
    let loaded = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
        r#"<UserControl xmlns="https://github.com/ferroui" xmlns:view="using:FerroUI.View">
             <view:ComponentHost Name="host" Component="XamlGreeting" />
           </UserControl>"#,
        None,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let view = Control::from_boxed(&loaded).expect("a control");
    let window = window_with(&view, 300.0, 200.0);
    window.set_data_context(Some(Rc::new(String::from("Grace")) as BoxedValue));
    assert!(view.get_control::<ComponentHost>("host").child().is_some());
    assert_eq!(texts_in(&view), vec!["Component in XAML for Grace"]);
    window.close();
}

#[cfg(feature = "xaml")]
#[test]
fn a_component_hosts_xaml_and_keeps_it_across_runs() {
    use crate::reactive::signal;
    let _app = app();
    let owner = Owner::new_root();
    let (count, panel) = owner.with(|| {
        let count = signal(0);
        let panel = stack_panel()
            .child(rerender(move || {
                let form = crate::use_xaml(
                    r#"<Border xmlns="https://github.com/ferroui"><TextBlock Name="label" Text="from XAML" /></Border>"#,
                );
                vec![crate::Element::Existing(form), text_block().text(format!("runs {}", count.get())).into_element()]
            }))
            .into_element()
            .mount();
        (count, panel)
    });
    let window = window_with(&panel, 300.0, 200.0);
    let border = panel.get_visual_descendants().find_map(|visual| visual.cast::<ferroui_controls::Border>()).expect("the XAML");
    assert_eq!(border.get_control::<TextBlock>("label").text().as_deref(), Some("from XAML"));
    count.set(1);
    let again = panel.get_visual_descendants().find_map(|visual| visual.cast::<ferroui_controls::Border>()).expect("the XAML");
    assert!(border.ptr_eq(&again), "the control of the XAML is kept when the component runs again");
    assert!(texts_in(&panel).contains(&String::from("runs 1")));
    window.close();
    owner.dispose();
}
