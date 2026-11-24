use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::metadata::{MarkupEnumMember, MarkupType, MarkupTypeKind, MarkupTyped};
use std::rc::Rc;
#[doc(hidden)]
pub struct RibbonItemSizeMetadata;
#[allow(non_upper_case_globals)]
impl RibbonItemSizeMetadata {
    pub const Large: ferroribbon_core::RibbonItemSize = ferroribbon_core::RibbonItemSize::Large;
    pub const Medium: ferroribbon_core::RibbonItemSize = ferroribbon_core::RibbonItemSize::Medium;
    pub const Small: ferroribbon_core::RibbonItemSize = ferroribbon_core::RibbonItemSize::Small;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon_core::RibbonItemSize {
        ferroribbon_core::RibbonItemSize::from_raw(value)
    }
}
const _: () = assert!(ferroribbon_core::RibbonItemSize::Large.raw() == 0);
const _: () = assert!(ferroribbon_core::RibbonItemSize::Medium.raw() == 1);
const _: () = assert!(ferroribbon_core::RibbonItemSize::Small.raw() == 2);
impl MarkupTyped for RibbonItemSizeMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new("RibbonItemSize", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroRibbon");
            m.handles = &[ValueType::of::<ferroribbon_core::RibbonItemSize>];
            m.nullable = Some(ValueType::of::<Option<ferroribbon_core::RibbonItemSize>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Large",
                    value: 0,
                    rust_variant: Some("Large"),
                },
                MarkupEnumMember {
                    name: "Medium",
                    value: 1,
                    rust_variant: Some("Medium"),
                },
                MarkupEnumMember {
                    name: "Small",
                    value: 2,
                    rust_variant: Some("Small"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon_core::RibbonItemSize::from_raw(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon_core::RibbonItemSize>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
#[doc(hidden)]
pub struct RibbonGroupStateMetadata;
#[allow(non_upper_case_globals)]
impl RibbonGroupStateMetadata {
    pub const Large: ferroribbon_core::RibbonGroupState = ferroribbon_core::RibbonGroupState::Large;
    pub const Medium: ferroribbon_core::RibbonGroupState =
        ferroribbon_core::RibbonGroupState::Medium;
    pub const Small: ferroribbon_core::RibbonGroupState = ferroribbon_core::RibbonGroupState::Small;
    pub const Collapsed: ferroribbon_core::RibbonGroupState =
        ferroribbon_core::RibbonGroupState::Collapsed;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon_core::RibbonGroupState {
        ferroribbon_core::RibbonGroupState::from_raw(value)
    }
}
const _: () = assert!(ferroribbon_core::RibbonGroupState::Large.raw() == 0);
const _: () = assert!(ferroribbon_core::RibbonGroupState::Medium.raw() == 1);
const _: () = assert!(ferroribbon_core::RibbonGroupState::Small.raw() == 2);
const _: () = assert!(ferroribbon_core::RibbonGroupState::Collapsed.raw() == 3);
impl MarkupTyped for RibbonGroupStateMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new("RibbonGroupState", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroRibbon");
            m.handles = &[ValueType::of::<ferroribbon_core::RibbonGroupState>];
            m.nullable = Some(ValueType::of::<Option<ferroribbon_core::RibbonGroupState>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Large",
                    value: 0,
                    rust_variant: Some("Large"),
                },
                MarkupEnumMember {
                    name: "Medium",
                    value: 1,
                    rust_variant: Some("Medium"),
                },
                MarkupEnumMember {
                    name: "Small",
                    value: 2,
                    rust_variant: Some("Small"),
                },
                MarkupEnumMember {
                    name: "Collapsed",
                    value: 3,
                    rust_variant: Some("Collapsed"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon_core::RibbonGroupState::from_raw(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon_core::RibbonGroupState>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
#[doc(hidden)]
pub struct RibbonDisplayModeMetadata;
#[allow(non_upper_case_globals)]
impl RibbonDisplayModeMetadata {
    pub const Classic: ferroribbon_core::RibbonDisplayMode =
        ferroribbon_core::RibbonDisplayMode::Classic;
    pub const Simplified: ferroribbon_core::RibbonDisplayMode =
        ferroribbon_core::RibbonDisplayMode::Simplified;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon_core::RibbonDisplayMode {
        ferroribbon_core::RibbonDisplayMode::from_raw(value)
    }
}
const _: () = assert!(ferroribbon_core::RibbonDisplayMode::Classic.raw() == 0);
const _: () = assert!(ferroribbon_core::RibbonDisplayMode::Simplified.raw() == 1);
impl MarkupTyped for RibbonDisplayModeMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new("RibbonDisplayMode", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroRibbon");
            m.handles = &[ValueType::of::<ferroribbon_core::RibbonDisplayMode>];
            m.nullable = Some(ValueType::of::<Option<ferroribbon_core::RibbonDisplayMode>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Classic",
                    value: 0,
                    rust_variant: Some("Classic"),
                },
                MarkupEnumMember {
                    name: "Simplified",
                    value: 1,
                    rust_variant: Some("Simplified"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon_core::RibbonDisplayMode::from_raw(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon_core::RibbonDisplayMode>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
#[doc(hidden)]
pub struct RibbonVisibilityModeMetadata;
#[allow(non_upper_case_globals)]
impl RibbonVisibilityModeMetadata {
    pub const AlwaysShow: ferroribbon_core::RibbonVisibilityMode =
        ferroribbon_core::RibbonVisibilityMode::AlwaysShow;
    pub const TabsOnly: ferroribbon_core::RibbonVisibilityMode =
        ferroribbon_core::RibbonVisibilityMode::TabsOnly;
    pub const FullScreen: ferroribbon_core::RibbonVisibilityMode =
        ferroribbon_core::RibbonVisibilityMode::FullScreen;
    pub const PanelButtons: ferroribbon_core::RibbonVisibilityMode =
        ferroribbon_core::RibbonVisibilityMode::PanelButtons;
    pub const PanelTitles: ferroribbon_core::RibbonVisibilityMode =
        ferroribbon_core::RibbonVisibilityMode::PanelTitles;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon_core::RibbonVisibilityMode {
        ferroribbon_core::RibbonVisibilityMode::from_raw(value)
    }
}
const _: () = assert!(ferroribbon_core::RibbonVisibilityMode::AlwaysShow.raw() == 0);
const _: () = assert!(ferroribbon_core::RibbonVisibilityMode::TabsOnly.raw() == 1);
const _: () = assert!(ferroribbon_core::RibbonVisibilityMode::FullScreen.raw() == 2);
const _: () = assert!(ferroribbon_core::RibbonVisibilityMode::PanelButtons.raw() == 3);
const _: () = assert!(ferroribbon_core::RibbonVisibilityMode::PanelTitles.raw() == 4);
impl MarkupTyped for RibbonVisibilityModeMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m =
                MarkupType::new("RibbonVisibilityMode", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroRibbon");
            m.handles = &[ValueType::of::<ferroribbon_core::RibbonVisibilityMode>];
            m.nullable = Some(ValueType::of::<Option<ferroribbon_core::RibbonVisibilityMode>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "AlwaysShow",
                    value: 0,
                    rust_variant: Some("AlwaysShow"),
                },
                MarkupEnumMember {
                    name: "TabsOnly",
                    value: 1,
                    rust_variant: Some("TabsOnly"),
                },
                MarkupEnumMember {
                    name: "FullScreen",
                    value: 2,
                    rust_variant: Some("FullScreen"),
                },
                MarkupEnumMember {
                    name: "PanelButtons",
                    value: 3,
                    rust_variant: Some("PanelButtons"),
                },
                MarkupEnumMember {
                    name: "PanelTitles",
                    value: 4,
                    rust_variant: Some("PanelTitles"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon_core::RibbonVisibilityMode::from_raw(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon_core::RibbonVisibilityMode>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
#[doc(hidden)]
pub struct RibbonMinimizeBehaviorMetadata;
#[allow(non_upper_case_globals)]
impl RibbonMinimizeBehaviorMetadata {
    pub const Tabs: ferroribbon_core::RibbonMinimizeBehavior =
        ferroribbon_core::RibbonMinimizeBehavior::Tabs;
    pub const PanelTitles: ferroribbon_core::RibbonMinimizeBehavior =
        ferroribbon_core::RibbonMinimizeBehavior::PanelTitles;
    pub const PanelButtons: ferroribbon_core::RibbonMinimizeBehavior =
        ferroribbon_core::RibbonMinimizeBehavior::PanelButtons;
    pub const CycleAll: ferroribbon_core::RibbonMinimizeBehavior =
        ferroribbon_core::RibbonMinimizeBehavior::CycleAll;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon_core::RibbonMinimizeBehavior {
        ferroribbon_core::RibbonMinimizeBehavior::from_raw(value)
    }
}
const _: () = assert!(ferroribbon_core::RibbonMinimizeBehavior::Tabs.raw() == 0);
const _: () = assert!(ferroribbon_core::RibbonMinimizeBehavior::PanelTitles.raw() == 1);
const _: () = assert!(ferroribbon_core::RibbonMinimizeBehavior::PanelButtons.raw() == 2);
const _: () = assert!(ferroribbon_core::RibbonMinimizeBehavior::CycleAll.raw() == 3);
impl MarkupTyped for RibbonMinimizeBehaviorMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new(
                "RibbonMinimizeBehavior",
                MarkupTypeKind::Enum,
                module_path!(),
            );
            m.explicit_namespace = Some("FerroRibbon");
            m.handles = &[ValueType::of::<ferroribbon_core::RibbonMinimizeBehavior>];
            m.nullable = Some(ValueType::of::<Option<ferroribbon_core::RibbonMinimizeBehavior>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Tabs",
                    value: 0,
                    rust_variant: Some("Tabs"),
                },
                MarkupEnumMember {
                    name: "PanelTitles",
                    value: 1,
                    rust_variant: Some("PanelTitles"),
                },
                MarkupEnumMember {
                    name: "PanelButtons",
                    value: 2,
                    rust_variant: Some("PanelButtons"),
                },
                MarkupEnumMember {
                    name: "CycleAll",
                    value: 3,
                    rust_variant: Some("CycleAll"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon_core::RibbonMinimizeBehavior::from_raw(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon_core::RibbonMinimizeBehavior>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
#[doc(hidden)]
pub struct RibbonQuickAccessPositionMetadata;
#[allow(non_upper_case_globals)]
impl RibbonQuickAccessPositionMetadata {
    pub const AboveRibbon: ferroribbon_core::RibbonQuickAccessPosition =
        ferroribbon_core::RibbonQuickAccessPosition::AboveRibbon;
    pub const BelowRibbon: ferroribbon_core::RibbonQuickAccessPosition =
        ferroribbon_core::RibbonQuickAccessPosition::BelowRibbon;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon_core::RibbonQuickAccessPosition {
        ferroribbon_core::RibbonQuickAccessPosition::from_raw(value)
    }
}
const _: () = assert!(ferroribbon_core::RibbonQuickAccessPosition::AboveRibbon.raw() == 0);
const _: () = assert!(ferroribbon_core::RibbonQuickAccessPosition::BelowRibbon.raw() == 1);
impl MarkupTyped for RibbonQuickAccessPositionMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new(
                "RibbonQuickAccessPosition",
                MarkupTypeKind::Enum,
                module_path!(),
            );
            m.explicit_namespace = Some("FerroRibbon");
            m.handles = &[ValueType::of::<ferroribbon_core::RibbonQuickAccessPosition>];
            m.nullable = Some(ValueType::of::<Option<ferroribbon_core::RibbonQuickAccessPosition>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "AboveRibbon",
                    value: 0,
                    rust_variant: Some("AboveRibbon"),
                },
                MarkupEnumMember {
                    name: "BelowRibbon",
                    value: 1,
                    rust_variant: Some("BelowRibbon"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon_core::RibbonQuickAccessPosition::from_raw(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon_core::RibbonQuickAccessPosition>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
#[doc(hidden)]
pub struct RibbonDensityMetadata;
#[allow(non_upper_case_globals)]
impl RibbonDensityMetadata {
    pub const Comfortable: ferroribbon_core::RibbonDensity =
        ferroribbon_core::RibbonDensity::Comfortable;
    pub const Compact: ferroribbon_core::RibbonDensity = ferroribbon_core::RibbonDensity::Compact;
    pub const Touch: ferroribbon_core::RibbonDensity = ferroribbon_core::RibbonDensity::Touch;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon_core::RibbonDensity {
        ferroribbon_core::RibbonDensity::from_raw(value)
    }
}
const _: () = assert!(ferroribbon_core::RibbonDensity::Comfortable.raw() == 0);
const _: () = assert!(ferroribbon_core::RibbonDensity::Compact.raw() == 1);
const _: () = assert!(ferroribbon_core::RibbonDensity::Touch.raw() == 2);
impl MarkupTyped for RibbonDensityMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new("RibbonDensity", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroRibbon");
            m.handles = &[ValueType::of::<ferroribbon_core::RibbonDensity>];
            m.nullable = Some(ValueType::of::<Option<ferroribbon_core::RibbonDensity>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Comfortable",
                    value: 0,
                    rust_variant: Some("Comfortable"),
                },
                MarkupEnumMember {
                    name: "Compact",
                    value: 1,
                    rust_variant: Some("Compact"),
                },
                MarkupEnumMember {
                    name: "Touch",
                    value: 2,
                    rust_variant: Some("Touch"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon_core::RibbonDensity::from_raw(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon_core::RibbonDensity>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
#[doc(hidden)]
pub struct RibbonSimplifiedVisibilityMetadata;
#[allow(non_upper_case_globals)]
impl RibbonSimplifiedVisibilityMetadata {
    pub const Auto: ferroribbon_core::RibbonSimplifiedVisibility =
        ferroribbon_core::RibbonSimplifiedVisibility::Auto;
    pub const Pinned: ferroribbon_core::RibbonSimplifiedVisibility =
        ferroribbon_core::RibbonSimplifiedVisibility::Pinned;
    pub const Overflow: ferroribbon_core::RibbonSimplifiedVisibility =
        ferroribbon_core::RibbonSimplifiedVisibility::Overflow;
    pub const Hidden: ferroribbon_core::RibbonSimplifiedVisibility =
        ferroribbon_core::RibbonSimplifiedVisibility::Hidden;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon_core::RibbonSimplifiedVisibility {
        ferroribbon_core::RibbonSimplifiedVisibility::from_raw(value)
    }
}
const _: () = assert!(ferroribbon_core::RibbonSimplifiedVisibility::Auto.raw() == 0);
const _: () = assert!(ferroribbon_core::RibbonSimplifiedVisibility::Pinned.raw() == 1);
const _: () = assert!(ferroribbon_core::RibbonSimplifiedVisibility::Overflow.raw() == 2);
const _: () = assert!(ferroribbon_core::RibbonSimplifiedVisibility::Hidden.raw() == 3);
impl MarkupTyped for RibbonSimplifiedVisibilityMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new(
                "RibbonSimplifiedVisibility",
                MarkupTypeKind::Enum,
                module_path!(),
            );
            m.explicit_namespace = Some("FerroRibbon");
            m.handles = &[ValueType::of::<ferroribbon_core::RibbonSimplifiedVisibility>];
            m.nullable =
                Some(ValueType::of::<Option<ferroribbon_core::RibbonSimplifiedVisibility>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Auto",
                    value: 0,
                    rust_variant: Some("Auto"),
                },
                MarkupEnumMember {
                    name: "Pinned",
                    value: 1,
                    rust_variant: Some("Pinned"),
                },
                MarkupEnumMember {
                    name: "Overflow",
                    value: 2,
                    rust_variant: Some("Overflow"),
                },
                MarkupEnumMember {
                    name: "Hidden",
                    value: 3,
                    rust_variant: Some("Hidden"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon_core::RibbonSimplifiedVisibility::from_raw(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon_core::RibbonSimplifiedVisibility>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
#[doc(hidden)]
pub struct RibbonGroupItemsLayoutMetadata;
#[allow(non_upper_case_globals)]
impl RibbonGroupItemsLayoutMetadata {
    pub const Columns: ferroribbon_core::RibbonGroupItemsLayout =
        ferroribbon_core::RibbonGroupItemsLayout::Columns;
    pub const Rows: ferroribbon_core::RibbonGroupItemsLayout =
        ferroribbon_core::RibbonGroupItemsLayout::Rows;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon_core::RibbonGroupItemsLayout {
        ferroribbon_core::RibbonGroupItemsLayout::from_raw(value)
    }
}
const _: () = assert!(ferroribbon_core::RibbonGroupItemsLayout::Columns.raw() == 0);
const _: () = assert!(ferroribbon_core::RibbonGroupItemsLayout::Rows.raw() == 1);
impl MarkupTyped for RibbonGroupItemsLayoutMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new(
                "RibbonGroupItemsLayout",
                MarkupTypeKind::Enum,
                module_path!(),
            );
            m.explicit_namespace = Some("FerroRibbon");
            m.handles = &[ValueType::of::<ferroribbon_core::RibbonGroupItemsLayout>];
            m.nullable = Some(ValueType::of::<Option<ferroribbon_core::RibbonGroupItemsLayout>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Columns",
                    value: 0,
                    rust_variant: Some("Columns"),
                },
                MarkupEnumMember {
                    name: "Rows",
                    value: 1,
                    rust_variant: Some("Rows"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon_core::RibbonGroupItemsLayout::from_raw(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon_core::RibbonGroupItemsLayout>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
#[doc(hidden)]
pub struct RibbonContextualActivationMetadata;
#[allow(non_upper_case_globals)]
impl RibbonContextualActivationMetadata {
    pub const None: ferroribbon_core::RibbonContextualActivation =
        ferroribbon_core::RibbonContextualActivation::None;
    pub const SelectOnShow: ferroribbon_core::RibbonContextualActivation =
        ferroribbon_core::RibbonContextualActivation::SelectOnShow;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon_core::RibbonContextualActivation {
        ferroribbon_core::RibbonContextualActivation::from_raw(value)
    }
}
const _: () = assert!(ferroribbon_core::RibbonContextualActivation::None.raw() == 0);
const _: () = assert!(ferroribbon_core::RibbonContextualActivation::SelectOnShow.raw() == 1);
impl MarkupTyped for RibbonContextualActivationMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new(
                "RibbonContextualActivation",
                MarkupTypeKind::Enum,
                module_path!(),
            );
            m.explicit_namespace = Some("FerroRibbon");
            m.handles = &[ValueType::of::<ferroribbon_core::RibbonContextualActivation>];
            m.nullable =
                Some(ValueType::of::<Option<ferroribbon_core::RibbonContextualActivation>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "None",
                    value: 0,
                    rust_variant: Some("None"),
                },
                MarkupEnumMember {
                    name: "SelectOnShow",
                    value: 1,
                    rust_variant: Some("SelectOnShow"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon_core::RibbonContextualActivation::from_raw(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon_core::RibbonContextualActivation>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
#[doc(hidden)]
pub struct RibbonMergeActionMetadata;
#[allow(non_upper_case_globals)]
impl RibbonMergeActionMetadata {
    pub const Merge: ferroribbon_core::RibbonMergeAction =
        ferroribbon_core::RibbonMergeAction::Merge;
    pub const Replace: ferroribbon_core::RibbonMergeAction =
        ferroribbon_core::RibbonMergeAction::Replace;
    pub const Remove: ferroribbon_core::RibbonMergeAction =
        ferroribbon_core::RibbonMergeAction::Remove;
    pub const Add: ferroribbon_core::RibbonMergeAction = ferroribbon_core::RibbonMergeAction::Add;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon_core::RibbonMergeAction {
        ferroribbon_core::RibbonMergeAction::from_raw(value)
    }
}
const _: () = assert!(ferroribbon_core::RibbonMergeAction::Merge.raw() == 0);
const _: () = assert!(ferroribbon_core::RibbonMergeAction::Replace.raw() == 1);
const _: () = assert!(ferroribbon_core::RibbonMergeAction::Remove.raw() == 2);
const _: () = assert!(ferroribbon_core::RibbonMergeAction::Add.raw() == 3);
impl MarkupTyped for RibbonMergeActionMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new("RibbonMergeAction", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroRibbon");
            m.handles = &[ValueType::of::<ferroribbon_core::RibbonMergeAction>];
            m.nullable = Some(ValueType::of::<Option<ferroribbon_core::RibbonMergeAction>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Merge",
                    value: 0,
                    rust_variant: Some("Merge"),
                },
                MarkupEnumMember {
                    name: "Replace",
                    value: 1,
                    rust_variant: Some("Replace"),
                },
                MarkupEnumMember {
                    name: "Remove",
                    value: 2,
                    rust_variant: Some("Remove"),
                },
                MarkupEnumMember {
                    name: "Add",
                    value: 3,
                    rust_variant: Some("Add"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon_core::RibbonMergeAction::from_raw(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon_core::RibbonMergeAction>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
#[doc(hidden)]
pub struct RibbonBackstagePlacementMetadata;
#[allow(non_upper_case_globals)]
impl RibbonBackstagePlacementMetadata {
    pub const Top: ferroribbon_core::RibbonBackstagePlacement =
        ferroribbon_core::RibbonBackstagePlacement::Top;
    pub const Bottom: ferroribbon_core::RibbonBackstagePlacement =
        ferroribbon_core::RibbonBackstagePlacement::Bottom;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon_core::RibbonBackstagePlacement {
        ferroribbon_core::RibbonBackstagePlacement::from_raw(value)
    }
}
const _: () = assert!(ferroribbon_core::RibbonBackstagePlacement::Top.raw() == 0);
const _: () = assert!(ferroribbon_core::RibbonBackstagePlacement::Bottom.raw() == 1);
impl MarkupTyped for RibbonBackstagePlacementMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new(
                "RibbonBackstagePlacement",
                MarkupTypeKind::Enum,
                module_path!(),
            );
            m.explicit_namespace = Some("FerroRibbon");
            m.handles = &[ValueType::of::<ferroribbon_core::RibbonBackstagePlacement>];
            m.nullable = Some(ValueType::of::<Option<ferroribbon_core::RibbonBackstagePlacement>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Top",
                    value: 0,
                    rust_variant: Some("Top"),
                },
                MarkupEnumMember {
                    name: "Bottom",
                    value: 1,
                    rust_variant: Some("Bottom"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon_core::RibbonBackstagePlacement::from_raw(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon_core::RibbonBackstagePlacement>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
#[doc(hidden)]
pub struct RibbonToolBarOrientationMetadata;
#[allow(non_upper_case_globals)]
impl RibbonToolBarOrientationMetadata {
    pub const Horizontal: ferroribbon_core::RibbonToolBarOrientation =
        ferroribbon_core::RibbonToolBarOrientation::Horizontal;
    pub const Vertical: ferroribbon_core::RibbonToolBarOrientation =
        ferroribbon_core::RibbonToolBarOrientation::Vertical;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon_core::RibbonToolBarOrientation {
        ferroribbon_core::RibbonToolBarOrientation::from_raw(value)
    }
}
const _: () = assert!(ferroribbon_core::RibbonToolBarOrientation::Horizontal.raw() == 0);
const _: () = assert!(ferroribbon_core::RibbonToolBarOrientation::Vertical.raw() == 1);
impl MarkupTyped for RibbonToolBarOrientationMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new(
                "RibbonToolBarOrientation",
                MarkupTypeKind::Enum,
                module_path!(),
            );
            m.explicit_namespace = Some("FerroRibbon");
            m.handles = &[ValueType::of::<ferroribbon_core::RibbonToolBarOrientation>];
            m.nullable = Some(ValueType::of::<Option<ferroribbon_core::RibbonToolBarOrientation>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Horizontal",
                    value: 0,
                    rust_variant: Some("Horizontal"),
                },
                MarkupEnumMember {
                    name: "Vertical",
                    value: 1,
                    rust_variant: Some("Vertical"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon_core::RibbonToolBarOrientation::from_raw(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon_core::RibbonToolBarOrientation>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
#[doc(hidden)]
pub struct RibbonChromeStyleMetadata;
#[allow(non_upper_case_globals)]
impl RibbonChromeStyleMetadata {
    pub const Colorful: ferroribbon_core::theming::RibbonChromeStyle =
        ferroribbon_core::theming::RibbonChromeStyle::Colorful;
    pub const Neutral: ferroribbon_core::theming::RibbonChromeStyle =
        ferroribbon_core::theming::RibbonChromeStyle::Neutral;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon_core::theming::RibbonChromeStyle {
        ferroribbon_core::theming::RibbonChromeStyle::from_raw(value)
    }
}
const _: () = assert!(ferroribbon_core::theming::RibbonChromeStyle::Colorful.raw() == 0);
const _: () = assert!(ferroribbon_core::theming::RibbonChromeStyle::Neutral.raw() == 1);
impl MarkupTyped for RibbonChromeStyleMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new("RibbonChromeStyle", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroRibbon.Theming");
            m.handles = &[ValueType::of::<ferroribbon_core::theming::RibbonChromeStyle>];
            m.nullable =
                Some(ValueType::of::<Option<ferroribbon_core::theming::RibbonChromeStyle>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Colorful",
                    value: 0,
                    rust_variant: Some("Colorful"),
                },
                MarkupEnumMember {
                    name: "Neutral",
                    value: 1,
                    rust_variant: Some("Neutral"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon_core::theming::RibbonChromeStyle::from_raw(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon_core::theming::RibbonChromeStyle>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
#[doc(hidden)]
pub struct RibbonThemeStyleMetadata;
#[allow(non_upper_case_globals)]
impl RibbonThemeStyleMetadata {
    pub const Office: ferroribbon_core::theming::RibbonThemeStyle =
        ferroribbon_core::theming::RibbonThemeStyle::Office;
    pub const Cad: ferroribbon_core::theming::RibbonThemeStyle =
        ferroribbon_core::theming::RibbonThemeStyle::Cad;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon_core::theming::RibbonThemeStyle {
        ferroribbon_core::theming::RibbonThemeStyle::from_raw(value)
    }
}
const _: () = assert!(ferroribbon_core::theming::RibbonThemeStyle::Office.raw() == 0);
const _: () = assert!(ferroribbon_core::theming::RibbonThemeStyle::Cad.raw() == 1);
impl MarkupTyped for RibbonThemeStyleMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new("RibbonThemeStyle", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroRibbon.Theming");
            m.handles = &[ValueType::of::<ferroribbon_core::theming::RibbonThemeStyle>];
            m.nullable = Some(ValueType::of::<Option<ferroribbon_core::theming::RibbonThemeStyle>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Office",
                    value: 0,
                    rust_variant: Some("Office"),
                },
                MarkupEnumMember {
                    name: "Cad",
                    value: 1,
                    rust_variant: Some("Cad"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon_core::theming::RibbonThemeStyle::from_raw(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon_core::theming::RibbonThemeStyle>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
#[doc(hidden)]
pub struct RibbonReductionStrategyMetadata;
#[allow(non_upper_case_globals)]
impl RibbonReductionStrategyMetadata {
    pub const Stepwise: ferroribbon_core::layout::RibbonReductionStrategy =
        ferroribbon_core::layout::RibbonReductionStrategy::Stepwise;
    pub const GroupByGroup: ferroribbon_core::layout::RibbonReductionStrategy =
        ferroribbon_core::layout::RibbonReductionStrategy::GroupByGroup;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon_core::layout::RibbonReductionStrategy {
        ferroribbon_core::layout::RibbonReductionStrategy::from_raw(value)
    }
}
const _: () = assert!(ferroribbon_core::layout::RibbonReductionStrategy::Stepwise.raw() == 0);
const _: () = assert!(ferroribbon_core::layout::RibbonReductionStrategy::GroupByGroup.raw() == 1);
impl MarkupTyped for RibbonReductionStrategyMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new(
                "RibbonReductionStrategy",
                MarkupTypeKind::Enum,
                module_path!(),
            );
            m.explicit_namespace = Some("FerroRibbon.Layout");
            m.handles = &[ValueType::of::<ferroribbon_core::layout::RibbonReductionStrategy>];
            m.nullable =
                Some(ValueType::of::<Option<ferroribbon_core::layout::RibbonReductionStrategy>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Stepwise",
                    value: 0,
                    rust_variant: Some("Stepwise"),
                },
                MarkupEnumMember {
                    name: "GroupByGroup",
                    value: 1,
                    rust_variant: Some("GroupByGroup"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon_core::layout::RibbonReductionStrategy::from_raw(
                        v,
                    )) as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon_core::layout::RibbonReductionStrategy>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
#[doc(hidden)]
pub struct RibbonPanelPresentationMetadata;
#[allow(non_upper_case_globals)]
impl RibbonPanelPresentationMetadata {
    pub const Full: ferroribbon::controls::RibbonPanelPresentation =
        ferroribbon::controls::RibbonPanelPresentation::Full;
    pub const Buttons: ferroribbon::controls::RibbonPanelPresentation =
        ferroribbon::controls::RibbonPanelPresentation::Buttons;
    pub const Titles: ferroribbon::controls::RibbonPanelPresentation =
        ferroribbon::controls::RibbonPanelPresentation::Titles;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon::controls::RibbonPanelPresentation {
        ferroribbon::controls::RibbonPanelPresentation::from_raw(value)
    }
}
const _: () = assert!(ferroribbon::controls::RibbonPanelPresentation::Full.raw() == 0);
const _: () = assert!(ferroribbon::controls::RibbonPanelPresentation::Buttons.raw() == 1);
const _: () = assert!(ferroribbon::controls::RibbonPanelPresentation::Titles.raw() == 2);
impl MarkupTyped for RibbonPanelPresentationMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new(
                "RibbonPanelPresentation",
                MarkupTypeKind::Enum,
                module_path!(),
            );
            m.explicit_namespace = Some("FerroRibbon.Controls");
            m.handles = &[ValueType::of::<ferroribbon::controls::RibbonPanelPresentation>];
            m.nullable =
                Some(ValueType::of::<Option<ferroribbon::controls::RibbonPanelPresentation>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Full",
                    value: 0,
                    rust_variant: Some("Full"),
                },
                MarkupEnumMember {
                    name: "Buttons",
                    value: 1,
                    rust_variant: Some("Buttons"),
                },
                MarkupEnumMember {
                    name: "Titles",
                    value: 2,
                    rust_variant: Some("Titles"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon::controls::RibbonPanelPresentation::from_raw(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon::controls::RibbonPanelPresentation>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
#[doc(hidden)]
pub struct RibbonIconKindMetadata;
#[allow(non_upper_case_globals)]
impl RibbonIconKindMetadata {
    pub const Glyph: ferroribbon_core::model::RibbonIconKind =
        ferroribbon_core::model::RibbonIconKind::Glyph;
    pub const Path: ferroribbon_core::model::RibbonIconKind =
        ferroribbon_core::model::RibbonIconKind::Path;
    pub const Image: ferroribbon_core::model::RibbonIconKind =
        ferroribbon_core::model::RibbonIconKind::Image;
    pub const Text: ferroribbon_core::model::RibbonIconKind =
        ferroribbon_core::model::RibbonIconKind::Text;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon_core::model::RibbonIconKind {
        ferroribbon_core::model::RibbonIconKind::from_raw(value)
    }
}
const _: () = assert!(ferroribbon_core::model::RibbonIconKind::Glyph.raw() == 0);
const _: () = assert!(ferroribbon_core::model::RibbonIconKind::Path.raw() == 1);
const _: () = assert!(ferroribbon_core::model::RibbonIconKind::Image.raw() == 2);
const _: () = assert!(ferroribbon_core::model::RibbonIconKind::Text.raw() == 3);
impl MarkupTyped for RibbonIconKindMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new("RibbonIconKind", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroRibbon.Model");
            m.handles = &[ValueType::of::<ferroribbon_core::model::RibbonIconKind>];
            m.nullable = Some(ValueType::of::<Option<ferroribbon_core::model::RibbonIconKind>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Glyph",
                    value: 0,
                    rust_variant: Some("Glyph"),
                },
                MarkupEnumMember {
                    name: "Path",
                    value: 1,
                    rust_variant: Some("Path"),
                },
                MarkupEnumMember {
                    name: "Image",
                    value: 2,
                    rust_variant: Some("Image"),
                },
                MarkupEnumMember {
                    name: "Text",
                    value: 3,
                    rust_variant: Some("Text"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon_core::model::RibbonIconKind::from_raw(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon_core::model::RibbonIconKind>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
#[doc(hidden)]
pub struct VisibilityMetadata;
#[allow(non_upper_case_globals)]
impl VisibilityMetadata {
    pub const Visible: ferroribbon::compat::Visibility = ferroribbon::compat::Visibility::Visible;
    pub const Collapsed: ferroribbon::compat::Visibility =
        ferroribbon::compat::Visibility::Collapsed;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon::compat::Visibility {
        ferroribbon::compat::Visibility::from_raw(value)
    }
}
const _: () = assert!(ferroribbon::compat::Visibility::Visible.raw() == 0);
const _: () = assert!(ferroribbon::compat::Visibility::Collapsed.raw() == 1);
impl MarkupTyped for VisibilityMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new("Visibility", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroRibbon.Compatibility");
            m.handles = &[ValueType::of::<ferroribbon::compat::Visibility>];
            m.nullable = Some(ValueType::of::<Option<ferroribbon::compat::Visibility>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Visible",
                    value: 0,
                    rust_variant: Some("Visible"),
                },
                MarkupEnumMember {
                    name: "Collapsed",
                    value: 1,
                    rust_variant: Some("Collapsed"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon::compat::Visibility::from_raw(v))
                        as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon::compat::Visibility>()
                    .map(|v| i64::from(v.raw()))
            });
            m.enum_i32_factory =
                ferroui_base::__ferro_compiler_metadata!(@value Some("__markup_from_i32"));
            m
        };
        &META
    };
}
pub fn register() {
    MarkupType::register_all(&[
        RibbonItemSizeMetadata::MARKUP,
        RibbonGroupStateMetadata::MARKUP,
        RibbonDisplayModeMetadata::MARKUP,
        RibbonVisibilityModeMetadata::MARKUP,
        RibbonMinimizeBehaviorMetadata::MARKUP,
        RibbonQuickAccessPositionMetadata::MARKUP,
        RibbonDensityMetadata::MARKUP,
        RibbonSimplifiedVisibilityMetadata::MARKUP,
        RibbonGroupItemsLayoutMetadata::MARKUP,
        RibbonContextualActivationMetadata::MARKUP,
        RibbonMergeActionMetadata::MARKUP,
        RibbonBackstagePlacementMetadata::MARKUP,
        RibbonToolBarOrientationMetadata::MARKUP,
        RibbonChromeStyleMetadata::MARKUP,
        RibbonThemeStyleMetadata::MARKUP,
        RibbonReductionStrategyMetadata::MARKUP,
        RibbonPanelPresentationMetadata::MARKUP,
        RibbonIconKindMetadata::MARKUP,
        VisibilityMetadata::MARKUP,
    ]);
    ValueTypes::register_nullable::<ferroribbon_core::RibbonItemSize>();
    ValueTypes::register_display::<ferroribbon_core::RibbonItemSize>();
    ValueTypes::register_nullable::<ferroribbon_core::RibbonGroupState>();
    ValueTypes::register_display::<ferroribbon_core::RibbonGroupState>();
    ValueTypes::register_nullable::<ferroribbon_core::RibbonDisplayMode>();
    ValueTypes::register_display::<ferroribbon_core::RibbonDisplayMode>();
    ValueTypes::register_nullable::<ferroribbon_core::RibbonVisibilityMode>();
    ValueTypes::register_display::<ferroribbon_core::RibbonVisibilityMode>();
    ValueTypes::register_nullable::<ferroribbon_core::RibbonMinimizeBehavior>();
    ValueTypes::register_display::<ferroribbon_core::RibbonMinimizeBehavior>();
    ValueTypes::register_nullable::<ferroribbon_core::RibbonQuickAccessPosition>();
    ValueTypes::register_display::<ferroribbon_core::RibbonQuickAccessPosition>();
    ValueTypes::register_nullable::<ferroribbon_core::RibbonDensity>();
    ValueTypes::register_display::<ferroribbon_core::RibbonDensity>();
    ValueTypes::register_nullable::<ferroribbon_core::RibbonSimplifiedVisibility>();
    ValueTypes::register_display::<ferroribbon_core::RibbonSimplifiedVisibility>();
    ValueTypes::register_nullable::<ferroribbon_core::RibbonGroupItemsLayout>();
    ValueTypes::register_display::<ferroribbon_core::RibbonGroupItemsLayout>();
    ValueTypes::register_nullable::<ferroribbon_core::RibbonContextualActivation>();
    ValueTypes::register_display::<ferroribbon_core::RibbonContextualActivation>();
    ValueTypes::register_nullable::<ferroribbon_core::RibbonMergeAction>();
    ValueTypes::register_display::<ferroribbon_core::RibbonMergeAction>();
    ValueTypes::register_nullable::<ferroribbon_core::RibbonBackstagePlacement>();
    ValueTypes::register_display::<ferroribbon_core::RibbonBackstagePlacement>();
    ValueTypes::register_nullable::<ferroribbon_core::RibbonToolBarOrientation>();
    ValueTypes::register_display::<ferroribbon_core::RibbonToolBarOrientation>();
    ValueTypes::register_nullable::<ferroribbon_core::theming::RibbonChromeStyle>();
    ValueTypes::register_display::<ferroribbon_core::theming::RibbonChromeStyle>();
    ValueTypes::register_nullable::<ferroribbon_core::theming::RibbonThemeStyle>();
    ValueTypes::register_display::<ferroribbon_core::theming::RibbonThemeStyle>();
    ValueTypes::register_nullable::<ferroribbon_core::layout::RibbonReductionStrategy>();
    ValueTypes::register_display::<ferroribbon_core::layout::RibbonReductionStrategy>();
    ValueTypes::register_nullable::<ferroribbon::controls::RibbonPanelPresentation>();
    ValueTypes::register_display::<ferroribbon::controls::RibbonPanelPresentation>();
    ValueTypes::register_nullable::<ferroribbon_core::model::RibbonIconKind>();
    ValueTypes::register_display::<ferroribbon_core::model::RibbonIconKind>();
    ValueTypes::register_nullable::<ferroribbon::compat::Visibility>();
    ValueTypes::register_display::<ferroribbon::compat::Visibility>();
}
