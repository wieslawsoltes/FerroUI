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
            for block in re.finditer(r"\bproperties:\s*\[",body):
                contents=body[block.end():framework.matching(body,block.end()-1,"[","]")]
                for prop in re.finditer(r"(?m)^\s*(\w+)\s*:\s*([^\n{]+)\s*\{",contents):
                    properties[prop.group(1)]=prop.group(2).strip()

types=[]
for name in sorted(set(model.bases)|model.creatable|set(extra)):
    properties={}
    for kind,prop,value_type,_,_ in model.properties.get(name,[]):
        label=pascal(prop)
        values=["True","False"] if value_type=="bool" else enums.get(value_type,[])
        properties[label]={"name":label,"type_name":value_type,"documentation":f"{name}.{label}: {value_type}","values":values,"attached":kind=="AttachedProperty"}
    for label,value_type in extra.get(name,{}).items():
        properties.setdefault(label,{"name":label,"type_name":value_type,"documentation":f"{name}.{label}: {value_type}"})
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
