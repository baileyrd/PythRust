"""Bounded #808 harness tests; no private data, network or release build needed."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import profile_workloads as p
import verify_profile_results as verifier


class HarnessTests(unittest.TestCase):
    def test_stats_and_invalid_samples(self):
        result = p.stats([1, 2, 3, 9])
        self.assertEqual(result['median'], 2.5)
        self.assertEqual(result['mad'], 1)
        self.assertEqual(result['samples'], [1, 2, 3, 9])
        for bad in ([], [0], [-1], [float('nan')], [float('inf')]):
            with self.assertRaises(ValueError):
                p.stats(bad)

    def test_failed_run_and_wrong_or_unstable_output_fail(self):
        with self.assertRaises(RuntimeError):
            p.run([sys.executable,'-c','raise SystemExit(7)'])
        for stdout,stderr in ((b'wrong',b''),(b'ok',b'unexpected')):
            with patch.object(p,'run',return_value=subprocess.CompletedProcess([],0,stdout,stderr)):
                with self.assertRaises(ValueError):
                    p.measure(['unused'],3,b'ok')
        sequence = [subprocess.CompletedProcess([],0,x,b'') for x in (b'ok',b'ok',b'wrong')]
        with patch.object(p,'run',side_effect=sequence):
            with self.assertRaises(ValueError):
                p.measure(['unused'],3,b'ok',warmups=0)
        with self.assertRaises(ValueError):
            p.measure(['unused'],0,b'')

    def test_strict_counts(self):
        keys = ('heap_allocations heap_reallocations requested_bytes copy_bytes str_inc str_dec '
                'seq_inc seq_dec seq_get seq_get_held seq_item_held seq_hold submit').split()
        raw = ''.join(f'{key}=1\n' for key in keys).encode()
        self.assertEqual(sum(p.parse_counts(raw).values()),13)
        for bad in (raw+b'submit=1\n',raw.replace(b'submit=1\n',b''),raw+b'extra=1\n',
                    raw.replace(b'submit=1',b'submit=-1'),raw.replace(b'submit=1',b'submit=NaN'),b'\xff'):
            with self.assertRaises(ValueError):
                p.parse_counts(bad)

    def test_fixture_determinism_shapes_and_group_oracle(self):
        with tempfile.TemporaryDirectory(prefix='pythrust-808-test-') as tmp:
            for dup,unique in ((True,False),(False,False),(False,True)):
                root=Path(tmp)/f'{dup}-{unique}'
                manifest=p.generate_tree(root,count=8,size=17,duplicates=dup,nested=True,unique_sizes=unique)
                again=p.generate_tree(Path(tmp)/f'copy-{dup}-{unique}',count=8,size=17,duplicates=dup,nested=True,unique_sizes=unique)
                self.assertEqual(manifest,again)
                for row in manifest:
                    data=(root/row['path']).read_bytes()
                    self.assertEqual(len(data),row['bytes'])
                    self.assertEqual(hashlib.sha256(data).hexdigest(),row['sha256'])
                got=p.run([sys.executable,p.ROOT/'apps/fclone-lite/fclone_lite.py',root]).stdout
                tasks=p.run([sys.executable,p.ROOT/'apps/fclone-lite/fclone_tasks.py',root]).stdout
                self.assertEqual(got,p.expected_groups(root,manifest))
                self.assertEqual(got,tasks)
                self.assertEqual(bool(got),dup)
                with self.assertRaises(FileExistsError):
                    p.generate_tree(root,count=1,size=1,duplicates=False)
                for phase in ('discover','read','loaded_sha256'):
                    src=Path(tmp)/'phase.py'
                    src.write_text(p.phase_source(phase,2))
                    out=p.run([sys.executable,src,root]).stdout
                    if phase=='discover':
                        self.assertEqual(out,f'8 {sum(x["bytes"] for x in manifest)}\n'.encode())
                    elif phase=='read':
                        self.assertEqual(out,f'{sum(x["bytes"] for x in manifest)}\n'.encode())
                    else:
                        last=sorted(manifest,key=lambda x:(x['bytes'],x['path']))[-1]['sha256']
                        self.assertEqual(out,f'1024 {last}\n'.encode())
            for count,size,dup,unique in ((0,1,False,False),(1,0,False,False),(1,1,True,True)):
                with self.assertRaises(ValueError):
                    p.generate_tree(Path(tmp)/'bad',count=count,size=size,duplicates=dup,unique_sizes=unique)

    def test_granularity_same_work_per_pair(self):
        for chunks in (8,128,8192):
            want=chunks*sum(i % 97 for i in range(3276800//chunks))
            for parallel in (False,True):
                out=p.run([sys.executable,'-c',p.granularity_source(chunks,parallel)]).stdout
                self.assertEqual(out,f'{want}\n'.encode())
        with self.assertRaises(ValueError):
            p.granularity_source(0,False)
        with self.assertRaises(ValueError):
            p.phase_source('unknown')

    def test_result_verifier_rejects_missing_and_wrong_statistics(self):
        timing = dict(wall=p.stats([1,2,3]), user_seconds=[0,0,0], system_seconds=[0,0,0],
                      context_switches_voluntary_involuntary=[[0,0]]*3)
        verifier.verify_timing(timing,3)
        with self.assertRaises(ValueError):
            verifier.verify_timing(timing,4)
        timing['wall']['median']=99
        with self.assertRaises(ValueError):
            verifier.verify_timing(timing,3)
        with self.assertRaises(ValueError):
            verifier.verify(dict(schema=1,environment=dict(runs=3),cases={}))

    def test_missing_profile_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(p,'run',return_value=subprocess.CompletedProcess([],0,b'',b'')):
                with self.assertRaises(ValueError):
                    p.profile('unused',[],b'',Path(tmp),'0',3)


if __name__ == '__main__':
    unittest.main()
