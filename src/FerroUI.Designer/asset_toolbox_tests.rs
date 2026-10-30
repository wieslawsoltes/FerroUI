use super::*;
use ferroui_base::Point;
use ferroui_base::input::{
    Key, KeyEventArgs, KeyModifiers, Pointer, PointerPointProperties, PointerType,
    PointerUpdateKind, RawInputModifiers,
};
use ferroui_controls::testing::{TestServices, UnitTestApplication};

fn custom_asset() -> DesignerAsset {
    DesignerAsset {
        id: "Project.Chart".into(),
        title: "Sales chart".into(),
        category: "Project".into(),
        description: "Monthly revenue visualization".into(),
        source: "<Border Width=\"200\" Height=\"120\" />".into(),
        icon: AssetIcon::CustomPath("M2 22V12H7V22Z M10 22V7H15V22Z M18 22V2H23V22Z".into()),
    }
}

#[test]
fn native_toolbox_filters_terms_categories_and_registered_controls() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let toolbox = AssetToolbox::new(default_designer_assets());
    let defaults = toolbox.visible_asset_ids();
    assert_eq!(defaults.len(), 20);
    toolbox.set_query("TEXT editable");
    assert_eq!(toolbox.visible_asset_ids(), ["TextBox"]);
    toolbox.set_query("");
    assert!(toolbox.set_category(Some("Shapes")));
    assert_eq!(toolbox.visible_asset_ids(), ["Rectangle", "Ellipse"]);
    assert!(!toolbox.set_category(Some("Missing category")));
    toolbox.register(custom_asset()).unwrap();
    assert_eq!(
        toolbox.visible_asset_ids(),
        ["Rectangle", "Ellipse"],
        "Registration preserves the filter"
    );
    assert!(toolbox.set_category(Some("Project")));
    toolbox.set_query("REVENUE monthly");
    assert_eq!(toolbox.visible_asset_ids(), ["Project.Chart"]);
    toolbox.set_query("missing");
    assert_eq!(
        toolbox.count.text().as_deref(),
        Some("No matching controls")
    );
    toolbox.set_query("");
    toolbox.set_category(None);
    assert_eq!(toolbox.visible_asset_ids().len(), defaults.len() + 1);
}

#[test]
fn registering_assets_replaces_by_id_and_validates_custom_vectors_without_mutation() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let toolbox = AssetToolbox::new(vec![custom_asset()]);
    let mut replacement = custom_asset();
    replacement.title = "Revenue chart".into();
    // A category label may equal the UI's "all" label without losing its identity.
    replacement.category = "All controls".into();
    toolbox.register(replacement.clone()).unwrap();
    assert_eq!(toolbox.assets(), [replacement.clone()]);
    assert!(toolbox.set_category(Some("All controls")));
    assert_eq!(toolbox.visible_asset_ids(), ["Project.Chart"]);
    replacement.icon = AssetIcon::CustomPath("this is not a vector".into());
    assert!(toolbox.register(replacement).is_err());
    assert_eq!(toolbox.assets().len(), 1);
    assert_eq!(toolbox.assets()[0].title, "Revenue chart");
}

#[test]
fn native_asset_buttons_accept_keyboard_activation_and_handled_pointer_drags() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let toolbox = AssetToolbox::new(vec![custom_asset()]);
    let activations = Rc::new(RefCell::new(Vec::new()));
    let observed = activations.clone();
    toolbox.on_activate(move |asset| observed.borrow_mut().push(asset.id.clone()));
    let drags = Rc::new(RefCell::new(Vec::new()));
    let observed = drags.clone();
    toolbox.on_begin_drag(move |asset, _| observed.borrow_mut().push(asset.source.clone()));
    let tile = toolbox.buttons.borrow()[0].clone();
    let mut enter = KeyEventArgs::new();
    enter.key = Key::Enter;
    enter.set_routed_event(Some(InputElement::key_down_event()));
    tile.raise_event(&enter);
    assert!(enter.handled());
    assert_eq!(*activations.borrow(), ["Project.Chart"]);
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let press = PointerPressedEventArgs::new(
        tile.to_ref(),
        pointer,
        &tile,
        Point::default(),
        1,
        PointerPointProperties::new(
            RawInputModifiers::LEFT_MOUSE_BUTTON,
            PointerUpdateKind::LeftButtonPressed,
        ),
        KeyModifiers::NONE,
        1,
    );
    tile.raise_event(&press);
    assert!(press.handled(), "Button handles its native pointer press");
    assert_eq!(
        *drags.borrow(),
        [custom_asset().source],
        "Toolbox must observe the handled event"
    );
    assert_eq!(
        activations.borrow().len(),
        1,
        "Dragging must not also activate the asset"
    );
}

#[test]
fn asset_list_view_preserves_filtering_and_native_insertion() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let toolbox = AssetToolbox::new(default_designer_assets());
    toolbox.register(custom_asset()).unwrap();
    assert!(toolbox.set_category(Some("Project")));
    toolbox.set_query("monthly revenue");
    let catalog = toolbox.assets();
    let inserted = Rc::new(RefCell::new(Vec::new()));
    let observed = inserted.clone();
    toolbox.on_activate(move |asset| observed.borrow_mut().push(asset.source.clone()));
    toolbox.set_view_mode(AssetViewMode::List);
    assert_eq!(toolbox.view_mode(), AssetViewMode::List);
    assert_eq!(toolbox.query(), "monthly revenue");
    assert_eq!(toolbox.selected_category().as_deref(), Some("Project"));
    assert_eq!(toolbox.visible_asset_ids(), ["Project.Chart"]);
    let mut enter = KeyEventArgs::new();
    enter.key = Key::Enter;
    enter.set_routed_event(Some(InputElement::key_down_event()));
    toolbox.buttons.borrow()[0].raise_event(&enter);
    assert!(enter.handled());
    assert_eq!(*inserted.borrow(), [custom_asset().source]);
    toolbox.set_view_mode(AssetViewMode::Grid);
    assert_eq!(toolbox.visible_asset_ids(), ["Project.Chart"]);
    assert_eq!(toolbox.assets(), catalog);
}

#[test]
fn default_asset_snippets_load_with_the_real_preview_loader() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    for asset in default_designer_assets() {
        let source = format!(
            "<Canvas xmlns=\"https://github.com/ferroui\">{}</Canvas>",
            asset.source
        );
        let session =
            crate::DesignerSession::new(source, ferroui_controls::ContentControl::new()).unwrap();
        assert!(
            matches!(session.preview_status(), crate::PreviewStatus::Ready { .. }),
            "{}: {:?}",
            asset.id,
            session.preview_status()
        );
    }
}
