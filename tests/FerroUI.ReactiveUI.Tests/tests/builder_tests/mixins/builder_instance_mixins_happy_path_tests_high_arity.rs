//! `BuilderInstanceMixinsHappyPathTests.HighArity` (`ReactiveUI.Builder.Tests/Mixins/BuilderInstanceMixinsHappyPathTests.HighArity.cs`):
//! arities 9 to 16.

use super::super::executors::builder_test;
use super::builder_instance_mixins_happy_path_tests::*;
use ferroui_reactiveui::*;

with_instance_happy_path_tests!(with_instance_9_types_invokes_action_with_resolved_services, with_instance_9_types_returns_early_when_current_is_null;
    Service1 => service1, captured1; Service2 => service2, captured2; Service3 => service3, captured3; Service4 => service4, captured4;
    Service5 => service5, captured5; Service6 => service6, captured6; Service7 => service7, captured7; Service8 => service8, captured8;
    Service9 => service9, captured9);

with_instance_happy_path_tests!(with_instance_10_types_invokes_action_with_resolved_services, with_instance_10_types_returns_early_when_current_is_null;
    Service1 => service1, captured1; Service2 => service2, captured2; Service3 => service3, captured3; Service4 => service4, captured4;
    Service5 => service5, captured5; Service6 => service6, captured6; Service7 => service7, captured7; Service8 => service8, captured8;
    Service9 => service9, captured9; Service10 => service10, captured10);

with_instance_happy_path_tests!(with_instance_11_types_invokes_action_with_resolved_services, with_instance_11_types_returns_early_when_current_is_null;
    Service1 => service1, captured1; Service2 => service2, captured2; Service3 => service3, captured3; Service4 => service4, captured4;
    Service5 => service5, captured5; Service6 => service6, captured6; Service7 => service7, captured7; Service8 => service8, captured8;
    Service9 => service9, captured9; Service10 => service10, captured10; Service11 => service11, captured11);

with_instance_happy_path_tests!(with_instance_12_types_invokes_action_with_resolved_services, with_instance_12_types_returns_early_when_current_is_null;
    Service1 => service1, captured1; Service2 => service2, captured2; Service3 => service3, captured3; Service4 => service4, captured4;
    Service5 => service5, captured5; Service6 => service6, captured6; Service7 => service7, captured7; Service8 => service8, captured8;
    Service9 => service9, captured9; Service10 => service10, captured10; Service11 => service11, captured11; Service12 => service12, captured12);

with_instance_happy_path_tests!(with_instance_13_types_invokes_action_with_resolved_services, with_instance_13_types_returns_early_when_current_is_null;
    Service1 => service1, captured1; Service2 => service2, captured2; Service3 => service3, captured3; Service4 => service4, captured4;
    Service5 => service5, captured5; Service6 => service6, captured6; Service7 => service7, captured7; Service8 => service8, captured8;
    Service9 => service9, captured9; Service10 => service10, captured10; Service11 => service11, captured11; Service12 => service12, captured12;
    Service13 => service13, captured13);

with_instance_happy_path_tests!(with_instance_14_types_invokes_action_with_resolved_services, with_instance_14_types_returns_early_when_current_is_null;
    Service1 => service1, captured1; Service2 => service2, captured2; Service3 => service3, captured3; Service4 => service4, captured4;
    Service5 => service5, captured5; Service6 => service6, captured6; Service7 => service7, captured7; Service8 => service8, captured8;
    Service9 => service9, captured9; Service10 => service10, captured10; Service11 => service11, captured11; Service12 => service12, captured12;
    Service13 => service13, captured13; Service14 => service14, captured14);

with_instance_happy_path_tests!(with_instance_15_types_invokes_action_with_resolved_services, with_instance_15_types_returns_early_when_current_is_null;
    Service1 => service1, captured1; Service2 => service2, captured2; Service3 => service3, captured3; Service4 => service4, captured4;
    Service5 => service5, captured5; Service6 => service6, captured6; Service7 => service7, captured7; Service8 => service8, captured8;
    Service9 => service9, captured9; Service10 => service10, captured10; Service11 => service11, captured11; Service12 => service12, captured12;
    Service13 => service13, captured13; Service14 => service14, captured14; Service15 => service15, captured15);

with_instance_happy_path_tests!(with_instance_16_types_invokes_action_with_resolved_services, with_instance_16_types_returns_early_when_current_is_null;
    Service1 => service1, captured1; Service2 => service2, captured2; Service3 => service3, captured3; Service4 => service4, captured4;
    Service5 => service5, captured5; Service6 => service6, captured6; Service7 => service7, captured7; Service8 => service8, captured8;
    Service9 => service9, captured9; Service10 => service10, captured10; Service11 => service11, captured11; Service12 => service12, captured12;
    Service13 => service13, captured13; Service14 => service14, captured14; Service15 => service15, captured15; Service16 => service16, captured16);
