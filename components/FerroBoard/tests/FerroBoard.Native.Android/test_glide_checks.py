"""Guard the independent source comparison using actual frozen native records."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

folder = Path(__file__).parent
spec = importlib.util.spec_from_file_location('glide_checks', folder / 'run_glide_checks.py')
glide = importlib.util.module_from_spec(spec)
spec.loader.exec_module(glide)


class GlideValidationTests(unittest.TestCase):
    def setUp(self):
        fixtures = folder.parent / 'fixtures'
        self.actual = json.loads((fixtures / 'native-android-glide-capture.json').read_text())
        self.expected = json.loads((fixtures / 'native-android-glide.json').read_text())

    def test_actual_native_capture_matches_source(self):
        glide.verify_reference(self.actual, self.expected, True)

    def test_reordered_rankings_are_rejected(self):
        altered = copy.deepcopy(self.actual)
        words = altered['cases'][0]['observed_rank']
        words[1], words[2] = words[2], words[1]
        with self.assertRaisesRegex(ValueError, 'ranked candidates'):
            glide.verify_reference(altered, self.expected, True)

    def test_missing_case_is_rejected(self):
        altered = copy.deepcopy(self.expected)
        altered['cases'].pop()
        with self.assertRaisesRegex(ValueError, 'case count'):
            glide.verify_reference(self.actual, altered, True)

    def test_incorrect_post_glide_casing_or_shift_is_rejected(self):
        fixtures = folder.parent / 'fixtures'
        actual = json.loads((fixtures / 'native-glide-casing-capture.json').read_text())
        expected = json.loads((fixtures / 'native-glide-casing.json').read_text())
        glide.verify_reference(actual, expected, True)
        for key, message in [('shift_after', 'final shift'), ('committed', 'committed casing')]:
            altered = copy.deepcopy(actual)
            case = next(c for c in altered['cases'] if c['name'] == 'caps')
            case[key] = 0 if key == 'shift_after' else [104, 101, 108, 108, 111]
            with self.assertRaisesRegex(ValueError, message):
                glide.verify_reference(altered, expected, True)

    def test_lost_native_sample_is_rejected(self):
        lines = ['BEGIN\tfixture']
        for key in self.actual['layout']:
            lines.append('\t'.join(['KEY', str(key['c']), *key['r']]))
        for case in self.actual['cases']:
            for point in case['points']:
                lines.append('\t'.join(['POINT', case['name'], *point]))
            available = case['available'][:-1] if case['name'] == 'batch' else case['available']
            for point in available:
                lines.append('\t'.join(['RAW', case['name'], *point]))
            if case['observed_rank'] is not None:
                lines.append('\t'.join(['RANK', case['name'], '|'.join(
                    ','.join(map(str, word)) for word in case['observed_rank'])]))
        lines.append('END\tfixture')
        with self.assertRaisesRegex(ValueError, 'lost/reordered'):
            glide.parse_records('\n'.join(lines))

    def logical_fixture(self):
        fixtures = folder.parent / 'fixtures'
        return (json.loads((fixtures / 'native-layout-change-capture.json').read_text()),
                json.loads((fixtures / 'native-layout-change.json').read_text()))

    def logical_log(self, actual):
        lines = ['BEGIN\tfixture']
        for key in actual['layout']:
            lines.append('\t'.join(['KEY', str(key['c']), *key['r']]))
        if 'popupKey' in actual:
            popup = actual['popupKey']
            lines.append('\t'.join(['POPUP', str(popup['selectedIndex']), '|'.join(
                ','.join(map(str, word)) for word in popup['choices'])]))
            lines.append('\t'.join(['POPUPCOMMIT', ','.join(map(str, popup['committed']))]))
        for case in actual['cases']:
            name = case['name']
            if 'shift' in case:
                lines.append(f'MODE\t{name}\t{case["shift"]}')
            for key in case.get('initialLayout', []):
                lines.append('\t'.join(['LAYOUT', name, 'before', str(key['c']), *key['r']]))
            for update in case.get('layoutUpdates', []):
                lines.append(f'UPDATE\t{name}\t{update["after"]}')
                for key in update['layout']:
                    lines.append('\t'.join(['LAYOUT', name, 'after', str(key['c']), *key['r']]))
            if 'observed_transition_counts' in case:
                lines.append('\t'.join(['SAMPLES', name, *map(str, case['observed_transition_counts'])]))
            for kind, key in [('POINT', 'points'), ('RAW', 'available')]:
                for point in case[key]:
                    lines.append('\t'.join([kind, name, *point]))
            if case['observed_rank'] is not None:
                lines.append('\t'.join(['RANK', name, '|'.join(
                    ','.join(map(str, word)) for word in case['observed_rank'])]))
            if 'shift_after' in case:
                lines.append(f'AFTER\t{name}\t{case["shift_after"]}')
                lines.append('\t'.join(['COMMIT', name, ','.join(map(str, case['committed']))]))
        return '\n'.join(lines + ['END\tfixture'])

    def test_logical_transition_capture_matches_source(self):
        actual, expected = self.logical_fixture()
        self.assertEqual(glide.parse_records(self.logical_log(actual)), actual)
        glide.verify_reference(actual, expected, True)
        for key in ['initialLayout', 'layoutUpdates']:
            altered = copy.deepcopy(actual)
            case = next(c for c in altered['cases'] if c['name'] == 'logical')
            if key == 'initialLayout':
                case[key][0]['r'][0] = '0000000000000000'
            else:
                case[key][0]['after'] += 1
            with self.assertRaisesRegex(ValueError, 'logical layout transition'):
                glide.verify_reference(altered, expected, True)

    def test_cleared_native_layout_samples_are_rejected(self):
        actual, expected = self.logical_fixture()
        altered = copy.deepcopy(actual)
        case = next(c for c in altered['cases'] if c['name'] == 'logical')
        case['observed_transition_counts'][1] = 0
        with self.assertRaisesRegex(ValueError, 'retained layout samples'):
            glide.verify_reference(altered, expected, True)

    def test_changed_popup_default_choices_or_commit_are_rejected(self):
        actual, expected = self.logical_fixture()
        for key in ['selectedIndex', 'choices', 'committed']:
            altered = copy.deepcopy(actual)
            if key == 'selectedIndex':
                altered['popupKey'][key] += 1
            elif key == 'choices':
                words = altered['popupKey'][key]
                words[0], words[1] = words[1], words[0]
            else:
                altered['popupKey'][key] = [97]
            with self.assertRaisesRegex(ValueError, 'popup layout behavior'):
                glide.verify_reference(altered, expected, True)

    def test_incomplete_or_mistimed_logical_transition_is_rejected(self):
        actual, _ = self.logical_fixture()
        for corruption in ['missing_key', 'bad_index', 'missing_counts']:
            altered = copy.deepcopy(actual)
            case = next(c for c in altered['cases'] if c['name'] == 'logical')
            if corruption == 'missing_key':
                case['initialLayout'].pop()
            elif corruption == 'bad_index':
                case['layoutUpdates'][0]['after'] = len(case['points'])
            else:
                case.pop('observed_transition_counts')
            with self.assertRaisesRegex(ValueError, 'logical layout'):
                glide.parse_records(self.logical_log(altered))


if __name__ == '__main__':
    unittest.main()
