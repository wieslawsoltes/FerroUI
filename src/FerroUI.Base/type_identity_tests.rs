//! The identities of the classes (#612): type checks compare them, and they give the answers the
//! comparison of the `TypeInfo`s gave.

use crate::media::{Brush, GradientBrush, LinearGradientBrush, SolidColorBrush};
use crate::animation::Animatable;
use crate::layout::Layoutable;
use crate::{FerroObject, ObjectType, StaticType, StyledElement, TypeIdentity, TypeInfo, Visual};

/// The classes of the checks, each with its identity and its `TypeInfo`.
fn classes() -> Vec<(&'static TypeIdentity, &'static TypeInfo)> {
    vec![
        (FerroObject::IDENTITY, FerroObject::TYPE),
        (Animatable::IDENTITY, <Animatable as StaticType>::TYPE),
        (StyledElement::IDENTITY, <StyledElement as StaticType>::TYPE),
        (Visual::IDENTITY, <Visual as StaticType>::TYPE),
        (Layoutable::IDENTITY, <Layoutable as StaticType>::TYPE),
        (Brush::IDENTITY, <Brush as StaticType>::TYPE),
        (GradientBrush::IDENTITY, <GradientBrush as StaticType>::TYPE),
        (SolidColorBrush::IDENTITY, <SolidColorBrush as StaticType>::TYPE),
        (LinearGradientBrush::IDENTITY, <LinearGradientBrush as StaticType>::TYPE),
    ]
}

#[test]
fn every_class_has_its_own_identity_and_its_type_points_to_it() {
    let classes = classes();
    for (identity, type_) in &classes {
        let own = type_.identity().expect("a class has an identity");
        assert!(std::ptr::eq(own, *identity), "{}", type_.name());
        assert_eq!(identity.name(), type_.name());
    }
    for (i, (a, _)) in classes.iter().enumerate() {
        for (b, _) in &classes[i + 1..] {
            assert!(!std::ptr::eq(*a, *b), "{} and {} share an identity", a.name(), b.name());
        }
    }
}

#[test]
fn the_identity_check_gives_the_answers_of_the_type_check() {
    let classes = classes();
    for (_, derived) in &classes {
        for (identity, base) in &classes {
            assert_eq!(
                derived.derives_from(identity),
                base.is_assignable_from(derived),
                "{} derives from {}",
                derived.name(),
                base.name()
            );
        }
    }
}

#[test]
fn a_type_that_is_not_a_class_has_no_identity_and_derives_from_none() {
    static NOT_A_CLASS: TypeInfo = TypeInfo::new("NotAClass", None);
    assert!(NOT_A_CLASS.identity().is_none());
    assert!(!NOT_A_CLASS.derives_from(FerroObject::IDENTITY));
}

#[test]
fn an_object_is_cast_by_the_identities_of_its_classes() {
    let brush = SolidColorBrush::new();
    let object = brush.clone().upcast::<FerroObject>();
    assert!(object.is::<SolidColorBrush>());
    assert!(object.is::<Brush>());
    assert!(object.is::<Animatable>());
    assert!(object.is::<FerroObject>());
    assert!(!object.is::<GradientBrush>());
    assert!(!object.is::<Visual>());
    assert!(object.clone().cast::<Brush>().is_some());
    assert!(object.clone().cast::<LinearGradientBrush>().is_none());
    let root: &FerroObject = &object;
    assert!(root.downcast_ref::<SolidColorBrush>().is_some());
    assert!(root.downcast_ref::<StyledElement>().is_none());
}
