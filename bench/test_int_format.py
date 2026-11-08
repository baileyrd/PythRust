"""#811 evidence rejects omissions, malformed resources and stale identities."""
import copy
import json
from pathlib import Path
import unittest

import int_format as pilot
import profile_workloads as p


class EvidenceTests(unittest.TestCase):
    def valid(self):
        item = dict(path='/example/binary', sha256='a' * 64)
        record = dict(commit='b' * 40, tree='c' * 40, source_diff='', compiler=item,
                      archive=item, cases={name: dict(source=source, source_sha256=p.sha(source.encode()),
                                                    stdout='ok\n', binary=item)
                                          for name, source in pilot.sources().items()})
        result = dict(schema=1, runs=7, identities={label: copy.deepcopy(record) for label in pilot.LABELS}, cases={})
        for name in pilot.NAMES:
            for cpus in (1, 4):
                result['cases'][f'{name}/{cpus}'] = dict(affinity=list(range(cpus)), stdout='ok\n',
                    order=[['before', 'after']] * 7,
                    resources={label: [dict(user=0.5, system=0.0, voluntary=0, involuntary=1) for _ in range(7)] for label in pilot.LABELS},
                    **{label: p.stats([1., 2., 3., 4., 5., 6., 7.]) for label in pilot.LABELS})
        return result

    def reject(self, result):
        with self.assertRaises((ValueError, KeyError, TypeError)):
            pilot.verify(result)

    def test_valid_recomputes(self):
        self.assertEqual(pilot.verify(self.valid()), 8)

    def test_exact_before_after_identities_required(self):
        for field in ('before', 'after'):
            bad = self.valid()
            del bad['identities'][field]
            self.reject(bad)
        bad = self.valid()
        bad['identities'] = {}
        self.reject(bad)
        for field in ('commit', 'tree', 'source_diff', 'compiler', 'archive', 'cases'):
            bad = self.valid()
            del bad['identities']['after'][field]
            self.reject(bad)
        for mutation in ('source', 'source_sha256', 'binary'):
            bad = self.valid()
            del bad['identities']['after']['cases']['strings'][mutation]
            self.reject(bad)
        bad = self.valid()
        bad['identities']['after']['compiler']['sha256'] = 'z' * 64
        self.reject(bad)

    def test_every_resource_field_required_in_both_conditions(self):
        for label in pilot.LABELS:
            for field in pilot.FIELDS:
                bad = self.valid()
                del bad['cases']['strings/1']['resources'][label][0][field]
                self.reject(bad)
            for field in ('user', 'system'):
                for value in (-1, float('nan'), float('inf'), None, '0', True):
                    bad = self.valid()
                    bad['cases']['strings/1']['resources'][label][0][field] = value
                    self.reject(bad)
            for field in ('voluntary', 'involuntary'):
                for value in (-1, 1.5, 0.0, None, '0', True):
                    bad = self.valid()
                    bad['cases']['strings/1']['resources'][label][0][field] = value
                    self.reject(bad)

    def test_missing_matrix_order_samples_and_changed_output_rejected(self):
        for field in ('matrix', 'order', 'resources', 'statistics', 'affinity', 'stdout'):
            bad = self.valid()
            row = bad['cases']['strings/4']
            if field == 'matrix':
                del bad['cases']['strings/4']
            elif field == 'order':
                row['order'][0] = ['before', 'before']
            elif field == 'resources':
                row['resources']['before'].pop()
            elif field == 'statistics':
                row['after']['mad'] = 99
            elif field == 'affinity':
                row['affinity'] = [0]
            else:
                row['stdout'] = 'wrong'
            self.reject(bad)

    def test_unchanged_workloads_and_focused_extremes(self):
        for name in pilot.NAMES[:3]:
            self.assertEqual(pilot.sources()[name], (pilot.ROOT / 'bench' / (name + '.pyr')).read_text())
        focused = pilot.sources()['int_format']
        self.assertIn('-9223372036854775808', focused)
        self.assertIn('9223372036854775807', focused)

    def test_committed_evidence_when_present(self):
        folder = pilot.ROOT / 'docs/benchmarks'
        for name in ('card811.json', 'card811-repeat.json'):
            if (folder / name).exists():
                self.assertEqual(pilot.verify(json.loads((folder / name).read_text())), 8)


if __name__ == '__main__':
    unittest.main()
