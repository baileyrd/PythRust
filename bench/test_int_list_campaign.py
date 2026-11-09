"""Acceptance mutations of the complete #815 evidence, including #810 gaps."""
import copy
import json
import hashlib
from pathlib import Path
import unittest
import int_list_campaign as c


class CampaignEvidence(unittest.TestCase):
    def test_saved_evidence_and_corruptions(self):
        folder=Path(__file__).resolve().parents[1]/'docs/benchmarks'
        for path in sorted(folder.glob('card815-pair*.json')):
            with self.subTest(path=path.name):
                valid=json.loads(path.read_text())
                self.assertEqual(c.verify(valid),10)
                mutations = [
                    lambda x: x['cases']['sorting']['samples'].pop(),
                    lambda x: x['cases']['float_access']['warmup_samples'].pop(),
                    lambda x: x['cases']['int_access']['summary']['candidate'].update(median=99),
                    lambda x: x['prepared']['candidate']['cases']['float_access'].update(command=['/bin/true']),
                    lambda x: x['prepared']['candidate']['cases']['int_access'].update(command=x['prepared']['baseline']['cases']['int_access']['command']),
                    lambda x: x['prepared']['candidate']['build_command'].append('--debug'),
                    lambda x: x['prepared']['candidate']['source']['build_inputs'].update({'new.rs':'0'*64}),
                    lambda x: x['prepared']['candidate']['cases']['int_access']['build_output'].update(sha256='0'*64),
                    lambda x: x['prepared']['candidate']['cases']['int_access'].update(expected='false\n'),
                    lambda x: x['prepared']['candidate']['refused'].pop('dyn'),
                    lambda x: x['prepared']['candidate']['environment'].update(RUSTFLAGS='-C opt-level=0'),
                ]
                for mutate in mutations:
                    bad=copy.deepcopy(valid); mutate(bad)
                    with self.assertRaises(ValueError): c.verify(bad)
        self.assertTrue(list(folder.glob('card815-pair*.json')), 'missing #815 evidence')
        state=json.loads((folder/'card815-validation.json').read_text())
        for item in state['log_artifacts'].values():
            self.assertEqual(hashlib.sha256((folder/item['path']).read_bytes()).hexdigest(),item['sha256'])
        code=json.loads((folder/'card815-code.json').read_text())
        for rows in code.values():
            for row in rows.values():
                item=row['disassembly']
                path=folder/'card815-code'/Path(item['path']).name
                self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(),item['sha256'])


    def test_probes_reject_count_and_provenance_corruption(self):
        import int_list_probes as probes
        path=Path(__file__).resolve().parents[1]/'docs/benchmarks/card815-probes.json'
        valid=json.loads(path.read_text())
        self.assertEqual(probes.verify(valid),8)
        for field in ('raw','count_build','count_cc','profile_command','gprof_command','pg_linker_source','profile_samples'):
            bad=copy.deepcopy(valid); row=bad['cases']['candidate']['int_access']
            row[field]='corrupt' if type(row[field]) is str else []
            with self.assertRaises((ValueError,TypeError)):probes.verify(bad)

    def test_identical_path_control_rejects_relabelled_or_incomplete_data(self):
        import int_list_path_control as control
        path=Path(__file__).resolve().parents[1]/'docs/benchmarks/card815-path-control.json'
        valid=json.loads(path.read_text());self.assertEqual(control.verify(valid),6)
        mutations=[
            lambda x:x['timings']['comparison']['samples'].pop(),
            lambda x:x['cases']['strings'].update(byte_identical=not x['cases']['strings']['byte_identical']),
            lambda x:x['cases']['float_access']['conditions']['candidate'].update(command=['/bin/true']),
            lambda x:x['timings']['self']['summary']['candidate'].update(median=99),
        ]
        for mutate in mutations:
            bad=copy.deepcopy(valid);mutate(bad)
            with self.assertRaises(ValueError):control.verify(bad)

    def test_matched_controls_and_monorphic_generic_inputs(self):
        s=c.sources()
        self.assertEqual(s['generic_int'],s['int_access'].replace('xs: list[int]','xs').replace(' -> int',''))
        self.assertEqual(s['generic_float'],s['float_access'].replace('xs: list[float]','xs').replace(' -> float',''))
        for ty in ('int','float'):
            self.assertIn('ex.submit(read, xs)',s['task_'+ty])
            self.assertNotIn('xs[k] =',s['task_'+ty])


if __name__=='__main__': unittest.main()
