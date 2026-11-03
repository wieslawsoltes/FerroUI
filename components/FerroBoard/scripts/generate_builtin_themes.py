"""Translate the pinned private palette constructors and call table (no evaluated C# or reflection)."""
from pathlib import Path
import re,json,subprocess,sys
root=Path(__file__).resolve().parents[1]
source=(root/'reference/ScreenKeyboardControl/src/ScreenKeyboard.Uno/Themes/KeyboardThemes.cs').read_text()
def snake(s): return re.sub(r'(?<!^)([A-Z])',r'_\1',s).lower()
def expression(s):
    s=s.strip()
    if '?' in s and '??' not in s:
        c,rest=s.split('?',1);a,b=rest.split(':',1)
        tail=expression(b)
        return f'if {snake(c.strip())} {{ {expression(a)} }} else '+(tail if tail.startswith('if ') else '{ '+tail+' }')
    if s.startswith('Hex('):
        inner=s[4:-1]
        if '??' in inner:
            a,b=inner.split('??');inner=f'{a.strip()}.unwrap_or({b.strip()})'
        return 'hex('+inner+')'
    if s.startswith('double.Parse('):return 'corner.parse::<f64>().expect("pinned theme corner")'
    if re.fullmatch(r'\d+',s):return s+'.0'
    return s
code='// Generated from Themes/KeyboardThemes.cs private palette algorithms and call table.\n// See ../NOTICE.md. Regenerate with scripts/generate_builtin_themes.py.\n'
signatures={
 'Floris':'id: &str, name: &str, dark: bool, borderless: bool, primary: &str, primary_variant: &str, secondary: &str, background: &str, surface: &str, surface_variant: &str, popup_surface: &str, focused_popup: &str, on_background: &str, on_surface: &str, on_surface_variant: &str, one_hand_bg: &str, one_hand_fg: &str',
 'Custom':'id: &str, name: &str, dark: bool, background: &str, key: &str, key_pressed: &str, key_fg: &str, function: &str, function_pressed: &str, accent: &str, accent_pressed: &str, accent_fg: &str, hint: &str, popup: &str, popup_selected: &str, trail: &str, corner: &str, font: Option<&str>, secondary: Option<&str>',
}
for name,signature in signatures.items():
    body=source[source.index('private static KeyboardTheme '+name+'('):].split('return t;')[0]
    initializer=body[body.index('var t = new KeyboardTheme'):]
    assignments=re.findall(r'^\s+(\w+) = (.+),$',initializer,re.M)
    assert len(assignments)==(35 if name=='Floris' else 37),len(assignments)
    code+='#[allow(clippy::too_many_arguments)]\nfn '+name.lower()+'('+signature+') -> Arc<KeyboardTheme> {\nlet t=KeyboardTheme::new();\n'
    for prop,value in assignments:
        for param in ['primaryVariant','surfaceVariant','popupSurface','focusedPopup','onBackground','onSurface','onSurfaceVariant','oneHandBg','oneHandFg','keyPressed','keyFg','functionPressed','accentPressed','accentFg','popupSelected']:
            value=re.sub(r'\b'+param+r'\b',snake(param),value)
        exp=expression(value)
        if prop in ['Id','Name']:exp=f'Some({exp}.into())'
        if prop=='FontFamily':exp='font.map(Into::into)'
        code+='t.set_'+snake(prop)+'('+exp+');\n'
    code+='t\n}\n'
calls=re.findall(r'yield return (Floris|Custom)\((.*?)\);',source,re.S)
assert len(calls)==12
code+='fn create_built_in() -> Vec<Arc<KeyboardTheme>> { vec![\n'
for name,args in calls:
    tokens=re.findall(r'"[^"]*"|\btrue\b|\bfalse\b|\b(?:corner|font|secondary)\s*:',args)
    values=[];named={};i=0
    while i<len(tokens):
        if tokens[i].endswith(':'):named[tokens[i].split(':')[0].strip()]=tokens[i+1];i+=2
        else:values.append(tokens[i]);i+=1
    if name=='Custom':values += [named.get('corner','"8"'), 'Some('+named['font']+')' if 'font' in named else 'None','Some('+named['secondary']+')' if 'secondary' in named else 'None']
    assert len(values)==(17 if name=='Floris' else 19),(name,values)
    code+=name.lower()+'('+','.join(values)+'),\n'
code+='] }\n'
code=subprocess.run(['rustfmt','--edition','2024','--emit','stdout'],input=code,text=True,capture_output=True,check=True).stdout
path=root/'src/FerroBoard.Controls/themes/builtin_themes.rs'
if '--check' in sys.argv:assert path.read_text()==code,path
else:path.write_text(code)
print('12 built-in themes and source palette constructor algorithms verified')
