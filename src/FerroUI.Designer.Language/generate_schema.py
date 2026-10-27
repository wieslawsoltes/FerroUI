#!/usr/bin/env python3
"""Regenerate the portable language schema from FerroUI's existing builder metadata reader."""
import importlib.util
import json
import pathlib
import re
import sys

ROOT=pathlib.Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location("framework_builders",ROOT/"scripts/generate_view_builders.py")
framework=importlib.util.module_from_spec(spec)
spec.loader.exec_module(framework)
model=framework.Model()
crates=list(framework.CRATES)
# Templates, resource includes and markup extensions are declared by the XAML runtime,
# not by the native view-builder crates. Read their real metadata through the same scanner.
crates.append(("src/Markup/FerroUI.Markup.Xaml", "ferroui_markup_xaml"))
for _,_,group,_ in framework.GROUPS:
    crates.extend(group)
for directory,crate in crates:
    framework.read_paths(model,directory,crate)
for directory,crate in crates:
    framework.read_classes(model,directory,crate)

def pascal(value):
    return "".join(word[:1].upper()+word[1:] for word in value.split("_"))

def read_markup_properties(body, properties):
    """Read declared members without treating getter/setter bodies as new members."""
    for block in re.finditer(r"\bproperties:\s*\[",body):
        contents=body[block.end():framework.matching(body,block.end()-1,"[","]")]
        position=0
        while match:=re.search(r"\b([A-Za-z_]\w*)\s*:\s*([^{}]+?)\s*\{",contents[position:]):
            start=position+match.end()-1
            end=framework.matching(contents,start,"{","}")
            # A getter-only collection can still be writable in XAML through adders or a
            # compiler transform (for example Classes). A missing Rust setter is therefore
            # insufficient evidence for a language-service read-only diagnostic.
            properties[match.group(1)]=match.group(2).strip()
            position=end+1

enums={}
extra={}
aliases={}
for directory,crate in crates:
    registration=framework.read(ROOT/directory/"register_types.rs")
    mappings=re.findall(r'\(\s*"([\w:]+)"\s*,\s*"([\w.]+)"\s*\)',registration)
    assembly=re.search(r'\bname:\s*"([^"]+)"',registration).group(1)
    def register_aliases(name,path,body):
        explicit=re.search(r'\bnamespace:\s*"([^"]+)"',body)
        relative=pathlib.Path(path).relative_to(ROOT/directory).with_suffix("")
        modules=list(relative.parts)
        if modules[-1] in ("mod","lib"):
            modules.pop()
        module="::".join([crate]+modules)
        candidates=[(prefix,namespace) for prefix,namespace in mappings if module==prefix or module.startswith(prefix+"::")]
        inferred=max(candidates,key=lambda entry:len(entry[0]))[1] if candidates else None
        namespace=explicit.group(1) if explicit else inferred
        if namespace:
            # Explicit markup namespaces override a helper module's inferred namespace.
            if name not in aliases or explicit:
                aliases[name]=[f"using:{namespace}",f"clr-namespace:{namespace};assembly={assembly}"]
    for path in framework.rust_files(str(ROOT/directory)):
        text=framework.strip_comments(framework.read(path))
        for match in re.finditer(r"pub enum (\w+)\s*\{",text):
            body=text[match.end():framework.matching(text,match.end()-1,"{","}")]
            enums[match.group(1)]=re.findall(r"(?m)^\s*([A-Z]\w*)\s*(?:,|=)",body)
        for match in re.finditer(r"ferro_class!\s*[({]\s*([A-Z]\w*)\s*:",text):
            register_aliases(match.group(1),path,"")
        for match in re.finditer(r"ferro_class_info!\s*\(\s*(\w+)\s*\{",text):
            body=text[match.end():framework.matching(text,match.end()-1,"{","}")]
            properties=extra.setdefault(match.group(1),{})
            read_markup_properties(body,properties)
            register_aliases(match.group(1),path,body)
        # Plain runtime markup classes include Setter and easing objects. They use the same
        # property table as object-model classes, but are not ferro_class! declarations.
        for match in re.finditer(r'ferro_markup_type!\s*\(\s*(?:class|value)\s+([A-Z]\w*)(?:\s+as\s+"([A-Z]\w*)")?\s*\{' ,text):
            name=match.group(2) or match.group(1)
            if name!=match.group(1) and match.group(1) in model.paths:
                model.paths[name]=model.paths[match.group(1)]
            body=text[match.end():framework.matching(text,match.end()-1,"{","}")]
            read_markup_properties(body,extra.setdefault(name,{}))
            register_aliases(name,path,body)
            # XAML also supplies IServiceProvider constructor arguments itself.
            constructors=re.search(r"\bconstructors:\s*\[",body)
            constructor_body=body[constructors.end():framework.matching(body,constructors.end()-1,"[","]")] if constructors else ""
            if re.search(r"\(\s*(?:Rc\s*<\s*dyn\s+IServiceProvider\s*>)?\s*\)\s*=>",constructor_body):
                model.creatable.add(name)
            if base:=re.search(r"\bbase:\s*((?:(?:Rc|Ref|Option)\s*<\s*)*(?:dyn\s+)?([A-Z]\w*)\s*>*)\s*,",body):
                model.bases[name]=base.group(2)
        # These framework macros emit real direct properties and derived transition classes.
        # Read their invocation arguments so timing members and new transitions follow source
        # changes automatically, rather than copying a second list of animation properties.
        for match in re.finditer(r"\bdirect_cell_property!\s*\(",text):
            end=framework.matching(text,match.end()-1,"(",")")
            args=framework.split_top_level(text[match.end():end])
            owner,_=framework.owner_at(text,match.start())
            if owner and len(args)>=3 and re.fullmatch(r'"\w+"',args[1]):
                extra.setdefault(owner,{})[json.loads(args[1])]=args[2]
        for match in re.finditer(r"\bferro_transition_class!\s*\(\s*([A-Z]\w*)\s*:",text):
            model.bases[match.group(1)]="TransitionBase"
            model.creatable.add(match.group(1))
            register_aliases(match.group(1),path,"")

types=[]
for name in sorted(set(model.bases)|model.creatable|set(extra)):
    properties={}
    for kind,prop,value_type,_,_ in model.properties.get(name,[]):
        label=pascal(prop)
        values=["True","False"] if value_type=="bool" else enums.get(value_type,[])
        properties[label]={"name":label,"type_name":value_type,"documentation":f"{name}.{label}: {value_type}","values":values,"attached":kind=="AttachedProperty"}
    for label,value_type in extra.get(name,{}).items():
        values=["True","False"] if value_type=="bool" else enums.get(value_type,[])
        properties.setdefault(label,{"name":label,"type_name":value_type,"documentation":f"{name}.{label}: {value_type}","values":values})
    for event,args in model.events.get(name,[]):
        label=pascal(event)
        properties[label]={"name":label,"type_name":args,"documentation":f"{name}.{label} routed event","event":True}
    types.append({"name":name,"namespace":"https://github.com/ferroui","documentation":model.paths.get(name,name),"base":model.bases.get(name),"constructible":name in model.creatable,"properties":properties,"namespace_aliases":aliases.get(name,[])})
output=pathlib.Path(__file__).with_name("schema.json")
content=json.dumps(types,ensure_ascii=False,indent=2)+"\n"
if "--check" in sys.argv:
    if not output.exists() or output.read_text()!=content:
        print("Language schema is stale",file=sys.stderr);sys.exit(1)
else:
    output.write_text(content)
print(f"{len(types)} framework types; {sum(len(t['properties']) for t in types)} declared properties/events")
