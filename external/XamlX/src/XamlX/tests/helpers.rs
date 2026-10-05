//! Port of the upstream test `Helpers.StructDiff`, implemented as a structural dump of the AST.

use std::rc::Rc;

use crate::ast::*;
use crate::exceptions::XamlResult;
use crate::transform::transformers::AdderSetter;

fn list<K: ?Sized + XamlAstCast>(nodes: &[Rc<K>]) -> String {
    let items: Vec<String> = nodes.iter().map(|n| dump(&n.as_node())).collect();
    format!("[{}]", items.join(", "))
}

fn setter(setter: &Rc<dyn IXamlPropertySetter>) -> String {
    let parameters: Vec<String> = setter.parameters().iter().map(|p| p.name()).collect();
    let kind = if setter.as_any().is::<AdderSetter>() {
        "adder"
    } else {
        "setter"
    };
    format!("{kind}({})", parameters.join(","))
}

/// A structural, line-info free description of a node tree.
pub fn dump(node: &Rc<dyn IXamlAstNode>) -> String {
    let any = node.as_any();
    if let Some(n) = any.downcast_ref::<XamlAstXmlTypeReference>() {
        let mut rv = n.to_node_string();
        let arguments = n.generic_arguments.borrow();
        if !arguments.is_empty() {
            rv.push_str(&list(&arguments));
        }
        if n.is_markup_extension() {
            rv.push_str("{ME}");
        }
        return rv;
    }
    if let Some(n) = any.downcast_ref::<XamlAstClrTypeReference>() {
        return format!(
            "clr!!{}{}",
            n.type_.full_name(),
            if n.is_markup_extension() { "{ME}" } else { "" }
        );
    }
    if let Some(n) = any.downcast_ref::<XamlAstObjectNode>() {
        return format!(
            "(Object {} args={} children={})",
            dump(&n.type_().as_node()),
            list(&n.arguments.borrow()),
            list(&n.children.borrow())
        );
    }
    if let Some(n) = any.downcast_ref::<XamlAstTextNode>() {
        return format!(
            "(Text {:?} preserve={} type={})",
            n.text(),
            n.preserve_whitespace,
            dump(&n.type_().as_node())
        );
    }
    if let Some(n) = any.downcast_ref::<XamlAstXmlDirective>() {
        return format!(
            "(Directive {}:{} {})",
            n.namespace.borrow().as_deref().unwrap_or("<null>"),
            n.name.borrow(),
            list(&n.values.borrow())
        );
    }
    if let Some(n) = any.downcast_ref::<XamlAstXamlPropertyValueNode>() {
        return format!(
            "(Prop {} attr={} {})",
            dump(&n.property().as_node()),
            n.is_attribute_syntax,
            list(&n.values.borrow())
        );
    }
    if let Some(n) = any.downcast_ref::<XamlAstNamePropertyReference>() {
        return format!(
            "(NameProp {}.{} target={})",
            dump(&n.declaring_type.borrow().as_node()),
            n.name(),
            dump(&n.target_type.borrow().as_node())
        );
    }
    if let Some(n) = any.downcast_ref::<XamlAstClrProperty>() {
        let setters: Vec<String> = n.setters().iter().map(setter).collect();
        return format!(
            "(ClrProp {}.{} [{}])",
            n.declaring_type().name(),
            n.name(),
            setters.join(" ")
        );
    }
    if let Some(n) = any.downcast_ref::<XamlPropertyAssignmentNode>() {
        let setters: Vec<String> = n.possible_setters.borrow().iter().map(setter).collect();
        return format!(
            "(Assign {}.{} [{}] {})",
            n.property.declaring_type().name(),
            n.property.name(),
            setters.join(" "),
            list(&n.values.borrow())
        );
    }
    if let Some(n) = any.downcast_ref::<XamlManipulationGroupNode>() {
        return format!("(Group {})", list(&n.children.borrow()));
    }
    if let Some(n) = any.downcast_ref::<XamlAstConstructableObjectNode>() {
        return format!(
            "(Constructable {} args={} children={})",
            dump(&n.type_().as_node()),
            list(&n.arguments.borrow()),
            list(&n.children.borrow())
        );
    }
    if let Some(n) = any.downcast_ref::<XamlAstNewClrObjectNode>() {
        return format!(
            "(New {} args={})",
            dump(&n.type_().as_node()),
            list(&n.arguments.borrow())
        );
    }
    if let Some(n) = any.downcast_ref::<XamlObjectInitializationNode>() {
        return format!(
            "(Init {} skipBeginInit={} {})",
            n.type_.borrow().name(),
            n.skip_begin_init.get(),
            dump(&n.manipulation().as_node())
        );
    }
    if let Some(n) = any.downcast_ref::<XamlMarkupExtensionNode>() {
        return format!(
            "(MarkupExt {}({}) {})",
            n.provide_value.name(),
            n.provide_value.parameters().len(),
            dump(&n.value().as_node())
        );
    }
    if let Some(n) = any.downcast_ref::<XamlConstantNode>() {
        return format!("(Const {} {:?})", dump(&n.type_().as_node()), n.constant);
    }
    if any.is::<XamlNullExtensionNode>() {
        return "(Null)".to_string();
    }
    if let Some(n) = any.downcast_ref::<XamlTypeExtensionNode>() {
        return format!("(TypeExt {})", dump(&n.value().as_node()));
    }
    if let Some(n) = any.downcast_ref::<XamlStaticExtensionNode>() {
        let target = n.target_type.borrow().clone();
        return format!(
            "(Static {}.{})",
            target.map(|t| dump(&t.as_node())).unwrap_or_default(),
            n.member.borrow()
        );
    }
    if let Some(n) = any.downcast_ref::<XamlAstRuntimeCastNode>() {
        return format!(
            "(Cast {} {})",
            dump(&n.type_().as_node()),
            dump(&n.value().as_node())
        );
    }
    if let Some(n) = any.downcast_ref::<XamlAstContextLocalNode>() {
        return format!("(ContextLocal {})", dump(&n.type_().as_node()));
    }
    if let Some(n) = any.downcast_ref::<XamlLoadMethodDelegateNode>() {
        return format!(
            "(LoadMethodDelegate {} {})",
            n.method.name(),
            dump(&n.base.value().as_node())
        );
    }
    if let Some(n) = any.downcast_ref::<XamlAstCompilerLocalNode>() {
        return format!("(Local {})", n.type_.name());
    }
    if let Some(n) = any.downcast_ref::<XamlAstLocalInitializationNodeEmitter>() {
        return format!("(LocalInit {})", dump(&n.base.value().as_node()));
    }
    if let Some(n) = any.downcast_ref::<XamlAstManipulationImperativeNode>() {
        return format!(
            "(ManipulationImperative {})",
            dump(&n.imperative().as_node())
        );
    }
    if let Some(n) = any.downcast_ref::<XamlAstImperativeValueManipulation>() {
        return format!(
            "(ImperativeValueManipulation {} {})",
            dump(&n.value().as_node()),
            dump(&n.manipulation().as_node())
        );
    }
    if let Some(n) = any.downcast_ref::<XamlDeferredContentNode>() {
        return format!("(Deferred {})", dump(&n.value().as_node()));
    }
    if let Some(n) = any.downcast_ref::<XamlDeferredContentInitializeIntermediateRootNode>() {
        return format!("(DeferredRoot {})", dump(&n.value().as_node()));
    }
    if let Some(n) = node.as_method_call_base_node() {
        let method = n.method();
        return format!(
            "({} {}.{} {})",
            node.type_name(),
            method.declaring_type().name(),
            method.name(),
            list(&n.arguments.borrow())
        );
    }
    if let Some(n) = node.as_value_with_manipulation_node() {
        return format!(
            "({} {} {})",
            node.type_name(),
            dump(&n.value().as_node()),
            n.manipulation()
                .map(|m| dump(&m.as_node()))
                .unwrap_or_else(|| "<null>".to_string())
        );
    }
    if let Some(n) = node.as_value_with_side_effect_node_base() {
        return format!("({} {})", node.type_name(), dump(&n.value().as_node()));
    }
    format!("({})", node.type_name())
}

struct LineInfoChecker {
    missing: Vec<String>,
}

impl IXamlAstVisitor for LineInfoChecker {
    fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if node.line() == 0 || node.position() == 0 {
            self.missing.push(dump(&node));
        }
        Ok(node)
    }
    fn push(&mut self, _node: Rc<dyn IXamlAstNode>) {}
    fn pop(&mut self) {}
}

/// Compares the structure of two trees and checks that every parsed node carries line info.
pub fn struct_diff(parsed: &Rc<dyn IXamlAstNode>, expected: &Rc<dyn IXamlAstNode>) {
    let mut checker = LineInfoChecker {
        missing: Vec::new(),
    };
    visit_node(parsed, &mut checker).expect("the line info visitor doesn't fail");
    assert!(
        checker.missing.is_empty(),
        "Missing line info on: {:#?}",
        checker.missing
    );
    let parsed = dump(parsed).replace(", ", ",\n");
    let expected = dump(expected).replace(", ", ",\n");
    assert_eq!(parsed, expected);
}
