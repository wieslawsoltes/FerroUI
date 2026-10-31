//! The provider side of UI Automation as the system declares it: the COM
//! interfaces with their enumerations and identifiers (generated from
//! `uia.idl` by the MicroCom generator in the build script of the crate;
//! the file says how it was written from the C# declarations of the
//! reference, `Interop/I*.cs`), and the functions of the automation core.
//!
//! The bindings are values and vtable calls, so they compile on every host.

#![allow(non_camel_case_types)]

mod uia_core_provider_api;
mod uia_core_types_api;

#[allow(
    clippy::all,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    missing_docs,
    unused_unsafe,
    unsafe_op_in_unsafe_fn
)]
mod uia {
    include!(concat!(env!("OUT_DIR"), "/uia.rs"));
}

pub use uia::*;
#[allow(unused_imports)] // The tests implement the trait.
pub(crate) use uia_core_provider_api::{IUiaCoreProviderApi, UiaCoreProviderApi};
#[allow(unused_imports)] // Nothing of the port looks an identifier up yet: the tests do.
pub(crate) use uia_core_types_api::{AutomationIdType, UiaCoreTypesApi};

#[cfg(test)]
mod tests {
    // Not from the reference.
    use super::*;
    use ferroui_microcom::{Guid, Interface};

    /// The identifiers of the interfaces, as `UIAutomationCore.h` of the
    /// Windows SDK has them.
    #[test]
    fn the_interfaces_have_the_identifiers_of_the_system() {
        let iid = |text: &str| Guid::parse(text).unwrap();
        assert_eq!(IRawElementProviderSimple::IID, iid("d6dd68d1-86fd-4332-8666-9abedea2d24c"));
        assert_eq!(IRawElementProviderSimple2::IID, iid("a0a839a9-8da1-4a82-806a-8e0d44e79f56"));
        assert_eq!(IRawElementProviderFragment::IID, iid("f7063da8-8359-439c-9297-bbc5299a7d87"));
        assert_eq!(IRawElementProviderFragmentRoot::IID, iid("620ce2a5-ab8f-40a9-86cb-de3c75599b58"));
        assert_eq!(IRawElementProviderAdviseEvents::IID, iid("a407b27b-0f6d-4427-9292-473c7bf93258"));
        assert_eq!(IInvokeProvider::IID, iid("54fcb24b-e18e-47a2-b4d3-eccbe77599a2"));
        assert_eq!(IToggleProvider::IID, iid("56d00bd0-c4f4-433c-a836-1a52a57e0892"));
        assert_eq!(IValueProvider::IID, iid("c7935180-6fb3-4201-b174-7df73adbf64a"));
        assert_eq!(IRangeValueProvider::IID, iid("36dc7aef-33e6-4691-afe1-2be7274b3d33"));
        assert_eq!(IExpandCollapseProvider::IID, iid("d847d3a5-cab0-4a98-8c32-ecb45c59ad24"));
        assert_eq!(ISelectionProvider::IID, iid("fb8b03af-3bdf-48d4-bd36-1a65793be168"));
        assert_eq!(ISelectionItemProvider::IID, iid("2acad808-b2d4-452d-a407-91ff1ad167b2"));
        assert_eq!(IScrollProvider::IID, iid("b38b8077-1fc3-42a5-8cae-d40c2215055a"));
        assert_eq!(IScrollItemProvider::IID, iid("2360c714-4bf1-4b26-ba65-9b21316127eb"));
        assert_eq!(IDockProvider::IID, iid("159bc72c-4ad3-485e-9637-d7052edf0146"));
        assert_eq!(IGridItemProvider::IID, iid("d02541f1-fb81-4d64-ae32-f520f8a6dbd1"));
        assert_eq!(IGridProvider::IID, iid("b17d6187-0907-464b-a168-0ef17a1572b1"));
        assert_eq!(IMultipleViewProvider::IID, iid("6278cab1-b556-4a1a-b4e0-418acc523201"));
        assert_eq!(ISynchronizedInputProvider::IID, iid("29db1a06-02ce-4cf7-9b42-565d4fab20ee"));
        assert_eq!(ITableItemProvider::IID, iid("b9734fa6-771f-4d78-9c90-2517999349cd"));
        assert_eq!(ITableProvider::IID, iid("9c860395-97b3-490a-b52a-858cc22af166"));
        assert_eq!(ITextProvider::IID, iid("3589c92c-63f3-4367-99bb-ada653b77cf2"));
        assert_eq!(ITextProvider2::IID, iid("0dc5e6ed-3e16-4bf1-8f9a-a979878bc195"));
        assert_eq!(ITextEditProvider::IID, iid("ea3605b4-3a05-400e-b5f9-4e91b40f6176"));
        assert_eq!(ITextChildProvider::IID, iid("4c2de2b9-c88f-4f88-a111-f1d336b7d1a9"));
        assert_eq!(ITextRangeProvider::IID, iid("5347ad7b-c355-46f8-aff5-909033582f63"));
        assert_eq!(ITransformProvider::IID, iid("6829ddc4-4f91-4ffa-b86f-bd3e2987cb4c"));
        assert_eq!(IWindowProvider::IID, iid("987df77b-db06-4d77-8f8a-86a9c3bb90b9"));
    }

