#!/usr/bin/env python3
"""Static correspondence audit of the pinned typography source; runtime gates are separate."""
from pathlib import Path
import argparse
import re, subprocess, hashlib, json
if not __debug__:
    raise RuntimeError("Run this audit without Python's -O flag; source checks use assertions")
parser = argparse.ArgumentParser(description="Compare the pinned typography declarations and feature projection with the Rust source. This is a static audit, not a shaping or native-runtime test.")
parser.add_argument("--reference", type=Path, required=True, help="Local microsoft-ui-xaml Git repository containing the pinned objects")
args = parser.parse_args()
repo = Path(__file__).resolve().parents[2]
native = args.reference
pin='fe244ca6ac4d482712b234a9de3cf771cc6481de'
paths={'cpp':'dxaml/xcp/core/text/common/Typography.cpp','model':'dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.cs','docs':'dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.Documents.cs'}
source={k:subprocess.check_output(['git','show',pin+':'+p],cwd=native).decode() for k,p in paths.items()}
cpp=source['cpp']
typ=(repo/'src/FerroUI.Controls/documents/typography.rs').read_text()
fmt=(repo/'src/FerroUI.Controls/documents/typography_format.rs').read_text()
enums=(repo/'src/FerroUI.Base/media/font_typography.rs').read_text()
def body(s,start):
 a=s.index('{',s.index(start));depth=1;b=a+1
 while depth:
  depth+=(s[b]=='{')-(s[b]=='}');b+=1
 return s[a+1:b-1]
def snake(s):return re.sub(r'(?<!^)(?=[A-Z])','_',s).lower()
def compact(s):return re.sub(r'\s+','',s)
def tags(s):return [''.join(v) for v in re.findall(r"DWRITE_MAKE_OPENTYPE_TAG\('(.?)','(.?)','(.?)','(.?)'\)",s)]
defaults={n:v.replace('DirectUI::','') for n,v in re.findall(r'm_(?:[nf])?([A-Z]\w*)\s*=\s*([^;]+);',body(cpp,'Typography::Typography()'))}
rust_defaults=dict(re.findall(r'"(\w+)",\s*StyledPropertyOptions::new\(([^)]+)\)\.inherits\(true\)',typ))
assert defaults==rust_defaults,(defaults,rust_defaults)
assert len(defaults)==43
native_class=body(source['docs'],'public static class Typography')
native_members=dict((name,typ) for typ,name in re.findall(r'public static ([\w.]+) Attached(\w+)',native_class))
assert set(native_members)==set(defaults)
assert native_class.count('IsValueInherited = true')==43
for name,default in defaults.items():
 field=snake(name)
 native_type=native_members[name].rsplit('.',1)[-1]
 typemap={'Int32':'i32','Boolean':'bool'}
 rust_type=typemap.get(native_type,native_type)
 assert f'pub {field}: Option<{rust_type}>' in fmt
 for fragment in [f'{field}: element.get_base_value(Typography::{field}_property())',f'{field}: Some(Typography::get_{field}(element))',f'{field}: patch.{field}.or(self.{field})',f'element.clear_value(Typography::{field}_property())']:
  assert compact(fragment) in compact(fmt),fragment
 assert f'Typography::set_{field}(element, value)' in fmt
 assert f'static fn Get{name}(Ref<FerroObject>)' in typ
 assert f'static fn Set{name}(Ref<FerroObject>,' in typ
projection=body(cpp,'HRESULT TypographyGenerateFeatureList(')
rust_projection=body(fmt,'pub fn to_font_features(')
cpp_order=tags(projection)
collapsed=[]
for tag in cpp_order:
 if collapsed and tag==collapsed[-1] and tag in ('liga','clig','calt','kern'):continue
 collapsed.append(tag)
