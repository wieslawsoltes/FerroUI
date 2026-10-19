//! `BuilderInstanceMixinsTests.Arity07to09` (`ReactiveUI.Builder.Tests/Mixins/BuilderInstanceMixinsTests.Arity07to09.cs`).

use super::super::executors::builder_test;
use super::builder_instance_mixins_tests::*;
use ferroui_reactiveui::*;

builder_with_instance_tests!(builder_with_instance_7_types_invokes_action, builder_with_instance_7_types_skips_when_null;
    InstanceService01 => s1, c1; InstanceService02 => s2, c2; InstanceService03 => s3, c3; InstanceService04 => s4, c4;
    InstanceService05 => s5, c5; InstanceService06 => s6, c6; InstanceService07 => s7, c7);

builder_with_instance_tests!(builder_with_instance_8_types_invokes_action, builder_with_instance_8_types_skips_when_null;
    InstanceService01 => s1, c1; InstanceService02 => s2, c2; InstanceService03 => s3, c3; InstanceService04 => s4, c4;
    InstanceService05 => s5, c5; InstanceService06 => s6, c6; InstanceService07 => s7, c7; InstanceService08 => s8, c8);

builder_with_instance_tests!(builder_with_instance_9_types_invokes_action, builder_with_instance_9_types_skips_when_null;
    InstanceService01 => s1, c1; InstanceService02 => s2, c2; InstanceService03 => s3, c3; InstanceService04 => s4, c4;
    InstanceService05 => s5, c5; InstanceService06 => s6, c6; InstanceService07 => s7, c7; InstanceService08 => s8, c8;
    InstanceService09 => s9, c9);