    /// The number of slots of every table: the three of `IUnknown` and the
    /// methods of the interface (and of its base).
    #[test]
    fn the_tables_have_the_slots_of_the_system() {
        fn slots<I: Interface>() -> usize {
            std::mem::size_of::<I::Vtbl>() / std::mem::size_of::<usize>()
        }
        assert_eq!(3 + 4, slots::<IRawElementProviderSimple>());
        assert_eq!(3 + 4 + 1, slots::<IRawElementProviderSimple2>());
        assert_eq!(3 + 6, slots::<IRawElementProviderFragment>());
        assert_eq!(3 + 2, slots::<IRawElementProviderFragmentRoot>());
        assert_eq!(3 + 2, slots::<IRawElementProviderAdviseEvents>());
        assert_eq!(3 + 1, slots::<IInvokeProvider>());
        assert_eq!(3 + 2, slots::<IToggleProvider>());
        assert_eq!(3 + 3, slots::<IValueProvider>());
        assert_eq!(3 + 7, slots::<IRangeValueProvider>());
        assert_eq!(3 + 3, slots::<IExpandCollapseProvider>());
        assert_eq!(3 + 3, slots::<ISelectionProvider>());
        assert_eq!(3 + 5, slots::<ISelectionItemProvider>());
        assert_eq!(3 + 8, slots::<IScrollProvider>());
        assert_eq!(3 + 1, slots::<IScrollItemProvider>());
        assert_eq!(3 + 2, slots::<IDockProvider>());
        assert_eq!(3 + 5, slots::<IGridItemProvider>());
        assert_eq!(3 + 3, slots::<IGridProvider>());
        assert_eq!(3 + 4, slots::<IMultipleViewProvider>());
        assert_eq!(3 + 2, slots::<ISynchronizedInputProvider>());
        assert_eq!(3 + 2, slots::<ITableItemProvider>());
        assert_eq!(3 + 3, slots::<ITableProvider>());
        assert_eq!(3 + 6, slots::<ITextProvider>());
        assert_eq!(3 + 6 + 2, slots::<ITextProvider2>());
        assert_eq!(3 + 6 + 2, slots::<ITextEditProvider>());
        assert_eq!(3 + 2, slots::<ITextChildProvider>());
        assert_eq!(3 + 18, slots::<ITextRangeProvider>());
        assert_eq!(3 + 6, slots::<ITransformProvider>());
        assert_eq!(3 + 9, slots::<IWindowProvider>());
    }

