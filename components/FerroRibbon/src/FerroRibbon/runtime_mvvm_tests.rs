//! Seven pinned shared MvvmRegressionTests scenarios, real native factory/bindings. Copyright
//! Wiesław Šoltés, MIT.
use crate::{
    controls::*,
    runtime_sample::{self as sample, Mounted, RuntimeHost},
};
use ferroribbon_core::{commands::RibbonCommandCatalog, model::*, theming::RibbonColor};
use ferroui_base::{data::core::ValueTypes, media::Color};
use ferroui_controls::{Control, Flyout, StackPanel, TextBlock, automation::AutomationProperties};
use std::{cell::RefCell, rc::Rc};

fn create_model(
    catalog: Option<Rc<RibbonCommandCatalog>>,
) -> (Rc<RibbonModel>, Rc<RibbonGroupModel>) {
    let model = RibbonModel::new();
    model.set_command_catalog(catalog);
    let group = RibbonGroupModel::new(Some("g".into()), Some("Group".into()));
    let tab = RibbonTabModel::new(Some("home".into()), Some("Home".into()));
    tab.groups().add(Some(group.clone()));
    model.tabs().add(Some(tab));
    (model, group)
}
fn mount(model: Rc<RibbonModel>, width: f64) -> Mounted {
    let ribbon = Ribbon::new();
    ribbon.set_model(Some(model));
    Mounted::with_ribbon(ribbon, width)
}
#[test]
fn command_items_follow_model_enabled_state_both_ways() {
    crate::group_tests::run(|| {
        let catalog = RibbonCommandCatalog::new();
        catalog.register_action(
            Some("save".into()),
            Some("Save".into()),
            Rc::new(|_| {}),
            None,
            Default::default(),
        );
        let (model, group) = create_model(Some(catalog));
        let save = RibbonButtonModel::new(Some("save".into()), Some("Save".into()));
        save.set_command_id(Some("save".into()));
        group.items().add(Some(save));
        let m = mount(model.clone(), 1200.);
        let button = sample::item::<RibbonButton>(&m.ribbon, "save");
        model.set_command_enabled(Some("save".into()), false);
        m.settle();
        assert!(!button.is_enabled());
        model.set_command_enabled(Some("save".into()), true);
        m.settle();
        assert!(button.is_enabled());
    });
}
#[test]
fn gallery_and_color_picker_execute_catalog_command_once_with_model_parameters() {
    crate::group_tests::run(|| {
        let catalog = RibbonCommandCatalog::new();
        let gallery_calls = Rc::new(RefCell::new(Vec::new()));
        let color_calls = Rc::new(RefCell::new(Vec::new()));
        let calls = gallery_calls.clone();
        catalog.register_action(
            Some("style".into()),
            Some("Style".into()),
            Rc::new(move |p| {
                calls
                    .borrow_mut()
                    .push(p.and_then(|v| v.downcast_ref::<String>()).cloned())
            }),
            None,
            Default::default(),
        );
        let calls = color_calls.clone();
        catalog.register_action(
            Some("fill".into()),
            Some("Fill".into()),
            Rc::new(move |p| {
                calls
                    .borrow_mut()
                    .push(p.and_then(|v| v.downcast_ref::<RibbonColor>()).copied())
            }),
            None,
            Default::default(),
        );
        let (model, group) = create_model(Some(catalog));
        let gallery = RibbonGalleryModel::new(Some("styles".into()), Some("Styles".into()));
        gallery.set_command_id(Some("style".into()));
        let item = RibbonGalleryItemModel::with_options(
            Some("normal".into()),
            Some("Normal".into()),
            None,
            None,
            Some(RibbonObject::value("normal-style".to_owned())),
        );
        gallery.items().add(Some(item.clone()));
        group.items().add(Some(gallery));
        let color = RibbonColorPickerModel::new(Some("fillColor".into()), Some("Fill".into()));
        color.set_command_id(Some("fill".into()));
        group.items().add(Some(color));
        let m = mount(model, 1400.);
        let payload =
            crate::metadata::model_objects::to_native(Some(RibbonObject::reference(item))).unwrap();
        sample::item::<RibbonGallery>(&m.ribbon, "styles").pick(payload);
        sample::item::<RibbonColorPicker>(&m.ribbon, "fillColor")
            .palette()
            .select(Some(Color::from_rgb(0, 0, 255)));
        m.settle();
        assert_eq!(*gallery_calls.borrow(), [Some("normal-style".into())]);
        assert_eq!(
            *color_calls.borrow(),
            [Some(RibbonColor::parse(Some(&"#0000FF".into())).unwrap())]
        );
    });
}
#[test]
fn collection_changes_keep_existing_elements_and_relink_quick_access() {
    crate::group_tests::run(|| {
        let (model, group) = create_model(None);
        let cut = RibbonButtonModel::new(Some("cut".into()), Some("Cut".into()));
        cut.set_icon(Some(RibbonIcon::glyph(Some(RibbonIcons::Cut.into()))));
        group.items().add(Some(cut.clone()));
        model.quick_access_items().add(Some(cut.clone()));
        let m = mount(model.clone(), 1200.);
        let tab = m.ribbon.find_tab("home").unwrap();
        let cut_element = sample::item::<RibbonButton>(&m.ribbon, "cut");
        let copy = RibbonButtonModel::new(Some("copy".into()), Some("Copy".into()));
        copy.set_icon(Some(RibbonIcon::glyph(Some(RibbonIcons::Copy.into()))));
        group.items().add(Some(copy));
        model.tabs().add(Some(RibbonTabModel::new(
            Some("view".into()),
            Some("View".into()),
        )));
        m.settle();
        assert!(tab.ptr_eq(&m.ribbon.find_tab("home").unwrap()));
        assert!(cut_element.ptr_eq(&sample::item::<RibbonButton>(&m.ribbon, "cut")));
        let other = RibbonGroupModel::new(Some("g2".into()), Some("Other".into()));
        model
            .tabs()
            .get(0)
            .unwrap()
            .groups()
            .add(Some(other.clone()));
        group
            .items()
            .remove(&Some(cut.clone() as Rc<dyn RibbonItemModelImpl>));
        other.items().add(Some(cut));
        m.settle();
        let new_element = sample::item::<RibbonButton>(&m.ribbon, "cut");
        assert!(!new_element.ptr_eq(&cut_element));
        let q = m.ribbon.quick_access_tool_bar().unwrap();
        assert_eq!(q.items().count(), 1);
        let copy = q.items().get(0).unwrap();
        assert!(
            RibbonItemHelper::get_source_item(&copy)
                .unwrap()
                .ptr_eq(&new_element)
        );
    });
}
#[test]
fn menu_rebuilds_without_reparenting_errors() {
    crate::group_tests::run(|| {
        let (model, group) = create_model(None);
        let content = TextBlock::new();
        content.set_text("Custom".into());
        let dropdown = RibbonDropDownButtonModel::new(Some("more".into()), Some("More".into()));
        dropdown.set_drop_down_content(crate::metadata::model_objects::from_native(Some(
            Control::boxed(content.clone()),
        )));
        dropdown.menu_items().add(Some(RibbonMenuItemModel::new(
            Some("a".into()),
            Some("A".into()),
        )));
        group.items().add(Some(dropdown.clone()));
        let m = mount(model, 1200.);
        for (id, label) in [("b", "B"), ("c", "C")] {
            dropdown.menu_items().add(Some(RibbonMenuItemModel::new(
                Some(id.into()),
                Some(label.into()),
            )));
        }
        m.settle();
        let flyout = sample::item::<RibbonDropDownButton>(&m.ribbon, "more")
            .flyout()
            .unwrap()
            .cast::<Flyout>()
            .unwrap();
        let panel = ValueTypes::as_object(&*flyout.content().unwrap())
            .unwrap()
            .cast::<StackPanel>()
            .unwrap();
        assert!(
            panel
                .children()
                .snapshot()
                .iter()
                .any(|c| c.ptr_eq(&content))
        );
    });
}
#[test]
fn backstage_items_added_later_replace_and_restore_xaml_backstage() {
    crate::group_tests::run(|| {
        let (model, _) = create_model(None);
        let declared = RibbonBackstage::new();
        let ribbon = Ribbon::new();
        ribbon.set_backstage(Some(declared.clone()));
        ribbon.set_model(Some(model.clone()));
        let m = Mounted::with_ribbon(ribbon, 1200.);
        assert!(m.ribbon.backstage().unwrap().ptr_eq(&declared));
        let info = RibbonBackstageItemModel::new(Some("info".into()), Some("Info".into()));
        info.set_content(Some(RibbonObject::value("Info page".to_owned())));
        model.backstage().items().add(Some(info));
        m.settle();
        assert!(!m.ribbon.backstage().unwrap().ptr_eq(&declared));
        m.ribbon.set_model(None);
        m.settle();
        assert!(m.ribbon.backstage().unwrap().ptr_eq(&declared));
    });
}
#[test]
fn backstage_item_changes_refresh_navigation() {
    crate::group_tests::run(|| {
        let backstage = RibbonBackstage::new();
        let item = RibbonBackstageItem::new();
        item.set_header(Some("Info".into()));
        item.set_content(Some(Rc::new("Page".to_owned())));
        backstage.items().add(Some(item.clone()));
        let _host = RuntimeHost::new(backstage.upcast(), Some(800.));
        item.set_header(Some("Information".into()));
        item.set_is_enabled(false);
        let button = item.navigation_button().unwrap();
        assert_eq!(
            AutomationProperties::get_name(&button).as_deref(),
            Some("Information")
        );
        assert!(!button.is_enabled());
    });
}
#[test]
fn menu_bar_rebuild_keeps_entries() {
    crate::group_tests::run(|| {
        let file = RibbonMenuBarItem::new();
        file.set_header(Some("File".into()));
        let open = crate::compat::MenuFlyoutItem::new();
        open.set_text(Some("Open".into()));
        file.items().add(Some(open.upcast()));
        let bar = RibbonMenuBar::new();
        bar.items().add(Some(file.clone()));
        let host = RuntimeHost::new(bar.clone().upcast(), Some(600.));
        let edit = RibbonMenuBarItem::new();
        edit.set_header(Some("Edit".into()));
        bar.items().add(Some(edit));
        let save = crate::compat::MenuFlyoutItem::new();
        save.set_text(Some("Save".into()));
        file.items().add(Some(save.upcast()));
        host.settle();
        assert_eq!(file.items().count(), 2);
        assert!(bar.open_menu(Some("File")));
        host.settle();
        assert!(bar.is_menu_open());
    });
}
