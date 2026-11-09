"""Reject incomplete/cross-linked/corrupted #814 evidence; no timing in CI."""
import copy
import json
import unittest
from pathlib import Path
import division_campaign as c
import verify_division_campaign as v

ROOT=Path(__file__).resolve().parent.parent/'docs/benchmarks'


def load(name):
    return json.loads((ROOT/('card814-'+name+'.json')).read_text())


class CampaignTests(unittest.TestCase):
    def test_committed_campaign(self):
        self.assertEqual(v.verify_bundle(load('baseline'),load('pair'),load('profiles-base'),load('profiles-candidate')),4)
        self.assertEqual(v.verify_controls(load('controls')),4)
        after=load('after');v.t.verify(after)
        for key in ('compiler','runtime'):
            self.assertEqual(after['prepared'][key],load('pair')['prepared']['candidate'][key])
        self.assertEqual(after['prepared']['source']['build_inputs'],load('pair')['prepared']['candidate']['source']['build_inputs'])
        self.assertEqual(c.verify_pair(load('repeat')),5)
        self.assertEqual(v.verify_path_control(load('path-control')),2)
        self.assertEqual(v.verify_probe_provenance(load('probe-provenance'),[load('profiles-base'),load('profiles-candidate')]),8)

    def reject(self,original,mutations,verify):
        for change in mutations:
            bad=copy.deepcopy(original);change(bad)
            with self.assertRaises((ValueError,KeyError,TypeError)):
                verify(bad)

    def test_baseline_identity_environment_input_and_warmups(self):
        self.reject(load('baseline'),[
            lambda d:d['prepared'].pop('environment'),
            lambda d:d['prepared']['environment'].pop('PYTHONHASHSEED'),
            lambda d:d['prepared']['environment'].update(LD_PRELOAD='probe.so'),
            lambda d:d['prepared']['source'].pop('build_inputs'),
            lambda d:d['prepared']['source']['build_inputs'].pop('Cargo.lock'),
            lambda d:d['prepared']['source'].update(index_diff='unrecorded staging'),
            lambda d:d['prepared'].update(cpu=-1),
            lambda d:d['prepared'].update(cpu=True),
            lambda d:d['prepared']['host'].update(permitted_affinity=['0']),
            lambda d:d['prepared'].update(domain='{}'),
            lambda d:d['prepared']['sources']['numeric'].update(rust_source='wrong algorithm'),
            lambda d:d['post_identities']['compiler'].update(sha256='0'*64),
            lambda d:d['cases']['numeric/primary']['warmup_samples'].pop(),
            lambda d:d['cases']['numeric/primary']['warmup_samples'][0].update(returncode=-6),
        ],v.verify_baseline)

    def test_pair_resources_seed_identity_fallback_and_statistics(self):
        self.reject(load('pair'),[
            lambda d:d.update(seed=0),
            lambda d:d['cases']['numeric']['samples'].pop(),
            lambda d:d['cases']['numeric']['warmup_samples'].pop(),
            lambda d:d['cases']['numeric']['samples'][0].pop('user_seconds'),
            lambda d:d['cases']['numeric']['samples'][0].update(returncode=-6),
            lambda d:d['cases']['numeric']['samples'][0].update(stderr='probe'),
            lambda d:d['cases']['numeric']['samples'][0].update(switches=[-1,0]),
            lambda d:d['cases']['numeric']['samples'][0].update(system_seconds=float('nan')),
            lambda d:d['cases']['numeric']['summary']['candidate'].update(median=0),
            lambda d:d['prepared']['candidate']['source']['build_inputs'].update({'Cargo.lock':'0'*64}),
            lambda d:d['identities_after']['candidate']['compiler'].update(sha256='0'*64),
            lambda d:d['prepared']['candidate']['conditions']['numeric']['pythrust']['executable'].update(sha256='0'*64),
        ],c.verify_pair)

    def test_pair_command_and_build_input_union(self):
        self.reject(load('pair'), [
            lambda d: d['prepared']['candidate']['conditions']['numeric']['pythrust'].update(
                command=d['prepared']['baseline']['conditions']['numeric']['pythrust']['command']),
            lambda d: d['prepared']['candidate']['conditions']['numeric']['pythrust']['command'].append('extra'),
            lambda d: d['prepared']['candidate']['sources']['numeric']['pythrust'].update(path='/different.pyr'),
            lambda d: d['prepared']['candidate']['source']['build_inputs'].update({'extra.rs': '0'*64}),
            lambda d: d['prepared']['baseline']['source']['build_inputs'].update({'extra.rs': '0'*64}),
            lambda d: d['prepared']['candidate']['source']['build_inputs'].pop('Cargo.lock'),
            lambda d: d['prepared']['baseline']['source']['build_inputs'].pop('Cargo.lock'),
            lambda d: d['prepared']['candidate']['source']['build_inputs'].pop('crates/pythrust-compiler/src/codegen/mod.rs'),
        ], c.verify_pair)

    def test_profiles_require_exact_prepared_pair_and_separate_counts(self):
        prepared=load('baseline')['prepared']
        self.reject(load('profiles-base'),[
            lambda d:d.update(status='running'),
            lambda d:d['prepared']['compiler'].update(sha256='0'*64),
            lambda d:d['cases'].pop('tuples'),
            lambda d:d['cases']['strings']['counts'].update(heap_allocations=0),
            lambda d:d['cases']['strings'].update(count_returncode=-6),
            lambda d:d['cases']['strings']['profile']['wall']['samples'].pop(),
            lambda d:d['cases']['strings'].update(count_binary=d['cases']['strings']['profile_binary']),
        ],lambda d:v.verify_profiles(d,prepared))

    def test_identical_binary_negative_control_and_probe_provenance(self):
        self.reject(load('path-control'),[
            lambda d:d['cases']['strings'].update(byte_identical=False),
            lambda d:d['cases']['strings']['samples'][0].update(returncode=-6),
            lambda d:d['cases']['strings']['build_commands']['baseline'].__setitem__(2,'different.pyr'),
            lambda d:d['cases']['tuples']['samples'][0].pop('system_seconds'),
        ],v.verify_path_control)
        self.reject(load('probe-provenance'),[
            lambda d:d['base']['strings']['profile'].update(sha256='0'*64),
            lambda d:d['candidate']['tuples'].update(pg_linker_source='bad'),
        ],lambda d:v.verify_probe_provenance(d,[load('profiles-base'),load('profiles-candidate')]))

    def test_controls_preserve_types_algorithms_and_abort_status(self):
        self.reject(load('controls'),[
            lambda d:d['prepared']['cases']['int_access'].update(source='different'),
            lambda d:d['prepared']['refused']['mixed'].update(returncode=-6),
            lambda d:d['cases']['float_access']['samples'][0].update(stdout='28000000\n'),
            lambda d:d['cases']['generic_access']['samples'][0].pop('switches'),
            lambda d:d['cases']['source_inline_diagnostic']['warmup_samples'].pop(),
            lambda d:d.update(seed=1),
        ],v.verify_controls)


if __name__=='__main__': unittest.main()
