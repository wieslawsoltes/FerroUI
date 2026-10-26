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
        while match:=re.search(r"\b([A-Z]\w*)\s*:\s*([^{}]+?)\s*\{",contents[position:]):
            start=position+match.end()-1
            end=framework.matching(contents,start,"{","}")
            accessor=contents[start+1:end]
            properties[match.group(1)]=(match.group(2).strip(),not re.search(r"\b(?:set|try_set)\s*:",accessor))
            position=end+1

enums={}
extra={}
for directory,_ in crates:
    for path in framework.rust_files(str(ROOT/directory)):
        text=framework.strip_comments(framework.read(path))
        for match in re.finditer(r"pub enum (\w+)\s*\{",text):
            body=text[match.end():framework.matching(text,match.end()-1,"{","}")]
            enums[match.group(1)]=re.findall(r"(?m)^\s*([A-Z]\w*)\s*(?:,|=)",body)
        for match in re.finditer(r"ferro_class_info!\s*\(\s*(\w+)\s*\{",text):
            body=text[match.end():framework.matching(text,match.end()-1,"{","}")]
            properties=extra.setdefault(match.group(1),{})
            read_markup_properties(body,properties)
        # Plain runtime markup classes include Setter and easing objects. They use the same
        # property table as object-model classes, but are not ferro_class! declarations.
        for match in re.finditer(r"ferro_markup_type!\s*\(\s*(?:class|value)\s+([A-Z]\w*)\s*\{",text):
            name=match.group(1)
            body=text[match.end():framework.matching(text,match.end()-1,"{","}")]
            read_markup_properties(body,extra.setdefault(name,{}))
            if re.search(r"\bconstructors:\s*\[\s*\(\s*\)\s*=>",body):
                model.creatable.add(name)
            if base:=re.search(r"\bbase:\s*([A-Z]\w*)\s*,",body):
                model.bases[name]=base.group(1)
        # These framework macros emit real direct properties and derived transition classes.
        # Read their invocation arguments so timing members and new transitions follow source
        # changes automatically, rather than copying a second list of animation properties.
        for match in re.finditer(r"\bdirect_cell_property!\s*\(",text):
            end=framework.matching(text,match.end()-1,"(",")")
            args=framework.split_top_level(text[match.end():end])
            owner,_=framework.owner_at(text,match.start())
            if owner and len(args)>=3 and re.fullmatch(r'"\w+"',args[1]):
                extra.setdefault(owner,{})[json.loads(args[1])]=(args[2],False)
        for match in re.finditer(r"\bferro_transition_class!\s*\(\s*([A-Z]\w*)\s*:",text):
            model.bases[match.group(1)]="TransitionBase"
            model.creatable.add(match.group(1))

types=[]
for name in sorted(set(model.bases)|model.creatable|set(extra)):
    properties={}
    for kind,prop,value_type,_,_ in model.properties.get(name,[]):
        label=pascal(prop)
        values=["True","False"] if value_type=="bool" else enums.get(value_type,[])
        properties[label]={"name":label,"type_name":value_type,"documentation":f"{name}.{label}: {value_type}","values":values,"attached":kind=="AttachedProperty"}
    for label,(value_type,read_only) in extra.get(name,{}).items():
        values=["True","False"] if value_type=="bool" else enums.get(value_type,[])
        properties.setdefault(label,{"name":label,"type_name":value_type,"documentation":f"{name}.{label}: {value_type}","values":values,"read_only":read_only})
    for event,args in model.events.get(name,[]):
        label=pascal(event)
        properties[label]={"name":label,"type_name":args,"documentation":f"{name}.{label} routed event","event":True}
    types.append({"name":name,"namespace":"https://github.com/ferroui","documentation":model.paths.get(name,name),"base":model.bases.get(name),"constructible":name in model.creatable,"properties":properties})
output=pathlib.Path(__file__).with_name("schema.json")
content=json.dumps(types,ensure_ascii=False,indent=2)+"\n"
if "--check" in sys.argv:
    if not output.exists() or output.read_text()!=content:
        print("Language schema is stale",file=sys.stderr);sys.exit(1)
else:
    output.write_text(content)
print(f"{len(types)} framework types; {sum(len(t['properties']) for t in types)} declared properties/events")
