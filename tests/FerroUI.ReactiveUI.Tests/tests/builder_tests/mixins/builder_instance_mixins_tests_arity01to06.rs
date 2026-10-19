//! `BuilderInstanceMixinsTests.Arity01to06` (`ReactiveUI.Builder.Tests/Mixins/BuilderInstanceMixinsTests.Arity01to06.cs`).
//!
//! The action takes the services as one tuple; the form of one service is a tuple of one.

use super::super::executors::builder_test;
use super::builder_instance_mixins_tests::*;
use ferroui_reactiveui::*;

builder_with_instance_tests!(builder_with_instance_1_type_invokes_action, builder_with_instance_1_type_skips_when_null;
    InstanceService01 => s1, c1);

builder_with_instance_tests!(builder_with_instance_2_types_invokes_action, builder_with_instance_2_types_skips_when_null;
    InstanceService01 => s1, c1; InstanceService02 => s2, c2);

builder_with_instance_tests!(builder_with_instance_3_types_invokes_action, builder_with_instance_3_types_skips_when_null;
    InstanceService01 => s1, c1; InstanceService02 => s2, c2; InstanceService03 => s3, c3);

builder_with_instance_tests!(builder_with_instance_4_types_invokes_action, builder_with_instance_4_types_skips_when_null;
    InstanceService01 => s1, c1; InstanceService02 => s2, c2; InstanceService03 => s3, c3; InstanceService04 => s4, c4);

builder_with_instance_tests!(builder_with_instance_5_types_invokes_action, builder_with_instance_5_types_skips_when_null;
    InstanceService01 => s1, c1; InstanceService02 => s2, c2; InstanceService03 => s3, c3; InstanceService04 => s4, c4;
    InstanceService05 => s5, c5);

builder_with_instance_tests!(builder_with_instance_6_types_invokes_action, builder_with_instance_6_types_skips_when_null;
    InstanceService01 => s1, c1; InstanceService02 => s2, c2; InstanceService03 => s3, c3; InstanceService04 => s4, c4;
    InstanceService05 => s5, c5; InstanceService06 => s6, c6);