    /// The identifiers of properties, patterns, events and control types
    /// are counted from the first of each range: these are values of
    /// `UIAutomationClient.h` along the ranges, which hold the count.
    #[test]
    fn the_identifiers_have_the_values_of_the_system() {
        assert_eq!(30000, UiaPropertyId::RuntimeId.0);
        assert_eq!(30005, UiaPropertyId::Name.0);
        assert_eq!(30019, UiaPropertyId::IsPassword.0);
        assert_eq!(30024, UiaPropertyId::FrameworkId.0);
        assert_eq!(30045, UiaPropertyId::ValueValue.0);
        assert_eq!(30047, UiaPropertyId::RangeValueValue.0);
        assert_eq!(30070, UiaPropertyId::ExpandCollapseExpandCollapseState.0);
        assert_eq!(30079, UiaPropertyId::SelectionItemIsSelected.0);
        assert_eq!(30086, UiaPropertyId::ToggleToggleState.0);
        assert_eq!(30090, UiaPropertyId::IsLegacyIAccessiblePatternAvailable.0);
        assert_eq!(30135, UiaPropertyId::LiveSetting.0);
        assert_eq!(30149, UiaPropertyId::IsTextEditPatternAvailable.0);
        assert_eq!(30157, UiaPropertyId::LandmarkType.0);
        assert_eq!(30158, UiaPropertyId::LocalizedLandmarkType.0);
        assert_eq!(30166, UiaPropertyId::Rotatation.0);
        assert_eq!(30173, UiaPropertyId::HeadingLevel.0);
        assert_eq!(30174, UiaPropertyId::IsDialog.0);

        assert_eq!(10000, UiaPatternId::Invoke.0);
        assert_eq!(10002, UiaPatternId::Value.0);
        assert_eq!(10003, UiaPatternId::RangeValue.0);
        assert_eq!(10005, UiaPatternId::ExpandCollapse.0);
        assert_eq!(10010, UiaPatternId::SelectionItem.0);
        assert_eq!(10015, UiaPatternId::Toggle.0);
        assert_eq!(10017, UiaPatternId::ScrollItem.0);
        assert_eq!(10024, UiaPatternId::Text2.0);
        assert_eq!(10029, UiaPatternId::TextChild.0);
        assert_eq!(10032, UiaPatternId::TextEdit.0);
        assert_eq!(10033, UiaPatternId::CustomNavigation.0);

        assert_eq!(50000, UiaControlTypeId::Button.0);
        assert_eq!(50002, UiaControlTypeId::CheckBox.0);
        assert_eq!(50004, UiaControlTypeId::Edit.0);
        assert_eq!(50015, UiaControlTypeId::Slider.0);
        assert_eq!(50020, UiaControlTypeId::Text.0);
        assert_eq!(50026, UiaControlTypeId::Group.0);
        assert_eq!(50032, UiaControlTypeId::Window.0);
        assert_eq!(50033, UiaControlTypeId::Pane.0);
        assert_eq!(50040, UiaControlTypeId::AppBar.0);

        assert_eq!(20000, UiaEventId::ToolTipOpened.0);
        assert_eq!(20002, UiaEventId::StructureChanged.0);
        assert_eq!(20004, UiaEventId::AutomationPropertyChanged.0);
        assert_eq!(20005, UiaEventId::AutomationFocusChanged.0);
        assert_eq!(20009, UiaEventId::Invoke_Invoked.0);
        assert_eq!(20024, UiaEventId::LiveRegionChanged.0);
        assert_eq!(20032, UiaEventId::TextEdit_TextChanged.0);
        assert_eq!(20033, UiaEventId::TextEdit_ConversionTargetChanged.0);
        assert_eq!(20034, UiaEventId::Changes.0);

        assert_eq!(80000, UiaLandmarkType::Custom.0);
        assert_eq!(80004, UiaLandmarkType::Search.0);
        assert_eq!(80050, UiaHeadingLevel::None.0);
        assert_eq!(80059, UiaHeadingLevel::Level9.0);
        assert_eq!(2, UiaLiveSetting::Assertive.0);
    }

