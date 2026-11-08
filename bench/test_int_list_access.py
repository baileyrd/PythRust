"""#810 harness tests: reject stale/incomplete evidence and malformed probes."""
import copy
import unittest
import json
from pathlib import Path
import shutil
import tempfile

import int_list_access as pilot
import profile_workloads as p


class EvidenceTests(unittest.TestCase):
    def valid(self):
        cases = {}
        for name, source in pilot.sources().items():
            cases[name] = dict(source=source, source_sha256=p.sha(source.encode()),
                               order=[['after', 'cpython', 'before']] * 7,
                               **{label: p.stats([1, 2, 3, 4, 5, 6, 7])
                                  for label in ('before', 'after', 'cpython')})
        return dict(schema=1, runs=7, cases=cases)

    def test_repeat_evidence_rejects_missing_order_resources_or_statistics(self):
        row = dict(order=[['before', 'after']] * 7,
                   resources={label: [{}] * 7 for label in ('before', 'after')},
                   **{label: p.stats([1, 2, 3, 4, 5, 6, 7]) for label in ('before', 'after')})
        result = dict(runs=7, cases={name: copy.deepcopy(row) for name in pilot.NAMES})
        pilot.verify_repeat(result)
        for field in ('order', 'resources', 'statistics'):
            bad = copy.deepcopy(result)
            target = bad['cases']['sorting']
            if field == 'order':
                target['order'].pop()
            elif field == 'resources':
                target['resources']['before'].pop()
            else:
                target['after']['median'] = 99
            with self.assertRaises(ValueError):
                pilot.verify_repeat(bad)

    def test_counter_parser_rejects_missing_duplicate_and_invalid(self):
        valid = b'reads=4\nwrites=2\nholds=0\nheld_items=0\n'
        self.assertEqual(pilot.parse_counts(valid)['reads'], 4)
        for bad in (valid + b'reads=1\n', valid.replace(b'holds=0\n', b''),
                    valid.replace(b'reads=4', b'reads=-1'), valid + b'extra=1\n'):
            with self.assertRaises(ValueError):
                pilot.parse_counts(bad)

    def test_bundle_checks_real_evidence_and_rejects_wrong_access_counts(self):
        folder = pilot.ROOT / 'docs/benchmarks'
        self.assertEqual(pilot.verify_bundle(folder), 6)
        with tempfile.TemporaryDirectory() as tmp:
            copy = Path(tmp)
            for name in ('card810.json', 'card810-repeat.json', 'card810-counts.json', 'card810-identities.json'):
                shutil.copyfile(folder / name, copy / name)
            path = copy / 'card810-counts.json'
            counts = json.loads(path.read_text())
            for label in ('before', 'after'):
                counts[label]['int_access']['counts']['reads'] = 1
            path.write_text(json.dumps(counts))
            with self.assertRaises(ValueError):
                pilot.verify_bundle(copy)

    def test_correct_evidence_recomputes(self):
        self.assertEqual(pilot.verify(self.valid()), 6)

    def test_wrong_or_incomplete_evidence_fails(self):
        result = self.valid()
        bad = copy.deepcopy(result)
        bad['cases']['sorting']['after']['median'] = 99
        with self.assertRaises(ValueError):
            pilot.verify(bad)
        bad = copy.deepcopy(result)
        bad['cases']['sorting']['after']['samples'].pop()
        with self.assertRaises(ValueError):
            pilot.verify(bad)
        bad = copy.deepcopy(result)
        bad['cases']['sorting']['source'] += '# changed\n'
        with self.assertRaises(ValueError):
            pilot.verify(bad)
        bad = copy.deepcopy(result)
        bad['cases']['sorting']['order'][0] = ['before'] * 3
        with self.assertRaises(ValueError):
            pilot.verify(bad)
        bad = copy.deepcopy(result)
        del bad['cases']['sorting']
        with self.assertRaises(ValueError):
            pilot.verify(bad)
        bad = copy.deepcopy(result)
        bad['runs'] = 6
        with self.assertRaises(ValueError):
            pilot.verify(bad)

    def test_exact_unchanged_primary_and_matched_access_control(self):
        sources = pilot.sources()
        self.assertEqual(sources['sorting'], (pilot.ROOT / 'bench/sorting.pyr').read_text())
        self.assertEqual(sources['int_access'].replace('list[int]', 'list[float]').replace('-> int:', '-> float:').replace('= 7', '= 7.0')
                         .replace('= 0', '= 0.0').replace('[7]', '[7.0]'), sources['float_access'])


if __name__ == '__main__':
    unittest.main()
