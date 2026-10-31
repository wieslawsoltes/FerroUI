use super::*;

fn expression(source: &str) -> Expr {
    let doc = parse(&format!("Example {{ property var result: {source} }}")).unwrap();
    let MemberKind::Property {
        value: Some(Binding::Expr(expr)),
        ..
    } = doc.root.members.into_iter().next().unwrap().kind
    else {
        panic!("expected an expression binding");
    };
    expr
}

#[test]
fn preserves_header_metadata_and_utf8_spans() {
    let source = "\u{feff}pragma ValueTypeBehavior: Addressable, Copy\r\nimport Acme.Widgets 6.10 as Ui\nimport \"../shared\" as Shared\nUi.Surface { objectName: \"Łódź\" }";
    let doc = parse(source).unwrap();
    assert_eq!(doc.pragmas[0].values, ["Addressable", "Copy"]);
    assert_eq!(doc.imports[0].version.as_deref(), Some("6.10"));
    assert_eq!(doc.imports[0].alias.as_deref(), Some("Ui"));
    assert_eq!(doc.imports[1].uri, "../shared");
    assert!(doc.imports[1].is_path);
    assert_eq!(
        &source[doc.root.span.start..doc.root.span.end],
        "Ui.Surface { objectName: \"Łódź\" }"
    );
    let span = doc.root.members[0].span;
    assert_eq!(&source[span.start..span.end], "objectName: \"Łódź\"");
}

#[test]
fn preserves_objects_lists_aliases_groups_and_inline_types() {
    let doc = parse(
        r#"
            Item {
                id: root
                default property list<Item> content: [ Item { id: a }, Ui.Label {}, ]
                property Item header: Ui.Label {}
                property alias highlighted: a.active
                anchors { left: parent.left; margins: 4 }
                component Delegate: Item { id: root }
                enum Position { First = -1, Middle, Last = 4, Beyond }
                Ui.Container {}
            }
        "#,
    )
    .unwrap();
    assert!(
        matches!(&doc.root.members[1].kind, MemberKind::Property {ty, modifiers, value: Some(Binding::Objects(items)), ..} if ty == "list<Item>" && modifiers.default && items.len() == 2)
    );
    assert!(
        matches!(&doc.root.members[2].kind, MemberKind::Property {value: Some(Binding::Object(item)), ..} if item.type_name=="Ui.Label")
    );
    assert!(
        matches!(&doc.root.members[3].kind, MemberKind::Property {ty, value: Some(Binding::Expr(Expr::Member(_, name))), ..} if ty=="alias" && name=="active")
    );
    assert!(
        matches!(&doc.root.members[4].kind, MemberKind::Group { name, members } if name=="anchors" && members.len()==2)
    );
    assert!(
        matches!(&doc.root.members[5].kind, MemberKind::Component { name, .. } if name=="Delegate")
    );
    assert!(
        matches!(&doc.root.members[6].kind, MemberKind::Enum { values, .. } if values.iter().map(|(_,n)| *n).collect::<Vec<_>>() == [-1,0,4,5])
    );
    assert!(
        matches!(&doc.root.members[7].kind, MemberKind::Child(item) if item.type_name=="Ui.Container")
    );
}

#[test]
fn parses_signal_and_method_signatures_and_handler_forms() {
    let doc = parse(
        r#"
        Surface {
            signal committed(value: int, string reason)
            signal done
            function scale(factor: real, bias): real { return factor * 2 + bias }
            onCommitted: (v, reason) => { if (v > 0) { result += scale(v, 1) } else { result = 0 } }
            onDone: function() { doneCount++ }
            Component.onCompleted: self.start()
            onMoved: position => result = position
        }
    "#,
    )
    .unwrap();
    assert!(
        matches!(&doc.root.members[0].kind, MemberKind::Signal {parameters,..} if parameters[0].ty.as_deref()==Some("int") && parameters[1].name=="reason")
    );
    assert!(
        matches!(&doc.root.members[2].kind, MemberKind::Function { parameters, return_type, body, .. } if parameters[1].ty.is_none() && return_type.as_deref()==Some("real") && matches!(body[0],Stmt::Return(Some(_))))
    );
    assert!(
        matches!(&doc.root.members[3].kind, MemberKind::Handler { parameters, body, .. } if parameters==&["v","reason"] && matches!(body[0],Stmt::If{..}))
    );
    assert!(
        matches!(&doc.root.members[4].kind, MemberKind::Handler { body, .. } if matches!(&body[0],Stmt::Assign{op,value:Expr::Number(1.), ..} if op=="+="))
    );
    assert!(
        matches!(&doc.root.members[5].kind, MemberKind::Handler {name, ..} if name=="Component.onCompleted")
    );
    assert!(
        matches!(&doc.root.members[6].kind, MemberKind::Handler {parameters, ..} if parameters == &["position"])
    );
}

