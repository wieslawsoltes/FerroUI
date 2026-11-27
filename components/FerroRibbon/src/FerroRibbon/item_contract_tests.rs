use crate::controls::{IRibbonItem, IRibbonItemEditable, RibbonItemLayout};
use crate::primitives::RibbonMetrics;
use crate::tests::run;
use ferroribbon_core::{RibbonGroupState, RibbonItemSize};
use ferroui_base::metadata::{MarkupType, MarkupTyped, from_markup_value, into_markup_value};
use serde_json::{Value, json};
use std::sync::Arc;
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/item-contracts.json")).unwrap()
}
fn shape(meta: &MarkupType) -> Value {
    let mut properties = meta
        .properties
        .iter()
        .map(|p| json!({"name":p.name,"read":p.get.is_some(),"write":p.set.is_some()}))
        .collect::<Vec<_>>();
    properties.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    let mut methods = meta
        .methods
        .iter()
        .filter(|m| !m.is_static)
        .map(|m| json!({"name":m.name,"parameters":m.parameters.len()}))
        .collect::<Vec<_>>();
    methods.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    let mut interfaces = meta
        .interfaces
        .iter()
        .map(|i| MarkupType::find_by_handle(i().id()).unwrap().name)
        .collect::<Vec<_>>();
    interfaces.sort();
    json!({"properties":properties,"methods":methods,"interfaces":interfaces})
}
#[test]
fn public_interface_declared_members_and_layout_constructors_match_unchanged_source() {
    run(|| {
        let fixture = corpus();
        assert_eq!(
            shape(<dyn IRibbonItem as MarkupTyped>::MARKUP),
            fixture["item"]
        );
        assert_eq!(
            shape(<dyn IRibbonItemEditable as MarkupTyped>::MARKUP),
            fixture["editable"]
        );
        let metrics = RibbonMetrics::touch();
        let mut values = Vec::new();
        for ctor in RibbonItemLayout::MARKUP.constructors {
            let mut args = vec![
                into_markup_value(RibbonItemSize::from_raw(-7)),
                into_markup_value(Some(metrics.clone())),
                into_markup_value(true),
                into_markup_value(Some(false)),
                into_markup_value(RibbonGroupState::from_raw(99)),
            ];
            args.truncate(ctor.parameters.len());
            let value =
                from_markup_value::<RibbonItemLayout>(&(ctor.invoke)(&args).unwrap()).unwrap();
            values.push(json!({"size":value.size.raw(),"hasMetrics":value.metrics.is_some(),"sameMetrics":value.metrics.as_ref().is_some_and(|m|Arc::ptr_eq(m,&metrics)),"simplified":value.is_simplified,"showLabel":value.show_label,"group":value.group_state.raw()}));
        }
        assert_eq!(json!(values), fixture["layouts"]);
    });
}

#[test]
fn keytip_singleton_callback_query_laziness_and_simplified_enum_match_source() {
    run(|| {
        use crate::controls::{RibbonKeyTipResult, RibbonSimplifiedLabel};
        use ferroribbon_core::model::RibbonItemsSource;
        use std::{cell::Cell, rc::Rc};
        let fixture = corpus();
        let calls = Rc::new(Cell::new(0));
        let capture = calls.clone();
        let scope = RibbonKeyTipResult::scope(Some(Rc::new(move || {
            let capture = capture.clone();
            RibbonItemsSource::new(move || {
                capture.set(capture.get() + 1);
                Box::new(std::iter::empty())
            })
        })));
        let before = calls.get();
        let query = scope.next_scope().unwrap()();
        let after = calls.get();
        assert_eq!(query.iter().count(), 0);
        assert_eq!(
            json!({"closeSame":Rc::ptr_eq(&RibbonKeyTipResult::close(),&RibbonKeyTipResult::close()),"newDistinct":!Rc::ptr_eq(&RibbonKeyTipResult::new(),&RibbonKeyTipResult::close()),"before":before,"after":after,"enumerated":calls.get()}),
            fixture["keyTip"]
        );
        let meta = crate::metadata::simplified_label::RibbonSimplifiedLabelMetadata::MARKUP;
        let values=meta.enum_members.iter().map(|m| {
            let value=m.get(meta).unwrap();
            json!({"name":m.name,"value":value.downcast_ref::<RibbonSimplifiedLabel>().unwrap().raw()})
        }).collect::<Vec<_>>();
        assert_eq!(json!(values), fixture["enumValues"]);
    });
}
