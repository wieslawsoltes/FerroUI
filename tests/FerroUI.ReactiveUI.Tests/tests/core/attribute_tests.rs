//! `AttributeTests` (`ReactiveUI.Tests/Core/AttributeTests.cs`): the attributes of ReactiveUI.

// Not ported: PreserveAttribute_Constructor_CreatesInstance - reflection (`PreserveAttribute` tells the
// .NET linker to keep members; Rust has no linker metadata of that kind)
// Not ported: SingleInstanceViewAttribute_AttributeUsage_IsClass - reflection

use ferroui_reactiveui::{SingleInstanceView, ViewContract};

#[test]
fn single_instance_view_attribute_constructor_creates_instance() {
    // Act
    let attribute = SingleInstanceView;

    // Assert
    let _: SingleInstanceView = attribute;
    assert_eq!(attribute, SingleInstanceView::default());
}

// Not ported: ViewContractAttribute_AttributeUsage_IsClass - reflection

#[test]
fn view_contract_attribute_constructor_stores_contract_value() {
    // Arrange
    const EXPECTED_CONTRACT: &str = "TestContract";

    // Act
    let attribute = ViewContract::new(EXPECTED_CONTRACT);

    // Assert
    assert_eq!(attribute.contract(), EXPECTED_CONTRACT);
}

// Not ported: ViewContractAttribute_NullContract_StoresNull - a Rust string cannot be null
// (`ViewContract::new` takes a `String`, so the null contract has no counterpart)