    #[test]
    fn the_enumerations_have_the_values_of_the_system() {
        assert_eq!(0, TextEditChangeType::None.0);
        assert_eq!(1, TextEditChangeType::AutoCorrect.0);
        assert_eq!(2, TextEditChangeType::Composition.0);
        assert_eq!(3, TextEditChangeType::CompositionFinalized.0);
        assert_eq!(4, TextEditChangeType::AutoComplete.0);
        assert_eq!(1, ProviderOptions::ClientSideProvider.0);
        assert_eq!(2, ProviderOptions::ServerSideProvider.0);
        assert_eq!(8, ProviderOptions::OverrideProvider.0);
        assert_eq!(0x20, ProviderOptions::UseComThreading.0);
        assert_eq!(
            [0, 1, 2, 3, 4],
            [
                NavigateDirection::Parent.0,
                NavigateDirection::NextSibling.0,
                NavigateDirection::PreviousSibling.0,
                NavigateDirection::FirstChild.0,
                NavigateDirection::LastChild.0
            ]
        );
        assert_eq!(2, StructureChangeType::ChildrenInvalidated.0);
        assert_eq!(5, StructureChangeType::ChildrenReordered.0);
        assert_eq!(5, DockPosition::None.0);
        assert_eq!(0x20, SynchronizedInputType::MouseRightButtonDown.0);
        assert_eq!(2, RowOrColumnMajor::Indeterminate.0);
        assert_eq!(2, SupportedTextSelection::Multiple.0);
        assert_eq!(1, TextPatternRangeEndpoint::End.0);
        assert_eq!(6, TextUnit::Document.0);
        assert_eq!(2, WindowVisualState::Minimized.0);
        assert_eq!(4, WindowInteractionState::NotResponding.0);
        assert_eq!(2, AsyncContentLoadedState::Completed.0);
    }

    /// The states and amounts the interfaces pass as numbers are the
    /// enumerations of the automation contracts of the controls, whose
    /// values are the ones of the system.
    #[test]
    fn the_enumerations_of_the_controls_that_cross_as_numbers() {
        use ferroui_controls::automation::provider::{ScrollAmount, ToggleState};
        assert_eq!([0, 1, 2], [ToggleState::Off as i32, ToggleState::On as i32, ToggleState::Indeterminate as i32]);
        assert_eq!(
            [0, 1, 2, 3, 4],
            [
                ScrollAmount::LargeDecrement as i32,
                ScrollAmount::SmallDecrement as i32,
                ScrollAmount::NoAmount as i32,
                ScrollAmount::LargeIncrement as i32,
                ScrollAmount::SmallIncrement as i32
            ]
        );
    }

    #[test]
    fn the_structures_have_the_layout_of_the_system() {
        assert_eq!(32, std::mem::size_of::<UiaRect>());
        assert_eq!(16, std::mem::size_of::<UiaPoint>());
        assert_eq!(16, std::mem::offset_of!(UiaRect, width));
    }

    /// On Windows the automation core knows the properties by their
    /// identifiers of the header: two of them, looked up.
    #[test]
    #[cfg(windows)]
    fn the_system_looks_the_identifiers_up() {
        // Name_Property_GUID and ToggleState_Property_GUID of the header.
        let name = Guid::parse("c3a6921b-4a99-44f1-bca6-61187052c431").unwrap();
        assert_eq!(UiaPropertyId::Name.0, UiaCoreTypesApi::uia_lookup_id(AutomationIdType::Property, &name));
        let toggle_state = Guid::parse("b23cdc52-22c2-4c6c-9ded-f5c422479ede").unwrap();
        assert_eq!(
            UiaPropertyId::ToggleToggleState.0,
            UiaCoreTypesApi::uia_lookup_id(AutomationIdType::Property, &toggle_state)
        );
    }
}
