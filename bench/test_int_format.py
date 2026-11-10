"""#811 evidence rejects omissions, malformed resources and stale identities."""
import copy
import json
from pathlib import Path
import unittest
import tempfile
import shutil

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

    def test_counts_require_all_fields_and_exact_allocation_mechanism(self):
        identities = self.valid()['identities']
        conversions = dict(tasks_alloc_serial=3200000, tasks_alloc=3200000, strings=2000000, int_format=3400000)
        cases = {name: dict(stdout='ok\n', binary=dict(path='/example/probe', sha256='d'*64),
                 source_sha256=p.sha(pilot.sources()[name].encode()), affinity=[0, 1, 2, 3], counts=dict(heap_allocations=2*n, heap_frees=2*n,
                 heap_reallocations=0, copy_bytes=0, conversions=n, decimal_allocations=2*n,
                 decimal_reallocations=0, decimal_copy_bytes=0)) for name, n in conversions.items()}
        cases['failures'] = {str(i): dict(returncode=-6, stdout='', stderr='memory allocation failed') for i in (1, 2)}
        result = {label: copy.deepcopy(cases) for label in pilot.LABELS}
        self.assertEqual(pilot.verify_counts(result, identities), 4)
        for key in cases['strings']['counts']:
            bad = copy.deepcopy(result)
            del bad['after']['strings']['counts'][key]
            with self.assertRaises(ValueError):
                pilot.verify_counts(bad, identities)
        for value in (1, -1, True, 0.0):
            bad = copy.deepcopy(result)
            bad['after']['strings']['counts']['decimal_reallocations'] = value
            with self.assertRaises(ValueError):
                pilot.verify_counts(bad, identities)

    def test_committed_bundle(self):
        self.assertEqual(pilot.verify_bundle(pilot.ROOT / 'docs/benchmarks'), 18)

    def test_bundle_rejects_omitted_and_different_snapshot(self):
        folder = pilot.ROOT / 'docs/benchmarks'
        with tempfile.TemporaryDirectory() as temp:
            target = Path(temp)
            for name in ('card811.json', 'card811-repeat.json', 'card811-numeric.json', 'card811-counts.json'):
                shutil.copyfile(folder / name, target / name)
            path = target / 'card811.json'
            valid = json.loads(path.read_text())
            for mutation in ('missing', 'different'):
                bad = copy.deepcopy(valid)
                if mutation == 'missing':
                    bad['identities'] = {}
                else:
                    bad['identities']['before']['commit'] = '0' * 40
                path.write_text(json.dumps(bad))
                with self.assertRaises(ValueError):
                    pilot.verify_bundle(target)

    def test_unexpected_control_matrix_rejected(self):
        bad = self.valid()
        bad['workloads'] = ['strings']
        self.reject(bad)



if __name__ == '__main__':
    unittest.main()
