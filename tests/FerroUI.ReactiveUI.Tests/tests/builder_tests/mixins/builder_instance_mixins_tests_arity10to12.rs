//! `BuilderInstanceMixinsTests.Arity10to12` (`ReactiveUI.Builder.Tests/Mixins/BuilderInstanceMixinsTests.Arity10to12.cs`).

use super::super::executors::builder_test;
use super::builder_instance_mixins_tests::*;
use ferroui_reactiveui::*;

builder_with_instance_tests!(builder_with_instance_10_types_invokes_action, builder_with_instance_10_types_skips_when_null;
    InstanceService01 => s1, c1; InstanceService02 => s2, c2; InstanceService03 => s3, c3; InstanceService04 => s4, c4;
    InstanceService05 => s5, c5; InstanceService06 => s6, c6; InstanceService07 => s7, c7; InstanceService08 => s8, c8;
    InstanceService09 => s9, c9; InstanceService10 => s10, c10);

builder_with_instance_tests!(builder_with_instance_11_types_invokes_action, builder_with_instance_11_types_skips_when_null;
    InstanceService01 => s1, c1; InstanceService02 => s2, c2; InstanceService03 => s3, c3; InstanceService04 => s4, c4;
    InstanceService05 => s5, c5; InstanceService06 => s6, c6; InstanceService07 => s7, c7; InstanceService08 => s8, c8;
    InstanceService09 => s9, c9; InstanceService10 => s10, c10; InstanceService11 => s11, c11);

builder_with_instance_tests!(builder_with_instance_12_types_invokes_action, builder_with_instance_12_types_skips_when_null;
    InstanceService01 => s1, c1; InstanceService02 => s2, c2; InstanceService03 => s3, c3; InstanceService04 => s4, c4;
    InstanceService05 => s5, c5; InstanceService06 => s6, c6; InstanceService07 => s7, c7; InstanceService08 => s8, c8;
    InstanceService09 => s9, c9; InstanceService10 => s10, c10; InstanceService11 => s11, c11; InstanceService12 => s12, c12);
