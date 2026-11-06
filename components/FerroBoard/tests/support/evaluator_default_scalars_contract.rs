//! Process-isolated Default mutation cases. Do not include this runner in a parallel shared-default suite.
use ferroboard_core::{keys::*, layouts::*};
use serde_json::{Value, json};
use std::sync::Arc;
pub trait Driver: 'static {
    fn shared() -> KeyEvaluator;
    fn set(value: &KeyEvaluator, field: &str, raw: i32);
    fn read(value: &KeyEvaluator) -> [i32; 6];
    fn cloned(value: &KeyEvaluator) -> KeyEvaluator;
}
pub struct Core;
impl Driver for Core {
    fn shared() -> KeyEvaluator {
        KeyEvaluator::default_reference()
    }
    fn set(v: &KeyEvaluator, field: &str, raw: i32) {
        match field {
            "Mode" => v.set_mode(KeyboardMode(raw)),
            "ShiftState" => v.set_shift_state(ShiftState(raw)),
            "Variation" => v.set_variation(KeyVariation(raw)),
            "Direction" => v.set_direction(LayoutDirection(raw)),
            "IsCharHalfWidth" => v.set_is_char_half_width(raw != 0),
            "IsKanaKata" => v.set_is_kana_kata(raw != 0),
            _ => panic!("Unknown field {field}"),
        }
    }
    fn read(v: &KeyEvaluator) -> [i32; 6] {
        [
            v.mode().0,
            v.shift_state().0,
            v.variation().0,
            v.direction().0,
            v.is_char_half_width() as i32,
            v.is_kana_kata() as i32,
        ]
    }
    fn cloned(v: &KeyEvaluator) -> KeyEvaluator {
        KeyEvaluator::from_values(v.values())
    }
}
fn reset() {
    let value = KeyEvaluator::default_reference();
    for (field, raw) in [
        "Mode",
        "ShiftState",
        "Variation",
        "Direction",
        "IsCharHalfWidth",
        "IsKanaKata",
    ]
    .into_iter()
    .zip([0, 0, 1, 0, 0, 0])
    {
        Core::set(&value, field, raw);
    }
}
struct Restore;
impl Drop for Restore {
    fn drop(&mut self) {
        reset();
    }
}
pub fn verify<D: Driver>(fixture: &str) -> usize {
    reset();
    let _restore = Restore;
    let rows: Vec<Value> = fixture
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    for expected in &rows {
        let mut expected = expected.clone();
        expected.as_object_mut().unwrap().remove("hashDelta");
        expected.as_object_mut().unwrap().remove("copyHashDelta");
        expected.as_object_mut().unwrap().remove("text");
        reset();
        let owner = D::shared();
        let alias = D::shared();
        let before = D::cloned(&owner);
        let field = expected["field"].as_str().unwrap().to_owned();
        let raw = expected["value"].as_i64().unwrap() as i32;
        let writer = expected["workerWrite"].as_bool().unwrap();
        if writer {
            let field = field.clone();
            std::thread::spawn(move || D::set(&D::shared(), &field, raw))
                .join()
                .unwrap();
        } else {
            D::set(&owner, &field, raw);
        }
        let state = D::read(&owner);
        let token = KeyEvaluator::shared_default();
        let observed = token.clone();
        let (worker_same, worker_state) = std::thread::spawn(move || {
            let value = D::shared();
            (
                Arc::ptr_eq(&observed, &KeyEvaluator::shared_default())
                    && value.ptr_eq(&KeyEvaluator::default_reference()),
                D::read(&value),
            )
        })
        .join()
        .unwrap();
        let face = owner.original_object();
        assert_eq!(
            Core::read(&owner),
            [
                face.mode().0,
                face.shift_state().0,
                face.variation().0,
                face.direction().0,
                face.is_char_half_width() as i32,
                face.is_kana_kata() as i32
            ]
        );
        assert_eq!(
            state,
            [
                token.mode().0,
                token.shift_state().0,
                token.variation().0,
                token.direction().0,
                token.is_char_half_width() as i32,
                token.is_kana_kata() as i32
            ]
        );
        assert_eq!(
            Core::read(&KeyEvaluator::from_values(token.values())),
            state
        );
        let copy = D::cloned(&owner);
        let before_equal = owner == before;
        let copy_equal = owner == copy;
        reset();
        let actual = json!({"field":field,"value":raw,"workerWrite":writer,"aliasSame":owner.ptr_eq(&alias),"workerSame":worker_same,"state":state,"workerState":worker_state,"copyBefore":D::read(&before),"copyAfter":D::read(&copy),"copyBeforeEqual":before_equal,"copyAfterEqual":copy_equal,"copySame":owner.ptr_eq(&copy),"restored":D::read(&owner)});
        assert_eq!(actual, expected);
    }
    assert_eq!(
        Core::read(&KeyEvaluator::default_reference()),
        [0, 0, 1, 0, 0, 0]
    );
    rows.len()
}
