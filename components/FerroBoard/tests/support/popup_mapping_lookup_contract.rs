use ferroboard_core::{keys::KeyVariation, layouts::*, text::Utf16Text};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    rc::Rc,
};
fn rows() -> impl Iterator<Item = Value> {
    include_str!("../fixtures/popup-mapping-state.jsonl")
        .lines()
        .map(|v| serde_json::from_str(v).unwrap())
}
fn units(v: Option<Utf16Text>) -> Value {
    json!(v.map(|v| v.units().to_vec()))
}
fn text(v: &Value) -> Option<Utf16Text> {
    v.as_array().map(|v| {
        Utf16Text::from_units(
            v.iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
#[derive(Debug)]
struct Marker;
struct Outer {
    trace: Rc<RefCell<Vec<String>>>,
    get: Rc<dyn Fn(KeyVariation) -> (bool, Option<PopupLabelMappings>)>,
}
struct Inner {
    trace: Rc<RefCell<Vec<String>>>,
    mode: String,
    data: ResourceDictionary<Utf16Text, Rc<PopupSet>>,
    target: Rc<PopupSet>,
}
macro_rules! unsupported_members {
    ($k:ty,$v:ty) => {
        fn count(&self) -> Result<i32, ResourceTextListError> {
            Err(ResourceTextListError::NotSupported)
        }
        fn get(&self, _: Option<&$k>) -> Result<Option<$v>, ResourceTextListError> {
            Err(ResourceTextListError::NotSupported)
        }
        fn contains_key(&self, _: Option<&$k>) -> Result<bool, ResourceTextListError> {
            Err(ResourceTextListError::NotSupported)
        }
        fn keys(&self) -> Result<Option<ResourceEnumerable<$k>>, ResourceTextListError> {
            Err(ResourceTextListError::NotSupported)
        }
        fn values(&self) -> Result<Option<ResourceEnumerable<$v>>, ResourceTextListError> {
            Err(ResourceTextListError::NotSupported)
        }
        fn iter(
            &self,
        ) -> Result<ResourceItems<ResourceDictionaryEntry<$k, $v>>, ResourceTextListError> {
            Err(ResourceTextListError::NotSupported)
        }
    };
}
impl IResourceReadOnlyDictionary<KeyVariation, PopupLabelMappings> for Outer {
    unsupported_members!(KeyVariation, PopupLabelMappings);
    fn try_get_value(
        &self,
        key: Option<&KeyVariation>,
    ) -> Result<(bool, Option<PopupLabelMappings>), ResourceTextListError> {
        let key = *key.unwrap();
        self.trace.borrow_mut().push(format!("outer:{}", key.0));
        Ok((self.get)(key))
    }
}
impl IResourceReadOnlyDictionary<Utf16Text, Rc<PopupSet>> for Inner {
    unsupported_members!(Utf16Text, Rc<PopupSet>);
    fn try_get_value(
        &self,
        key: Option<&Utf16Text>,
    ) -> Result<(bool, Option<Rc<PopupSet>>), ResourceTextListError> {
        self.trace.borrow_mut().push(format!(
            "inner:{}",
            key.map_or_else(
                || "null".into(),
                |k| k
                    .units()
                    .iter()
                    .map(u16::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        ));
        match self.mode.as_str() {
            "throw-inner" => resume_unwind(Box::new(Marker)),
            "false-inner-value" => Ok((false, Some(self.target.clone()))),
            "true-inner-null" => Ok((true, None)),
            _ => self.data.try_get_value(key),
        }
    }
}
pub fn verify(
    lookup: impl Fn(
        &Rc<PopupMapping>,
        KeyVariation,
        Option<&Utf16Text>,
    ) -> Result<Option<Rc<PopupSet>>, KeyDataError>,
) {
    let mut count = 0;
    for row in rows().filter(|r| r["tag"] == "lookup") {
        let mode = row["mode"].as_str().unwrap().to_owned();
        let m = Rc::new(PopupMapping::default());
        let target = Rc::new(PopupSet::from_parts(
            Some(TextKeyData::char("é".into(), None).unwrap() as Key),
            ResourceReadOnlyList::default(),
        ));
        let data = ResourceDictionary::default();
        data.set("a".into(), Some(target.clone()));
        data.set(
            Utf16Text::from_units(vec![120, 0xd800]),
            Some(target.clone()),
        );
        let trace = Rc::new(RefCell::new(Vec::new()));
        let inner = PopupLabelMappings::from_custom(Rc::new(Inner {
            trace: trace.clone(),
            mode: mode.clone(),
            data: data.clone(),
            target: target.clone(),
        }));
        let weak = Rc::downgrade(&m);
        let behavior = mode.clone();
        let table = data.clone();
        let outer = PopupVariationMappings::from_custom(Rc::new(Outer {
            trace: trace.clone(),
            get: Rc::new(move |key| {
                if behavior == "throw-outer" {
                    resume_unwind(Box::new(Marker))
                }
                let owner = weak.upgrade().unwrap();
                if behavior == "replace-null" {
                    owner.set_mappings(None)
                }
                if behavior == "replace-other" {
                    owner.set_mappings(Some(PopupVariationMappings::default()))
                }
                if behavior == "clear-inner" {
                    table.clear()
                }
                match behavior.as_str() {
                    "false-value" => (false, Some(inner.clone())),
                    "false-null" => (false, None),
                    "true-null" => (true, None),
                    _ => {
                        if key == KeyVariation::Normal {
                            (true, Some(inner.clone()))
                        } else {
                            (false, None)
                        }
                    }
                }
            }),
        }));
        m.set_mappings(match mode.as_str() {
            "null" => None,
            "default" => Some(PopupVariationMappings::default()),
            _ => Some(outer.clone()),
        });
        let variation = KeyVariation(row["variation"].as_i64().unwrap() as i32);
        let label = text(&row["label"]);
        let result = catch_unwind(AssertUnwindSafe(|| lookup(&m, variation, label.as_ref())));
        let (result, error) = match result {
            Ok(Ok(value)) => (value, None),
            Ok(Err(
                KeyDataError::Collection(ResourceTextListError::NullReference)
                | KeyDataError::NullReference,
            )) => (None, Some("NullReferenceException")),
            Ok(Err(KeyDataError::Collection(ResourceTextListError::ArgumentNull))) => {
                (None, Some("ArgumentNullException"))
            }
            Ok(Err(e)) => panic!("unexpected {e:?}"),
            Err(e) => {
                if e.downcast_ref::<Marker>().is_some() {
                    (None, Some("MarkerException"))
                } else {
                    resume_unwind(e)
                }
            }
        };
        let actual = json!({"tag":"lookup","mode":mode,"variation":variation.0,"label":units(label),"error":error,"result":result.as_ref().map_or("null",|r|if Rc::ptr_eq(r,&target){"target"}else{"other"}),"trace":*trace.borrow(),"mappingSame":m.mappings().is_some_and(|map|map.ptr_eq(&outer)),"mappingNull":m.mappings().is_none(),"remaining":data.count().unwrap()});
        assert_eq!(actual, row);
        count += 1;
    }
    assert_eq!(count, 130);
}
