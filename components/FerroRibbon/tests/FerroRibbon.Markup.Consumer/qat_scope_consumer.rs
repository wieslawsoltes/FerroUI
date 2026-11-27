//! Downstream virtual factory records actual binding disposal, not map internals.
use ferroribbon::mvvm::RibbonItemModelReference;
use ferroribbon::mvvm::{
    RibbonElementFactory, RibbonElementFactoryImpl, RibbonElementFactoryImplExt,
    RibbonModelBindings,
};
use ferroribbon_core::model::RibbonNodeModelExt;
use ferroui_base::{FerroObjectImpl, Ref, ferro_class, ferro_class_info, instantiate};
use ferroui_controls::Control;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
type OnDispose = Rc<dyn Fn(String)>;
#[repr(C)]
pub struct QatScopeFactory {
    base: RibbonElementFactory,
    observer: RefCell<Option<OnDispose>>,
    scopes: RefCell<Vec<Weak<RibbonModelBindings>>>,
}
ferro_class!(QatScopeFactory:RibbonElementFactory);
ferro_class_info!(QatScopeFactory{new:QatScopeFactory::new,markup:{namespace:"FerroRibbon.Consumer"}});
impl FerroObjectImpl for QatScopeFactory {}
impl QatScopeFactory {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonElementFactory::construct(),
            observer: RefCell::default(),
            scopes: RefCell::default(),
        })
    }
    pub fn set_observer(&self, observer: OnDispose) {
        self.observer.replace(Some(observer));
    }
    pub fn live_scopes(&self) -> usize {
        self.scopes
            .borrow()
            .iter()
            .filter(|s| s.upgrade().is_some())
            .count()
    }
}
impl RibbonElementFactoryImpl for QatScopeFactory {
    fn create_item(
        this: &Self,
        model: Option<RibbonItemModelReference>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Option<Ref<Control>> {
        if let Some(model) = &model {
            let id = model.label().unwrap().to_utf8().unwrap();
            if let Some(observer) = this.observer.borrow().clone() {
                bindings.add(Some(Rc::new(move || observer(id.clone()))));
            }
            this.scopes.borrow_mut().push(Rc::downgrade(&bindings));
        }
        Self::parent_create_item(this, model, bindings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroribbon::controls::Ribbon;
    use ferroribbon_core::model::*;
    use ferroui_base::{TypeInfo, data::core::ValueTypes, threading::Dispatcher};
    use ferroui_controls::{StackPanel, Window};
    use serde_json::json;
    #[test]
    fn both_loaders_preserve_source_qat_scope_disposal_order_reentry_identity_and_release() {
        crate::tests::run(|| {
            let _theme = crate::tests::NativeThemeScope::new();
            TypeInfo::register_namespaces(&[(
                "ferroribbon_markup_consumer::qat_scope_consumer",
                "FerroRibbon.Consumer",
            )]);
            TypeInfo::register_all(&[QatScopeFactory::TYPE]);
            let cases: serde_json::Value =
                serde_json::from_str(include_str!("../fixtures/qat-scope-order.json")).unwrap();
            for runtime in [false, true] {
                for case in cases.as_array().unwrap() {
                    let value = if runtime {
                        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                            include_str!("QatScopes.xaml"),
                            Some(&crate::CONSUMER_ASSEMBLY),
                        )
                        .unwrap()
                    } else {
                        crate::compiled_xaml::try_load(
                            None,
                            "ferres://ferroribbon_markup_consumer/QatScopes.xaml",
                        )
                        .unwrap()
                        .unwrap()
                    };
                    let panel = ValueTypes::as_object(&*value)
                        .unwrap()
                        .cast::<StackPanel>()
                        .unwrap();
                    let ribbon = panel.children().get(0).cast::<Ribbon>().unwrap();
                    let factory = ribbon.item_factory().cast::<QatScopeFactory>().unwrap();
                    let trace = Rc::new(RefCell::new(Vec::new()));
                    let log = trace.clone();
                    let weak = ribbon.downgrade();
                    let end = case["end"].as_str().unwrap().to_string();
                    let reentered = Rc::new(std::cell::Cell::new(false));
                    let entered = reentered.clone();
                    let first_new = if case["multipleRemovals"] == true {
                        "e"
                    } else {
                        "d"
                    };
                    factory.set_observer(Rc::new(move |id| {
                        log.borrow_mut().push(id.clone());
                        if end == "reentrant" && id == first_new && !entered.replace(true) {
                            weak.upgrade().unwrap().set_model(None);
                        }
                    }));
                    let model = RibbonModel::new();
                    for id in ["a", "b", "c"] {
                        model.quick_access_items().add(Some(RibbonButtonModel::new(
                            Some(
                                if case["duplicateIds"] == true {
                                    "duplicate"
                                } else {
                                    id
                                }
                                .into(),
                            ),
                            Some(id.into()),
                        )));
                    }
                    ribbon.set_model(Some(model.clone()));
                    let window = Window::new();
                    window.set_content(Some(Control::boxed(panel.clone())));
                    window.show();
                    window.update_layout();
                    Dispatcher::ui_thread().run_jobs(None);
                    let items = ribbon.quick_access_tool_bar().unwrap().items();
                    let b = items.get(1);
                    let c = items.get(2);
                    if case["removeFromModel"] == true {
                        model.quick_access_items().remove_at(0);
                    } else {
                        items.remove_at(0);
                    }
                    if case["multipleRemovals"] == true {
                        if case["removeFromModel"] == true {
                            model.quick_access_items().remove_at(0);
                        } else {
                            items.remove_at(0);
                        }
                    }
                    let additions = if case["multipleRemovals"] == true {
                        vec!["d", "e"]
                    } else {
                        vec!["d"]
                    };
                    for id in additions {
                        model.quick_access_items().add(Some(RibbonButtonModel::new(
                            Some(
                                if case["duplicateIds"] == true {
                                    "duplicate"
                                } else {
                                    id
                                }
                                .into(),
                            ),
                            Some(id.into()),
                        )));
                    }
                    assert!(
                        if case["multipleRemovals"] == true {
                            items.get(0) == c
                        } else {
                            items.get(0) == b && items.get(1) == c
                        },
                        "surviving controls keep identity"
                    );
                    match case["end"].as_str().unwrap() {
                        "model" => ribbon.set_model(None),
                        "factory" => ribbon.set_item_factory(Some(RibbonElementFactory::new())),
                        _ => items.clear(),
                    }
                    assert_eq!(json!(*trace.borrow()), case["trace"], "{case}");
                    assert_eq!(json!(reentered.get()), case["reentered"]);
                    assert_eq!(json!(ribbon.model().is_some()), case["modelPresent"]);
                    assert_eq!(json!(items.count()), case["qatCount"]);
                    ribbon.set_model(None);
                    Dispatcher::ui_thread().run_jobs(None);
                    assert_eq!(
                        factory.live_scopes(),
                        0,
                        "disposed children release with no owning registry/cycle"
                    );
                    window.close();
                    window.set_content(None);
                }
            }
        });
    }
}
