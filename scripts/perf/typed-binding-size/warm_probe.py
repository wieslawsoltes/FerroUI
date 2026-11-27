"""Extend the existing two-way timing probe with sustained warmup and a longer measured window."""
from pathlib import Path

def source(original):
    text = Path(original).read_text()
    point = '    assert_eq!(model.number(), 13);'
    assert text.count(point) == 1 and text.count('50_000') == 4 and text.count('49_999') == 2
    return text.replace('50_000', '1_000_000').replace('49_999', '999_999').replace(point, point + '''
    for i in 0..2_000_000 {
        model.write(black_box(i));
        target.set_value(property, black_box(!i));
    }''')
