"""Fail-closed tests for the bounded interleaved SHA pilot."""
import copy
import subprocess
import unittest
from unittest.mock import patch

import sha256_pilot as s
import verify_sha256_pilot as v


class PilotTests(unittest.TestCase):
    def test_order_has_every_runtime_once_and_is_reproducible(self):
        rows=s.order(809,15,('baseline','candidate','cpython'))
        self.assertEqual(rows,s.order(809,15,('baseline','candidate','cpython')))
        self.assertGreater(len(set(map(tuple,rows))),1)
        for row in rows:
            self.assertEqual(sorted(row),['baseline','candidate','cpython'])

    def test_checker_rejects_stderr_or_wrong_stdout(self):
        for out,err in [(b'wrong',b''),(b'ok',b'warning')]:
            with patch.object(s.p,'run',return_value=subprocess.CompletedProcess([],0,out,err)):
                with self.assertRaises(ValueError):
                    s.checked(['unused'],b'ok')

    def test_interleaving_excludes_warmup_and_checks_every_run(self):
        with patch.object(s,'checked',return_value=b'ok') as checked:
            row=s.interleaved({'baseline':['a'],'candidate':['b']},b'ok','0',7,809)
        self.assertEqual(checked.call_count,16)
        v.verify_timing(row,7,{'baseline','candidate'})
        bad=copy.deepcopy(row); bad['wall']['baseline']['median']=999
        with self.assertRaises(ValueError):
            v.verify_timing(bad,7,{'baseline','candidate'})
        bad=copy.deepcopy(row); bad['order'][0]=['baseline','baseline']
        with self.assertRaises(ValueError):
            v.verify_timing(bad,7,{'baseline','candidate'})

    def test_micro_requires_success_and_exactly_one_time(self):
        for out in (b'',b'sha256_micro_seconds=1.0',b'sha256_micro_seconds=1.0\n1 passed; 0 failed\nsha256_micro_seconds=2.0'):
            with patch.object(s,'checked',return_value=out):
                with self.assertRaises(ValueError):
                    s.interleaved({'baseline':['x']},None,'0',7,809,micro=True)

    def test_independent_statistics_and_invalid_samples(self):
        v.stats(dict(samples=[1,2,3,9],median=2.5,mad=1,minimum=1,maximum=9),4)
        for xs in ([],[0],[-1],[float('nan')],[float('inf')]):
            with self.assertRaises(ValueError):
                v.stats(dict(samples=xs),len(xs))
        with self.assertRaises(ValueError):
            v.verify(dict(schema=809,runs=7,warmups=1,inputs={},affinities=[]))


if __name__=='__main__':
    unittest.main()
