//! Verbatim CreateCustom/FindTemplate corpus plus native ownership regressions.
use super::*;
use crate::{compat::DataTemplateSelectorImpl, controls::Ribbon};
use ferroribbon_core::model::*;
use ferroui_base::{BoxedValue, Ref, data::core::ValueTypes};
use ferroui_controls::{
    Application, Border, ContentControl, Control, StackPanel, Window, presenters::ContentPresenter,
    templates::IDataTemplate,
};
use ferroui_markup_xaml::templates::DataTemplate;
use ferroui_system::component_model::INotifyPropertyChanged;
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
fn source(name: &str) -> Value {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/custom-content.json")).unwrap();
    fixture["corpus"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap()
        .clone()
}
fn content(element: &Control) -> Option<RibbonObject> {
    crate::metadata::model_objects::from_native(Some(Control::boxed(element.to_ref())))
}
fn value(text: &str) -> Option<RibbonObject> {
    Some(RibbonObject::value(text.to_owned()))
}
fn presented(p: &ContentPresenter) -> Option<String> {
    p.content()
        .as_ref()
        .map(|v| ValueTypes::to_display_string(Some(v)))
}
struct Selector {
    template: Rc<dyn IDataTemplate>,
    trace: RefCell<Vec<String>>,
}
impl Selector {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            template: DataTemplate::new(),
            trace: RefCell::new(vec![]),
        })
    }
}
impl DataTemplateSelectorImpl for Selector {
    fn select_template_core(&self, content: Option<BoxedValue>) -> Option<Rc<dyn IDataTemplate>> {
        self.trace.borrow_mut().push(
            content
                .as_ref()
                .map(|v| ValueTypes::to_display_string(Some(v)))
                .unwrap_or("<null>".into()),
        );
        Some(self.template.clone())
    }
}
#[test]
fn source_custom_native_element_identity_detachment_recreation_and_disposal() {
    crate::group_tests::run(|| {
        for parent in ["none", "panel", "border", "presenter", "control"] {
            let element = Control::new();
            let owner: Option<Ref<Control>> = match parent {
                "panel" => {
                    let p = StackPanel::new();
                    p.children().add(element.clone());
                    Some(p.upcast())
                }
                "border" => {
                    let b = Border::new();
                    b.set_child(Some(element.clone()));
                    Some(b.upcast())
                }
                "presenter" => {
                    let p = ContentPresenter::new();
                    p.set_content(Some(Control::boxed(element.clone())));
                    Some(p.upcast())
                }
                "control" => {
                    let c = ContentControl::new();
                    c.set_content(Some(Control::boxed(element.clone())));
                    Some(c.upcast())
                }
                _ => None,
            };
            let w = owner.as_ref().map(|owner| {
                let w = Window::new();
                w.set_width(300.);
                w.set_height(200.);
                w.set_content(Some(Control::boxed(owner.clone())));
                w.show();
                w.update_layout();
                w
            });
            let f = RibbonElementFactory::new();
            let b = RibbonModelBindings::new();
            let m = RibbonCustomItemModel::new(Some("custom".into()), content(&element));
            let recreate = Rc::new(Cell::new(0));
            let r = recreate.clone();
            b.set_recreate_handler(Some(Rc::new(move || r.set(r.get() + 1))));
            let result = f.create_custom(m.clone(), b.clone());
            let empty = owner.as_ref().is_none_or(|owner| {
                if let Some(p) = owner.cast::<StackPanel>() {
                    p.children().count() == 0
                } else if let Some(border) = owner.cast::<Border>() {
                    border.child().is_none()
                } else if let Some(p) = owner.cast::<ContentPresenter>() {
                    p.content().is_none()
                } else {
                    owner.cast::<ContentControl>().unwrap().content().is_none()
                }
            });
            let initial = json!({"same":result.ptr_eq(&element),"detached":element.visual_parent().is_none()&&element.get_logical_parent().is_none(),"empty":empty});
            m.set_template_key(Some("unused".into()));
            m.set_content(content(&element));
            let unchanged = recreate.get();
            m.set_content(content(&Control::new()));
            let changed = recreate.get();
            b.dispose();
            m.set_content(value("after disposal"));
            let expected = source(&format!("element_{parent}"));
            assert_eq!(initial, expected["initial"], "{parent}");
            assert_eq!(json!(unchanged), expected["unchanged"]);
            assert_eq!(json!(changed), expected["changed"]);
            assert_eq!(json!(recreate.get()), expected["disposed"]);
            if let Some(w) = w {
                w.close();
                w.set_content(None);
            }
        }
    });
}
#[test]
fn source_custom_presenter_updates_suspension_recreation_and_disposal() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let selector = Selector::new();
        f.set_content_template_selector(Some(selector.clone()));
        let m = RibbonCustomItemModel::new(Some("vm".into()), value("first"));
        let b = RibbonModelBindings::new();
        let recreate = Rc::new(Cell::new(0));
        let r = recreate.clone();
        b.set_recreate_handler(Some(Rc::new(move || r.set(r.get() + 1))));
        let p = f
            .create_custom(m.clone(), b.clone())
            .cast::<ContentPresenter>()
            .unwrap();
        assert!(Rc::ptr_eq(
            &p.content_template().unwrap(),
            &selector.template
        ));
        let initial = json!({"content":presented(&p),"template":"selector"});
        m.set_content(value("second"));
        let changed = presented(&p);
        b.suspend();
        m.set_content(value("suspended"));
        let suspended = presented(&p);
        b.resume();
        let resumed = presented(&p);
        m.set_content(content(&Control::new()));
        let transition = json!({"recreate":recreate.get(),"old":presented(&p)});
        b.dispose();
        m.set_content(value("disposed"));
        let expected = source("presenter");
        assert_eq!(initial, expected["initial"]);
        assert_eq!(json!(changed), expected["changed"]);
        assert_eq!(json!(suspended), expected["suspended"]);
        assert_eq!(json!(resumed), expected["resumed"]);
        assert_eq!(transition, expected["transition"]);
        assert_eq!(json!(presented(&p)), expected["disposed"]);
        assert_eq!(json!(*selector.trace.borrow()), expected["selector"]);
    });
}
#[test]
fn source_custom_template_lookup_uses_local_application_then_selector_with_wrong_type_fallback() {
    crate::group_tests::run(|| {
        let app = Application::current().unwrap();
        let key = "FerroRibbon.CustomContent.TestTemplate";
        for mode in [
            "local",
            "global",
            "wrong_local",
            "wrong_both",
            "missing",
            "no_key",
            "empty_key",
        ] {
            assert!(!app.resources().contains_key(&key.into()));
            let f = RibbonElementFactory::new();
            let ribbon = Ribbon::new();
            f.set_ribbon(Some(ribbon.clone()));
            let selector = Selector::new();
            f.set_content_template_selector(Some(selector.clone()));
            let local = DataTemplate::new();
            let global = DataTemplate::new();
            if ["local", "wrong_local", "wrong_both"].contains(&mode) {
                if mode == "local" {
                    ribbon.resources().add_value(key, local.clone());
                } else {
                    ribbon
                        .resources()
                        .add_value(key, "not a template".to_owned());
                }
            }
            if ["local", "global", "wrong_local", "wrong_both"].contains(&mode) {
                if mode == "wrong_both" {
                    app.resources().add_value(key, "not a template".to_owned());
                } else {
                    app.resources().add_value(key, global.clone());
                }
            }
            let picked = f
                .find_template(
                    if mode == "no_key" {
                        None
                    } else if mode == "empty_key" {
                        Some("".into())
                    } else {
                        Some(key.into())
                    },
                    Some(Rc::new("value".to_owned())),
                )
                .unwrap();
            let local: Rc<dyn IDataTemplate> = local;
            let global: Rc<dyn IDataTemplate> = global;
            let chosen = if Rc::ptr_eq(&picked, &local) {
                "local"
            } else if Rc::ptr_eq(&picked, &global) {
                "global"
            } else {
                assert!(Rc::ptr_eq(&picked, &selector.template));
                "selector"
            };
            let expected = source(&format!("template_{mode}"));
            assert_eq!(json!(chosen), expected["chosen"], "{mode}");
            assert_eq!(
                json!(*selector.trace.borrow()),
                expected["selector"],
                "{mode}"
            );
            app.resources().remove(&key.into());
        }
    });
}
#[test]
fn source_custom_content_reentry_rereads_model_before_selecting_template() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let selector = Selector::new();
        f.set_content_template_selector(Some(selector.clone()));
        let b = RibbonModelBindings::new();
        let m = RibbonCustomItemModel::new(Some("reentry".into()), value("first"));
        let p = f
            .create_custom(m.clone(), b.clone())
            .cast::<ContentPresenter>()
            .unwrap();
        let weak = p.downgrade();
        let model = m.clone();
        let token = p.add_property_changed_handler(Rc::new(move |e| {
            if e.is_effective_value_change()
                && e.property() == ContentPresenter::content_property().as_property()
                && weak
                    .upgrade()
                    .is_some_and(|p| presented(&p).as_deref() == Some("outer"))
            {
                model.set_content(value("inner"));
            }
        }));
        m.set_content(value("outer"));
        let expected = source("presenter_reentry");
        assert_eq!(json!(presented(&p)), expected["content"]);
        assert_eq!(json!(*selector.trace.borrow()), expected["selector"]);
        assert!(Rc::ptr_eq(
            &p.content_template().unwrap(),
            &selector.template
        ));
        p.remove_property_changed_handler(token);
        b.dispose();
    });
}
#[test]
fn native_binding_owner_drop_releases_direct_custom_model_control_and_subscriptions_without_cycles()
{
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let element = Control::new();
        let weak_element = element.downgrade();
        let m = RibbonCustomItemModel::new(None, content(&element));
        let weak_model = Rc::downgrade(&m);
        let scope = RibbonModelBindings::new();
        let result = f.create_item(Some(m.clone()), scope.clone()).unwrap();
        assert!(result.ptr_eq(&element));
        assert!(RibbonElementFactory::get_model(&result).is_some());
        assert!(m.property_changed().handler_count() > 0);
        drop(result);
        drop(element);
        drop(m);
        assert!(weak_model.upgrade().is_some());
        assert!(weak_element.upgrade().is_some());
        drop(scope);
        assert!(
            weak_model.upgrade().is_none(),
            "scope is the model owner, no model -> control -> model cycle"
        );
        assert!(weak_element.upgrade().is_none());
        let m = RibbonCustomItemModel::new(None, value("data"));
        let scope = RibbonModelBindings::new();
        let _view = f.create_item(Some(m.clone()), scope.clone()).unwrap();
        assert!(m.property_changed().handler_count() > 0);
        drop(scope);
        assert_eq!(
            m.property_changed().handler_count(),
            0,
            "RAII unsubscribes from still-live models"
        );
        let calls = Rc::new(Cell::new(0));
        {
            let scope = RibbonModelBindings::new();
            let count = calls.clone();
            scope.add(Some(Rc::new(move || count.set(count.get() + 1))));
            scope.dispose();
        }
        assert_eq!(
            calls.get(),
            1,
            "explicit disposal plus Drop runs cleanup once"
        );
    });
}

#[test]
fn source_custom_explicit_content_template_preserves_visual_parent_detach_behavior() {
    crate::group_tests::run(|| {
        let element = Control::new();
        let owner = ContentControl::new();
        let template = owner.template();
        owner.set_template(template);
        owner.set_content(Some(Control::boxed(element.clone())));
        let w = Window::new();
        w.set_content(Some(Control::boxed(owner.clone())));
        w.show();
        w.update_layout();
        assert!(owner.is_set(
            ferroui_controls::primitives::TemplatedControl::template_property().as_property()
        ));
        let presenter = element
            .visual_parent()
            .unwrap()
            .cast::<ContentPresenter>()
            .unwrap();
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let m = RibbonCustomItemModel::new(None, content(&element));
        let result = f.create_custom(m, b.clone());
        let actual = json!({"name":"element_control_presenter","same":result.ptr_eq(&element),"detached":element.visual_parent().is_none()&&element.get_logical_parent().is_none(),"ownerRetains":owner.content().as_ref().and_then(Control::from_boxed).is_some_and(|p|p.ptr_eq(&element)),"presenterEmpty":presenter.content().is_none()});
        assert_eq!(actual, source("element_control_presenter"));
        b.dispose();
        w.close();
        w.set_content(None);
    });
}
