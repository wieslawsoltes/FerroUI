use crate::primitives::RibbonMetrics;
use ferroribbon_core::RibbonDensity;
use ferroui_base::{
    BoxedValue,
    metadata::{MarkupType, MarkupTyped},
};
use std::{rc::Rc, sync::Arc};
fn corpus() -> serde_json::Value {
    serde_json::from_str(include_str!("../../tests/fixtures/metrics.json")).unwrap()
}
fn values(metrics: Arc<RibbonMetrics>) -> serde_json::Value {
    let instance: BoxedValue = Rc::new(metrics);
    let mut result = serde_json::Map::new();
    for property in RibbonMetrics::MARKUP.properties {
        let value = property.get.unwrap()(&[Some(instance.clone())])
            .unwrap()
            .unwrap();
        result.insert(
            property.name.into(),
            format!("{:016x}", value.downcast_ref::<f64>().unwrap().to_bits()).into(),
        );
    }
    result.into()
}
#[test]
fn metrics_presets_defaults_invalid_density_and_record_equality_match_source() {
    let source = corpus();
    for case in source["densities"].as_array().unwrap() {
        let density = RibbonDensity::from_raw(case["density"].as_i64().unwrap() as i32);
        let metrics = RibbonMetrics::for_density(density);
        assert_eq!(values(metrics.clone()), case["values"]);
        assert_eq!(
            Arc::ptr_eq(&metrics, &RibbonMetrics::for_density(density)),
            case["stable"].as_bool().unwrap()
        );
        assert_eq!(
            Arc::ptr_eq(&metrics, &RibbonMetrics::comfortable()),
            case["comfortable"].as_bool().unwrap()
        );
    }
    assert_eq!(
        RibbonMetrics::new() == RibbonMetrics::comfortable(),
        source["equalNew"].as_bool().unwrap()
    );
    assert_eq!(
        Arc::ptr_eq(&RibbonMetrics::new(), &RibbonMetrics::comfortable()),
        source["newIdentity"].as_bool().unwrap()
    );
    let custom = RibbonMetrics::comfortable().with_values(|v| {
        v.row_height = f64::NAN;
        v.item_padding = -0.;
        v.group_content_height = f64::INFINITY;
    });
    let mut expected = source["custom"].clone();
    // Runtime NaN sign/payload is not an equality or metric contract.
    expected["RowHeight"] = format!("{:016x}", f64::NAN.to_bits()).into();
    assert_eq!(values(custom.clone()), expected);
    let copy = custom.with_values(|_| {});
    assert_eq!(copy == custom, source["copyEqual"].as_bool().unwrap());
    assert_eq!(
        Arc::ptr_eq(&copy, &custom),
        source["copyIdentity"].as_bool().unwrap()
    );
    let zero = custom.with_values(|v| v.row_height = -0.);
    assert_eq!(
        zero == zero.with_values(|v| v.row_height = 0.),
        source["zeroEqual"].as_bool().unwrap()
    );
    let from_other_thread = std::thread::spawn(RibbonMetrics::comfortable)
        .join()
        .unwrap();
    assert!(Arc::ptr_eq(
        &from_other_thread,
        &RibbonMetrics::comfortable()
    ));
}
#[test]
fn foreign_enum_metadata_preserves_core_values_unknown_numbers_and_nullables() {
    crate::register_types();
    let source = corpus();
    for enumeration in source["enums"].as_array().unwrap() {
        let name = enumeration["name"].as_str().unwrap();
        let metadata = MarkupType::find("FerroRibbon", name).unwrap();
        assert!(!metadata.is_flags);
        assert_eq!(
            metadata.enum_members.len(),
            enumeration["members"].as_array().unwrap().len()
        );
        for member in enumeration["members"].as_array().unwrap() {
            let native = metadata
                .find_enum_member(member["name"].as_str().unwrap(), false)
                .unwrap();
            assert_eq!(native.value, member["value"].as_i64().unwrap());
            assert_eq!(
                metadata.enum_to_value.unwrap()(native.get(metadata).unwrap().as_ref()),
                Some(native.value)
            );
        }
        for number in [i32::MIN as i64, -1, 0, 99, i32::MAX as i64] {
            let value = metadata.enum_from_value.unwrap()(number).unwrap();
            assert_eq!(
                metadata.enum_to_value.unwrap()(value.as_ref()),
                Some(number)
            );
        }
        assert!(metadata.enum_from_value.unwrap()(i64::MAX).is_none());
        assert!(MarkupType::find_by_nullable_handle(metadata.nullable.unwrap()().id()).is_some());
    }
}
#[test]
fn metrics_static_for_metadata_accepts_original_core_enum() {
    crate::register_types();
    let method = RibbonMetrics::MARKUP
        .methods
        .iter()
        .find(|m| m.name == "For")
        .unwrap();
    let value = (method.invoke)(&[Some(Rc::new(RibbonDensity::Touch))])
        .unwrap()
        .unwrap();
    let actual = value.downcast_ref::<Arc<RibbonMetrics>>().unwrap();
    assert!(Arc::ptr_eq(actual, &RibbonMetrics::touch()));
    assert!((method.invoke)(&[Some(Rc::new(2_i32))]).is_err());
}