#[test]
fn expression_precedence_and_lazy_branches_are_structural() {
    let expr = expression("flag && enabled ? values[index + 1] : f(-2, 1 + 2 * 3)");
    let Expr::Conditional(test, yes, no) = expr else {
        panic!()
    };
    assert!(matches!(*test,Expr::Binary(ref op,..) if op=="&&"));
    assert!(
        matches!(*yes,Expr::Index(_, ref i) if matches!(i.as_ref(),Expr::Binary(op,..) if op=="+"))
    );
    assert!(
        matches!(*no,Expr::Call(_,ref args) if matches!(&args[1],Expr::Binary(op,_,rhs) if op=="+" && matches!(rhs.as_ref(), Expr::Binary(op,..) if op=="*")))
    );
    assert!(
        matches!(expression("2 ** 3 ** 2"), Expr::Binary(op,_,r) if op=="**" && matches!(*r,Expr::Binary(ref op,..) if op=="**"))
    );
    assert!(
        matches!(expression("a - b - c"),Expr::Binary(op,l,_) if op=="-" && matches!(*l,Expr::Binary(ref op,..) if op=="-"))
    );
    assert!(
        matches!(expression("a ? b : c ? d : e"),Expr::Conditional(_,_,r) if matches!(*r,Expr::Conditional(..)))
    );
}

#[test]
fn numeric_and_unicode_literals() {
    assert_eq!(
        expression("[0x1f,0b101,0o77,.5,1.e2,]"),
        Expr::Array(vec![
            Expr::Number(31.),
            Expr::Number(5.),
            Expr::Number(63.),
            Expr::Number(0.5),
            Expr::Number(100.)
        ])
    );
    assert_eq!(
        expression(r#""\uD83D\uDC4B\u{1f30d}\x21""#),
        Expr::String("👋🌍!".into())
    );
}

#[test]
fn preserves_newlines_in_comments_for_automatic_termination() {
    let doc = parse("Surface { function f() { return /*x\r\ny*/ 5 } }").unwrap();
    assert!(
        matches!(&doc.root.members[0].kind, MemberKind::Function { body, .. } if body == &[Stmt::Return(None),Stmt::Expr(Expr::Number(5.))])
    );
    let doc = parse("Surface { property int x: 5 /*\n*/ property int y: x\n + 1 }").unwrap();
    assert_eq!(doc.root.members.len(), 2);
}

#[test]
fn pragma_and_modifier_syntax() {
    let doc = parse("pragma Singleton\nItem { virtual property int value: 4; override final property int count: 1; required property string label }").unwrap();
    assert!(
        matches!(&doc.root.members[0].kind,MemberKind::Property{modifiers,..} if modifiers.virtual_)
    );
    assert!(
        matches!(&doc.root.members[1].kind,MemberKind::Property{modifiers,..} if modifiers.override_ && modifiers.final_)
    );
    assert!(
        matches!(&doc.root.members[2].kind,MemberKind::Property{modifiers,..} if modifiers.required)
    );
}

#[test]
fn rejects_malformed_sources_where_they_occur() {
    for bad in [
        "Surface {} Surface {}",
        "import Foo 2.5.1\nSurface {}",
        "Surface { id: Item }",
        "Surface { property int result: 1 prop: 2 }",
        "Surface { property int value: 1e+ }",
        "Surface { required property int p: 4 }",
        "Surface { readonly property int p }",
        "Surface { property alias wrong: 2+3 }",
        "Surface { property alias wrong: a.b.c.d }",
        "Surface { onReady: 1 = 2 }",
        "Surface {signal sig(value)}",
        "Surface { function run(v,v) {} }",
        "Surface { /*",
        "Surface { property int x: 'a }",
        "Surface { property var x: '\\uD800' }",
        "Surface { property string class }",
        "Surface { property int result: 0b102 }",
        "Surface { readonly readonly property int value: 1 }",
    ] {
        assert!(parse(bad).is_err(), "accepted {bad:?}");
    }
    let err = parse("Surface {\r\n  title: 'Żółć'; @\r\n}").unwrap_err();
    assert_eq!((err.line, err.column), (2, 18));
}

#[test]
fn bounds_depth_even_for_flat_chains() {
    for bad in [
        format!("Item {{ property var v: x{} }}", ".x".repeat(4096)),
        format!(
            "Item {{ property var v: {}x{} }}",
            "(".repeat(4096),
            ")".repeat(4096)
        ),
        format!("{}{}", "Item { ".repeat(4096), "}".repeat(4096)),
        format!(
            "Item {{ function f() {{ {}x() }} }}",
            "if(true) ".repeat(4096)
        ),
    ] {
        assert!(parse(&bad).is_err());
    }
}
