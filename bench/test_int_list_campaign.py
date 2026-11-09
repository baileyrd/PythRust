"""Acceptance mutations of the complete #815 evidence, including #810 gaps."""
import copy
import json
import hashlib
from pathlib import Path
import unittest
import tempfile
from unittest import mock
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

    def test_complete_source_and_tool_inventory_review_mutations(self):
        path=Path(__file__).resolve().parents[1]/'docs/benchmarks/card815-pair24.json'
        valid=json.loads(path.read_text())
        def remove_input(x):
            for p in x['prepared'].values():p['source']['build_inputs'].pop('Cargo.lock')
        def add_input(x):
            for p in x['prepared'].values():p['source']['build_inputs']['new.rs']='0'*64
        def clear_tools(x):
            for p in x['prepared'].values():p['tools']={}
        def source_digest(x):
            x['prepared']['candidate']['source']['build_inputs']['crates/pythrust-rt/src/seq.rs']='0'*64
        def malformed_tool(x):
            for p in x['prepared'].values():p['tools']['cargo']['identity']['sha256']=''
        def partial_tools(x):
            for p in x['prepared'].values():p['tools'].pop('ld')
        for mutate in (remove_input,add_input,clear_tools,source_digest,malformed_tool,partial_tools):
            with self.subTest(mutation=mutate.__name__):
                bad=copy.deepcopy(valid);mutate(bad)
                with self.assertRaises(ValueError):c.verify(bad)

    def test_supporting_probe_review_mutations(self):
        import int_list_probes as probes
        path=Path(__file__).resolve().parents[1]/'docs/benchmarks/card815-probes.json'
        valid=json.loads(path.read_text())
        def no_profile_digest(x):x['cases']['candidate']['int_access']['profile_binary'].pop('sha256')
        def empty_gmon_digests(x):
            for g in x['cases']['candidate']['int_access']['gmon']:g['sha256']=''
        def compiler_substitution(x):x['prepared']['candidate']['compiler']['sha256']='0'*64
        def remove_profile_flags(x):
            r=x['cases']['candidate']['int_access']
            r['pg_linker_source']=r['pg_linker_source'].replace("['-pg', '-no-pie']","[]")
            r['pg_linker']['sha256']=hashlib.sha256(r['pg_linker_source'].encode()).hexdigest()
        def remove_wraps(x):
            r=x['cases']['candidate']['int_access']
            r['count_linker_source']=r['count_linker_source'].replace(", '-Wl,--wrap=pt_int_list_get'",'')
            r['count_linker']['sha256']=hashlib.sha256(r['count_linker_source'].encode()).hexdigest()
        def missing_probe_tools(x):x['tools']={}
        def malformed_object(x):x['cases']['candidate']['int_access']['count_object']['sha256']=''
        def malformed_assembly(x):x['cases']['baseline']['sorting']['assembly'].pop('path')
        def malformed_wrapper(x):x['cases']['candidate']['int_access']['count_linker'].pop('sha256')
        def wrong_tool(x):x['tools']['gprof']['identity']['sha256']='0'*64
        for mutate in (no_profile_digest,empty_gmon_digests,compiler_substitution,
                       remove_profile_flags,remove_wraps,missing_probe_tools,malformed_object,
                       malformed_assembly,malformed_wrapper,wrong_tool):
            with self.subTest(mutation=mutate.__name__):
                bad=copy.deepcopy(valid);mutate(bad)
                with self.assertRaises(ValueError):probes.verify(bad)

    def test_cross_bundle_substitutions_and_future_checkout_independence(self):
        import int_list_evidence as evidence
        folder=c.evidence_root()
        names=('pair24','pair42','probes','path-control','code','validation')
        valid=[json.loads((folder/('card815-'+n+'.json')).read_text()) for n in names]
        self.assertEqual(evidence.verify(*valid),10)
        mutations=[
            (1,lambda x:x['prepared']['candidate']['compiler'].update(sha256='0'*64)),
            (2,lambda x:x['prepared']['candidate']['runtime'].update(sha256='0'*64)),
            (2,lambda x:x['prepared']['baseline']['cases']['sorting']['source_identity'].update(sha256='0'*64)),
            (3,lambda x:x['prepared']['candidate']['compiler'].update(sha256='0'*64)),
            (3,lambda x:x['cases']['float_access']['source'].update(sha256='0'*64)),
            (4,lambda x:x['candidate']['int_access']['binary'].update(sha256='0'*64)),
            (4,lambda x:x['baseline']['float_access'].update(commands=[])),
            (5,lambda x:x['final_build_artifacts']['runtime'].update(sha256='0'*64)),
            (5,lambda x:x['final_build_artifacts']['compiler'].update(path='/tmp/unrelated')),
            (4,lambda x:x['candidate']['sorting']['disassembly'].update(path='/tmp/sorting-candidate.s')),
        ]
        for index,mutate in mutations:
            with self.subTest(bundle=names[index]):
                bad=copy.deepcopy(valid);mutate(bad[index])
                with self.assertRaises(ValueError):evidence.verify(*bad)
        # Future source edits, including workload edits, cannot invalidate this
        # campaign's frozen export and archived evidence.
        with tempfile.TemporaryDirectory() as tmp:
            with mock.patch.object(c,'sources',side_effect=AssertionError('live workload read')):
                with mock.patch.object(c.b,'ROOT',Path(tmp)):
                    self.assertEqual(evidence.verify(*valid),10)
            # A modified export is not silently adopted as a new source truth.
            raw=(folder/'card815-source-manifest.json').read_bytes()
            (Path(tmp)/'card815-source-manifest.json').write_bytes(raw+b' ')
            with mock.patch.object(c,'evidence_root',return_value=Path(tmp)):
                with self.assertRaises(ValueError):c.frozen()
        for role,version in c.frozen()['versions'].items():
            self.assertEqual(len(version['inputs']),126)
            self.assertEqual(set(version['inputs']),set(version['git_blobs']))
        self.assertEqual(c.frozen()['versions']['baseline']['commit'],c.BASE)
        self.assertEqual(c.frozen()['versions']['candidate']['commit'],c.REVIEWED)

    def test_matched_controls_and_monorphic_generic_inputs(self):
        s=c.sources()
        self.assertEqual(s['generic_int'],s['int_access'].replace('xs: list[int]','xs').replace(' -> int',''))
        self.assertEqual(s['generic_float'],s['float_access'].replace('xs: list[float]','xs').replace(' -> float',''))
        for ty in ('int','float'):
            self.assertIn('ex.submit(read, xs)',s['task_'+ty])
            self.assertNotIn('xs[k] =',s['task_'+ty])


if __name__=='__main__': unittest.main()
