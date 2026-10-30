// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use ferrodock_model::core::IDockSerializer;
use std::{
    any::TypeId,
    cell::{Cell, RefCell},
    rc::Rc,
};
type Base = Rc<Model>;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    A,
    B,
    C,
    X,
    Both,
}
struct Model {
    kind: Kind,
    name: RefCell<Option<String>>,
    next: RefCell<Option<Base>>,
    a: Cell<i32>,
    b: Cell<i32>,
    l: Cell<i32>,
    r: Cell<i32>,
}
impl Model {
    fn new(kind: Kind) -> Base {
        Rc::new(Self {
            kind,
            name: RefCell::new(None),
            next: RefCell::new(None),
            a: Cell::new(0),
            b: Cell::new(0),
            l: Cell::new(0),
            r: Cell::new(0),
        })
    }
}
trait View: Clone + 'static {
    fn base(&self) -> &Base;
    fn create() -> Self;
}
macro_rules! view {
    ($name:ident,$kind:ident) => {
        #[derive(Clone)]
        struct $name(Base);
        impl View for $name {
            fn base(&self) -> &Base {
                &self.0
            }
            fn create() -> Self {
                Self(Model::new(Kind::$kind))
            }
        }
    };
}
view!(AView, A);
view!(BView, B);
view!(CView, C);
view!(XView, X);
view!(BothView, Both);
view!(LeftView, Both);
view!(RightView, Both);
fn runtime_type(v: &Base) -> TypeId {
    match v.kind {
        Kind::A => TypeId::of::<AView>(),
        Kind::B => TypeId::of::<BView>(),
        Kind::C => TypeId::of::<CView>(),
        Kind::X => TypeId::of::<XView>(),
        Kind::Both => TypeId::of::<BothView>(),
    }
}
fn fields<T: View>() -> Vec<JsonPropertyInfo<T>> {
    vec![
        JsonPropertyInfo::new(
            "Name",
            |v: &T| v.base().name.borrow().clone(),
            |v, x| {
                v.base().name.replace(x);
            },
        ),
        JsonPropertyInfo::new(
            "Next",
            |v: &T| v.base().next.borrow().clone(),
            |v, x| {
                v.base().next.replace(x);
            },
        ),
    ]
}
fn descriptor<T: View>(fields: Vec<JsonPropertyInfo<T>>) -> Rc<JsonTypeInfo> {
    JsonTypeInfo::with_reference_cast::<T, Base>(
        JsonTypeInfo::object(T::create, |v: &T| Rc::as_ptr(v.base()) as usize, fields),
        |v| v.base().clone(),
    )
}
#[derive(Default)]
struct Holder {
    first: RefCell<Option<Base>>,
    second: RefCell<Option<Base>>,
}
fn serializer(
    policy: JsonUnknownDerivedTypeHandling,
    ambiguity: bool,
    exact: bool,
) -> Rc<DockSerializer> {
    let mut a = fields::<AView>();
    a.push(JsonPropertyInfo::new(
        "A",
        |v: &AView| v.0.a.get(),
        |v, x| v.0.a.set(x),
    ));
    let mut b = vec![JsonPropertyInfo::new(
        "B",
        |v: &BView| v.0.b.get(),
        |v, x| v.0.b.set(x),
    )];
    b.extend(fields::<BView>());
    b.push(JsonPropertyInfo::new(
        "A",
        |v: &BView| v.0.a.get(),
        |v, x| v.0.a.set(x),
    ));
    let mut both = fields::<BothView>();
    both.push(JsonPropertyInfo::new(
        "L",
        |v: &BothView| v.0.l.get(),
        |v, x| v.0.l.set(x),
    ));
    both.push(JsonPropertyInfo::new(
        "R",
        |v: &BothView| v.0.r.get(),
        |v, x| v.0.r.set(x),
    ));
    let derived = if ambiguity {
        let mut values = vec![
            JsonDerivedType::new::<LeftView>(
                "left",
                |v: &Base| (v.kind == Kind::Both).then(|| LeftView(v.clone())),
                |v| v.0,
            ),
            JsonDerivedType::new::<RightView>(
                "right",
                |v: &Base| (v.kind == Kind::Both).then(|| RightView(v.clone())),
                |v| v.0,
            ),
        ];
        if exact {
            values.push(
                JsonDerivedType::new::<BothView>(
                    "both",
                    |v: &Base| (v.kind == Kind::Both).then(|| BothView(v.clone())),
                    |v| v.0,
                )
                .with_ancestors(vec![TypeId::of::<LeftView>(), TypeId::of::<RightView>()]),
            );
        }
        values
    } else {
        vec![
            JsonDerivedType::new::<AView>(
                "a",
                |v: &Base| matches!(v.kind, Kind::A | Kind::B | Kind::C).then(|| AView(v.clone())),
                |v| v.0,
            ),
            JsonDerivedType::new::<BView>(
                "b",
                |v: &Base| matches!(v.kind, Kind::B | Kind::C).then(|| BView(v.clone())),
                |v| v.0,
            )
            .with_ancestors(vec![TypeId::of::<AView>()]),
        ]
    };
    let base = JsonTypeInfo::abstract_object(
        |v: &Base| Rc::as_ptr(v) as usize,
        vec![
            JsonPropertyInfo::new(
                "Name",
                |v: &Base| v.name.borrow().clone(),
                |v, x| {
                    v.name.replace(x);
                },
            ),
            JsonPropertyInfo::new(
                "Next",
                |v: &Base| v.next.borrow().clone(),
                |v, x| {
                    v.next.replace(x);
                },
            ),
        ],
    );
    let poly =
        JsonTypeInfo::polymorphic_with_policy(derived, Some(base), true, policy, runtime_type);
    let holder = JsonTypeInfo::object(
        || Rc::new(Holder::default()),
        |v: &Rc<Holder>| Rc::as_ptr(v) as usize,
        vec![
            JsonPropertyInfo::new(
                "First",
                |v: &Rc<Holder>| v.first.borrow().clone(),
                |v, x| {
                    v.first.replace(x);
                },
            ),
            JsonPropertyInfo::new(
                "Second",
                |v: &Rc<Holder>| v.second.borrow().clone(),
                |v, x| {
                    v.second.replace(x);
                },
            ),
        ],
    );
    DockSerializer::with_resolver(Some(GeneratedJsonTypeInfoResolver::new(vec![
        poly,
        descriptor::<AView>(a),
        descriptor::<BView>(b),
        descriptor::<BothView>(both),
        holder,
        JsonTypeInfo::optional::<Base>(),
        JsonTypeInfo::optional::<String>(),
    ])))
    .unwrap()
}
fn policy(name: &str) -> JsonUnknownDerivedTypeHandling {
    match name {
        "FailSerialization" | "empty" => JsonUnknownDerivedTypeHandling::FailSerialization,
        "FallBackToBaseType" => JsonUnknownDerivedTypeHandling::FallBackToBaseType,
        "FallBackToNearestAncestor" | "ambiguity" => {
            JsonUnknownDerivedTypeHandling::FallBackToNearestAncestor
        }
        _ => panic!("source policy"),
    }
}
fn field<'a>(v: &'a JsonValue, k: &str) -> &'a str {
    let JsonValue::String(v) = v.property(k).unwrap() else {
        panic!("source field")
    };
    v
}
fn rows() -> JsonValue {
    JsonValue::parse(include_str!(
        "../../../tests/Dock/data/serializer-json-audit.polymorphic.json"
    ))
    .unwrap()
}
#[test]
fn all_source_unknown_runtime_policies_exact_selection_and_ambiguous_ancestors_match() {
    let data = rows();
    let JsonValue::Array(rows) = data.property("writes").unwrap() else {
        panic!("source rows")
    };
    assert_eq!(rows.len(), 15);
    for row in rows {
        let name = field(row, "policy");
        let kind = field(row, "kind");
        let serializer = if name == "empty" {
            empty_serializer()
        } else {
            serializer(policy(name), name == "ambiguity", kind == "both-exact")
        };
        let value = Model::new(match kind {
            "a" => Kind::A,
            "b" => Kind::B,
            "c" => Kind::C,
            "x" => Kind::X,
            _ => Kind::Both,
        });
        value.name.replace(Some(
            if name == "ambiguity" { "both" } else { "value" }.into(),
        ));
        value.a.set(11);
        value.b.set(22);
        value.l.set(1);
        value.r.set(2);
        let result = serializer.serialize(value);
        if matches!(field(row, "error"), "unsupported" | "invalid-config") {
            assert_eq!(
                result
                    .unwrap_err()
                    .cause()
                    .downcast_ref::<JsonError>()
                    .unwrap()
                    .kind,
                JsonErrorKind::MissingMetadata,
                "{name} {kind}"
            );
        } else {
            assert_eq!(result.unwrap(), field(row, "wire"), "{name} {kind}");
        }
        assert_eq!(
            JsonValue::number(policy(name) as i64),
            row.property("policyValue").unwrap().clone()
        );
    }
}
#[test]
fn abstract_base_fallback_reads_fail_while_registered_classes_keep_actual_self_references() {
    let data = rows();
    let JsonValue::Array(rows) = data.property("reads").unwrap() else {
        panic!("source rows")
    };
    assert_eq!(rows.len(), 16);
    for row in rows {
        let serializer = if field(row, "policy") == "empty" {
            empty_serializer()
        } else {
            serializer(policy(field(row, "policy")), false, false)
        };
        let result = serializer.deserialize::<Base>(Some(field(row, "input")));
        if matches!(field(row, "error"), "unsupported" | "invalid-config") {
            assert_eq!(
                result
                    .err()
                    .unwrap()
                    .cause()
                    .downcast_ref::<JsonError>()
                    .unwrap()
                    .kind,
                JsonErrorKind::MissingMetadata
            );
            continue;
        }
        let value = result.unwrap();
        let kind = value
            .as_ref()
            .map(|v| match v.kind {
                Kind::A => "PolyA",
                Kind::B => "PolyB",
                _ => panic!("source actual class"),
            })
            .unwrap_or("null");
        assert_eq!(kind, field(row, "kind"));
        let own = value.as_ref().is_some_and(|v| {
            v.next
                .borrow()
                .as_ref()
                .is_some_and(|next| Rc::ptr_eq(v, next))
        });
        assert_eq!(JsonValue::Bool(own), row.property("self").unwrap().clone());
        assert_eq!(
            serializer.serialize(value.clone()).unwrap(),
            field(row, "wire")
        );
        if let Some(value) = value {
            value.next.take();
        }
    }
}
#[test]
fn nearest_class_schema_retains_shared_references_and_existing_reference_casts() {
    let serializer = serializer(
        JsonUnknownDerivedTypeHandling::FallBackToNearestAncestor,
        false,
        false,
    );
    let source = include_str!("../../../tests/Dock/data/serializer-json-audit.polymorphic.holder.json");
    let holder = serializer
        .deserialize::<Rc<Holder>>(Some(source))
        .unwrap()
        .unwrap();
    let first = holder.first.borrow().clone().unwrap();
    let second = holder.second.borrow().clone().unwrap();
    let checks = [
        (
            "polymorphic-nearest-registered-class",
            first.kind == Kind::B,
        ),
        (
            "polymorphic-nearest-actual-shared-listener",
            Rc::ptr_eq(&first, &second),
        ),
        (
            "polymorphic-nearest-self-reference",
            Rc::ptr_eq(&first, first.next.borrow().as_ref().unwrap()),
        ),
    ];
    for (name, value) in checks {
        assert!(value, "{name}");
        assert!(
            include_str!("../../../tests/Dock/data/serializer-json-audit.polymorphic.tsv")
                .lines()
                .any(|row| row == format!("{name}\t1"))
        );
    }
    assert_eq!(serializer.serialize(holder).unwrap(), source);
    first.next.take();
}

fn empty_serializer() -> Rc<DockSerializer> {
    DockSerializer::with_resolver(Some(GeneratedJsonTypeInfoResolver::new(vec![
        JsonTypeInfo::empty_polymorphic::<Base>(),
    ])))
    .unwrap()
}
