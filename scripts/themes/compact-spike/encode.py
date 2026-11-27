"""Experimental backend for the generated WinUI theme. Inputs are Cargo OUT_DIR copies."""
import argparse
import re
from collections import Counter
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
STYLES = ROOT / 'src/Markup/FerroUI.Markup.Xaml/xaml_il/runtime/theme_styles.rs'

def pool_index(pool, value):
    if value not in pool:
        pool[value] = len(pool)
    return pool[value]

def array(values):
    return '&[' + ','.join(str(value) for value in values) + ']'

def encode(source, mode):
    stats = Counter()
    def words(values):
        if not all(0 <= value <= 65535 for value in values):
            raise ValueError('operand exceeds u16')
        if mode == 'fixed':
            stats['instruction_bytes'] += 2 * len(values)
            return array(values)
        stream = []
        for value in values:
            while value >= 128:
                stream.append((value & 127) | 128)
                value >>= 7
            stream.append(value)
        stats['instruction_bytes'] += len(stream)
        return array(stream)

    def theme(match):
        name, parent, body = match.groups()
        properties, strings, selectors, native = {}, {}, {}, {}
        code = []
        lines = iter(line.strip() for line in body.splitlines())
        depth = 0
        for line in lines:
            if line == 'rt::StyleMember::Style {':
                nested = re.fullmatch(r'parent: (true|false),', next(lines))[1]
                selector = re.fullmatch(r'selector: ::core::option::Option::Some\((\w+)\),', next(lines))[1]
                row = re.fullmatch(r'line: (\d+),', next(lines))[1]
                col = re.fullmatch(r'position: (\d+),', next(lines))[1]
                assert next(lines) == 'members: &['
                code += [3, int(nested == 'true'), pool_index(selectors, selector), int(row), int(col)]
                depth += 1
                stats['nested_styles'] += 1
            elif line == '],':
                assert depth > 0 and next(lines) == '},'
                code.append(0)
                depth -= 1
            elif result := re.fullmatch(r'rt::StyleMember::Setter\(rt::ThemeSetter \{ property: (\w+), form: rt::SetterForm::\w+, value: rt::ThemeValue::Dynamic\(("[^"]*")\) \}\),', line):
                prop, key = result.groups()
                code += [2, pool_index(properties, prop), pool_index(strings, key)]
                stats['dynamic_setters'] += 1
            elif result := re.fullmatch(r'rt::StyleMember::Setter\(rt::ThemeSetter \{ property: (\w+), form: rt::SetterForm::(\w+), value: rt::ThemeValue::Static \{ key: ("[^"]*"), line: (\d+), position: (\d+) \} \}\),', line):
                prop, form, key, row, col = result.groups()
                code += [4, pool_index(properties, prop), pool_index(strings, key),
                         ['Plain','Created','Parented'].index(form), int(row), int(col)]
                stats['static_setters'] += 1
            else:
                assert line.startswith('rt::StyleMember::') and line.endswith(','), line
                code += [1, pool_index(native, line[:-1])]
                stats['native_members'] += 1
        assert depth == 0
        stats['theme_programs'] += 1
        return (f'static {name}: compact_spike::Program = compact_spike::Program {{ parent: {parent}, '
                f'words: {words(code + [0])}, properties: {array(properties)}, strings: {array(strings)}, '
                f'selectors: {array(selectors)}, native: {array(native)} }};')

    source = re.sub(r'static (\w+): rt::ControlThemeTable = rt::ControlThemeTable \{\n    parent: (true|false),\n    members: &\[\n(.*?)\n    \],\n\};', theme, source, flags=re.S)
    source, calls = re.subn(r'rt::build_control_theme\(&context, &(\w+)\)', r'compact_spike::build(&context, &\1)', source)
    assert calls == stats['theme_programs'] > 0

    def bindings(match):
        name, body = match.groups()
        props, code = {}, []
        for line in body.splitlines():
            target, src, row, col = re.fullmatch(r'\s*rt::TemplateBindingRow \{ target: (\w+), source: (\w+), line: (\d+), position: (\d+) \},', line).groups()
            code += [pool_index(props, target), pool_index(props, src), int(row), int(col)]
            stats['template_bindings'] += 1
        stats['binding_runs'] += 1
        return f'static {name}: compact_spike::Bindings = compact_spike::Bindings {{ properties: {array(props)}, words: {words(code)} }};'

    source = re.sub(r'static (\w+): &\[rt::TemplateBindingRow\] = &\[\n(.*?)\n\];', bindings, source, flags=re.S)
    source, calls = re.subn(r'rt::bind_template_bindings\(([^\n,]+), (\w+)\)', r'compact_spike::bind_rows(\1, &\2)', source)
    assert calls == stats['binding_runs'] > 0
    def aliases(match):
        name, body = match.groups()
        keys, code, scopes = {}, [], set()
        for line in body.splitlines():
            row = re.fullmatch(r'\s*rt::ResourceAliasRow \{ key: ("[^"]*"), target: ("[^"]*"), namespaces: (.*?), line: (\d+), position: (\d+), value_line: (\d+), value_position: (\d+) \},', line)
            key, target, scope, ln, col, value_ln, value_col = row.groups()
            assert (ln, col) == (value_ln, value_col)
            scopes.add(scope)
            code += [pool_index(keys, key), pool_index(keys, target), int(ln), int(col)]
        assert len(scopes) == 1
        count = len(code) // 4
        stats['resource_aliases'] += count
        stats['alias_runs'] += 1
        return (f'static {name}: compact_spike::Aliases = compact_spike::Aliases {{ strings: {array(keys)}, '
                f'words: {words(code)}, count: {count}, namespaces: {scopes.pop()}, cache: std::sync::OnceLock::new() }};')

    source = re.sub(r'static (\w+): &\[rt::ResourceAliasRow\] = &\[\n(.*?)\n\];', aliases, source, flags=re.S)
    source, calls = re.subn(r'(rt::add_resource_aliases\([^\n]+, )(\w+)\)', r'\1\2.rows())', source)
    assert calls == stats['alias_runs'] > 0
    runtime = (HERE / 'runtime.rs').read_text()
    if mode == 'packed':
        runtime = runtime.replace('usize::from(*value)', '''let mut result = usize::from(value & 127);
        let mut shift = 7;
        let mut byte = *value;
        while byte >= 128 {
            let (next, rest) = self.0.split_first().expect("truncated operand");
            self.0 = rest;
            byte = *next;
            result |= usize::from(byte & 127) << shift;
            shift += 7;
        }
        result''')
    # Reuse the production materializers verbatim. Only the instruction reader varies.
    helpers = STYLES.read_text()
    based_on = helpers[helpers.index('fn based_on('):helpers.index('/// Adds the setter or the nested style')]
    setters = helpers[helpers.index('fn add_theme_setter('):helpers.index('// --- the setter of a resource value')]
    word = 'u8' if mode == 'packed' else 'u16'
    return source + f'\nmod compact_spike {{\ntype Word = {word};\n' + runtime + based_on + setters + '\n}\n', stats

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['fixed','packed'])
    parser.add_argument('input', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    result, stats = encode(args.input.read_text(), args.mode)
    args.output.write_text(result)
    print(dict(stats))
