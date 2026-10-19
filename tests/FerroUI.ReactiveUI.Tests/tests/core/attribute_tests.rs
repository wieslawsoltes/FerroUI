//! `AttributeTests` (`ReactiveUI.Tests/Core/AttributeTests.cs`): the attributes of ReactiveUI.

// Not ported: PreserveAttribute_Constructor_CreatesInstance - reflection (`PreserveAttribute` tells the
// .NET linker to keep members; Rust has no linker metadata of that kind)
// Not ported: SingleInstanceViewAttribute_AttributeUsage_IsClass - reflection
// Not ported: SingleInstanceViewAttribute_Constructor_CreatesInstance - needs SingleInstanceViewAttribute (view location, a later wave)
// Not ported: ViewContractAttribute_AttributeUsage_IsClass - reflection
// Not ported: ViewContractAttribute_Constructor_StoresContractValue - needs ViewContractAttribute (view location, a later wave)
// Not ported: ViewContractAttribute_NullContract_StoresNull - needs ViewContractAttribute (view location, a later wave)
