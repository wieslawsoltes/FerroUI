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
#[doc(hidden)]
pub struct SymbolMetadata;
#[allow(non_upper_case_globals)]
impl SymbolMetadata {
    pub const Previous: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Previous;
    pub const Next: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Next;
    pub const Play: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Play;
    pub const Pause: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Pause;
    pub const Edit: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Edit;
    pub const Save: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Save;
    pub const Clear: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Clear;
    pub const Delete: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Delete;
    pub const Remove: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Remove;
    pub const Add: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Add;
    pub const Cancel: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Cancel;
    pub const Accept: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Accept;
    pub const More: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::More;
    pub const Redo: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Redo;
    pub const Undo: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Undo;
    pub const Home: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Home;
    pub const Up: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Up;
    pub const Forward: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Forward;
    pub const Back: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Back;
    pub const Favorite: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Favorite;
    pub const Camera: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Camera;
    pub const Setting: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Setting;
    pub const Video: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Video;
    pub const Sync: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Sync;
    pub const Download: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Download;
    pub const Mail: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Mail;
    pub const Find: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Find;
    pub const Help: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Help;
    pub const Upload: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Upload;
    pub const Emoji: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Emoji;
    pub const TwoPage: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::TwoPage;
    pub const LeaveChat: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::LeaveChat;
    pub const MailForward: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::MailForward;
    pub const Clock: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Clock;
    pub const Send: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Send;
    pub const Crop: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Crop;
    pub const RotateCamera: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::RotateCamera;
    pub const People: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::People;
    pub const OpenPane: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::OpenPane;
    pub const ClosePane: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::ClosePane;
    pub const World: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::World;
    pub const Flag: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Flag;
    pub const PreviewLink: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::PreviewLink;
    pub const Globe: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Globe;
    pub const Trim: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Trim;
    pub const AttachCamera: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::AttachCamera;
    pub const ZoomIn: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::ZoomIn;
    pub const Bookmarks: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Bookmarks;
    pub const Document: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Document;
    pub const ProtectedDocument: ferroribbon::compat::Symbol =
        ferroribbon::compat::Symbol::ProtectedDocument;
    pub const Page: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Page;
    pub const Bullets: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Bullets;
    pub const Comment: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Comment;
    pub const MailFilled: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::MailFilled;
    pub const ContactInfo: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::ContactInfo;
    pub const HangUp: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::HangUp;
    pub const ViewAll: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::ViewAll;
    pub const MapPin: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::MapPin;
    pub const Phone: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Phone;
    pub const VideoChat: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::VideoChat;
    pub const Switch: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Switch;
    pub const Contact: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Contact;
    pub const Rename: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Rename;
    pub const Pin: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Pin;
    pub const MusicInfo: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::MusicInfo;
    pub const Go: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Go;
    pub const Keyboard: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Keyboard;
    pub const DockLeft: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::DockLeft;
    pub const DockRight: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::DockRight;
    pub const DockBottom: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::DockBottom;
    pub const Remote: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Remote;
    pub const Refresh: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Refresh;
    pub const Rotate: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Rotate;
    pub const Shuffle: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Shuffle;
    pub const List: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::List;
    pub const Shop: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Shop;
    pub const SelectAll: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::SelectAll;
    pub const Orientation: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Orientation;
    pub const Import: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Import;
    pub const ImportAll: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::ImportAll;
    pub const BrowsePhotos: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::BrowsePhotos;
    pub const WebCam: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::WebCam;
    pub const Pictures: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Pictures;
    pub const SaveLocal: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::SaveLocal;
    pub const Caption: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Caption;
    pub const Stop: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Stop;
    pub const ShowResults: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::ShowResults;
    pub const Volume: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Volume;
    pub const Repair: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Repair;
    pub const Message: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Message;
    pub const Page2: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Page2;
    pub const CalendarDay: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::CalendarDay;
    pub const CalendarWeek: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::CalendarWeek;
    pub const Calendar: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Calendar;
    pub const Character: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Character;
    pub const MailReplyAll: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::MailReplyAll;
    pub const Read: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Read;
    pub const Link: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Link;
    pub const Account: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Account;
    pub const ShowBcc: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::ShowBcc;
    pub const HideBcc: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::HideBcc;
    pub const Cut: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Cut;
    pub const Attach: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Attach;
    pub const Paste: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Paste;
    pub const Filter: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Filter;
    pub const Copy: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Copy;
    pub const Emoji2: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Emoji2;
    pub const Important: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Important;
    pub const MailReply: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::MailReply;
    pub const SlideShow: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::SlideShow;
    pub const Sort: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Sort;
    pub const Manage: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Manage;
    pub const AllApps: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::AllApps;
    pub const DisconnectDrive: ferroribbon::compat::Symbol =
        ferroribbon::compat::Symbol::DisconnectDrive;
    pub const MapDrive: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::MapDrive;
    pub const NewWindow: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::NewWindow;
    pub const OpenWith: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::OpenWith;
    pub const ContactPresence: ferroribbon::compat::Symbol =
        ferroribbon::compat::Symbol::ContactPresence;
    pub const Priority: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Priority;
    pub const GoToToday: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::GoToToday;
    pub const Font: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Font;
    pub const FontColor: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::FontColor;
    pub const Contact2: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Contact2;
    pub const Folder: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Folder;
    pub const Audio: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Audio;
    pub const Placeholder: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Placeholder;
    pub const View: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::View;
    pub const SetLockScreen: ferroribbon::compat::Symbol =
        ferroribbon::compat::Symbol::SetLockScreen;
    pub const SetTile: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::SetTile;
    pub const ClosedCaption: ferroribbon::compat::Symbol =
        ferroribbon::compat::Symbol::ClosedCaption;
    pub const StopSlideShow: ferroribbon::compat::Symbol =
        ferroribbon::compat::Symbol::StopSlideShow;
    pub const Permissions: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Permissions;
    pub const Highlight: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Highlight;
    pub const DisableUpdates: ferroribbon::compat::Symbol =
        ferroribbon::compat::Symbol::DisableUpdates;
    pub const UnFavorite: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::UnFavorite;
    pub const UnPin: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::UnPin;
    pub const OpenLocal: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::OpenLocal;
    pub const Mute: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Mute;
    pub const Italic: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Italic;
    pub const Underline: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Underline;
    pub const Bold: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Bold;
    pub const MoveToFolder: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::MoveToFolder;
    pub const LikeDislike: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::LikeDislike;
    pub const Dislike: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Dislike;
    pub const Like: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Like;
    pub const AlignRight: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::AlignRight;
    pub const AlignCenter: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::AlignCenter;
    pub const AlignLeft: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::AlignLeft;
    pub const Zoom: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Zoom;
    pub const ZoomOut: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::ZoomOut;
    pub const OpenFile: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::OpenFile;
    pub const OtherUser: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::OtherUser;
    pub const Admin: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Admin;
    pub const Street: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Street;
    pub const Map: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Map;
    pub const ClearSelection: ferroribbon::compat::Symbol =
        ferroribbon::compat::Symbol::ClearSelection;
    pub const FontDecrease: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::FontDecrease;
    pub const FontIncrease: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::FontIncrease;
    pub const FontSize: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::FontSize;
    pub const CellPhone: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::CellPhone;
    pub const ReShare: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::ReShare;
    pub const Tag: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Tag;
    pub const RepeatOne: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::RepeatOne;
    pub const RepeatAll: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::RepeatAll;
    pub const OutlineStar: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::OutlineStar;
    pub const SolidStar: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::SolidStar;
    pub const Calculator: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Calculator;
    pub const Directions: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Directions;
    pub const Target: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Target;
    pub const Library: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Library;
    pub const PhoneBook: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::PhoneBook;
    pub const Memo: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Memo;
    pub const Microphone: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Microphone;
    pub const PostUpdate: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::PostUpdate;
    pub const BackToWindow: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::BackToWindow;
    pub const FullScreen: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::FullScreen;
    pub const NewFolder: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::NewFolder;
    pub const CalendarReply: ferroribbon::compat::Symbol =
        ferroribbon::compat::Symbol::CalendarReply;
    pub const UnSyncFolder: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::UnSyncFolder;
    pub const ReportHacked: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::ReportHacked;
    pub const SyncFolder: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::SyncFolder;
    pub const BlockContact: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::BlockContact;
    pub const SwitchApps: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::SwitchApps;
    pub const AddFriend: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::AddFriend;
    pub const TouchPointer: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::TouchPointer;
    pub const GoToStart: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::GoToStart;
    pub const ZeroBars: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::ZeroBars;
    pub const OneBar: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::OneBar;
    pub const TwoBars: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::TwoBars;
    pub const ThreeBars: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::ThreeBars;
    pub const FourBars: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::FourBars;
    pub const Scan: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Scan;
    pub const Preview: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Preview;
    pub const GlobalNavigationButton: ferroribbon::compat::Symbol =
        ferroribbon::compat::Symbol::GlobalNavigationButton;
    pub const Share: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Share;
    pub const Print: ferroribbon::compat::Symbol = ferroribbon::compat::Symbol::Print;
    pub const XboxOneConsole: ferroribbon::compat::Symbol =
        ferroribbon::compat::Symbol::XboxOneConsole;
    #[doc(hidden)]
    pub fn __markup_from_i32(value: i32) -> ferroribbon::compat::Symbol {
        ferroribbon::compat::Symbol::from_raw(value)
    }
}
const _: () = assert!(ferroribbon::compat::Symbol::Previous.raw() == 57600);
const _: () = assert!(ferroribbon::compat::Symbol::Next.raw() == 57601);
const _: () = assert!(ferroribbon::compat::Symbol::Play.raw() == 57602);
const _: () = assert!(ferroribbon::compat::Symbol::Pause.raw() == 57603);
const _: () = assert!(ferroribbon::compat::Symbol::Edit.raw() == 57604);
const _: () = assert!(ferroribbon::compat::Symbol::Save.raw() == 57605);
const _: () = assert!(ferroribbon::compat::Symbol::Clear.raw() == 57606);
const _: () = assert!(ferroribbon::compat::Symbol::Delete.raw() == 57607);
const _: () = assert!(ferroribbon::compat::Symbol::Remove.raw() == 57608);
const _: () = assert!(ferroribbon::compat::Symbol::Add.raw() == 57609);
const _: () = assert!(ferroribbon::compat::Symbol::Cancel.raw() == 57610);
const _: () = assert!(ferroribbon::compat::Symbol::Accept.raw() == 57611);
const _: () = assert!(ferroribbon::compat::Symbol::More.raw() == 57612);
const _: () = assert!(ferroribbon::compat::Symbol::Redo.raw() == 57613);
const _: () = assert!(ferroribbon::compat::Symbol::Undo.raw() == 57614);
const _: () = assert!(ferroribbon::compat::Symbol::Home.raw() == 57615);
const _: () = assert!(ferroribbon::compat::Symbol::Up.raw() == 57616);
const _: () = assert!(ferroribbon::compat::Symbol::Forward.raw() == 57617);
const _: () = assert!(ferroribbon::compat::Symbol::Back.raw() == 57618);
const _: () = assert!(ferroribbon::compat::Symbol::Favorite.raw() == 57619);
const _: () = assert!(ferroribbon::compat::Symbol::Camera.raw() == 57620);
const _: () = assert!(ferroribbon::compat::Symbol::Setting.raw() == 57621);
const _: () = assert!(ferroribbon::compat::Symbol::Video.raw() == 57622);
const _: () = assert!(ferroribbon::compat::Symbol::Sync.raw() == 57623);
const _: () = assert!(ferroribbon::compat::Symbol::Download.raw() == 57624);
const _: () = assert!(ferroribbon::compat::Symbol::Mail.raw() == 57625);
const _: () = assert!(ferroribbon::compat::Symbol::Find.raw() == 57626);
const _: () = assert!(ferroribbon::compat::Symbol::Help.raw() == 57627);
const _: () = assert!(ferroribbon::compat::Symbol::Upload.raw() == 57628);
const _: () = assert!(ferroribbon::compat::Symbol::Emoji.raw() == 57629);
const _: () = assert!(ferroribbon::compat::Symbol::TwoPage.raw() == 57630);
const _: () = assert!(ferroribbon::compat::Symbol::LeaveChat.raw() == 57631);
const _: () = assert!(ferroribbon::compat::Symbol::MailForward.raw() == 57632);
const _: () = assert!(ferroribbon::compat::Symbol::Clock.raw() == 57633);
const _: () = assert!(ferroribbon::compat::Symbol::Send.raw() == 57634);
const _: () = assert!(ferroribbon::compat::Symbol::Crop.raw() == 57635);
const _: () = assert!(ferroribbon::compat::Symbol::RotateCamera.raw() == 57636);
const _: () = assert!(ferroribbon::compat::Symbol::People.raw() == 57637);
const _: () = assert!(ferroribbon::compat::Symbol::OpenPane.raw() == 57638);
const _: () = assert!(ferroribbon::compat::Symbol::ClosePane.raw() == 57639);
const _: () = assert!(ferroribbon::compat::Symbol::World.raw() == 57640);
const _: () = assert!(ferroribbon::compat::Symbol::Flag.raw() == 57641);
const _: () = assert!(ferroribbon::compat::Symbol::PreviewLink.raw() == 57642);
const _: () = assert!(ferroribbon::compat::Symbol::Globe.raw() == 57643);
const _: () = assert!(ferroribbon::compat::Symbol::Trim.raw() == 57644);
const _: () = assert!(ferroribbon::compat::Symbol::AttachCamera.raw() == 57645);
const _: () = assert!(ferroribbon::compat::Symbol::ZoomIn.raw() == 57646);
const _: () = assert!(ferroribbon::compat::Symbol::Bookmarks.raw() == 57647);
const _: () = assert!(ferroribbon::compat::Symbol::Document.raw() == 57648);
const _: () = assert!(ferroribbon::compat::Symbol::ProtectedDocument.raw() == 57649);
const _: () = assert!(ferroribbon::compat::Symbol::Page.raw() == 57650);
const _: () = assert!(ferroribbon::compat::Symbol::Bullets.raw() == 57651);
const _: () = assert!(ferroribbon::compat::Symbol::Comment.raw() == 57652);
const _: () = assert!(ferroribbon::compat::Symbol::MailFilled.raw() == 57653);
const _: () = assert!(ferroribbon::compat::Symbol::ContactInfo.raw() == 57654);
const _: () = assert!(ferroribbon::compat::Symbol::HangUp.raw() == 57655);
const _: () = assert!(ferroribbon::compat::Symbol::ViewAll.raw() == 57656);
const _: () = assert!(ferroribbon::compat::Symbol::MapPin.raw() == 57657);
const _: () = assert!(ferroribbon::compat::Symbol::Phone.raw() == 57658);
const _: () = assert!(ferroribbon::compat::Symbol::VideoChat.raw() == 57659);
const _: () = assert!(ferroribbon::compat::Symbol::Switch.raw() == 57660);
const _: () = assert!(ferroribbon::compat::Symbol::Contact.raw() == 57661);
const _: () = assert!(ferroribbon::compat::Symbol::Rename.raw() == 57662);
const _: () = assert!(ferroribbon::compat::Symbol::Pin.raw() == 57665);
const _: () = assert!(ferroribbon::compat::Symbol::MusicInfo.raw() == 57666);
const _: () = assert!(ferroribbon::compat::Symbol::Go.raw() == 57667);
const _: () = assert!(ferroribbon::compat::Symbol::Keyboard.raw() == 57668);
const _: () = assert!(ferroribbon::compat::Symbol::DockLeft.raw() == 57669);
const _: () = assert!(ferroribbon::compat::Symbol::DockRight.raw() == 57670);
const _: () = assert!(ferroribbon::compat::Symbol::DockBottom.raw() == 57671);
const _: () = assert!(ferroribbon::compat::Symbol::Remote.raw() == 57672);
const _: () = assert!(ferroribbon::compat::Symbol::Refresh.raw() == 57673);
const _: () = assert!(ferroribbon::compat::Symbol::Rotate.raw() == 57674);
const _: () = assert!(ferroribbon::compat::Symbol::Shuffle.raw() == 57675);
const _: () = assert!(ferroribbon::compat::Symbol::List.raw() == 57676);
const _: () = assert!(ferroribbon::compat::Symbol::Shop.raw() == 57677);
const _: () = assert!(ferroribbon::compat::Symbol::SelectAll.raw() == 57678);
const _: () = assert!(ferroribbon::compat::Symbol::Orientation.raw() == 57679);
const _: () = assert!(ferroribbon::compat::Symbol::Import.raw() == 57680);
const _: () = assert!(ferroribbon::compat::Symbol::ImportAll.raw() == 57681);
const _: () = assert!(ferroribbon::compat::Symbol::BrowsePhotos.raw() == 57685);
const _: () = assert!(ferroribbon::compat::Symbol::WebCam.raw() == 57686);
const _: () = assert!(ferroribbon::compat::Symbol::Pictures.raw() == 57688);
const _: () = assert!(ferroribbon::compat::Symbol::SaveLocal.raw() == 57689);
const _: () = assert!(ferroribbon::compat::Symbol::Caption.raw() == 57690);
const _: () = assert!(ferroribbon::compat::Symbol::Stop.raw() == 57691);
const _: () = assert!(ferroribbon::compat::Symbol::ShowResults.raw() == 57692);
const _: () = assert!(ferroribbon::compat::Symbol::Volume.raw() == 57693);
const _: () = assert!(ferroribbon::compat::Symbol::Repair.raw() == 57694);
const _: () = assert!(ferroribbon::compat::Symbol::Message.raw() == 57695);
const _: () = assert!(ferroribbon::compat::Symbol::Page2.raw() == 57696);
const _: () = assert!(ferroribbon::compat::Symbol::CalendarDay.raw() == 57697);
const _: () = assert!(ferroribbon::compat::Symbol::CalendarWeek.raw() == 57698);
const _: () = assert!(ferroribbon::compat::Symbol::Calendar.raw() == 57699);
const _: () = assert!(ferroribbon::compat::Symbol::Character.raw() == 57700);
const _: () = assert!(ferroribbon::compat::Symbol::MailReplyAll.raw() == 57701);
const _: () = assert!(ferroribbon::compat::Symbol::Read.raw() == 57702);
const _: () = assert!(ferroribbon::compat::Symbol::Link.raw() == 57703);
const _: () = assert!(ferroribbon::compat::Symbol::Account.raw() == 57704);
const _: () = assert!(ferroribbon::compat::Symbol::ShowBcc.raw() == 57705);
const _: () = assert!(ferroribbon::compat::Symbol::HideBcc.raw() == 57706);
const _: () = assert!(ferroribbon::compat::Symbol::Cut.raw() == 57707);
const _: () = assert!(ferroribbon::compat::Symbol::Attach.raw() == 57708);
const _: () = assert!(ferroribbon::compat::Symbol::Paste.raw() == 57709);
const _: () = assert!(ferroribbon::compat::Symbol::Filter.raw() == 57710);
const _: () = assert!(ferroribbon::compat::Symbol::Copy.raw() == 57711);
const _: () = assert!(ferroribbon::compat::Symbol::Emoji2.raw() == 57712);
const _: () = assert!(ferroribbon::compat::Symbol::Important.raw() == 57713);
const _: () = assert!(ferroribbon::compat::Symbol::MailReply.raw() == 57714);
const _: () = assert!(ferroribbon::compat::Symbol::SlideShow.raw() == 57715);
const _: () = assert!(ferroribbon::compat::Symbol::Sort.raw() == 57716);
const _: () = assert!(ferroribbon::compat::Symbol::Manage.raw() == 57720);
const _: () = assert!(ferroribbon::compat::Symbol::AllApps.raw() == 57721);
const _: () = assert!(ferroribbon::compat::Symbol::DisconnectDrive.raw() == 57722);
const _: () = assert!(ferroribbon::compat::Symbol::MapDrive.raw() == 57723);
const _: () = assert!(ferroribbon::compat::Symbol::NewWindow.raw() == 57724);
const _: () = assert!(ferroribbon::compat::Symbol::OpenWith.raw() == 57725);
const _: () = assert!(ferroribbon::compat::Symbol::ContactPresence.raw() == 57729);
const _: () = assert!(ferroribbon::compat::Symbol::Priority.raw() == 57730);
const _: () = assert!(ferroribbon::compat::Symbol::GoToToday.raw() == 57732);
const _: () = assert!(ferroribbon::compat::Symbol::Font.raw() == 57733);
const _: () = assert!(ferroribbon::compat::Symbol::FontColor.raw() == 57734);
const _: () = assert!(ferroribbon::compat::Symbol::Contact2.raw() == 57735);
const _: () = assert!(ferroribbon::compat::Symbol::Folder.raw() == 57736);
const _: () = assert!(ferroribbon::compat::Symbol::Audio.raw() == 57737);
const _: () = assert!(ferroribbon::compat::Symbol::Placeholder.raw() == 57738);
const _: () = assert!(ferroribbon::compat::Symbol::View.raw() == 57739);
const _: () = assert!(ferroribbon::compat::Symbol::SetLockScreen.raw() == 57740);
const _: () = assert!(ferroribbon::compat::Symbol::SetTile.raw() == 57741);
const _: () = assert!(ferroribbon::compat::Symbol::ClosedCaption.raw() == 57744);
const _: () = assert!(ferroribbon::compat::Symbol::StopSlideShow.raw() == 57745);
const _: () = assert!(ferroribbon::compat::Symbol::Permissions.raw() == 57746);
const _: () = assert!(ferroribbon::compat::Symbol::Highlight.raw() == 57747);
const _: () = assert!(ferroribbon::compat::Symbol::DisableUpdates.raw() == 57748);
const _: () = assert!(ferroribbon::compat::Symbol::UnFavorite.raw() == 57749);
const _: () = assert!(ferroribbon::compat::Symbol::UnPin.raw() == 57750);
const _: () = assert!(ferroribbon::compat::Symbol::OpenLocal.raw() == 57751);
const _: () = assert!(ferroribbon::compat::Symbol::Mute.raw() == 57752);
const _: () = assert!(ferroribbon::compat::Symbol::Italic.raw() == 57753);
const _: () = assert!(ferroribbon::compat::Symbol::Underline.raw() == 57754);
const _: () = assert!(ferroribbon::compat::Symbol::Bold.raw() == 57755);
const _: () = assert!(ferroribbon::compat::Symbol::MoveToFolder.raw() == 57756);
const _: () = assert!(ferroribbon::compat::Symbol::LikeDislike.raw() == 57757);
const _: () = assert!(ferroribbon::compat::Symbol::Dislike.raw() == 57758);
const _: () = assert!(ferroribbon::compat::Symbol::Like.raw() == 57759);
const _: () = assert!(ferroribbon::compat::Symbol::AlignRight.raw() == 57760);
const _: () = assert!(ferroribbon::compat::Symbol::AlignCenter.raw() == 57761);
const _: () = assert!(ferroribbon::compat::Symbol::AlignLeft.raw() == 57762);
const _: () = assert!(ferroribbon::compat::Symbol::Zoom.raw() == 57763);
const _: () = assert!(ferroribbon::compat::Symbol::ZoomOut.raw() == 57764);
const _: () = assert!(ferroribbon::compat::Symbol::OpenFile.raw() == 57765);
const _: () = assert!(ferroribbon::compat::Symbol::OtherUser.raw() == 57766);
const _: () = assert!(ferroribbon::compat::Symbol::Admin.raw() == 57767);
const _: () = assert!(ferroribbon::compat::Symbol::Street.raw() == 57795);
const _: () = assert!(ferroribbon::compat::Symbol::Map.raw() == 57796);
const _: () = assert!(ferroribbon::compat::Symbol::ClearSelection.raw() == 57797);
const _: () = assert!(ferroribbon::compat::Symbol::FontDecrease.raw() == 57798);
const _: () = assert!(ferroribbon::compat::Symbol::FontIncrease.raw() == 57799);
const _: () = assert!(ferroribbon::compat::Symbol::FontSize.raw() == 57800);
const _: () = assert!(ferroribbon::compat::Symbol::CellPhone.raw() == 57801);
const _: () = assert!(ferroribbon::compat::Symbol::ReShare.raw() == 57802);
const _: () = assert!(ferroribbon::compat::Symbol::Tag.raw() == 57803);
const _: () = assert!(ferroribbon::compat::Symbol::RepeatOne.raw() == 57804);
const _: () = assert!(ferroribbon::compat::Symbol::RepeatAll.raw() == 57805);
const _: () = assert!(ferroribbon::compat::Symbol::OutlineStar.raw() == 57806);
const _: () = assert!(ferroribbon::compat::Symbol::SolidStar.raw() == 57807);
const _: () = assert!(ferroribbon::compat::Symbol::Calculator.raw() == 57808);
const _: () = assert!(ferroribbon::compat::Symbol::Directions.raw() == 57809);
const _: () = assert!(ferroribbon::compat::Symbol::Target.raw() == 57810);
const _: () = assert!(ferroribbon::compat::Symbol::Library.raw() == 57811);
const _: () = assert!(ferroribbon::compat::Symbol::PhoneBook.raw() == 57812);
const _: () = assert!(ferroribbon::compat::Symbol::Memo.raw() == 57813);
const _: () = assert!(ferroribbon::compat::Symbol::Microphone.raw() == 57814);
const _: () = assert!(ferroribbon::compat::Symbol::PostUpdate.raw() == 57815);
const _: () = assert!(ferroribbon::compat::Symbol::BackToWindow.raw() == 57816);
const _: () = assert!(ferroribbon::compat::Symbol::FullScreen.raw() == 57817);
const _: () = assert!(ferroribbon::compat::Symbol::NewFolder.raw() == 57818);
const _: () = assert!(ferroribbon::compat::Symbol::CalendarReply.raw() == 57819);
const _: () = assert!(ferroribbon::compat::Symbol::UnSyncFolder.raw() == 57821);
const _: () = assert!(ferroribbon::compat::Symbol::ReportHacked.raw() == 57822);
const _: () = assert!(ferroribbon::compat::Symbol::SyncFolder.raw() == 57823);
const _: () = assert!(ferroribbon::compat::Symbol::BlockContact.raw() == 57824);
const _: () = assert!(ferroribbon::compat::Symbol::SwitchApps.raw() == 57825);
const _: () = assert!(ferroribbon::compat::Symbol::AddFriend.raw() == 57826);
const _: () = assert!(ferroribbon::compat::Symbol::TouchPointer.raw() == 57827);
const _: () = assert!(ferroribbon::compat::Symbol::GoToStart.raw() == 57828);
const _: () = assert!(ferroribbon::compat::Symbol::ZeroBars.raw() == 57829);
const _: () = assert!(ferroribbon::compat::Symbol::OneBar.raw() == 57830);
const _: () = assert!(ferroribbon::compat::Symbol::TwoBars.raw() == 57831);
const _: () = assert!(ferroribbon::compat::Symbol::ThreeBars.raw() == 57832);
const _: () = assert!(ferroribbon::compat::Symbol::FourBars.raw() == 57833);
const _: () = assert!(ferroribbon::compat::Symbol::Scan.raw() == 58004);
const _: () = assert!(ferroribbon::compat::Symbol::Preview.raw() == 58005);
const _: () = assert!(ferroribbon::compat::Symbol::GlobalNavigationButton.raw() == 59136);
const _: () = assert!(ferroribbon::compat::Symbol::Share.raw() == 59181);
const _: () = assert!(ferroribbon::compat::Symbol::Print.raw() == 59209);
const _: () = assert!(ferroribbon::compat::Symbol::XboxOneConsole.raw() == 59792);
impl MarkupTyped for SymbolMetadata {
    const MARKUP: &'static MarkupType = {
        static META: MarkupType = {
            let mut m = MarkupType::new("Symbol", MarkupTypeKind::Enum, module_path!());
            m.explicit_namespace = Some("FerroRibbon.Compatibility");
            m.handles = &[ValueType::of::<ferroribbon::compat::Symbol>];
            m.nullable = Some(ValueType::of::<Option<ferroribbon::compat::Symbol>>);
            m.enum_members = &[
                MarkupEnumMember {
                    name: "Previous",
                    value: 57600,
                    rust_variant: Some("Previous"),
                },
                MarkupEnumMember {
                    name: "Next",
                    value: 57601,
                    rust_variant: Some("Next"),
                },
                MarkupEnumMember {
                    name: "Play",
                    value: 57602,
                    rust_variant: Some("Play"),
                },
                MarkupEnumMember {
                    name: "Pause",
                    value: 57603,
                    rust_variant: Some("Pause"),
                },
                MarkupEnumMember {
                    name: "Edit",
                    value: 57604,
                    rust_variant: Some("Edit"),
                },
                MarkupEnumMember {
                    name: "Save",
                    value: 57605,
                    rust_variant: Some("Save"),
                },
                MarkupEnumMember {
                    name: "Clear",
                    value: 57606,
                    rust_variant: Some("Clear"),
                },
                MarkupEnumMember {
                    name: "Delete",
                    value: 57607,
                    rust_variant: Some("Delete"),
                },
                MarkupEnumMember {
                    name: "Remove",
                    value: 57608,
                    rust_variant: Some("Remove"),
                },
                MarkupEnumMember {
                    name: "Add",
                    value: 57609,
                    rust_variant: Some("Add"),
                },
                MarkupEnumMember {
                    name: "Cancel",
                    value: 57610,
                    rust_variant: Some("Cancel"),
                },
                MarkupEnumMember {
                    name: "Accept",
                    value: 57611,
                    rust_variant: Some("Accept"),
                },
                MarkupEnumMember {
                    name: "More",
                    value: 57612,
                    rust_variant: Some("More"),
                },
                MarkupEnumMember {
                    name: "Redo",
                    value: 57613,
                    rust_variant: Some("Redo"),
                },
                MarkupEnumMember {
                    name: "Undo",
                    value: 57614,
                    rust_variant: Some("Undo"),
                },
                MarkupEnumMember {
                    name: "Home",
                    value: 57615,
                    rust_variant: Some("Home"),
                },
                MarkupEnumMember {
                    name: "Up",
                    value: 57616,
                    rust_variant: Some("Up"),
                },
                MarkupEnumMember {
                    name: "Forward",
                    value: 57617,
                    rust_variant: Some("Forward"),
                },
                MarkupEnumMember {
                    name: "Back",
                    value: 57618,
                    rust_variant: Some("Back"),
                },
                MarkupEnumMember {
                    name: "Favorite",
                    value: 57619,
                    rust_variant: Some("Favorite"),
                },
                MarkupEnumMember {
                    name: "Camera",
                    value: 57620,
                    rust_variant: Some("Camera"),
                },
                MarkupEnumMember {
                    name: "Setting",
                    value: 57621,
                    rust_variant: Some("Setting"),
                },
                MarkupEnumMember {
                    name: "Video",
                    value: 57622,
                    rust_variant: Some("Video"),
                },
                MarkupEnumMember {
                    name: "Sync",
                    value: 57623,
                    rust_variant: Some("Sync"),
                },
                MarkupEnumMember {
                    name: "Download",
                    value: 57624,
                    rust_variant: Some("Download"),
                },
                MarkupEnumMember {
                    name: "Mail",
                    value: 57625,
                    rust_variant: Some("Mail"),
                },
                MarkupEnumMember {
                    name: "Find",
                    value: 57626,
                    rust_variant: Some("Find"),
                },
                MarkupEnumMember {
                    name: "Help",
                    value: 57627,
                    rust_variant: Some("Help"),
                },
                MarkupEnumMember {
                    name: "Upload",
                    value: 57628,
                    rust_variant: Some("Upload"),
                },
                MarkupEnumMember {
                    name: "Emoji",
                    value: 57629,
                    rust_variant: Some("Emoji"),
                },
                MarkupEnumMember {
                    name: "TwoPage",
                    value: 57630,
                    rust_variant: Some("TwoPage"),
                },
                MarkupEnumMember {
                    name: "LeaveChat",
                    value: 57631,
                    rust_variant: Some("LeaveChat"),
                },
                MarkupEnumMember {
                    name: "MailForward",
                    value: 57632,
                    rust_variant: Some("MailForward"),
                },
                MarkupEnumMember {
                    name: "Clock",
                    value: 57633,
                    rust_variant: Some("Clock"),
                },
                MarkupEnumMember {
                    name: "Send",
                    value: 57634,
                    rust_variant: Some("Send"),
                },
                MarkupEnumMember {
                    name: "Crop",
                    value: 57635,
                    rust_variant: Some("Crop"),
                },
                MarkupEnumMember {
                    name: "RotateCamera",
                    value: 57636,
                    rust_variant: Some("RotateCamera"),
                },
                MarkupEnumMember {
                    name: "People",
                    value: 57637,
                    rust_variant: Some("People"),
                },
                MarkupEnumMember {
                    name: "OpenPane",
                    value: 57638,
                    rust_variant: Some("OpenPane"),
                },
                MarkupEnumMember {
                    name: "ClosePane",
                    value: 57639,
                    rust_variant: Some("ClosePane"),
                },
                MarkupEnumMember {
                    name: "World",
                    value: 57640,
                    rust_variant: Some("World"),
                },
                MarkupEnumMember {
                    name: "Flag",
                    value: 57641,
                    rust_variant: Some("Flag"),
                },
                MarkupEnumMember {
                    name: "PreviewLink",
                    value: 57642,
                    rust_variant: Some("PreviewLink"),
                },
                MarkupEnumMember {
                    name: "Globe",
                    value: 57643,
                    rust_variant: Some("Globe"),
                },
                MarkupEnumMember {
                    name: "Trim",
                    value: 57644,
                    rust_variant: Some("Trim"),
                },
                MarkupEnumMember {
                    name: "AttachCamera",
                    value: 57645,
                    rust_variant: Some("AttachCamera"),
                },
                MarkupEnumMember {
                    name: "ZoomIn",
                    value: 57646,
                    rust_variant: Some("ZoomIn"),
                },
                MarkupEnumMember {
                    name: "Bookmarks",
                    value: 57647,
                    rust_variant: Some("Bookmarks"),
                },
                MarkupEnumMember {
                    name: "Document",
                    value: 57648,
                    rust_variant: Some("Document"),
                },
                MarkupEnumMember {
                    name: "ProtectedDocument",
                    value: 57649,
                    rust_variant: Some("ProtectedDocument"),
                },
                MarkupEnumMember {
                    name: "Page",
                    value: 57650,
                    rust_variant: Some("Page"),
                },
                MarkupEnumMember {
                    name: "Bullets",
                    value: 57651,
                    rust_variant: Some("Bullets"),
                },
                MarkupEnumMember {
                    name: "Comment",
                    value: 57652,
                    rust_variant: Some("Comment"),
                },
                MarkupEnumMember {
                    name: "MailFilled",
                    value: 57653,
                    rust_variant: Some("MailFilled"),
                },
                MarkupEnumMember {
                    name: "ContactInfo",
                    value: 57654,
                    rust_variant: Some("ContactInfo"),
                },
                MarkupEnumMember {
                    name: "HangUp",
                    value: 57655,
                    rust_variant: Some("HangUp"),
                },
                MarkupEnumMember {
                    name: "ViewAll",
                    value: 57656,
                    rust_variant: Some("ViewAll"),
                },
                MarkupEnumMember {
                    name: "MapPin",
                    value: 57657,
                    rust_variant: Some("MapPin"),
                },
                MarkupEnumMember {
                    name: "Phone",
                    value: 57658,
                    rust_variant: Some("Phone"),
                },
                MarkupEnumMember {
                    name: "VideoChat",
                    value: 57659,
                    rust_variant: Some("VideoChat"),
                },
                MarkupEnumMember {
                    name: "Switch",
                    value: 57660,
                    rust_variant: Some("Switch"),
                },
                MarkupEnumMember {
                    name: "Contact",
                    value: 57661,
                    rust_variant: Some("Contact"),
                },
                MarkupEnumMember {
                    name: "Rename",
                    value: 57662,
                    rust_variant: Some("Rename"),
                },
                MarkupEnumMember {
                    name: "Pin",
                    value: 57665,
                    rust_variant: Some("Pin"),
                },
                MarkupEnumMember {
                    name: "MusicInfo",
                    value: 57666,
                    rust_variant: Some("MusicInfo"),
                },
                MarkupEnumMember {
                    name: "Go",
                    value: 57667,
                    rust_variant: Some("Go"),
                },
                MarkupEnumMember {
                    name: "Keyboard",
                    value: 57668,
                    rust_variant: Some("Keyboard"),
                },
                MarkupEnumMember {
                    name: "DockLeft",
                    value: 57669,
                    rust_variant: Some("DockLeft"),
                },
                MarkupEnumMember {
                    name: "DockRight",
                    value: 57670,
                    rust_variant: Some("DockRight"),
                },
                MarkupEnumMember {
                    name: "DockBottom",
                    value: 57671,
                    rust_variant: Some("DockBottom"),
                },
                MarkupEnumMember {
                    name: "Remote",
                    value: 57672,
                    rust_variant: Some("Remote"),
                },
                MarkupEnumMember {
                    name: "Refresh",
                    value: 57673,
                    rust_variant: Some("Refresh"),
                },
                MarkupEnumMember {
                    name: "Rotate",
                    value: 57674,
                    rust_variant: Some("Rotate"),
                },
                MarkupEnumMember {
                    name: "Shuffle",
                    value: 57675,
                    rust_variant: Some("Shuffle"),
                },
                MarkupEnumMember {
                    name: "List",
                    value: 57676,
                    rust_variant: Some("List"),
                },
                MarkupEnumMember {
                    name: "Shop",
                    value: 57677,
                    rust_variant: Some("Shop"),
                },
                MarkupEnumMember {
                    name: "SelectAll",
                    value: 57678,
                    rust_variant: Some("SelectAll"),
                },
                MarkupEnumMember {
                    name: "Orientation",
                    value: 57679,
                    rust_variant: Some("Orientation"),
                },
                MarkupEnumMember {
                    name: "Import",
                    value: 57680,
                    rust_variant: Some("Import"),
                },
                MarkupEnumMember {
                    name: "ImportAll",
                    value: 57681,
                    rust_variant: Some("ImportAll"),
                },
                MarkupEnumMember {
                    name: "BrowsePhotos",
                    value: 57685,
                    rust_variant: Some("BrowsePhotos"),
                },
                MarkupEnumMember {
                    name: "WebCam",
                    value: 57686,
                    rust_variant: Some("WebCam"),
                },
                MarkupEnumMember {
                    name: "Pictures",
                    value: 57688,
                    rust_variant: Some("Pictures"),
                },
                MarkupEnumMember {
                    name: "SaveLocal",
                    value: 57689,
                    rust_variant: Some("SaveLocal"),
                },
                MarkupEnumMember {
                    name: "Caption",
                    value: 57690,
                    rust_variant: Some("Caption"),
                },
                MarkupEnumMember {
                    name: "Stop",
                    value: 57691,
                    rust_variant: Some("Stop"),
                },
                MarkupEnumMember {
                    name: "ShowResults",
                    value: 57692,
                    rust_variant: Some("ShowResults"),
                },
                MarkupEnumMember {
                    name: "Volume",
                    value: 57693,
                    rust_variant: Some("Volume"),
                },
                MarkupEnumMember {
                    name: "Repair",
                    value: 57694,
                    rust_variant: Some("Repair"),
                },
                MarkupEnumMember {
                    name: "Message",
                    value: 57695,
                    rust_variant: Some("Message"),
                },
                MarkupEnumMember {
                    name: "Page2",
                    value: 57696,
                    rust_variant: Some("Page2"),
                },
                MarkupEnumMember {
                    name: "CalendarDay",
                    value: 57697,
                    rust_variant: Some("CalendarDay"),
                },
                MarkupEnumMember {
                    name: "CalendarWeek",
                    value: 57698,
                    rust_variant: Some("CalendarWeek"),
                },
                MarkupEnumMember {
                    name: "Calendar",
                    value: 57699,
                    rust_variant: Some("Calendar"),
                },
                MarkupEnumMember {
                    name: "Character",
                    value: 57700,
                    rust_variant: Some("Character"),
                },
                MarkupEnumMember {
                    name: "MailReplyAll",
                    value: 57701,
                    rust_variant: Some("MailReplyAll"),
                },
                MarkupEnumMember {
                    name: "Read",
                    value: 57702,
                    rust_variant: Some("Read"),
                },
                MarkupEnumMember {
                    name: "Link",
                    value: 57703,
                    rust_variant: Some("Link"),
                },
                MarkupEnumMember {
                    name: "Account",
                    value: 57704,
                    rust_variant: Some("Account"),
                },
                MarkupEnumMember {
                    name: "ShowBcc",
                    value: 57705,
                    rust_variant: Some("ShowBcc"),
                },
                MarkupEnumMember {
                    name: "HideBcc",
                    value: 57706,
                    rust_variant: Some("HideBcc"),
                },
                MarkupEnumMember {
                    name: "Cut",
                    value: 57707,
                    rust_variant: Some("Cut"),
                },
                MarkupEnumMember {
                    name: "Attach",
                    value: 57708,
                    rust_variant: Some("Attach"),
                },
                MarkupEnumMember {
                    name: "Paste",
                    value: 57709,
                    rust_variant: Some("Paste"),
                },
                MarkupEnumMember {
                    name: "Filter",
                    value: 57710,
                    rust_variant: Some("Filter"),
                },
                MarkupEnumMember {
                    name: "Copy",
                    value: 57711,
                    rust_variant: Some("Copy"),
                },
                MarkupEnumMember {
                    name: "Emoji2",
                    value: 57712,
                    rust_variant: Some("Emoji2"),
                },
                MarkupEnumMember {
                    name: "Important",
                    value: 57713,
                    rust_variant: Some("Important"),
                },
                MarkupEnumMember {
                    name: "MailReply",
                    value: 57714,
                    rust_variant: Some("MailReply"),
                },
                MarkupEnumMember {
                    name: "SlideShow",
                    value: 57715,
                    rust_variant: Some("SlideShow"),
                },
                MarkupEnumMember {
                    name: "Sort",
                    value: 57716,
                    rust_variant: Some("Sort"),
                },
                MarkupEnumMember {
                    name: "Manage",
                    value: 57720,
                    rust_variant: Some("Manage"),
                },
                MarkupEnumMember {
                    name: "AllApps",
                    value: 57721,
                    rust_variant: Some("AllApps"),
                },
                MarkupEnumMember {
                    name: "DisconnectDrive",
                    value: 57722,
                    rust_variant: Some("DisconnectDrive"),
                },
                MarkupEnumMember {
                    name: "MapDrive",
                    value: 57723,
                    rust_variant: Some("MapDrive"),
                },
                MarkupEnumMember {
                    name: "NewWindow",
                    value: 57724,
                    rust_variant: Some("NewWindow"),
                },
                MarkupEnumMember {
                    name: "OpenWith",
                    value: 57725,
                    rust_variant: Some("OpenWith"),
                },
                MarkupEnumMember {
                    name: "ContactPresence",
                    value: 57729,
                    rust_variant: Some("ContactPresence"),
                },
                MarkupEnumMember {
                    name: "Priority",
                    value: 57730,
                    rust_variant: Some("Priority"),
                },
                MarkupEnumMember {
                    name: "GoToToday",
                    value: 57732,
                    rust_variant: Some("GoToToday"),
                },
                MarkupEnumMember {
                    name: "Font",
                    value: 57733,
                    rust_variant: Some("Font"),
                },
                MarkupEnumMember {
                    name: "FontColor",
                    value: 57734,
                    rust_variant: Some("FontColor"),
                },
                MarkupEnumMember {
                    name: "Contact2",
                    value: 57735,
                    rust_variant: Some("Contact2"),
                },
                MarkupEnumMember {
                    name: "Folder",
                    value: 57736,
                    rust_variant: Some("Folder"),
                },
                MarkupEnumMember {
                    name: "Audio",
                    value: 57737,
                    rust_variant: Some("Audio"),
                },
                MarkupEnumMember {
                    name: "Placeholder",
                    value: 57738,
                    rust_variant: Some("Placeholder"),
                },
                MarkupEnumMember {
                    name: "View",
                    value: 57739,
                    rust_variant: Some("View"),
                },
                MarkupEnumMember {
                    name: "SetLockScreen",
                    value: 57740,
                    rust_variant: Some("SetLockScreen"),
                },
                MarkupEnumMember {
                    name: "SetTile",
                    value: 57741,
                    rust_variant: Some("SetTile"),
                },
                MarkupEnumMember {
                    name: "ClosedCaption",
                    value: 57744,
                    rust_variant: Some("ClosedCaption"),
                },
                MarkupEnumMember {
                    name: "StopSlideShow",
                    value: 57745,
                    rust_variant: Some("StopSlideShow"),
                },
                MarkupEnumMember {
                    name: "Permissions",
                    value: 57746,
                    rust_variant: Some("Permissions"),
                },
                MarkupEnumMember {
                    name: "Highlight",
                    value: 57747,
                    rust_variant: Some("Highlight"),
                },
                MarkupEnumMember {
                    name: "DisableUpdates",
                    value: 57748,
                    rust_variant: Some("DisableUpdates"),
                },
                MarkupEnumMember {
                    name: "UnFavorite",
                    value: 57749,
                    rust_variant: Some("UnFavorite"),
                },
                MarkupEnumMember {
                    name: "UnPin",
                    value: 57750,
                    rust_variant: Some("UnPin"),
                },
                MarkupEnumMember {
                    name: "OpenLocal",
                    value: 57751,
                    rust_variant: Some("OpenLocal"),
                },
                MarkupEnumMember {
                    name: "Mute",
                    value: 57752,
                    rust_variant: Some("Mute"),
                },
                MarkupEnumMember {
                    name: "Italic",
                    value: 57753,
                    rust_variant: Some("Italic"),
                },
                MarkupEnumMember {
                    name: "Underline",
                    value: 57754,
                    rust_variant: Some("Underline"),
                },
                MarkupEnumMember {
                    name: "Bold",
                    value: 57755,
                    rust_variant: Some("Bold"),
                },
                MarkupEnumMember {
                    name: "MoveToFolder",
                    value: 57756,
                    rust_variant: Some("MoveToFolder"),
                },
                MarkupEnumMember {
                    name: "LikeDislike",
                    value: 57757,
                    rust_variant: Some("LikeDislike"),
                },
                MarkupEnumMember {
                    name: "Dislike",
                    value: 57758,
                    rust_variant: Some("Dislike"),
                },
                MarkupEnumMember {
                    name: "Like",
                    value: 57759,
                    rust_variant: Some("Like"),
                },
                MarkupEnumMember {
                    name: "AlignRight",
                    value: 57760,
                    rust_variant: Some("AlignRight"),
                },
                MarkupEnumMember {
                    name: "AlignCenter",
                    value: 57761,
                    rust_variant: Some("AlignCenter"),
                },
                MarkupEnumMember {
                    name: "AlignLeft",
                    value: 57762,
                    rust_variant: Some("AlignLeft"),
                },
                MarkupEnumMember {
                    name: "Zoom",
                    value: 57763,
                    rust_variant: Some("Zoom"),
                },
                MarkupEnumMember {
                    name: "ZoomOut",
                    value: 57764,
                    rust_variant: Some("ZoomOut"),
                },
                MarkupEnumMember {
                    name: "OpenFile",
                    value: 57765,
                    rust_variant: Some("OpenFile"),
                },
                MarkupEnumMember {
                    name: "OtherUser",
                    value: 57766,
                    rust_variant: Some("OtherUser"),
                },
                MarkupEnumMember {
                    name: "Admin",
                    value: 57767,
                    rust_variant: Some("Admin"),
                },
                MarkupEnumMember {
                    name: "Street",
                    value: 57795,
                    rust_variant: Some("Street"),
                },
                MarkupEnumMember {
                    name: "Map",
                    value: 57796,
                    rust_variant: Some("Map"),
                },
                MarkupEnumMember {
                    name: "ClearSelection",
                    value: 57797,
                    rust_variant: Some("ClearSelection"),
                },
                MarkupEnumMember {
                    name: "FontDecrease",
                    value: 57798,
                    rust_variant: Some("FontDecrease"),
                },
                MarkupEnumMember {
                    name: "FontIncrease",
                    value: 57799,
                    rust_variant: Some("FontIncrease"),
                },
                MarkupEnumMember {
                    name: "FontSize",
                    value: 57800,
                    rust_variant: Some("FontSize"),
                },
                MarkupEnumMember {
                    name: "CellPhone",
                    value: 57801,
                    rust_variant: Some("CellPhone"),
                },
                MarkupEnumMember {
                    name: "ReShare",
                    value: 57802,
                    rust_variant: Some("ReShare"),
                },
                MarkupEnumMember {
                    name: "Tag",
                    value: 57803,
                    rust_variant: Some("Tag"),
                },
                MarkupEnumMember {
                    name: "RepeatOne",
                    value: 57804,
                    rust_variant: Some("RepeatOne"),
                },
                MarkupEnumMember {
                    name: "RepeatAll",
                    value: 57805,
                    rust_variant: Some("RepeatAll"),
                },
                MarkupEnumMember {
                    name: "OutlineStar",
                    value: 57806,
                    rust_variant: Some("OutlineStar"),
                },
                MarkupEnumMember {
                    name: "SolidStar",
                    value: 57807,
                    rust_variant: Some("SolidStar"),
                },
                MarkupEnumMember {
                    name: "Calculator",
                    value: 57808,
                    rust_variant: Some("Calculator"),
                },
                MarkupEnumMember {
                    name: "Directions",
                    value: 57809,
                    rust_variant: Some("Directions"),
                },
                MarkupEnumMember {
                    name: "Target",
                    value: 57810,
                    rust_variant: Some("Target"),
                },
                MarkupEnumMember {
                    name: "Library",
                    value: 57811,
                    rust_variant: Some("Library"),
                },
                MarkupEnumMember {
                    name: "PhoneBook",
                    value: 57812,
                    rust_variant: Some("PhoneBook"),
                },
                MarkupEnumMember {
                    name: "Memo",
                    value: 57813,
                    rust_variant: Some("Memo"),
                },
                MarkupEnumMember {
                    name: "Microphone",
                    value: 57814,
                    rust_variant: Some("Microphone"),
                },
                MarkupEnumMember {
                    name: "PostUpdate",
                    value: 57815,
                    rust_variant: Some("PostUpdate"),
                },
                MarkupEnumMember {
                    name: "BackToWindow",
                    value: 57816,
                    rust_variant: Some("BackToWindow"),
                },
                MarkupEnumMember {
                    name: "FullScreen",
                    value: 57817,
                    rust_variant: Some("FullScreen"),
                },
                MarkupEnumMember {
                    name: "NewFolder",
                    value: 57818,
                    rust_variant: Some("NewFolder"),
                },
                MarkupEnumMember {
                    name: "CalendarReply",
                    value: 57819,
                    rust_variant: Some("CalendarReply"),
                },
                MarkupEnumMember {
                    name: "UnSyncFolder",
                    value: 57821,
                    rust_variant: Some("UnSyncFolder"),
                },
                MarkupEnumMember {
                    name: "ReportHacked",
                    value: 57822,
                    rust_variant: Some("ReportHacked"),
                },
                MarkupEnumMember {
                    name: "SyncFolder",
                    value: 57823,
                    rust_variant: Some("SyncFolder"),
                },
                MarkupEnumMember {
                    name: "BlockContact",
                    value: 57824,
                    rust_variant: Some("BlockContact"),
                },
                MarkupEnumMember {
                    name: "SwitchApps",
                    value: 57825,
                    rust_variant: Some("SwitchApps"),
                },
                MarkupEnumMember {
                    name: "AddFriend",
                    value: 57826,
                    rust_variant: Some("AddFriend"),
                },
                MarkupEnumMember {
                    name: "TouchPointer",
                    value: 57827,
                    rust_variant: Some("TouchPointer"),
                },
                MarkupEnumMember {
                    name: "GoToStart",
                    value: 57828,
                    rust_variant: Some("GoToStart"),
                },
                MarkupEnumMember {
                    name: "ZeroBars",
                    value: 57829,
                    rust_variant: Some("ZeroBars"),
                },
                MarkupEnumMember {
                    name: "OneBar",
                    value: 57830,
                    rust_variant: Some("OneBar"),
                },
                MarkupEnumMember {
                    name: "TwoBars",
                    value: 57831,
                    rust_variant: Some("TwoBars"),
                },
                MarkupEnumMember {
                    name: "ThreeBars",
                    value: 57832,
                    rust_variant: Some("ThreeBars"),
                },
                MarkupEnumMember {
                    name: "FourBars",
                    value: 57833,
                    rust_variant: Some("FourBars"),
                },
                MarkupEnumMember {
                    name: "Scan",
                    value: 58004,
                    rust_variant: Some("Scan"),
                },
                MarkupEnumMember {
                    name: "Preview",
                    value: 58005,
                    rust_variant: Some("Preview"),
                },
                MarkupEnumMember {
                    name: "GlobalNavigationButton",
                    value: 59136,
                    rust_variant: Some("GlobalNavigationButton"),
                },
                MarkupEnumMember {
                    name: "Share",
                    value: 59181,
                    rust_variant: Some("Share"),
                },
                MarkupEnumMember {
                    name: "Print",
                    value: 59209,
                    rust_variant: Some("Print"),
                },
                MarkupEnumMember {
                    name: "XboxOneConsole",
                    value: 59792,
                    rust_variant: Some("XboxOneConsole"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v).ok().map(|v| {
                    Rc::new(ferroribbon::compat::Symbol::from_raw(v)) as ferroui_base::BoxedValue
                })
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<ferroribbon::compat::Symbol>()
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
        SymbolMetadata::MARKUP,
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
    ValueTypes::register_nullable::<ferroribbon::compat::Symbol>();
    ValueTypes::register_display::<ferroribbon::compat::Symbol>();
}
