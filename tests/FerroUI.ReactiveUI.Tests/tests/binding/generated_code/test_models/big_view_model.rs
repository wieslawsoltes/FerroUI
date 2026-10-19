//! `BigViewModel` (`ReactiveUI.Binding.GeneratedCode.TestModels/TestModels/BigViewModel.cs`).

use super::Address;
use std::rc::Rc;

const PROP2_SEED: i32 = 2;
const PROP3_SEED: f64 = 3.0;
const PROP6_SEED: i32 = 6;
const PROP7_SEED: f64 = 7.0;
const PROP10_SEED: i32 = 10;
const PROP11_SEED: f64 = 11.0;
const PROP14_SEED: i32 = 14;
const PROP15_SEED: f64 = 15.0;

generated_code_model! {
    /// A view model with sixteen properties of four types, for the observations of many
    /// properties, and an address, for the deep chains (`BigViewModel`).
    ///
    /// Upstream compares the `double` properties within `double.Epsilon`, which is equality.
    pub struct BigViewModel (changing: true) {
        prop1 / set_prop1: String = String::new(),
        prop2 / set_prop2: i32 = 0,
        prop3 / set_prop3: f64 = 0.0,
        prop4 / set_prop4: bool = false,
        prop5 / set_prop5: String = String::new(),
        prop6 / set_prop6: i32 = 0,
        prop7 / set_prop7: f64 = 0.0,
        prop8 / set_prop8: bool = false,
        prop9 / set_prop9: String = String::new(),
        prop10 / set_prop10: i32 = 0,
        prop11 / set_prop11: f64 = 0.0,
        prop12 / set_prop12: bool = false,
        prop13 / set_prop13: String = String::new(),
        prop14 / set_prop14: i32 = 0,
        prop15 / set_prop15: f64 = 0.0,
        prop16 / set_prop16: bool = false,
        address / set_address: Rc<Address> = Address::new(),
    }
}

impl BigViewModel {
    /// A view model whose sixteen properties hold distinct values (`CreatePopulated`).
    pub fn create_populated() -> Rc<Self> {
        let vm = Self::new();
        vm.set_prop1("a".to_string());
        vm.set_prop2(PROP2_SEED);
        vm.set_prop3(PROP3_SEED);
        vm.set_prop4(true);
        vm.set_prop5("e".to_string());
        vm.set_prop6(PROP6_SEED);
        vm.set_prop7(PROP7_SEED);
        vm.set_prop8(false);
        vm.set_prop9("i".to_string());
        vm.set_prop10(PROP10_SEED);
        vm.set_prop11(PROP11_SEED);
        vm.set_prop12(true);
        vm.set_prop13("m".to_string());
        vm.set_prop14(PROP14_SEED);
        vm.set_prop15(PROP15_SEED);
        vm.set_prop16(false);
        vm
    }
}