rust_order=re.findall(r'add\s*\(\s*"(.{4})",',rust_projection)
assert collapsed==rust_order,(collapsed,rust_order)
enum_count=0;choice_count=0
for enum in re.findall(r'pub enum (Font\w+)\s*\{',enums):
 native_values=dict(re.findall(r'\b(\w+)\s*=\s*(\d+)',body(source['model'],'public enum '+enum)))
 rust_values=dict(re.findall(r'\b(\w+)\s*=\s*(\d+)',body(enums,'pub enum '+enum)))
 assert native_values==rust_values,(enum,native_values,rust_values)
 enum_count+=1;choice_count+=len(native_values)
 for choice in native_values:
  if choice=='Normal':expected=[]
  else:
   case=re.search(r'case DirectUI::'+enum+r'::'+choice+r':(.*?)break;',projection,re.S)
   assert case,(enum,choice)
   expected=tags(case.group(1))
  arm=body(rust_projection,enum+'::'+choice+' =>')
  actual=re.findall(r'add\("(.{4})", 1\)',arm)
  assert expected==actual,(enum,choice,expected,actual)
assert (enum_count,choice_count)==(7,38)
for name,tag in [('AnnotationAlternates','nalt'),('StandardSwashes','swsh'),('ContextualSwashes','cswh'),('StylisticAlternates','salt')]:
 field=snake(name)
 assert re.search(r'let value = self\.'+field+r'\.unwrap_or\(0\);\s*if value != 0\s*\{\s*add\("'+tag+r'", value\);',rust_projection),name
 assert re.search(r"DWRITE_MAKE_OPENTYPE_TAG\("+','.join("'"+c+"'" for c in tag)+r"\), pProperties->m_n"+name,projection),name
for name,tag in [('StandardLigatures','liga'),('ContextualLigatures','clig'),('ContextualAlternates','calt'),('Kerning','kern')]:
 assert compact(f'add("{tag}", i32::from(self.{snake(name)}.unwrap_or(true)))') in compact(rust_projection).replace(',)', ')')
false_boolean_count=0
for name, default in defaults.items():
 field=snake(name)
 expected=f'self.{field}.unwrap_or({default})'
 if default=='false':expected='!'+expected
 elif default!='true':expected+='=='+default
 assert compact(expected) in compact(body(fmt,'pub fn is_default(')),name
 if default=='false':
  native_if=body(projection,'if (pProperties->m_f'+name+')')
  rust_if=body(rust_projection,'if self.'+field+'.unwrap_or(false)')
  assert tags(native_if)==re.findall(r'add\("(.{4})", 1\)',rust_if),name
  false_boolean_count+=1
 elif default.startswith('Font'):
  enum=default.split('::')[0]
  assert compact(f'match self.{field}.unwrap_or({enum}::Normal)') in compact(rust_projection),name
assert false_boolean_count==28
assert 'start: 0,' in rust_projection and 'end: -1,' in rust_projection
raw=body(fmt,'pub(crate) fn with_font_features(')
assert 'if self.is_default() {\n            return explicit;' in raw
assert raw.index('let features = self.to_font_features()') < raw.index('for feature in explicit.list().snapshot().iter()')
assert 'features.add(feature.clone())' in raw
out={'source_pin':pin,'source_files':paths,'source_sha256':{k:hashlib.sha256(v.encode()).hexdigest() for k,v in source.items()},'checks':{'inherited_property_names_types_defaults':len(defaults),'property_snapshot_apply_overlay_clear_families':43,'markup_accessors':86,'enum_types':enum_count,'enum_values_and_projection_arms':choice_count,'ordered_add_feature_sites':len(rust_order),'signed_selector_parameters':4,'default_on_boolean_explicit_false_projections':4,'default_off_boolean_property_tag_gates':false_boolean_count,'raw_features':'Derived first, explicit records appended in original order with complete cloned tags/values/ranges; default-only values retain original collection identity'},'conclusion':'PASS for static source comparison; no Rust compilation or runtime result implied'}
print(json.dumps(out,indent=2))
