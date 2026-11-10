"""Tests for #813 measurement boundaries, ordering and evidence rejection."""
import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import three_way as b
import verify_three_way as verifier


def bundle():
    result=dict(schema=1,status='complete',prepared={'status':'prepared'},runs=6,seed=813,cases={})
    for name,expected in b.EXPECTED.items():
        passes=[('primary',list(b.KINDS))]
        if name!='startup': passes.append(('checked',['rust','rust_checked']))
        if name=='strings': passes.append(('materialized',['rust','rust_materialized']))
        for label,kinds in passes:
            orders=b.order(kinds,6,813+len(result['cases']))
            row=dict(order=orders,samples=[],summary={})
            for i,seq in enumerate(orders):
                for kind in seq:
                    row['samples'].append(dict(round=i,kind=kind,wall_seconds=1+i,user_seconds=0.1,
                                               system_seconds=0,switches=[0,1],stdout=expected,stderr='',returncode=0))
            row['summary']={k:b.stats([x['wall_seconds'] for x in row['samples'] if x['kind']==k]) for k in kinds}
            result['cases'][name+'/'+label]=row
    return result


class ThreeWayTests(unittest.TestCase):
    def test_stats_and_bad_values(self):
        self.assertEqual(b.stats([1,2,3,9]),dict(median=2.5,mad=1,minimum=1,maximum=9))
        for values in ([],[0],[-1],[float('nan')],[float('inf')],[True]):
            with self.assertRaises(ValueError): b.stats(values)

    def test_balanced_order_and_invalid_counts(self):
        rows=b.order(b.KINDS,18,813)
        self.assertEqual(rows,b.order(b.KINDS,18,813))
        for pos in range(3):
            self.assertEqual({k:sum(x[pos]==k for x in rows) for k in b.KINDS},{k:6 for k in b.KINDS})
        for n in (-1,0,1,5,7):
            with self.assertRaises(ValueError): b.order(b.KINDS,n,813)

    def test_sample_failures(self):
        for out,err in ((b'bad',b''),(b'ok',b'bad')):
            with patch.object(b,'run',return_value=subprocess.CompletedProcess([],0,out,err)):
                with self.assertRaises(ValueError): b.sample(['none'],'ok')
        with self.assertRaises(RuntimeError): b.run([sys.executable,'-c','raise SystemExit(9)'])
        with self.assertRaises(FileNotFoundError): b.run(['/does/not/exist'])
        with self.assertRaises(subprocess.TimeoutExpired):
            b.run([sys.executable,'-c','import time; time.sleep(1)'],timeout=0.01)
        with patch.object(b.os,'sched_getaffinity',return_value={2}):
            with self.assertRaises(ValueError): b.pin(0)

    def test_complete_bundle_and_corruption(self):
        original=bundle(); self.assertEqual(b.verify(original),10)
        mutations=[lambda d:d.update(status='running'),
                   lambda d:d['cases'].pop('startup/primary'),
                   lambda d:d['cases']['numeric/primary']['samples'].pop(),
                   lambda d:d['cases']['numeric/primary']['samples'].reverse(),
                   lambda d:d['cases']['numeric/primary']['samples'][0].update(stdout='bad'),
                   lambda d:d['cases']['numeric/primary']['samples'][0].update(stderr='bad'),
                   lambda d:d['cases']['numeric/primary']['samples'][0].update(returncode=2),
                   lambda d:d['cases']['numeric/primary']['samples'][0].update(user_seconds=-1),
                   lambda d:d['cases']['numeric/primary']['samples'][0].update(system_seconds=float('inf')),
                   lambda d:d['cases']['numeric/primary']['samples'][0].update(switches=[-1,0]),
                   lambda d:d['cases']['numeric/primary']['samples'][0].update(wall_seconds=float('nan')),
                   lambda d:d['cases']['numeric/primary']['summary']['rust'].update(median=42)]
        for mutate in mutations:
            bad=copy.deepcopy(original);mutate(bad)
            with self.assertRaises(ValueError): b.verify(bad)
        with self.assertRaises(json.JSONDecodeError): json.loads(json.dumps(original)[:-4])

    def test_preparation_evidence_rejects_corruption(self):
        data=bundle(); data['warmups']=2
        p=data['prepared']; p.update(cpu=0,host={'permitted_affinity':[0]},
            source={'head':'a'*40,'tree':'b'*40},sources={},conditions={},builds={},
            rust_tests={},interpreter_checks={})
        ident={'path':'/example','sha256':'a'*64}
        for key in ('compiler','runtime','harness'): p[key]=ident.copy()
        for name in b.EXPECTED:
            source='pass\n' if name=='startup' else (b.ROOT/'bench'/(name+'.pyr')).read_text()
            rust='fn main() {}'
            p['sources'][name]={k:dict(path='/example',sha256=b.digest(v.encode())) for k,v in [('pythrust',source),('rust',rust)]}
            p['sources'][name].update(pythrust_source=source,rust_source=rust)
            kinds=list(b.KINDS)
            if name!='startup':
                kinds.append('rust_checked');p['rust_tests'][name]='1 passed; 0 failed';p['interpreter_checks'][name]={}
            if name=='strings': kinds.append('rust_materialized')
            p['host'].update(python_executable='/python', python_executable_identity=dict(path='/python',sha256='a'*64))
            p['conditions'][name]={k:dict(executable=dict(path=('/python' if k=='cpython' else '/example' if k=='pythrust' else '/'+k),sha256='a'*64),
                command=(['/python','-B','/example'] if k=='cpython' else ['/example' if k=='pythrust' else '/'+k])) for k in kinds}
            for kind in ('pythrust','rust'):
                p['builds'][name+'/'+kind]=dict(command=['example'],samples=[1]*7,**b.stats([1]*7))
        self.assertEqual(verifier.verify(data),10)
        for mutate in (lambda d:d['prepared']['sources']['numeric'].update(rust_source='changed'),
                       lambda d:d['prepared'].update(cpu=3),
                       lambda d:d['prepared']['builds'].pop('numeric/rust'),
                       lambda d:d['prepared']['rust_tests'].update(numeric='failed')):
            bad=copy.deepcopy(data);mutate(bad)
            with self.assertRaises(ValueError): verifier.verify(bad)

    def test_historical_schema1_bundle(self):
        data = json.loads((b.ROOT/'docs/benchmarks/card813.json').read_text())
        self.assertEqual(verifier.verify(data), 10)

    def test_schema2_build_and_invocation_mutations(self):
        path = b.ROOT/'docs/benchmarks/card814-baseline.json'
        original = json.loads(path.read_text())
        self.assertEqual(verifier.verify(original), 10)
        mutations = [
            lambda p: p['conditions']['numeric']['pythrust'].update(command=['/different']),
            lambda p: p['conditions']['numeric']['rust'].update(command=['/different']),
            lambda p: p['conditions']['numeric']['rust']['command'].append('--extra'),
            lambda p: p['conditions']['numeric']['cpython']['command'].__setitem__(2, '/wrong.pyr'),
            lambda p: p['conditions']['numeric']['cpython']['command'].__setitem__(0, '/wrong-python'),
            lambda p: p['conditions']['numeric']['cpython']['command'].remove('-B'),
            lambda p: p['host']['python_executable_identity'].update(sha256='0'*64),
            lambda p: p['builds']['numeric/pythrust']['command'].__setitem__(0, '/wrong-compiler'),
            lambda p: p['builds']['numeric/pythrust']['command'].__setitem__(2, '/wrong.pyr'),
        ]
        cmd = original['prepared']['builds']['numeric/rust']['command']
        for index, value in enumerate(cmd):
            replacement = 'opt-level=0' if value == 'opt-level=3' else '/wrong' if index in (0, len(cmd)-1, len(cmd)-3) else 'wrong'
            mutations.append(lambda p, i=index, v=replacement: p['builds']['numeric/rust']['command'].__setitem__(i, v))
        mutations.append(lambda p: p['rust_flags'].__setitem__(2, 'opt-level=0'))
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                bad = copy.deepcopy(original); mutate(bad['prepared'])
                with self.assertRaises(ValueError): verifier.verify(bad)

    def test_cpython_alias_invocation_and_identity(self):
        with tempfile.TemporaryDirectory() as temp:
            alias = Path(temp)/'python3'
            alias.symlink_to(Path(sys.executable).resolve())
            code = ("import json,sys; sys.path.insert(0, " + repr(str(b.ROOT/'bench')) +
                    "); import three_way; print(json.dumps(three_way.host()))")
            host = json.loads(subprocess.check_output([str(alias), '-B', '-c', code]))
            self.assertEqual(host['python_executable'], str(alias))
            self.assertNotEqual(host['python_executable'], host['python_executable_identity']['path'])
            original = json.loads((b.ROOT/'docs/benchmarks/card814-baseline.json').read_text())
            p = original['prepared']
            for key in ('python_executable', 'python_executable_identity'):
                p['host'][key] = host[key]
            for name, conditions in p['conditions'].items():
                conditions['cpython']['executable'] = host['python_executable_identity'].copy()
                conditions['cpython']['command'][0] = str(alias)
            self.assertEqual(verifier.verify(original), 10)
            mutations = [
                lambda p: p['conditions']['numeric']['cpython']['command'].__setitem__(0, '/different-python'),
                lambda p: p['host'].update(python_executable='/different-python'),
                lambda p: p['conditions']['numeric']['cpython']['command'].__setitem__(2, '/different.pyr'),
                lambda p: p['conditions']['numeric']['cpython']['executable'].update(sha256='0'*64),
            ]
            for mutate in mutations:
                bad = copy.deepcopy(original); mutate(bad['prepared'])
                with self.assertRaises(ValueError): verifier.verify(bad)

    def test_recorded_control_build_modes(self):
        original = json.loads((b.ROOT/'docs/benchmarks/card814-baseline.json').read_text())
        p = original['prepared']
        for kind in ('rust_checked', 'rust_materialized'):
            flags = b.FLAGS.copy()
            if kind == 'rust_checked': flags[flags.index('overflow-checks=off')] = 'overflow-checks=on'
            else: flags += ['--cfg', 'materialize']
            item = p['conditions']['strings'][kind]
            item['build_command'] = [p['rustc_identity']['path'], *flags, p['sources']['strings']['rust']['path'], '-o', item['executable']['path']]
        p['control_build_protocol'] = 1
        # Supply the other checked commands required by new preparations.
        for name in b.EXPECTED:
            if name in ('startup', 'strings'): continue
            item = p['conditions'][name]['rust_checked']
            flags = b.FLAGS.copy(); flags[flags.index('overflow-checks=off')] = 'overflow-checks=on'
            item['build_command'] = [p['rustc_identity']['path'], *flags, p['sources'][name]['rust']['path'], '-o', item['executable']['path']]
        self.assertEqual(verifier.verify(original), 10)
        bad = copy.deepcopy(original)
        bad['prepared']['conditions']['strings']['rust_checked'].pop('build_command')
        with self.assertRaises(ValueError): verifier.verify(bad)
        for kind in ('rust_checked', 'rust_materialized'):
            for index in range(len(p['conditions']['strings'][kind]['build_command'])):
                bad = copy.deepcopy(original)
                bad['prepared']['conditions']['strings'][kind]['build_command'][index] = 'wrong'
                with self.assertRaises(ValueError): verifier.verify(bad)

    def test_identity_corruption_and_refuse_existing_output(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp); f=root/'file';f.write_text('original')
            record={k:b.identity(f) for k in ('compiler','runtime','harness')}
            record.update(sources={},conditions={},source={'build_inputs':{}})
            b.check_identities(record)
            f.write_text('changed')
            with self.assertRaises(ValueError): b.check_identities(record)
            (root/'prepared.json').write_text('{"status":"prepared"}')
            out=root/'result';out.write_text('keep')
            class Args: work=root;output=out
            with self.assertRaises(FileExistsError): b.measure(Args())
            self.assertEqual(out.read_text(),'keep')


if __name__=='__main__': unittest.main()
