// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::*;
use std::{collections::HashMap, fmt::Write};
fn failure(message: impl Into<String>) -> CompilationError {
    CompilationError(message.into())
}
fn ignored<'a>(model: &'a GenerationModel, expression: &str) -> &'a [String] {
    model
        .ignored_members
        .iter()
        .rev()
        .find(|i| i.type_expression == expression)
        .map(|i| i.member_names.as_slice())
        .unwrap_or(&[])
}
fn properties(
    out: &mut String,
    owner: &str,
    fields: &[RustPropertyBinding],
    excluded: &[String],
    runtime: &str,
) -> Result<(), CompilationError> {
    out.push_str("vec![");
    for p in fields {
        if excluded.contains(&p.name) {
            continue;
        }
        let converter = p
            .converter
            .clone()
            .or_else(|| p.object_payload.then(|| "__object_payload.clone()".into()));
        let name = format!("{:?}", p.name);
        let api = format!("{runtime}::JsonPropertyInfo::<{owner}>");
        let value = &p.value_type;
        if (p.getter_fallible && p.getter.is_none()) || (p.setter_fallible && p.setter.is_none()) {
            return Err(failure(format!(
                "Property {} declares a missing fallible accessor",
                p.name
            )));
        }
        let fallible = p.getter_fallible || p.setter_fallible;
        let get = p.getter.as_ref().map(|get| {
            if fallible && !p.getter_fallible {
                format!("{{let getter={get};move|v:&{owner}|Ok(getter(v))}}")
            } else {
                get.clone()
            }
        });
        let set = p.setter.as_ref().map(|set| {
            if fallible && !p.setter_fallible {
                format!(
                    "{{let setter={set};move|v:&{owner},value: {value}|{{setter(v,value);Ok(())}}}}"
                )
            } else {
                set.clone()
            }
        });
        let prefix = if fallible { "try_" } else { "" };
        match (&get, &set, converter) {
            (Some(get), Some(set), None) => {
                write!(out, "{api}::{prefix}new::<{value}>({name},{get},{set}),")
            }
            (Some(get), None, None) => {
                write!(out, "{api}::{prefix}readonly::<{value}>({name},{get}),")
            }
            (None, Some(set), None) => {
                write!(out, "{api}::{prefix}writeonly::<{value}>({name},{set}),")
            }
            (Some(get), Some(set), Some(codec)) => write!(
                out,
                "{api}::{prefix}with_converter::<{value}>({name},{get},{set},{codec}),"
            ),
            (Some(get), None, Some(codec)) => write!(
                out,
                "{api}::{prefix}readonly_with_converter::<{value}>({name},{get},{codec}),"
            ),
            (None, Some(set), Some(codec)) => write!(
                out,
                "{api}::{prefix}writeonly_with_converter::<{value}>({name},{set},{codec}),"
            ),
            (None, None, _) => {
                return Err(failure(format!(
                    "Property {} has no accessible accessor",
                    p.name
                )))
            }
        }
        .expect("String formatter");
    }
    out.push(']');
    Ok(())
}
/// Emits executable Rust context/resolver/factory code from the semantic plan and resolved
/// original accessors. No field reflection or substitute model constructors are generated.
pub fn emit_rust(
    c: &Compilation,
    model: &GenerationModel,
    bindings: &RustBindings,
) -> Result<Option<String>, CompilationError> {
    if !model.should_generate {
        return Ok(None);
    }
    let runtime = &bindings.runtime_crate;
    let model_crate = &bindings.model_crate;
    let mut types = HashMap::new();
    for binding in &bindings.types {
        if types.insert(binding.symbol, binding).is_some() {
            return Err(failure("Duplicate Rust type binding"));
        }
    }
    let mut families = HashMap::new();
    for (i, binding) in bindings.families.iter().enumerate() {
        if families
            .insert(binding.kind as usize, (i, binding))
            .is_some()
        {
            return Err(failure("Duplicate Rust family binding"));
        }
    }
    for t in &model.serializable_types {
        if !types.contains_key(&t.type_id) {
            return Err(failure(format!(
                "No Rust codec binding for {}",
                t.display_name
            )));
        }
    }
    let mut out = String::from("// Generated from compiled source metadata.\n");
    writeln!(out,"/// Source-generated JSON context.\npub struct {};\nimpl {} {{\n/// Creates a compiled descriptor resolver.\npub fn resolver() -> ::std::rc::Rc<{runtime}::GeneratedJsonTypeInfoResolver> {{",model.context_type_name,model.context_type_name).expect("String formatter");
    write!(
        out,
        "let __object_payload = {runtime}::JsonTypeInfo::object_payload(vec!["
    )
    .expect("String formatter");
    let payload = model
        .polymorphisms
        .iter()
        .find(|p| p.kind == DockBaseKind::Object)
        .ok_or_else(|| failure("Missing object payload semantic plan"))?;
    for t in &payload.derived_types {
        let binding = types
            .get(&t.type_id)
            .ok_or_else(|| failure("Missing payload codec binding"))?;
        write!(
            out,
            "{runtime}::JsonObjectPayloadType::new::<{}>({:?}),",
            binding.value_type, t.discriminator_key
        )
        .expect("String formatter");
    }
    out.push_str("]);\n");
    for t in &model.serializable_types {
        let b = types[&t.type_id];
        let handle = &b.value_type;
        write!(out, "let __type_{} = ", t.type_id).expect("String formatter");
        if let Some(codec) = &b.custom_codec {
            out.push_str(codec);
        } else {
            if c.types
                .get(t.type_id)
                .ok_or_else(|| failure("Missing compiler type"))?
                .kind
                != TypeKind::Class
            {
                return Err(failure(format!(
                    "{} requires a source value codec",
                    t.display_name
                )));
            }
            if let Some(create) = &b.constructor {
                write!(
                    out,
                    "{runtime}::JsonTypeInfo::object::<{handle}>({create},{},",
                    b.identity
                )
                .expect("String formatter");
            } else {
                write!(
                    out,
                    "{runtime}::JsonTypeInfo::abstract_object::<{handle}>({},",
                    b.identity
                )
                .expect("String formatter");
            }
            properties(
                &mut out,
                handle,
                &b.properties,
                ignored(model, &t.type_expression),
                runtime,
            )?;
            out.push(')');
        }
        out.push_str(";\n");
        for cast in &b.reference_casts {
            writeln!(out,"let __type_{0} = {runtime}::JsonTypeInfo::with_reference_cast::<{handle},{1}>(__type_{0},{2});",t.type_id,cast.target,cast.convert).expect("String formatter");
        }
        for p in &model.polymorphisms {
            if p.kind == DockBaseKind::Object
                || !p.derived_types.iter().any(|d| d.type_id == t.type_id)
            {
                continue;
            }
            let (_, family) = families
                .get(&(p.kind as usize))
                .ok_or_else(|| failure("Missing Rust family binding"))?;
            let cast = family
                .derived
                .iter()
                .find(|d| d.symbol == t.type_id)
                .ok_or_else(|| failure("Missing derived upcast binding"))?;
            writeln!(out,"let __upcast_{0}_{3} = {2}; let __type_{0} = {runtime}::JsonTypeInfo::with_reference_cast::<{handle},{1}>(__type_{0},move |v:&{handle}|__upcast_{0}_{3}(v.clone()));",t.type_id,family.value_type,cast.upcast,p.kind as usize).expect("String formatter");
        }
    }
    let mut family_variables = Vec::new();
    for p in &model.polymorphisms {
        if p.kind == DockBaseKind::Object {
            continue;
        }
        let (index, b) = families
            .get(&(p.kind as usize))
            .ok_or_else(|| failure(format!("Missing Rust {:?} family", p.kind)))?;
        let base = &b.value_type;
        if p.derived_types.is_empty() {
            writeln!(
                out,
                "let __family_{index} = {runtime}::JsonTypeInfo::empty_polymorphic::<{base}>();"
            )
            .expect("String formatter");
            family_variables.push(format!("__family_{index}"));
            continue;
        }
        let identity = b
            .identity
            .as_ref()
            .ok_or_else(|| failure("Missing base identity binding"))?;
        let runtime_type = b
            .runtime_type
            .as_ref()
            .ok_or_else(|| failure("Missing runtime type binding"))?;
        write!(
            out,
            "let __fallback_{index} = {runtime}::JsonTypeInfo::abstract_object::<{base}>({},",
            identity
        )
        .expect("String formatter");
        properties(
            &mut out,
            base,
            &b.properties,
            ignored(model, &p.base_type_expression),
            runtime,
        )?;
        out.push_str(");\n");
        write!(out,"let __family_{index} = {runtime}::JsonTypeInfo::polymorphic_with_policy::<{base}>(vec![").expect("String formatter");
        for d in &p.derived_types {
            let codec = types
                .get(&d.type_id)
                .ok_or_else(|| failure("Missing derived type binding"))?;
            let cast = b
                .derived
                .iter()
                .find(|v| v.symbol == d.type_id)
                .ok_or_else(|| failure("Missing derived cast binding"))?;
            write!(
                out,
                "{runtime}::JsonDerivedType::<{base}>::new::<{}>({:?},{},{}).with_ancestors(vec![",
                codec.value_type, d.discriminator_key, cast.downcast, cast.upcast
            )
            .expect("String formatter");
            for ancestor in &p.derived_types {
                if d.type_id != ancestor.type_id
                    && crate::generation_model_builder::assignable(c, d.type_id, ancestor.type_id)?
                {
                    write!(
                        out,
                        "::std::any::TypeId::of::<{}>(),",
                        types[&ancestor.type_id].value_type
                    )
                    .expect("String formatter");
                }
            }
            out.push_str("]),");
        }
        writeln!(
            out,
            "],Some(__fallback_{index}),{},{runtime}::JsonUnknownDerivedTypeHandling::{:?},{});",
            p.ignore_unrecognized_type_discriminators,
            p.unknown_derived_type_handling,
            runtime_type
        )
        .expect("String formatter");
        family_variables.push(format!("__family_{index}"));
    }
    let mutability = if bindings.additional_metadata_collections.is_empty() {
        ""
    } else {
        "mut "
    };
    write!(
        out,
        "{runtime}::GeneratedJsonTypeInfoResolver::new({{let {mutability}__types=vec!["
    )
    .expect("String formatter");
    for t in &model.serializable_types {
        write!(out, "__type_{},", t.type_id).expect("String formatter");
    }
    for variable in family_variables {
        write!(out, "{variable},").expect("String formatter");
    }
    for factory in &bindings.additional_codecs {
        write!(out, "{factory},").expect("String formatter");
    }
    out.push_str("];");
    for collection in &bindings.additional_metadata_collections {
        write!(out, "__types.extend({collection});").expect("String formatter");
    }
    out.push_str("__types})\n}\n}\n");
    writeln!(out,"/// Original generated serializer factory overloads.\npub struct DockSystemTextJsonGenerated;\nimpl DockSystemTextJsonGenerated {{\n/// Creates the generated serializer with the source observable list policy.\npub fn create_serializer() -> {model_crate}::core::SerializationResult<::std::rc::Rc<{runtime}::DockSerializer>> {{ {runtime}::DockSerializer::with_resolver(Some({}::resolver())) }}\n/// Creates the generated serializer with an explicit source list policy.\npub fn create_serializer_with_list_type(list: Option<{runtime}::JsonListType>) -> {model_crate}::core::SerializationResult<::std::rc::Rc<{runtime}::DockSerializer>> {{ {runtime}::DockSerializer::with_list_type_and_resolver(list,Some({}::resolver())) }}\n}}",model.context_type_name,model.context_type_name).expect("String formatter");
    Ok(Some(out))
}

#[cfg(test)]
mod property_contract_tests {
    use super::*;
    #[test]
    fn missing_fallible_accessors_fail_but_source_ignores_still_win() {
        for missing_getter in [true, false] {
            let property = RustPropertyBinding {
                name: "Value".into(),
                value_type: "i32".into(),
                getter: (!missing_getter).then(|| "getter".into()),
                setter: missing_getter.then(|| "setter".into()),
                getter_fallible: missing_getter,
                setter_fallible: !missing_getter,
                converter: None,
                object_payload: false,
            };
            assert!(properties(
                &mut String::new(),
                "Owner",
                std::slice::from_ref(&property),
                &[],
                "runtime"
            )
            .unwrap_err()
            .0
            .contains("missing fallible accessor"));
            let mut text = String::new();
            properties(
                &mut text,
                "Owner",
                &[property],
                &["Value".into()],
                "runtime",
            )
            .unwrap();
            assert_eq!(text, "vec![]");
        }
    }
}
