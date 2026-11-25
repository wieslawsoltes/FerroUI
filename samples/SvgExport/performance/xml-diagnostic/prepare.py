"""Reconstruct the recorded XML-only diagnostic in a separate scratch directory."""

import argparse
from pathlib import Path
import hashlib
import json
import re
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--repo', type=Path, default=Path(__file__).resolve().parents[4])
parser.add_argument('--fixture', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True, help='New directory outside the repository workspace')
args = parser.parse_args()
repo = args.repo
out = args.output
out.mkdir(parents=True, exist_ok=False)
(out / 'src').mkdir()
for name in ['Cargo.toml', 'Cargo.lock']:
    (out / name).write_bytes((Path(__file__).resolve().parent / (name + '.template')).read_bytes())
(out / 'src/main.rs').write_bytes((Path(__file__).resolve().parent / 'main.rs').read_bytes())
relative = 'src/Svg/FerroUI.Svg/xml.rs'
revision = '7e2d2cd899cac0b6a1a6f887e3e5ebba15ffb2f6'
baseline = subprocess.check_output(['git', 'show', f'{revision}:{relative}'], cwd=repo, text=True)
current_revision = '447cef96e659fe5d579fd69d2d17afad2b631761'
current = subprocess.check_output(['git', 'show', f'{current_revision}:{relative}'], cwd=repo, text=True)

def function_span(source, first, following):
    start = source.index(first)
    end = source.index(following, start)
    return source[start:end]

old_escape = function_span(baseline, 'fn escape(', 'fn write_child(')
new_escape = function_span(current, 'fn escape(', 'fn write_child(')
old_rewrite = function_span(baseline, 'fn rewrite(', '/// Parse CSS URL tokens')
new_rewrite = function_span(current, 'fn rewrite(', '/// Visit CSS URL tokens')
old_map_urls = function_span(baseline, 'fn map_urls(', 'fn css_comment_end(')
variants = {
    'baseline': baseline,
    'current': current,
    'old_escape': current.replace(new_escape, old_escape, 1),
    'old_rewrite': current.replace(new_rewrite, old_rewrite, 1) + '\n' + old_map_urls,
}
for name, source in variants.items():
    (out / 'src' / f'{name}.rs').write_text(source)

fixture = args.fixture
normal = fixture.read_text()
image = re.search(r'<image\b[^>]+/>', normal).group(0)
assert 'id="r-0"' in image
assert normal.count(image) == 1
expanded = normal.replace('<defs>' + image + '</defs>', '', 1)
assert image not in expanded
for index in range(8):
    pattern = rf'<g\b[^>]*\bid="ferro-layer-{index}"[^>]*>.*?</g>'
    group = re.search(pattern, expanded).group(0)
    image_id = f'r{index}-raster'
    new_image = image.replace('id="r-0"', f'id="{image_id}"', 1)
    assert group.count('xlink:href="#r-0"') == 1
    new_group = group.replace('xlink:href="#r-0"', f'xlink:href="#{image_id}"', 1)
    expanded = expanded.replace(group, f'<defs>{new_image}</defs>{new_group}', 1)
assert expanded.count('<image ') == 8
assert expanded.count('<defs>') == 8
(out / 'raster-normal.svg').write_text(normal)
(out / 'raster-expanded.svg').write_text(expanded)
(out / 'input-metadata.json').write_text(json.dumps({
    'baseline_revision': revision,
    'fixture': str(fixture),
    'normal_bytes': len(normal.encode()),
    'expanded_bytes': len(expanded.encode()),
    'variants_sha256': {name: hashlib.sha256(source.encode()).hexdigest() for name, source in variants.items()},
    'input_sha256': {
        'normal': hashlib.sha256(normal.encode()).hexdigest(),
        'expanded': hashlib.sha256(expanded.encode()).hexdigest(),
    },
    'restoration': 'Restore one document-scope defs/image before each of the 8 semantic layer groups, using the exact embedded PNG and per-layer href. Optimizing must exactly reproduce raster-normal.svg.',
}, indent=2) + '\n')
print(f'Prepared {len(variants)} variants; normal {len(normal)} bytes; expanded {len(expanded)} bytes')
